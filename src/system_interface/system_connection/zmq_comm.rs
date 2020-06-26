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

// Import the relevant structures into the correct namespace
use super::{EventConnection, ItemId, ReadResult};

// Import standard library features
use std::path::PathBuf;

// Import the ZMQ C-bindings
extern crate zmq;
use self::zmq::{Context, Socket};

// Import FNV HashMap
extern crate fnv;
use self::fnv::FnvHashMap;

// Import the failure features
use failure::Error;

// Import program constants
use super::POLLING_RATE; // the polling rate for the system

/// A structure to hold and manipulate the connection over zmq
///
pub struct ZmqBind {
    zmq_send: Socket, // the ZMQ send connection
    zmq_recv: Socket, // the ZMQ receive connection
}

// Implement key functionality for the ZMQ structure
impl ZmqBind {
    /// A function to create a new instance of the ZmqBind
    ///
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
}

// Implement the event connection trait for ZmqBind
impl EventConnection for ZmqBind {
    /// A method to receive new events from the zmq connection
    ///
    fn read_events(&mut self) -> Vec<ReadResult> {
        // Read any events from the zmq connection
        let mut results = Vec::new();
        while let Some(result) = read_from_zmq(&self.zmq_recv) {
            results.push(result);
        }

        // Return the list of results
        results
    }

    /// A method to send a new event to the zmq connection
    ///
    fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        // Send a multipart ZMQ message, formatted as strings
        self.zmq_send.send_str(&id.id().to_string(), zmq::SNDMORE)?;
        self.zmq_send.send_str(&data1.to_string(), zmq::SNDMORE)?;
        self.zmq_send.send_str(&data2.to_string(), 0)?;
        Ok(())
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
    zmq_send: Socket,                    // the ZMQ send connection
    zmq_recv: Socket,                    // the ZMQ receive connection
    filter_in: Vec<(ItemId, u32, u32)>,  // events to filter, incoming
    filter_out: Vec<(ItemId, u32, u32)>, // events to filter, outgoing
}

// Implement key functionality for the ZMQ structure
impl ZmqConnect {
    /// A function to create a new instance of the ZmqConnect
    ///
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
}

// Implement the event connection trait for ZmqToEcho
impl EventConnection for ZmqConnect {
    /// A method to receive a new event, empty for this connection type
    ///
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

    /// A method to send a new event to the zmq connection
    ///
    fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        // Send a multipart ZMQ message, formatted as strings
        self.zmq_send.send_str(&id.id().to_string(), zmq::SNDMORE)?;
        self.zmq_send.send_str(&data1.to_string(), zmq::SNDMORE)?;
        self.zmq_send.send_str(&data2.to_string(), 0)?;

        // Add the event to the filter
        self.filter_out.push((id, data1, data2));

        // Indicate success
        Ok(())
    }

    /// A method to echo events back to the zmq connection. This method filters
    /// out events that were received on the zmq connection.
    ///
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
}

// A helper function to read a single event from the zmq connection
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

/// A type to store a hashmap of event ids and strings
///
pub type EventToString = FnvHashMap<ItemId, String>;
pub type StringToEvent = FnvHashMap<String, ItemId>;

/// A structure to hold and manipulate the lookup connection over zmq
///
pub struct ZmqLookup {
    zmq_send: Socket,            // the ZMQ send connection
    zmq_recv: Socket,            // the ZMQ receive connection
    event_string: EventToString, // the event -> string dictionary
    string_event: StringToEvent, // the string -> event dictionary
    pending_size: u32,           // the size of the new pending string
    pending: Vec<u8>,            // a pending vector which may be converted to a string
}

// Implement key functionality for the ZMQ structure
impl ZmqLookup {
    /// A function to create a new instance of the ZmqBind
    ///
    pub fn new(
        send_path: &PathBuf,
        recv_path: &PathBuf,
        event_string: EventToString,
        string_event: StringToEvent,
    ) -> Result<ZmqLookup, Error> {
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
        Ok(ZmqLookup {
            zmq_send,
            zmq_recv,
            event_string,
            string_event,
            pending: Vec::new(),
            pending_size: 0,
        })
    }
}

