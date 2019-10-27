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

//! A module to monitor send and receive event updates from the rest of the
//! system.
//!
//! Event updates are received on the provided input line and echoed to the
//! rest of the system. Updates from the system are passed back to the event
//! handler system via the event_send line.

// Define private submodules
mod comedy_comm;
mod dmx_comm;
mod zmq_comm;

// Import the relevant structures into the correct namespace
use self::comedy_comm::ComedyComm;
use self::dmx_comm::{DmxComm, DmxFade, DmxMap};
use self::zmq_comm::{EventToString, StringToEvent, ZmqBind, ZmqConnect, ZmqLookup};
use super::event_handler::event::EventUpdate;
use super::event_handler::item::{ItemId, COMM_ERROR, READ_ERROR};
use super::GeneralUpdate;

// Import standard library modules and traits
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

// Import the failure features
use failure::Error;

// Import program constants
use super::POLLING_RATE; // the polling rate for the system

/// An enum to specify the type of system connection.
///
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    /// A variant to connect with a ComedyComm serial port. This implementation
    /// assumes the serial connection uses the ComedyComm protocol.
    ComedySerial {
        path: PathBuf, // the location of the serial port
        baud: usize,   // the baud rate of the serial port
    },

    /// A variant to create a ZeroMQ connection. The connection type allows
    /// messages to be the sent and received. Received messages are echoed back
    /// to the send line so that all recipients will see the message.
    ZmqPrimary {
        send_path: PathBuf, // the location to bind the ZMQ sender
        recv_path: PathBuf, // the location to bind the ZMQ receiver
    },

    /// A variant to connect to an existing ZeroMQ connection over ZMQ.
    /// This connection presumes that a fully-functioning Minerva instance is
    /// is operating at the other end of the connection.
    ZmqSecondary {
        send_path: PathBuf, // the location to connect the ZMQ sender
        recv_path: PathBuf, // the location to connect the ZMQ receiver
    },

    /// A variant to create a ZeroMQ connection. This connection type translates
    /// event numbers to specific messages (one event for each message).
    /// Although messages can be sent and received, received messages are not
    /// echoed back to the system. NOT RECOMMENDED FOR NEW DESIGNS.
    ZmqTranslate {
        send_path: PathBuf,          // the location to bind the ZMQ sender
        recv_path: PathBuf,          // the location to bind the ZMQ receiver
        event_string: EventToString, // a map of event:string pairs
        string_event: StringToEvent, // a map of string:event pairs (not every event:string pair will appear in string:event and vice versa)
    },

    /// A variant to connect with a DMX serial port. This connection type allows
    /// messages to be the sent only.
    DmxSerial {
        path: PathBuf,              // the location of the serial port
        all_stop_dmx: Vec<DmxFade>, // a vector of dmx fades for all stop
        dmx_map: DmxMap,            // the map of event ids to dmx fades
    },
}

// Implement key connection type features
impl ConnectionType {
    /// An internal method to create a Live Connection from this Connection
    /// Type. This method estahblishes the connection to the underlying system.
    /// If the connection fails, it will return the Error.
    ///
    fn initialize(&self) -> Result<LiveConnection, Error> {
        // Switch between the different connection types
        match self {
            // Connect to a live version of the comedy serial port
            &ConnectionType::ComedySerial { ref path, ref baud } => {
                // Create the new comedy connection
                let connection = ComedyComm::new(path, baud.clone(), POLLING_RATE)?;
                Ok(LiveConnection::ComedySerial { connection })
            }

            // Connect to a live version of the zmq port
            &ConnectionType::ZmqPrimary {
                ref send_path,
                ref recv_path,
            } => {
                // Create the new zmq connection
                let connection = ZmqBind::new(send_path, recv_path)?;
                Ok(LiveConnection::ZmqPrimary { connection })
            }

            // Connect to a live version of the zmq port
            &ConnectionType::ZmqSecondary {
                ref send_path,
                ref recv_path,
            } => {
                // Create a new zmq to main connection
                let connection = ZmqConnect::new(send_path, recv_path)?;
                Ok(LiveConnection::ZmqSecondary { connection })
            }

            // Connect to a live version of the zmq port
            &ConnectionType::ZmqTranslate {
                ref send_path,
                ref recv_path,
                ref event_string,
                ref string_event,
            } => {
                // Create a new zmq to main connection
                let connection = ZmqLookup::new(
                    send_path,
                    recv_path,
                    event_string.clone(),
                    string_event.clone(),
                )?;
                Ok(LiveConnection::ZmqTranslate { connection })
            }

            // Connect to a live version of the DMX serial port
            &ConnectionType::DmxSerial {
                ref path,
                ref all_stop_dmx,
                ref dmx_map,
            } => {
                // Create the new dmx connection
                let connection = DmxComm::new(path, all_stop_dmx.clone(), dmx_map.clone())?;
                Ok(LiveConnection::DmxSerial { connection })
            }
        }
    }
}

/// A type to contain any number of connection types
///
pub type ConnectionSet = Vec<ConnectionType>;

