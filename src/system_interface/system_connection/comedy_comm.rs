// Copyright (c) 2019 Decode Detroit
// Author: Patton Doyle
// Licence: GNU GPLv3
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! A module to communicate using the ComedyComm serial protocol
//!
//! # Note
//!
//! This module uses a protocol that is a limited version of the CmdMessenger
//! protocol. While this means that there is some compatibility with the
//! CmdMessenger library, this protocol is not guaranteed to be compatible and
//! may become completely incompatible in the furture.

// Import the relevant structures into the correct namespace
use super::{ItemId, ReadEvents, WriteEvent, COMM_ERROR, READ_ERROR};

// Import standard library modules and traits
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

// Import the serial module
extern crate serial;
use self::serial::prelude::*;

// Import the byteorder module for converting between types
extern crate byteorder;
use self::byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

// Define the communication constants
const FIELD_SEPARATOR: u8 = 0x2C as u8; // the divider between the three fields
const COMMAND_SEPARATOR: u8 = 0x3B as u8; // the divider between commands
const ESCAPE_CHARACTER: u8 = 0x2F as u8; // the character to escape other characters
const NULL_CHARACTER: u8 = 0x00 as u8; // the null character
const COMMAND_CHARACTER: u8 = '0' as u8; // the default command character
const ACK_CHARACTER: u8 = '1' as u8; // the default ack character
const ACK_DELAY: u64 = 200; // the longest delay to wait for an acknowledgement, in ms
const MAX_SEND_BUFFER: usize = 100; // the largest number of events allowed to pile up in the buffer

/// A structure to hold and manipulate the connection over serial
///
/// # Note
///
/// This module uses a protocol that is a limited version of the CmdMessenger
/// protocol. While this means that there is some compatibility with the
/// CmdMessenger library, this protocol is not guaranteed to be compatible and
/// may become completely incompatible in the furture.
///
pub struct ComedyComm {
    port: serial::SystemPort,          // the serial port of the connection
    buffer: Vec<u8>,                   // the current input buffer
    outgoing: Vec<(ItemId, u32, u32)>, // the outgoing event buffer
    last_ack: Option<Instant>,         // Some(instant) if we are still waiting on ack from instant
}

// Implement key functionality for the CmdMessenger structure
impl ComedyComm {
    /// A function to create a new instance of the CmdMessenger
    ///
    pub fn new(path: &PathBuf, baud: usize, polling_rate: u64) -> Option<ComedyComm> {
        // Connect to the underlying serial port
        let mut port = match serial::open(path) {
            Ok(conn) => conn,
            _ => return None,
        };

        // Try to configure the serial port
        let settings = serial::PortSettings {
            baud_rate: serial::BaudRate::from_speed(baud.clone()),
            char_size: serial::Bits8,
            parity: serial::ParityNone,
            stop_bits: serial::Stop1,
            flow_control: serial::FlowNone,
        };
        if let Err(..) = port.configure(&settings) {
            return None; // return on failure to configure
        }

        // Adjust the timeout for the serial port
        if let Err(..) = port.set_timeout(Duration::from_millis(polling_rate)) {
            return None; // return on failure to configure
        }

        // Return the new CmdMessenger instance
        Some(ComedyComm {
            port,
            buffer: Vec::new(),
            outgoing: Vec::new(),
            last_ack: None,
        })
    }

    /// A helper function to escape special characters before sending the message
    ///
    /// This function consumes the provided message and returns an escaped message.
    ///
    fn escape(message: Vec<u8>) -> Vec<u8> {
        // Create a new vector to hold the escaped message
        let mut fixed = Vec::new();

        // Look through the vector and replace any offending characters
        for character in message {
            // Escape any offending characters
            if character == FIELD_SEPARATOR
                || character == COMMAND_SEPARATOR
                || character == ESCAPE_CHARACTER
                || character == NULL_CHARACTER
            {
                fixed.push(ESCAPE_CHARACTER);
            }

            // Add the original character to the message
            fixed.push(character.clone());
        }

        // Return the completed message
        fixed
    }

