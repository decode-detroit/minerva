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

//! A module to communicate using a DMX serial connection
//!
//! # Note
//!
//! This module is currently limited to Enttec DMX USB Pro compatible hardware.

// Import the relevant structures into the correct namespace
use super::{ItemId, EventConnection, READ_ERROR};

// Import standard library features
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

// Import the serial module
extern crate serial;
use self::serial::prelude::*;

// Import FNV HashMap
extern crate fnv;
use self::fnv::FnvHashMap;

// Import the failure features
use failure::Error;

// Define the communication constants
const COMMAND_START: u8 = 0x7E as u8; // the start of the command
const MESSAGE_LABEL: u8 = 0x06 as u8; // the message type label
const DATA_LSB: u8 = 0x01 as u8; // the data least significant bit
const DATA_MSB: u8 = 0x02 as u8; // the data most significant bit FIXME check this
const DMX_START_CODE: u8 = 0x00 as u8; // the DMX start code
const COMMAND_END: u8 = 0xE7 as u8; // the end of the command

/// A struct to define a single fade of a DMX channel
///
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmxFade {
    channel: u32,                       // the dmx channel to fade
    value: u8,                          // the final value at the end of the fade
    duration: Option<Duration>,         // the duration of the fade (None if instantaneous)
}

/// A type to store a hashmap of event ids and DMX fades
///
pub type DmxMap = FnvHashMap<ItemId, DmxFade>;

/// A structure to hold and manipulate the connection over serial
///
pub struct DmxComm {
    port: serial::SystemPort,           // the serial port of the connection
    status: Vec<u8>,                    // the current status of all the channels
    dmx_map: DmxMap,                    // the map of event ids to fade instructions
    //last_update: Option<Instant>,       // Some(instant) when the last update was sent
}

// Implement key functionality for the DMX structure
impl DmxComm {
    /// A function to create a new instance of the DmxComm
    ///
    pub fn new(path: &PathBuf, dmx_map: DmxMap) -> Result<DmxComm, Error> {
        // Connect to the underlying serial port
        let mut port = serial::open(path)?;

        // Try to configure the serial port
        let settings = serial::PortSettings {
            baud_rate: serial::BaudRate::from_speed(9600),
            char_size: serial::Bits8,
            parity: serial::ParityNone,
            stop_bits: serial::Stop1,
            flow_control: serial::FlowNone,
        };
        port.configure(&settings)?;

        // Adjust the timeout for the serial port
        port.set_timeout(Duration::from_millis(100))?;

        // Return the new CmdMessenger instance
        Ok(DmxComm {
            port,
            status: vec![0 as u8; 512],
            dmx_map,
            //last_update: None,
        })
    }

    /// A helper function to write the existing frame to the serial port after
    /// changing the value of the selected channel
    ///
    /// # Note
    ///
    /// Assumes the channels are one-indexed (the DMX standard) rather than
    /// zero-indexed.
    ///
    fn write_frame(&mut self, channel: u32, value: u8) -> Result<(), Error> {
        // Verify the range of the selected channel
        if (channel > 512) | (channel < 1) {
            return Err(format_err!("Selected DMX channel is out of range."));
        }
        
        // Change the value of the selected channel
        self.status[(channel - 1) as usize] = value;
        
        // Add the message header
        let mut bytes = Vec::new();
        bytes.push(COMMAND_START);
        bytes.push(MESSAGE_LABEL);
        bytes.push(DATA_LSB);
        bytes.push(DATA_MSB);
        bytes.push(DMX_START_CODE);

        // Add the current status to the message
        let mut status_copy = self.status.clone();
        bytes.append(&mut status_copy);
        
        // Add the message ending
        bytes.push(COMMAND_END);
        
        // Send the bytes to the board
        self.port.write(bytes.as_slice())?;
        Ok(())
    }
}

// Implement the event connection trait for ComedyComm
impl EventConnection for DmxComm {
    /// A method to receive a new event, empty for this connection type
    ///
    fn read_events(&mut self) -> Vec<(ItemId, u32, u32)> {
        Vec::new() // return an empty vector
    }

    /// A method to send a new event to the serial connection
    ///
    fn write_event(&mut self, id: ItemId, _data1: u32, _data2: u32) -> Result<(), Error> {
        // Check to see if the event is in the DMX map
        if let Some(dmx_fade) = self.dmx_map.get(&id) {
            // Send the event immediately FIXME use the fade information
            return self.write_frame(dmx_fade.channel.clone(), dmx_fade.value.clone());
        };
        
        // If the event wasn't found, indicate success
        Ok(())
    }
    
    /// A method to echo an event to the serial connection
    fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        self.write_event(id, data1, data2)
    }
}

// Tests of the DMXComm module
/*#[cfg(test)]
mod tests {
    use super::*;
    
    // Import the library items for the testing function
    /*use std::thread;
    use std::time::{Duration, Instant};

    // Test the function by 
    fn main() {
        // Print the current step
        println!("Starting up ...");
        
        // Try to open the serial connection
        match DmxComm::new(&PathBuf::from("/dev/ttyUSB0")) {
            // If the connection is a success
            Some(mut connection) => {
                println!("Connected.");
                
                // Running the dimming cycle indefinitely
                loop {
                    // Change the value from low to high over time
                    for count in 0..125 {
                        // Notify of the change
                        println!("Dimming ... {}", count);
                        
                        // Send the updated value
                        connection.write_frame_now(count * 2);
                        
                        // Wait a little bit
                        thread::sleep(Duration::from_millis(25));
                    }
                    
                    // Dim the light back down
                    for count in 0..125 {
                        // Notify of the change
                        println!("Dimming ... {}", 125-count);
                        
                        // Send the updated value
                        connection.write_frame_now((125-count) * 2);
                        
                        // Wait a little bit
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            },
            
            // Otherwise, warn of the error
            None => println!("Unable to connect."),
        }
    }*/

}*/