/// An internal enum to hold the different types of a system connection.
/// Unlike the Connection Type, this structure holds a fully initialized
/// connection to the underlying system.
///
enum LiveConnection {
    /// A variant to connect with a ComedyComm serial port. This implementation
    /// assumes the serial connection uses the ComedyComm protocol.
    ComedySerial {
        connection: ComedyComm, // the comedy connection
    },

    /// A variant to create a ZeroMQ connection. The connection type allows
    /// messages to be the sent and received. Received messages are echoed back
    /// to the send line so that all recipients will see the message
    ZmqPrimary {
        connection: ZmqBind, // the zmq connection
    },

    /// A variant to connect to an existing ZeroMQ connection over ZMQ.
    /// This connection presumes that a fully-functioning Minerva instance is
    /// is operating at the other end of the connection.
    ZmqSecondary {
        connection: ZmqConnect, // the zmq connection
    },

    /// A variant to create a ZeroMQ connection. This connection type translates
    /// event numbers to specific messages (one event for each message).
    /// Although messages can be sent and received, received messages are not
    /// echoed back to the system. NOT RECOMMENDED FOR NEW DESIGNS.
    ZmqTranslate {
        connection: ZmqLookup, // the zmq connection
    },

    /// A variant to connect with a DMX serial port. The connection type allows
    /// messages to be the sent only.
    DmxSerial {
        connection: DmxComm, // the DMX serial connection
    },
}

// Implement event connection for LiveConnection
impl EventConnection for LiveConnection {
    /// The read event method
    fn read_events(&mut self) -> Vec<(ItemId, u32, u32)> {
        // Read from the interior connection
        match self {
            &mut LiveConnection::ComedySerial { ref mut connection } => connection.read_events(),
            &mut LiveConnection::ZmqPrimary { ref mut connection } => connection.read_events(),
            &mut LiveConnection::ZmqSecondary { ref mut connection } => connection.read_events(),
            &mut LiveConnection::ZmqTranslate { ref mut connection } => connection.read_events(),
            &mut LiveConnection::DmxSerial { ref mut connection } => connection.read_events(),
        }
    }

    /// The write event method (does not check duplicates)
    fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        // Write to the interior connection
        match self {
            &mut LiveConnection::ComedySerial { ref mut connection } => {
                connection.write_event(id, data1, data2)
            }
            &mut LiveConnection::ZmqPrimary { ref mut connection } => {
                connection.write_event(id, data1, data2)
            }
            &mut LiveConnection::ZmqSecondary { ref mut connection } => {
                connection.write_event(id, data1, data2)
            }
            &mut LiveConnection::ZmqTranslate { ref mut connection } => {
                connection.write_event(id, data1, data2)
            }
            &mut LiveConnection::DmxSerial { ref mut connection } => {
                connection.write_event(id, data1, data2)
            }
        }
    }

    /// The echo event method (checks for duplicates from recently read events)
    fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        // Echo events to the interior connection
        match self {
            &mut LiveConnection::ComedySerial { ref mut connection } => {
                connection.echo_event(id, data1, data2)
            }
            &mut LiveConnection::ZmqPrimary { ref mut connection } => {
                connection.echo_event(id, data1, data2)
            }
            &mut LiveConnection::ZmqSecondary { ref mut connection } => {
                connection.echo_event(id, data1, data2)
            }
            &mut LiveConnection::ZmqTranslate { ref mut connection } => {
                connection.echo_event(id, data1, data2)
            }
            &mut LiveConnection::DmxSerial { ref mut connection } => {
                connection.echo_event(id, data1, data2)
            }
        }
    }
}

/// An private enum to send broadcast events to the system connection
///
enum ConnectionUpdate {
    /// A variant to indicate an event should be broadcast
    ///
    Broadcast(ItemId, Option<u32>),

    /// A variant to indicate that the connection process should stop
    Stop,
}

/// A structure to handle all the input and output with the rest of the system.
///
pub struct SystemConnection {
    general_update: GeneralUpdate, // sending structure for new events from the system
    connection_send: Option<mpsc::Sender<ConnectionUpdate>>, // receiving structure for new events from the program
                                                             //connection: Option<LiveConnection>, // an element that implements both read and write
}

// Implement key Logger struct features
impl SystemConnection {
    /// A function to create a new system connection instance.
    ///
    /// This function requires a general_send line for passing events from the
    /// system back to the event handler.
    ///
    /// # Errors
    ///
    /// This function will raise an error if a connection type was provided and
    /// it was unable to connect to the underlying system.
    ///
    /// Like all SystemInterface functions and methods, this function will fail
    /// gracefully by warning the user and returning a default system connection.
    ///
    pub fn new(
        general_update: GeneralUpdate,
        connections: Option<(ConnectionSet, ItemId)>,
    ) -> SystemConnection {
        // Create an empty system connection
        let mut system_connection = SystemConnection {
            general_update,
            connection_send: None,
        };

        // Try to update the system connection using the provided connection type(s)
        system_connection.update_system_connection(connections);

        // Return the system connection
        system_connection
    }