    /// A helper function to write to the serial port (skip any ack checking)
    ///
    fn write_event_now(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), ()> {
        // Format the command as a byte
        let mut bytes = Vec::new();
        bytes.push(COMMAND_CHARACTER);

        // Add the three arguments to the message
        // Add the separator and convert each argument to a character vector
        bytes.push(FIELD_SEPARATOR);
        let mut tmp = Vec::new();
        if let Err(_) = tmp.write_u32::<LittleEndian>(id.id()) {
            return Err(());
        }

        // Escape the new argument and then add it
        if let Err(_) = bytes.write(ComedyComm::escape(tmp).as_slice()) {
            return Err(());
        }

        // Add the separator and convert each argument to a character vector
        bytes.push(FIELD_SEPARATOR);
        let mut tmp = Vec::new();
        if let Err(_) = tmp.write_u32::<LittleEndian>(data1) {
            return Err(());
        }

        // Escape the new argument and then add it
        if let Err(_) = bytes.write(ComedyComm::escape(tmp).as_slice()) {
            return Err(());
        }

        // Add the separator and convert each argument to a character vector
        bytes.push(FIELD_SEPARATOR);
        let mut tmp = Vec::new();
        if let Err(_) = tmp.write_u32::<LittleEndian>(data2) {
            return Err(());
        }

        // Escape the new argument and then add it
        if let Err(_) = bytes.write(ComedyComm::escape(tmp).as_slice()) {
            return Err(());
        }

        // Append the command separator
        bytes.push(COMMAND_SEPARATOR);

        // Send the bytes to the board
        if let Err(_) = self.port.write(bytes.as_slice()) {
            return Err(());
        }

        // Set the start time waiting for the ack
        self.last_ack = Some(Instant::now());

        // Indicate that the event was sent
        Ok(())
    }
}

// Implement the read event trait for ComedyComm
impl ReadEvents for ComedyComm {
    /// A method to receive a new event from the serial connection
    ///
    fn read_events(&mut self) -> Vec<(ItemId, u32, u32)> {
        // Create a list of events to return
        let mut events = Vec::new();

        // If there are pending outgoing messages
        if self.outgoing.len() > 0 {
            // Check for a pending ack
            match self.last_ack {
                // If there isn't a pending ack
                None => {
                    // Copy and send the next event
                    let (id, data1, data2) = self.outgoing[0];
                    self.write_event_now(id.clone(), data1.clone(), data2.clone())
                        .unwrap_or(());
                }

                // If there is a pending ack
                Some(instant) => {
                    // And if time has expired
                    if (instant + Duration::from_millis(ACK_DELAY)) < Instant::now() {
                        // Notify the system of a communication error
                        events.push((ItemId::new_unchecked(COMM_ERROR), 0, 0));

                        // Copy and resend the current event
                        let (id, data1, data2) = self.outgoing[0];
                        self.write_event_now(id.clone(), data1.clone(), data2.clone())
                            .unwrap_or(());
                    }
                }
            }
        }

        // Load any new characters into the buffer
        self.port.read_to_end(&mut self.buffer).unwrap_or(0);

        // Create temporary variables to track the message and status
        let mut message = Vec::new();
        let mut escaped = false; // indicates whether or not the currect character is escaped
        let mut new_message = true; // indicates if this character should start a new message
        let mut message_until = 0; // indicates the last character of a valid message

        // Read through each of the characters and count them
        for (count, character) in self.buffer.iter().enumerate() {
            // Try to read the command type for a new message
            if new_message {
                // Verify the command character
                if *character == COMMAND_CHARACTER {
                    // Reset the message variables
                    message = Vec::new();
                    escaped = false;
                    new_message = false;

                // Verify the ack character
                // (command separator will be skipped, as we do not reset new_message)
                } else if *character == ACK_CHARACTER {
                    // Reset the last ack to none
                    self.last_ack = None;

                    // Remove this character from the buffer
                    message_until = count + 1; // remove the last character as well

                    // Remove the first event from the buffer
                    if self.outgoing.len() > 0 {
                        self.outgoing.remove(0);
                    }

                    // If the character is incorrect, throw it away
                }

            // If the last message was an escape character, unescape this one
            } else if escaped {
                // If the escape was not valid
                if *character != FIELD_SEPARATOR
                    && *character != COMMAND_SEPARATOR
                    && *character != ESCAPE_CHARACTER
                    && *character != NULL_CHARACTER
                {
                    // Add both characters
                    message.push(ESCAPE_CHARACTER);
                }

                // Append the new character to the arguments
                message.push(character.clone());
                escaped = false;

            // Interpret the other, non-escaped, non-message-beginning characters
            } else {
                // Catch the command separator
                if *character == COMMAND_SEPARATOR {
                    // Note the end of the valid message and start a new message
                    message_until = count + 1; // remove the last character as well
                    new_message = true;

                    // Try to read the three arguments from the message
                    let mut cursor = Cursor::new(message.clone());
                    let id = match cursor.read_u32::<LittleEndian>() {
                        Ok(id) => id,
                        _ => {
                            // Return an error and exit
                            events.push((ItemId::new_unchecked(READ_ERROR), 0, 0));
                            break; // end prematurely
                        }
                    };
                    let data1 = match cursor.read_u32::<LittleEndian>() {
                        Ok(data1) => data1,
                        _ => {
                            // Return an error and exit
                            events.push((ItemId::new_unchecked(READ_ERROR), 0, 0));
                            break; // end prematurely
                        }
                    };
                    let data2 = match cursor.read_u32::<LittleEndian>() {
                        Ok(data2) => data2,
                        _ => {
                            // Return an error and exit
                            events.push((ItemId::new_unchecked(READ_ERROR), 0, 0));
                            break; // end prematurely
                        }
                    };

                    // Append the resulting event to the events vector
                    events.push((ItemId::new_unchecked(id), data1, data2));

                    // Send the acknowledgement
                    let bytes = vec![ACK_CHARACTER, COMMAND_SEPARATOR];
                    self.port.write(bytes.as_slice()).unwrap_or(0);

                // Catch the escape character
                } else if *character == ESCAPE_CHARACTER {
                    escaped = true;

                // Ignore the field separator
                } else if *character != FIELD_SEPARATOR {
                    message.push(character.clone());
                }
            }
        }

        // Remove all valid messages from the buffer
        self.buffer.drain(0..message_until);

        // Return the resulting events
        events
    }
}

