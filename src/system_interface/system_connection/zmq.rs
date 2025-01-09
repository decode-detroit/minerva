// Copyright (c) 2019-24 Decode Detroit
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
use super::{EventConnection, EventWithData};

// Import standard library features
use std::path::Path;
use std::time::Duration;

// Import the tokio and tokio serial features
use tokio::time::sleep;

// Import tracing features
use tracing::error;

// Import anyhow features
use anyhow::Result;

// Import the ZMQ C-bindings
use zmq::{Context, Socket};

// Define module constants
const POLLING_RATE: u64 = 1; // the polling rate for the connection in ms

/// A structure to hold and manipulate the connection over zmq
///
pub struct ZmqBind {
    zmq_send: Socket, // the ZMQ send connection
    zmq_recv: Socket, // the ZMQ receive connection
}

// Implement key functionality for ZMQ Bind
impl ZmqBind {
    /// A function to create a new instance of the ZmqBind, active version
    ///
    pub async fn new(send_path: &Path, recv_path: &Path) -> Result<ZmqBind> {
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
    /// A method to receive new events from the zmq connection.
    ///
    async fn read_event(&mut self) -> Option<EventWithData> {
        // Keep looking for events
        loop {
            // See if there's an event at the connection
            if let Some(event) = read_from_zmq(&mut self.zmq_recv) {
                // Return the event
                return Some(event);
            }

            // Otherwise, wait a little for other events to process
            sleep(Duration::from_millis(POLLING_RATE)).await;
        }
    }

    /// A method to send a new event to the zmq connection.
    ///
    async fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()> {
        // Send the zmq message
        write_to_zmq(&mut self.zmq_send, id, data1, data2)
    }

    /// A method to echo events back to the zmq connection. This method does
    /// not check for duplicate messages (as it isn't necessary for this
    /// connection)
    ///
    async fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()> {
        self.write_event(id, data1, data2).await
    }

    /// A method to process any pending sends. Since the ZMQ connection does
    /// not have this concept, this method does nothing
    async fn process_pending(&mut self) -> bool {
        false
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

// Implement key functionality for ZMQ Connect
impl ZmqConnect {
    /// A function to create a new instance of the ZmqConnect, active version
    ///
    pub async fn new(send_path: &Path, recv_path: &Path) -> Result<ZmqConnect> {
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

// Implement the event connection trait for ZmqConnect
impl EventConnection for ZmqConnect {
    /// A method to receive new events from the ZMQ connection.
    ///
    async fn read_event(&mut self) -> Option<EventWithData> {
        // Keep looking for events
        let (id, data1, data2);
        loop {
            // See if there's an event at the connection
            if let Some(event) = read_from_zmq(&mut self.zmq_recv) {
                // Return the event
                (id, data1, data2) = event;
                break;
            }

            // Otherwise, wait a little for other events to process
            sleep(Duration::from_millis(POLLING_RATE)).await;
        }

        // Filter the event before returning it
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

        // Otherwise, return the event
        } else {
            // Add the new event to the filter
            self.filter_in.push((id, data1, data2));

            // Return the event
            return Some((id, data1, data2));
        }

        // Otherwise, return none
        None
    }

    /// A method to send a new event to the ZMQ connection.
    ///
    async fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()> {
        // Send the zmq message
        write_to_zmq(&mut self.zmq_send, id, data1, data2)?;

        // Add the event to the filter
        self.filter_out.push((id, data1, data2));

        // Indicate success
        Ok(())
    }

    /// A method to echo events back to the ZMQ connection, active version.
    /// This method filters out events that were received on the ZMQ connection.
    ///
    async fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()> {
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
            return self.write_event(id, data1, data2).await;
        }
    }

    /// A method to process any pending sends. Since the ZMQ connection does
    /// not have this concept, this method does nothing.
    ///
    async fn process_pending(&mut self) -> bool {
        false
    }
}

// A helper function to read a single event from the zmq connection
fn read_from_zmq(zmq_recv: &mut Socket) -> Option<EventWithData> {
    // fn read_from_zmq(zmq_recv: &zmq::Socket) -> Option<ReadResult> {
    // Read the first component of the message
    let id;
    let data1;
    let data2;
    if let Ok(message) = zmq_recv.recv_msg(0) {
        // Try to convert the message
        id = match message.as_str().unwrap_or("").parse::<u32>() {
            Ok(new_data) => new_data,
            _ => {
                error!("Communication read error: Invalid Event Id for ZMQ.");
                return None;
            }
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
            _ => {
                error!("Communication read error: Invalid second field for ZMQ.");
                return None;
            }
        };

    // Notify the system of a read error
    } else {
        error!("Communication read error: Missing second field for ZMQ.");
        return None;
    }

    // Read the third component of the message
    if let Ok(message) = zmq_recv.recv_msg(0) {
        // Try to convert the message
        data2 = match message.as_str().unwrap_or("").parse::<u32>() {
            Ok(new_data) => new_data,
            _ => {
                error!("Communication read error: Invalid third field for ZMQ.");
                return None;
            }
        };

    // Notify the system of a read error
    } else {
        error!("Communication read error: Missing third field for ZMQ.");
        return None;
    }

    // Return the received id
    return Some((ItemId::new_unchecked(id), data1, data2));
}

// A helper function to write a single event from the zmq connection
fn write_to_zmq(zmq_send: &mut Socket, event_id: ItemId, data1: u32, data2: u32) -> Result<()> {
    // Send a multipart ZMQ message, formatted as strings
    zmq_send.send(&event_id.id().to_string(), zmq::SNDMORE)?;
    zmq_send.send(&data1.to_string(), zmq::SNDMORE)?;
    zmq_send.send(&data2.to_string(), 0)?;
    Ok(())
}