    /// A method to update the system connection type. This method returns false
    /// if it was unable to connect to the underlying system and warns the user.
    ///
    pub fn update_system_connection(
        &mut self,
        connections: Option<(ConnectionSet, ItemId)>,
    ) -> bool {
        // Close the existing connection, if it exists
        if let Some(ref conn_send) = self.connection_send {
            conn_send.send(ConnectionUpdate::Stop).unwrap_or(());
        }

        // Reset the connection
        self.connection_send = None;

        // Check to see if there is a provided connection set
        if let Some((conn_set, identifier)) = connections {
            // Initialize the system connections
            let mut live_connections = Vec::new();
            for connection in conn_set {
                // Attempt to initialize each connection
                match connection.initialize() {
                    Ok(conn) => live_connections.push(conn),

                    // If it fails, warn the user FIXME pass the error upstream
                    Err(_) => {
                        update!(err self.general_update => "Unable To Initialize One Of The Underlying System Connections.");
                        return false;
                    }
                };
            }

            // Spin a new thread with the connection(s)
            let (conn_send, conn_recv) = mpsc::channel();
            let gen_update = self.general_update.clone();
            thread::spawn(move || {
                // Loop indefinitely
                SystemConnection::run_loop(live_connections, gen_update, conn_recv, identifier);
            });

            // Update the system connection
            self.connection_send = Some(conn_send);
            return true;
        }

        // Otherwise, leave the system disconnected
        true
    }

    /// A method to send messages between the underlying system and the program.
    ///
    pub fn broadcast(&mut self, new_event: ItemId, data: Option<u32>) {
        // Extract the connection, if it exists
        if let Some(ref mut conn) = self.connection_send {
            // Send the new event
            if let Err(_) = conn.send(ConnectionUpdate::Broadcast(new_event, data)) {
                update!(err &self.general_update => "Unable To Contact The Underlying System.");
            }
        }
    }

    /// An internal function to run a loop of the system connection
    ///
    fn run_loop(
        mut connections: Vec<LiveConnection>,
        gen_update: GeneralUpdate,
        conn_recv: mpsc::Receiver<ConnectionUpdate>,
        identifier: ItemId,
    ) {
        // Run the loop until there is an error or instructed to quit
        loop {
            // Save the start time of the loop
            let loop_start = Instant::now();

            // Read all events from the system connections
            let mut events = Vec::new();
            for connection in connections.iter_mut() {
                events.append(&mut connection.read_events());
            }

            // Read all the events from the list
            for (id, game_id, data2) in events.drain(..) {
                // If there was a read error, notify the system
                if id == ItemId::new_unchecked(READ_ERROR) {
                    update!(err &gen_update => "There Was A Read Error.");

                // If there was a communication error on the network, notify the system
                } else if id == ItemId::new_unchecked(COMM_ERROR) {
                    update!(err &gen_update => "There Was A Communication Error.");

                // Echo all valid events back to the system
                } else {
                    // Echo the event to every connection
                    for connection in connections.iter_mut() {
                        connection
                            .echo_event(id.clone(), game_id.clone(), data2.clone())
                            .unwrap_or(());
                    }

                    // Verify the game id is correct
                    if identifier.id() == game_id {
                        // Create a new id and send it to the program
                        gen_update.send_nobroadcast(id); // FIXME Handle incoming data

                    // Otherwise send a notification of an incorrect game number
                    } else {
                        update!(warnevent &gen_update => id => "Game Id Does Not Match. Event Ignored.");
                    }
                }
            }

            // Send any new events to the system
            match conn_recv.try_recv() {
                // Send the new event
                Ok(ConnectionUpdate::Broadcast(id, data)) => {
                    // Translate the data to a placeholder, if necessary
                    let data2 = match data {
                        Some(number) => number,
                        None => 0,
                    };
                
                    // Try to send the new event to every connection
                    for connection in connections.iter_mut() {
                        // Catch any write errors
                        if let Err(_) = connection.write_event(id, identifier.id(), data2) {
                            // Wait a little bit and try again
                            thread::sleep(Duration::from_millis(POLLING_RATE));
                            if let Err(_) = connection.write_event(id, identifier.id(), data2) {
                                // If failed twice in a row, notify the system
                                update!(err &gen_update => "Unable To Contact The Underlying System.");
                            }
                        }
                    }
                }

                // Quit when instructed or when there is an error
                Ok(ConnectionUpdate::Stop) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,

                // Otherwise continue
                _ => (),
            }

            // Make sure that some time elapses in each loop
            if Duration::from_millis(POLLING_RATE) > loop_start.elapsed() {
                thread::sleep(Duration::from_millis(POLLING_RATE));
            }
        }
    }
}

/// Define the EventConnection Trait
///
/// This is a convience trait to standardize reading from and writing to the
/// event connection across all event connection types.
///
pub trait EventConnection {
    /// The read event method
    fn read_events(&mut self) -> Vec<(ItemId, u32, u32)>;

    /// The write event method (does not check duplicates)
    fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error>;

    /// The echo event method (checks for duplicates from recently read events)
    fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error>;
}

// Tests of the system connection module
#[cfg(test)]
mod tests {
    use super::*;

    // FIXME Define tests of this module
    #[test]
    fn test_system_connection() {
        // FIXME: Implement this
        unimplemented!();
    }
}
