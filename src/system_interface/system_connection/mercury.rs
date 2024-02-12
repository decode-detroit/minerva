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

//! A module to communicate using the Mercury serial protocol
//!
//! # Note
//!
//! This module is based off of the  CmdMessenger protocol. While this means
//! that there is some compatibility with the CmdMessenger library, this protocol
//! is not designed to be compatible. We recommend using the MercuryComm
//! library instead.

// Import crate definitions
use crate::definitions::*;

// Import other definitions
use super::{EventConnection, ReadResult};

// Import standard library modules and traits
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

// Import FNV HashSet
use fnv::FnvHashSet;

// Import the tokio and tokio serial features
use tokio::time::sleep;
use tokio_serial as serial;

// Import tracing features
use tracing::error;

// Import anyhow features
use anyhow::Result;

// Import the byteorder module for converting between types
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

// Define the communication constants
const FIELD_SEPARATOR: u8 = 0x2C as u8; // the divider between the three fields
const COMMAND_SEPARATOR: u8 = 0x3B as u8; // the divider between commands
const ESCAPE_CHARACTER: u8 = 0x2F as u8; // the character to escape other characters
const NULL_CHARACTER: u8 = 0x00 as u8; // the null character
const COMMAND_CHARACTER: u8 = '0' as u8; // the default command character
const ACK_CHARACTER: u8 = '1' as u8; // the default ack character
const ACK_DELAY: u64 = 200; // the longest delay to wait for an acknowledgement, in ms
const RETRY_DELAY: u64 = 5000; // the delay to wait between retrying to establish a connection
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
pub struct Mercury {
    path: PathBuf,                              // the desired path of the serial port
    baud: u32,                                // the baud rate of the serial port
    use_checksum: bool,                         // a flag indicating the system should use and verify 32bit checksums
    allowed_events: Option<FnvHashSet<ItemId>>, // if specified, the only events that can be sent to this connection
    polling_rate: u64,                          // the polling rate of the port
    stream: Option<serial::SerialStream>,       // the serial port of the connection, if available
    buffer: Vec<u8>,                            // the current input buffer
    outgoing: Vec<(ItemId, u32, u32)>,          // the outgoing event buffer
    last_ack: Option<Instant>,                  // Some instant if we are still waiting on ack from instant
    filter_events: Vec<(ItemId, u32, u32)>,     // events to filter out that we received from this connection
    last_retry: Option<Instant>,                // Some instant if we have lost connection to the port
}

// Implement key functionality for the Mercury structure
impl Mercury {
    /// A function to create a new instance of the Mercury
    ///
    pub fn new(path: &PathBuf, baud: u32, use_checksum: bool, allowed_events: Option<FnvHashSet<ItemId>>, polling_rate: u64) -> Result<Self> {
        // Create the new instance
        let mut mercury = Mercury {
            path: path.clone(),
            baud,
            use_checksum,
            allowed_events,
            polling_rate,
            stream: None,
            buffer: Vec::new(),
            outgoing: Vec::new(),
            last_ack: None,
            filter_events: Vec::new(),
            last_retry: None,
        };

        // Try to connect to the requested serial port, and return any errors
        mercury.connect()?;

        // Return the new Mercury instance
        Ok(mercury)
    }

    /// A helper function to connect to the serial port
    ///
    /// Returns the error if unable to connect.
    ///
    fn connect(&mut self) -> Result<()> {
        // Disconnect from the port, if there is one
        drop(self.stream.take());

        // Create and configure a builder to connect to the underlying serial port
        let builder = serial::new(self.path.to_str().unwrap_or(""), self.baud)
            .data_bits(serial::DataBits::Eight)
            .parity(serial::Parity::None)
            .stop_bits(serial::StopBits::One)
            .flow_control(serial::FlowControl::None)
            .timeout(Duration::from_millis(self.polling_rate));

        // Try to open the serial port
        let stream = serial::SerialStream::open(&builder)?;

        // Save the new port
        self.stream.replace(stream);

        // Reset last retry to none
        self.last_retry = None;

        // Indicate success
        Ok(())
    }

