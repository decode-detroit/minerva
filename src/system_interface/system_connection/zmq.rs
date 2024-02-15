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
use super::{EventConnection, EventWithData};

// Import standard library features
use std::path::Path;
#[cfg(feature = "zmq-comm")]
use std::io::{Cursor, Write};

// Import tracing features
#[cfg(feature = "zmq-comm")]
use tracing::error;

// Import anyhow features
use anyhow::Result;

// Import the ZMQ C-bindings
#[cfg(feature = "zmq-comm")]
use zeromq::{Socket, PubSocket, SubSocket};

// Import the byteorder module for converting between types
#[cfg(feature = "zmq-comm")]
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

// Import program constants
#[cfg(feature = "zmq-comm")]
use super::POLLING_RATE; // the polling rate for the system

/// A structure to hold and manipulate the connection over zmq
///
pub struct ZmqBind {
    #[cfg(feature = "zmq-comm")]
    zmq_send: PubSocket, // the ZMQ send connection
    #[cfg(feature = "zmq-comm")]
    zmq_recv: SubSocket, // the ZMQ receive connection
}

// Implement key functionality for ZMQ Bind
impl ZmqBind {
    /// A function to create a new instance of the ZmqBind, active version
    ///
    #[cfg(feature = "zmq-comm")]
    pub async fn new(send_path: &Path, recv_path: &Path) -> Result<ZmqBind> {
        // Create the new ZMQ sending socket
        let zmq_send = PubSocket::new();

        // Bind to a new ZMQ send path
        zmq_send.bind(send_path.to_str().unwrap_or(""))?;

        // Create the new ZMQ receiving socket
        let zmq_recv = SubSocket::new();

        // Set the socket to subscribe to all messages
        zmq_recv.subscribe("").await?;

        // Bind to a new ZMQ receive path
        zmq_recv.bind(recv_path.to_str().unwrap_or(""))?;

        // Return the new connection
        Ok(ZmqBind { zmq_send, zmq_recv })
    }

    /// A function to create a new instance of the ZmqBind, inactive version
    ///
    #[cfg(not(feature = "zmq-comm"))]
    pub async fn new(_send_path: &Path, _recv_path: &Path) -> Result<ZmqBind> {
        Ok(ZmqBind {})
    }
}

// Implement the event connection trait for ZmqBind
#[cfg(feature = "zmq-comm")]
impl EventConnection for ZmqBind {
    /// A method to receive new events from the zmq connection.
    ///
    async fn read_events(&mut self) -> Result<Vec<EventWithData>> {
        // Read any events from the zmq connection
        return Ok(read_from_zmq(&self.zmq_recv).await);
    }

