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

//! A module to communicate using a ZMQ connection

// Import crate definitions
use crate::definitions::*;

// Import other definitions
use super::{EventConnection, ReadResult};

// Import standard library features
use std::path::PathBuf;

// Import the failure features
use failure::Error;

// Import the ZMQ C-bindings
#[cfg(feature = "zmq-comm")]
use zmq::{Context, Socket};

// Import program constants
#[cfg(feature = "zmq-comm")]
use super::POLLING_RATE; // the polling rate for the system

/// A structure to hold and manipulate the connection over zmq
///
pub struct ZmqBind {
    #[cfg(feature = "zmq-comm")]
    zmq_send: Socket, // the ZMQ send connection
    #[cfg(feature = "zmq-comm")]
    zmq_recv: Socket, // the ZMQ receive connection
}

// Implement key functionality for ZMQ Bind
impl ZmqBind {
    /// A function to create a new instance of the ZmqBind, active version
    ///
    #[cfg(feature = "zmq-comm")]
    pub fn new(send_path: &PathBuf, recv_path: &PathBuf) -> Result<ZmqBind, Error> {
        // Create the new ZMQ sending socket
        let context = Context::new();
        let zmq_send = context.socket(zmq::PUB)?;

        // Bind to a new ZMQ send path
        zmq_send.bind(send_path.to_str().unwrap_or(""))?;

        // Create the new ZMQ receiving socket
        let zmq_recv = context.socket(zmq::SUB)?;

        // Set the socket timeout and subscribe to all messages
        zmq_recv.set_rcvtimeo(POLLING_RATE as i32)?;
        zmq_recv.set_subscribe(&[])?;

        // Bind to a new ZMQ receive path
        zmq_recv.bind(recv_path.to_str().unwrap_or(""))?;

        // Return the new connection
        Ok(ZmqBind { zmq_send, zmq_recv })
    }
    
    /// A function to create a new instance of the ZmqBind, inactive version
    ///
    #[cfg(not(feature = "zmq-comm"))]
    pub fn new(_send_path: &PathBuf, _recv_path: &PathBuf) -> Result<ZmqBind, Error> {
        Ok(ZmqBind {})
    }
}

// Implement the event connection trait for ZmqBind
impl EventConnection for ZmqBind {
    /// A method to receive new events from the zmq connection, active version
    ///
    #[cfg(feature = "zmq-comm")]
    fn read_events(&mut self) -> Vec<ReadResult> {
        // Read any events from the zmq connection
        let mut results = Vec::new();
        while let Some(result) = read_from_zmq(&self.zmq_recv) {
            results.push(result);
        }

        // Return the list of results
        results
    }

    /// A method to receive new events from the zmq connection, inactive version
    ///
    #[cfg(not(feature = "zmq-comm"))]
    fn read_events(&mut self) -> Vec<ReadResult> {
        Vec::new() // return an empty vector
    }

    /// A method to send a new event to the zmq connection, active version
    ///
    #[cfg(feature = "zmq-comm")]
    fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        // Send a multipart ZMQ message, formatted as strings
        self.zmq_send.send(&id.id().to_string(), zmq::SNDMORE)?;
        self.zmq_send.send(&data1.to_string(), zmq::SNDMORE)?;
        self.zmq_send.send(&data2.to_string(), 0)?;
        Ok(())
    }

    /// A method to send a new event to the zmq connection, inactive version
    ///
    #[cfg(not(feature = "zmq-comm"))]
    fn write_event(&mut self, _id: ItemId, _data1: u32, _data2: u32) -> Result<(), Error> {
        return Err(format_err!("Program compiled without ZMQ support. See documentation."));
    }

    /// A method to echo events back to the zmq connection. This method does
    /// not check for duplicate messages (as it isn't necessary for this
    /// connection)
    ///
    fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        self.write_event(id, data1, data2)
    }
}

/// A structure to hold and manipulate the connection over zmq
/// This variant filters out events that have been echoed back to the
/// connection.
///
pub struct ZmqConnect {
    #[cfg(feature = "zmq-comm")]
    zmq_send: Socket,                    // the ZMQ send connection
    #[cfg(feature = "zmq-comm")]
    zmq_recv: Socket,                    // the ZMQ receive connection
    #[cfg(feature = "zmq-comm")]
    filter_in: Vec<(ItemId, u32, u32)>,  // events to filter, incoming
    #[cfg(feature = "zmq-comm")]
    filter_out: Vec<(ItemId, u32, u32)>, // events to filter, outgoing
}

// Implement key functionality for ZMQ Connect
impl ZmqConnect {
    /// A function to create a new instance of the ZmqConnect, active version
    ///
    #[cfg(feature = "zmq-comm")]
    pub fn new(send_path: &PathBuf, recv_path: &PathBuf) -> Result<ZmqConnect, Error> {
        // Create the new ZMQ sending socket
        let context = Context::new();
        let zmq_send = context.socket(zmq::PUB)?;

        // Connect to the existing ZMQ send path
        zmq_send.connect(send_path.to_str().unwrap_or(""))?;

        // Create the new ZMQ receiving socket
        let zmq_recv = context.socket(zmq::SUB)?;

        // Set the socket timeout and subscribe to all messages
        zmq_recv.set_rcvtimeo(POLLING_RATE as i32)?;
        zmq_recv.set_subscribe(&[])?;

        // Connect to the existing ZMQ receive path
        zmq_recv.connect(recv_path.to_str().unwrap_or(""))?;

        // Return a new live version
        Ok(ZmqConnect {
            zmq_send,
            zmq_recv,
            filter_in: Vec::new(),
            filter_out: Vec::new(),
        })
    }
    