    /// A helper function to check on the status of the connection, and if it's
    /// broken, try to reestablish it periodically
    ///
    fn check_connection(&mut self) -> Result<()> {
        // Check to see if the port exists
        if self.stream.is_none() {
            // Look at the last retry
            match self.last_retry {
                // If this is the first time
                None => {
                    // Save this retry
                    self.last_retry = Some(Instant::now());

                    // Try to reconnect
                    if self.connect().is_err() {
                        return Err(anyhow!("Mercury port is unavailable."));
                    }
                }

                // Otherwise, check how long it's been
                Some(instant) => {
                    // If we haven't retried in a while
                    if (instant + Duration::from_millis(RETRY_DELAY)) < Instant::now() {
                        // Save this retry
                        self.last_retry = Some(Instant::now());

                        // Try to reconnect
                        if self.connect().is_err() {
                            return Err(anyhow!("Mercury port is unavailable."));
                        }
                    }
                }
            }
        }

        // Indicate all is normal
        Ok(())
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
    /// Note: Ack timer must be manually reset by caller
    ///
    async fn write_event_now(
        &mut self,
        id: ItemId,
        data1: u32,
        data2: u32,
    ) -> Result<()> {
        // Format the command as a byte
        let mut bytes = Vec::new();
        bytes.push(COMMAND_CHARACTER);

        // Add the three arguments to the message
        // Add the separator and convert each argument to a character vector
        bytes.push(FIELD_SEPARATOR);
        let mut tmp = Vec::new();
        tmp.write_u32::<LittleEndian>(id.id())?;

        // Escape the new argument and then add it
        bytes.write(Mercury::escape(tmp).as_slice())?;

        // Add the separator and convert each argument to a character vector
        bytes.push(FIELD_SEPARATOR);
        let mut tmp = Vec::new();
        tmp.write_u32::<LittleEndian>(data1)?;

        // Escape the new argument and then add it
        bytes.write(Mercury::escape(tmp).as_slice())?;

        // Add the separator and convert each argument to a character vector
        bytes.push(FIELD_SEPARATOR);
        let mut tmp = Vec::new();
        tmp.write_u32::<LittleEndian>(data2)?;

        // Escape the new argument and then add it
        bytes.write(Mercury::escape(tmp).as_slice())?;

        // If directed
        if self.use_checksum {
            // Calculate the checksum and add it
            bytes.push(FIELD_SEPARATOR);
            let mut tmp = Vec::new();
            tmp.write_u32::<LittleEndian>(id.id() ^ data1 ^ data2)?;

            // Escape the new argument and then add it
            bytes.write(Mercury::escape(tmp).as_slice())?;
        }

        // Append the command separator
        bytes.push(COMMAND_SEPARATOR);

        // Try to write to the serial port
        if let Some(ref mut stream) = self.stream {
            Mercury::write_bytes(stream, bytes, self.polling_rate).await?;

        // If the stream doesn't exist (it always should)
        } else {
            return Err(anyhow!("Unable to write to Mercury port."));
        }

        // Indicate that the event was sent
        Ok(())
    }

    // A helper function to write asyncronously to a serial port stream
    // Returns false if the port was unavailable immediately or after the
    // polling rate has expired.
    async fn write_bytes(stream: &mut serial::SerialStream, bytes: Vec<u8>, polling_rate: u64) -> Result<()> {
        // Wait for the up to the polling rate for the stream to be ready
        tokio::select! {
            // If the serial stream is available
            Ok(_) = stream.writable() => {
                // Try to send message to the Mercury port
                if let Ok(sent_bytes) = stream.try_write(bytes.as_slice()) {
                    // If the bytes don't match
                    if sent_bytes != bytes.len() {
                        // Indicate failure
                        return Err(anyhow!("Incomplete write to Mercury port."));
                    }

                // Otherwise, mark the write as incomplete
                } else {
                    return Err(anyhow!("Unable to write to Mercury port."));
                }
            }

            // Only wait for the polling rate
            _ = sleep(Duration::from_millis(polling_rate)) => {
                // Mark the write as still waiting
                return Err(anyhow!("Timeout while writing to Mercury port."));
            }
        }

        // Indicate success
        Ok(())
    }

}

// Implement the event connection trait for Mercury
impl EventConnection for Mercury {
    /// A method to receive a new event from the serial connection
    ///
    async fn read_events(&mut self) -> Vec<ReadResult> {
        // Check the serial connection
        if let Err(error) = self.check_connection() {
            return vec![ReadResult::ReadError(error)];
        };

        // Create a list of results to return
        let mut results = Vec::new();

        // Load any new characters into the buffer
        if let Some(ref mut stream) = self.stream {
            stream.read_to_end(&mut self.buffer) // FIXME unwrap_or(0);
        }

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
                            results.push(ReadResult::ReadError(anyhow!(
                                "Invalid Event Id for Mercury port."
                            )));
                            break; // end prematurely
                        }
                    };
                    let data1 = match cursor.read_u32::<LittleEndian>() {
                        Ok(data1) => data1,
                        _ => {
                            // Return an error and exit
                            results.push(ReadResult::ReadError(anyhow!(
                                "Invalid second field for Mercury port."
                            )));
                            break; // end prematurely
                        }
                    };
                    let data2 = match cursor.read_u32::<LittleEndian>() {
                        Ok(data2) => data2,
                        _ => {
                            // Return an error and exit
                            results.push(ReadResult::ReadError(anyhow!(
                                "Invalid third field for Mercury port."
                            )));
                            break; // end prematurely
                        }
                    };