// Implement the event connection trait for ZmqBind
impl EventConnection for ZmqLookup {
    /// A method to receive new events from the zmq connection
    ///
    fn read_events(&mut self) -> Vec<ReadResult> {
        // Collect any results
        let mut results = Vec::new();

        // Read any strings from the zmq connection
        loop {
            // Listen for an individual string
            match self.zmq_recv.recv_string(0) {
                // If a string is found
                Ok(Ok(string)) => {
                    // Try to translate the string (must be an exact match)
                    if let Some(id) = self.string_event.get(&string) {
                        // Add the event to the list
                        results.push(ReadResult::Normal(id.clone(), 0, 0));
                    }
                }

                // If a string is invalid, add a read error
                Ok(_) => results.push(ReadResult::ReadError(format_err!("Unable to ZMQ Lookup Input as String."))),

                // Otherwise, exit the loop
                _ => break,
            }
        }

        // Return the list of results
        results
    }

    /// A method to send a new event to the zmq connection
    ///
    fn write_event(&mut self, id: ItemId, _data1: u32, data2: u32) -> Result<(), Error> {
        // Try to translate the event to a string
        if let Some(text) = self.event_string.get(&id) {
            // Make a copy of the string
            let mut string = text.clone();

            // If the string ends with the time notation
            if string.ends_with("%t") {
                // Strip the ending from the string
                string.pop();
                string.pop();

                // Convert data2 and append it to the string
                let result = (((data2 / 60) * 100) + (data2 % 60)) as u32;
                string.push_str(&format!("{:04}", result));

            // If the string ends with the data notation
            } else if string.ends_with("%d") {
                // Strip the ending from the string
                string.pop();
                string.pop();

                // Append the raw data2 to the string
                string.push_str(&format!("{}", data2));

            // If the string ends with the string notaion
            } else if string.ends_with("%s") {
                // If there isn't a pending string, save the length
                if self.pending_size == 0 {
                    self.pending_size = data2;
                    return Ok(());

                // Otherwise, decrease the pending length and append the bytes
                } else {
                    // Append the first byte and decrement
                    self.pending.push((data2 >> 24) as u8);
                    self.pending_size = self.pending_size - 1;

                    // If there are still bytes pending
                    if self.pending_size > 0 {
                        // Append the second byte and decrement
                        self.pending.push((data2 >> 16) as u8);
                        self.pending_size = self.pending_size - 1;

                        // If there are still bytes pending
                        if self.pending_size > 0 {
                            // Append the third byte and decrement
                            self.pending.push((data2 >> 8) as u8);
                            self.pending_size = self.pending_size - 1;

                            // If there are still bytes pending
                            if self.pending_size > 0 {
                                // Append the fourth byte and decrement
                                self.pending.push(data2 as u8);
                                self.pending_size = self.pending_size - 1;
                            }
                        }
                    }
                }

                // If there are no more pending, try to send the string
                if self.pending_size == 0 {
                    // Should always succeed
                    if let Ok(new_string) = String::from_utf8(self.pending.drain(..).collect()) {
                        // Strip the ending from the string
                        string.pop();
                        string.pop();

                        // Append the new string to the existing one
                        string.push_str(&new_string);
                    }

                // Otherwise, return
                } else {
                    return Ok(());
                }
            }

            // Try to send the string
            self.zmq_send.send_str(&string, 0)?;
        }

        // Indicate success
        Ok(())
    }

    /// A method to echo events back to the zmq connection. This method does
    /// not check for duplicate messages (as it isn't necessary for this
    /// connection)
    ///
    fn echo_event(&mut self, _id: ItemId, _data1: u32, _data2: u32) -> Result<(), Error> {
        // Do not echo events
        Ok(())
    }
}