    /// A method to send a new event to the zmq connection.
    ///
    async fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()> {
        // Send the zmq message
        write_to_zmq(&self.zmq_send, id, data1, data2).await
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

// Implement the inactive event connection trait for ZmqBind
#[cfg(not(feature = "zmq-comm"))]
impl EventConnection for ZmqBind {
    /// A method to receive new events from the zmq connection, inactive version.
    ///
    async fn read_events(&mut self) -> Result<Vec<EventWithData>> {
        return Err(anyhow!(
            "Program compiled without ZMQ support. See documentation."
        ));
    }

    /// A method to send a new event to the zmq connection, inactive version.
    ///
    async fn write_event(&mut self, _id: ItemId, _data1: u32, _data2: u32) -> Result<()> {
        return Err(anyhow!(
            "Program compiled without ZMQ support. See documentation."
        ));
    }

    /// A method to echo events back to the zmq connection, inactive version.
    ///
    async fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()> {
        self.write_event(id, data1, data2).await
    }

    /// A method to process any pending sends, inactive version
    async fn process_pending(&mut self) -> bool {
        false
    }
}

/// A structure to hold and manipulate the connection over zmq
/// This variant filters out events that have been echoed back to the
/// connection.
///
pub struct ZmqConnect {
    #[cfg(feature = "zmq-comm")]
    zmq_send: PubSocket, // the ZMQ send connection
    #[cfg(feature = "zmq-comm")]
    zmq_recv: SubSocket, // the ZMQ receive connection
    #[cfg(feature = "zmq-comm")]
    filter_in: Vec<(ItemId, u32, u32)>, // events to filter, incoming
    #[cfg(feature = "zmq-comm")]
    filter_out: Vec<(ItemId, u32, u32)>, // events to filter, outgoing
}

// Implement key functionality for ZMQ Connect
impl ZmqConnect {
    /// A function to create a new instance of the ZmqConnect, active version
    ///
    #[cfg(feature = "zmq-comm")]
    pub async fn new(send_path: &Path, recv_path: &Path) -> Result<ZmqConnect> {
        // Create the new ZMQ sending socket
        let zmq_send = PubSocket::new();

        // Connect to the existing ZMQ send path
        zmq_send.connect(send_path.to_str().unwrap_or(""))?;

        // Create the new ZMQ receiving socket
        let zmq_recv = SubSocket::new();

        // Set the socket to subscribe to all messages
        zmq_recv.subscribe("").await?;

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
    pub fn new(_send_path: &Path, _recv_path: &Path) -> Result<ZmqConnect> {
        Ok(ZmqConnect {})
    }
}

// Implement the event connection trait for ZmqConnect
#[cfg(feature = "zmq-comm")]
impl EventConnection for ZmqConnect {
    /// A method to receive new events from the ZMQ connection.
    ///
    async fn read_events(&mut self) -> Result<Vec<EventWithData>> {
        // Read any events from the zmq connection
        let mut events = Vec::new();
        while let Some((id, data1, data2)) = read_from_zmq(&self.zmq_recv).await {
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
                events.push((id.clone(), data1.clone(), data2.clone()));

                // Add the new event to the filter
                self.filter_in.push((id, data1, data2));
            }
        }

        // Return the list of results
        events
    }

    /// A method to send a new event to the ZMQ connection.
    ///
    async fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()> {
        // Send the zmq message
        write_to_zmq(&self.zmq_send, id, data1, data2).await?;

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

// Implement the event connection trait for ZmqConnect
#[cfg(not(feature = "zmq-comm"))]
impl EventConnection for ZmqConnect {
    /// A method to receive new events from the ZMQ connection, inactive version.
    ///
    async fn read_events(&mut self) -> Result<Vec<EventWithData>> {
        return Err(anyhow!(
            "Program compiled without ZMQ support. See documentation."
        ));
    }

    /// A method to send a new event to the ZMQ connection, inactive version.
    /// 
    async fn write_event(&mut self, _id: ItemId, _data1: u32, _data2: u32) -> Result<()> {
        return Err(anyhow!(
            "Program compiled without ZMQ support. See documentation."
        ));
    }

    /// A method to echo events back to the ZMQ connection, inactive version.
    ///
    async fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()> {
        self.write_event(id, data1, data2).await
    }

    /// A method to process any pending sends, inactive version.
    /// 
    async fn process_pending(&mut self) -> bool {
        false
    }
}

// A helper function to read a single event from the zmq connection
#[cfg(feature = "zmq-comm")]
async fn read_from_zmq(zmq_recv: &zeromq::SubSocket) -> Vec<EventWithData> {
    // Wait for a message message from the receiver
    if let Ok(message) = zmq_recv.recv().await {
        // Try to read the three arguments from the message
        let mut cursor = Cursor::new(message.into_vec());
        let id = match cursor.read_u32::<LittleEndian>() {
            Ok(id) => id,
            _ => {
                // Return an error and exit
                error!("Communication read error: Invalid Event Id for ZMQ.");
                break; // end prematurely
            }
        };
        let data1 = match cursor.read_u32::<LittleEndian>() {
            Ok(data1) => data1,
            _ => {
                // Return an error and exit
                error!("Communication read error: Invalid second field for ZMQ.");
                break; // end prematurely
            }
        };
        let data2 = match cursor.read_u32::<LittleEndian>() {
            Ok(data2) => data2,
            _ => {
                // Return an error and exit
                error!("Communication read error: Invalid third field for ZMQ.");
                break; // end prematurely
            }
        };

        // Return the received event
        return vec!((ItemId::new_unchecked(id), data1, data2));

    // If nothing was received, return nothing
    } else {
        return Vec::new();
    }
}

// A helper function to write a single event from the zmq connection
#[cfg(feature = "zmq-comm")]
async fn write_to_zmq(zmq_send: &zeromq::PubSocket, event_id: ItemId, data1: u32, data2: u32) -> Result<()> {    
    // Create a byte array to fill with the data
    let mut bytes = Vec::new();

    // Add all three elements
    bytes.write_u32::<LittleEndian>(event_id.id())?;
    bytes.write_u32::<LittleEndian>(data1)?;
    bytes.write_u32::<LittleEndian>(data2)?;

    // Write the mssage to the zmq socket
    zmq_send.send(bytes).await;
}