                    // If verifying the checksum
                    if self.use_checksum {
                        // Try to read the checksum from Mercury
                        let checksum = match cursor.read_u32::<LittleEndian>() {
                            Ok(checksum) => checksum,
                            _ => {
                                // Return an error and exit
                                results.push(ReadResult::ReadError(anyhow!(
                                    "Invalid checksum field for Mercury port."
                                )));
                                break; // end prematurely
                            }
                        };

                        // Verify the value
                        if checksum != (id ^ data1 ^ data2) {
                            // Return an error and exit
                            results.push(ReadResult::ReadError(anyhow!(
                                "Invalid checksum for Mercury port."
                            )));
                            break; // end prematurely
                        }
                    }

                    // Append the resulting event to the results vector
                    results.push(ReadResult::Normal(ItemId::new_unchecked(id), data1, data2));

                    // Send the acknowledgement
                    let bytes = vec![ACK_CHARACTER, COMMAND_SEPARATOR];
                    // FIXME Mercury::write_bytes(stream, bytes, self.polling_rate).await.unwrap_or(());

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

        // Add the incoming events to the filter
        for result in results.iter() {
            // Check to make sure it's a valid event
            if let ReadResult::Normal(id, data1, data2) = result {
                self.filter_events
                    .push((id.clone(), data1.clone(), data2.clone()));
            }
        }