// Implement the write event trait for ComedyComm
impl WriteEvent for ComedyComm {
    /// A method to send a new event to the serial connection
    ///
    fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), ()> {
        // If this event is not already in the outgoing buffer
        let mut found = false;
        for &(ref existing_id, ref existing_data1, ref existing_data2) in self.outgoing.iter() {
            if (*existing_id == id) && (*existing_data1 == data1) && (*existing_data2 == data2) {
                found = true;
            }
        }

        // Add this event to the outgoing buffer
        if !found {
            self.outgoing
                .push((id.clone(), data1.clone(), data2.clone()));
        }

        // If the port is not ready to receive bytes
        if self.outgoing.len() > 1 {
            // If the number of events as piled up, send an error
            if self.outgoing.len() > MAX_SEND_BUFFER {
                return Err(());
            }

            // Otherwise just return normally
            return Ok(());
        }

        // Try to write the event to serial
        self.write_event_now(id, data1, data2)
    }
}

// Tests of the CmdMessenger module
#[cfg(test)]
mod tests {
    use super::*;

    // Write to and read from an Arduino
    #[test]
    fn write_and_read() {
        // Import std library features
        use std::thread;
        use std::time::Duration;

        // Create a new CmdMessenger instance
        if let Some(mut cc) = ComedyComm::new(&PathBuf::from("/dev/ttyACM0"), 115200, 100) {
            // Wait for the Arduino to boot
            thread::sleep(Duration::from_secs(3));

            // Write a message to the Arduino
            let id_ref = ItemId::new_unchecked(205);
            let data1_ref: u32 = 29387;
            let data2_ref: u32 = 0;
            cc.write_event(id_ref, data1_ref, data2_ref);

            // Wait for a response
            thread::sleep(Duration::from_secs(1));

            // Read a response
            for (id, data1, data2) in cc.read_events() {
                // Verify that it is correct
                assert_eq!(id, id_ref);
                assert_eq!(data1, data1_ref);
                assert_eq!(data2, data2_ref);

            // Indicate failure
            } else {
                panic!("Reply from the board was incorrect or does not exist");
            }

        // Indicate failure
        } else {
            panic!("Could not initialize the Command Messenger.");
        }
    }
}