    /// A function to create a new instance of the ZmqConnect, inactive version
    ///
    #[cfg(not(feature = "zmq-comm"))]
    pub fn new(_send_path: &PathBuf, _recv_path: &PathBuf) -> Result<ZmqConnect, Error> {
        Ok(ZmqConnect {})
    }
}

// Implement the event connection trait for ZmqConnect
impl EventConnection for ZmqConnect {
    /// A method to receive new events from the ZMQ connection, active version
    ///
    #[cfg(feature = "zmq-comm")]
    fn read_events(&mut self) -> Vec<ReadResult> {
        // Read any results from the zmq connection
        let mut results = Vec::new();
        while let Some(result) = read_from_zmq(&self.zmq_recv) {
            // Match based on the result
            if let ReadResult::Normal(id, data1, data2) = result {
                // Filter each event before adding it to the list
                let mut count = 0;
                for &(ref filter_id, ref filter_data1, ref filter_data2) in self.filter_out.iter() {
                    // If the event matches an event in the filter
                    if (id == *filter_id) && (data1 == *filter_data1) && (data2 == *filter_data2) {
                        break; // exit with the found event count
                    }

                    // Increment the count
                    count = count + 1;
                }

                // Filter the event and remove it from the filter
                if count < self.filter_out.len() {
                    // Remove that event from the filter
                    self.filter_out.remove(count);

                // Otherwise, add the event to the list
                } else {
                    // Add the new event to the list
                    results.push(ReadResult::Normal(id.clone(), data1.clone(), data2.clone()));

                    // Add the new event to the filter
                    self.filter_in.push((id, data1, data2));
                }
            
            // Otherwise, pass the error upstream
            } else {
                results.push(result);
            }
        }

        // Return the list of results
        results
    }
    
    /// A method to receive new events from the ZMQ connection, inactive version
    ///
    #[cfg(not(feature = "zmq-comm"))]
    fn read_events(&mut self) -> Vec<ReadResult> {
        Vec::new() // return an empty vector
    }

    /// A method to send a new event to the ZMQ connection, active version
    ///
    #[cfg(feature = "zmq-comm")]
    fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        // Send a multipart ZMQ message, formatted as strings
        self.zmq_send.send(&id.id().to_string(), zmq::SNDMORE)?;
        self.zmq_send.send(&data1.to_string(), zmq::SNDMORE)?;
        self.zmq_send.send(&data2.to_string(), 0)?;

        // Add the event to the filter
        self.filter_out.push((id, data1, data2));

        // Indicate success
        Ok(())
    }
    
    /// A method to send a new event to the ZMQ connection, inactive version
    #[cfg(not(feature = "zmq-comm"))]
    fn write_event(&mut self, _id: ItemId, _data1: u32, _data2: u32) -> Result<(), Error> {
        return Err(format_err!("Program compiled without ZMQ support. See documentation."));
    }

    /// A method to echo events back to the ZMQ connection, active version.
    /// This method filters out events that were received on the ZMQ connection.
    ///
    #[cfg(feature = "zmq-comm")]
    fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        // Filter each event before adding it to the list
        let mut count = 0;
        for &(ref filter_id, ref filter_data1, ref filter_data2) in self.filter_in.iter() {
            // If the event matches an event in the filter
            if (id == *filter_id) && (data1 == *filter_data1) && (data2 == *filter_data2) {
                break; // exit with the found event count
            }

            // Increment the count
            count = count + 1;
        }

        // Filter the event and remove it from the filter
        if count < self.filter_in.len() {
            // Remove that event from the filter
            self.filter_in.remove(count);
            return Ok(());

        // Otherwise, send the event
        } else {
            return self.write_event(id, data1, data2);
        }
    }
    
    /// A method to echo events back to the ZMQ connection, inactive version.
    ///
    #[cfg(not(feature = "zmq-comm"))]
    fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        self.write_event(id, data1, data2)
    }
}

// A helper function to read a single event from the zmq connection
#[cfg(feature = "zmq-comm")]
fn read_from_zmq(zmq_recv: &zmq::Socket) -> Option<ReadResult> {
    // Read the first component of the message
    let id;
    let data1;
    let data2;
    if let Ok(message) = zmq_recv.recv_msg(0) {
        // Try to convert the message
        id = match message.as_str().unwrap_or("").parse::<u32>() {
            Ok(new_data) => new_data,
            _ => return Some(ReadResult::ReadError(format_err!("Invalid Event Id for ZMQ."))),
        };

    // If nothing was received, return nothing
    } else {
        return None;
    }

    // Read the second component of the message
    if let Ok(message) = zmq_recv.recv_msg(0) {
        // Try to convert the message
        data1 = match message.as_str().unwrap_or("").parse::<u32>() {
            Ok(new_data) => new_data,
            _ => return Some(ReadResult::ReadError(format_err!("Invalid second field for ZMQ."))),
        };

    // Notify the system of a read error
    } else {
        return Some(ReadResult::ReadError(format_err!("Missing second field for ZMQ.")));
    }

    // Read the third component of the message
    if let Ok(message) = zmq_recv.recv_msg(0) {
        // Try to convert the message
        data2 = match message.as_str().unwrap_or("").parse::<u32>() {
            Ok(new_data) => new_data,
            _ => return Some(ReadResult::ReadError(format_err!("Invalid third field for ZMQ."))),
        };

    // Notify the system of a read error
    } else {
        return Some(ReadResult::ReadError(format_err!("Missing third field for ZMQ.")));
    }

    // Return the received id
    return Some(ReadResult::Normal(ItemId::new_unchecked(id), data1, data2));
}