        // Return the resulting events
        results
    }

    /// A method to send a new event to the serial connection
    ///
    async fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()> {
        // Check the serial port connection
        self.check_connection()?;

        // If there's a filter, apply it (and return early, if not found)
        if let Some(ref events) = &self.allowed_events {
            if !events.contains(&id) {
                return Ok(());
            }
        }

        // Add this event to the outgoing buffer
        self.outgoing.push((id.clone(), data1.clone(), data2.clone()));

        // If the port is not ready to receive bytes
        if self.outgoing.len() > 1 {
            // If the number of events as piled up, send an error
            if self.outgoing.len() > MAX_SEND_BUFFER {
                return Err(anyhow!("Too many events in outgoing buffer."));
            }

            // Otherwise just return normally
            return Ok(());
        }

        // Try to write the event to the port
        self.write_event_now(id, data1, data2).await?;

        // Reset the start time waiting for the ack
        self.last_ack = Some(Instant::now());

        // Otherwise, indicate success
        Ok(())
    }

    /// A method to echo an event to the serial connection
    ///
    async fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()> {
        // Filter each event before echoing it to the system
        let mut count = 0;
        for &(ref filter_id, ref filter_data1, ref filter_data2) in self.filter_events.iter() {
            // If the event matches an event in the filter
            if (id == *filter_id) && (data1 == *filter_data1) && (data2 == *filter_data2) {
                break; // exit with the found event count
            }

            // Increment the count
            count = count + 1;
        }

        // Filter the event and remove it from the filter
        if count < self.filter_events.len() {
            // Remove that event from the filter
            self.filter_events.remove(count);
            return Ok(());

        // Otherwise, echo the event to the system
        } else {
            // Write the event to the system
            return self.write_event(id, data1, data2).await;
        }
    }

    /// A method to process any pending writes to the serial connection
    /// 
    async fn process_pending(&mut self) -> bool {
        // If there are no pending outgoing messages
        if self.outgoing.len() == 0 {
            return false;
        
        // Otherwise, try to process the messages
        } else {
            // Check the serial connection
            if let Err(error) = self.check_connection() {
                return true; // Indicate messages are still pending
            };

            // Flag to indicate the port is inaccessible
            let mut is_unavailable = false;

            // Check for a pending ack
            match self.last_ack {
                // If there isn't a pending ack
                None => {
                    // Copy and send the next event
                    let (id, data1, data2) = self.outgoing[0];
                    if self.write_event_now(
                        id.clone(),
                        data1.clone(),
                        data2.clone(),
                    ).await
                    .is_err()
                    {
                        is_unavailable = true;
                    }

                    // Reset the start time waiting for the ack
                    self.last_ack = Some(Instant::now());
                }

                // If there is a pending ack
                Some(instant) => {
                    // And if time has expired
                    if (instant + Duration::from_millis(ACK_DELAY)) < Instant::now() {
                        // Notify the system of a communication error
                        error!("Communication write error: No Acknowledgement from Mercury port. Retrying ...");

                        // Copy and resend the current event
                        let (id, data1, data2) = self.outgoing[0];
                        if self.write_event_now(
                            id.clone(),
                            data1.clone(),
                            data2.clone(),
                        ).await
                        .is_err()
                        {
                            is_unavailable = true;
                        }

                        // Reset the start time waiting for the ack
                        self.last_ack = Some(Instant::now());
                    }
                }
            }

            // If the port is unavailable, drop the connection
            if is_unavailable {
                if let Some(bad_port) = self.stream.take() {
                    drop(bad_port); // Ensure the port is promptly dropped

                    // Notify the system of a communication error
                    error!("Communication write error: Lost connection to Mercury port.");
                }
            }
        }

        // Indicate more events may be pending
        true
    }
}

// Tests of the Mercury module
#[cfg(test)]
mod tests {
    use super::*;

    // Write to and read from an Arduino
    #[tokio::test]
    async fn write_and_read() {
        // Import std library features
        use std::thread;
        use std::time::Duration;

        // Create a new CmdMessenger instance
        if let Ok(mut cc) = Mercury::new(&PathBuf::from("/dev/ttyACM0"), 115200, true, None, 100) {
            // Wait for the Arduino to boot
            thread::sleep(Duration::from_secs(3));

            // Write a message to the Arduino
            let id_ref = ItemId::new_unchecked(205);
            let data1_ref: u32 = 29387;
            let data2_ref: u32 = 0;
            cc.write_event(id_ref, data1_ref, data2_ref).await.unwrap_or(());

            // Wait for a response
            thread::sleep(Duration::from_millis(500));

            // Read a response
            for result in cc.read_events().await {
                if let ReadResult::Normal(id, data1, data2) = result {
                    // Verify that it is correct
                    assert_eq!(id, id_ref);
                    assert_eq!(data1, data1_ref);
                    assert_eq!(data2, data2_ref);
                } else {
                    panic!("Read error in the Commedy Comm.")
                }
            }

        // Indicate failure
        } else {
            panic!("Could not initialize the Commedy Comm.");
        }
    }
}
