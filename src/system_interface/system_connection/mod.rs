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
//!
//! # TODO
//!
//! * Abstract the system connection so that it can accept any number of connections
//!


// Define private submodules
mod comedy_comm;

// Import the relevant structures into the correct namespace
use self::comedy_comm::ComedyComm;
use super::event_handler::item::{ItemId, COMM_ERROR, READ_ERROR};
use super::event_handler::event::EventUpdate;
use super::GeneralUpdate;

// Import standard library modules and traits
use std::thread;
use std::sync::mpsc;
use std::time::Duration;
use std::path::PathBuf;

// Import the ZMQ C-bindings
extern crate zmq;
use self::zmq::{Context, Socket};

// Import program constants
use super::POLLING_RATE; // the polling rate for the system


/// An enum to specify the type of system connection.
///
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    
    /// A dummy variant to use for debugging
    Dummy,
    
    /// A variant to connect with a ComedyComm serial port. This implementation
    /// assumes the serial connection uses the ComedyComm protocol.
    ComedySerial {
        path: PathBuf, // the location of the serial port
        baud: usize, // the baud rate of the serial port
    },
    
    /// A variant to connect with a ZeroMQ connection. The connection type allows
    /// messages to be the sent and received. Received messages are echoed back
    /// to the send line so that all recipients will see the message
    ZmqConnection {
        send_path: PathBuf, // the location to bind the ZMQ sender
        recv_path: PathBuf, // the location to bind the ZMQ receiver
    },
    
    /// A variant to connect to the main server system over a ZeroMQ connection.
    /// This connection presumes that a fully-functioning Minerva instance is
    /// is operating at the other end of the connection.
    ZmqToMain {
        send_path: PathBuf, // the location to connect the ZMQ sender
        recv_path: PathBuf, // the location to connect the ZMQ receiver
    },
    
    /// A variant to connect with a ComedyComm serial port in combination with
    /// a ZeroMQ sender and receiver. The implementation assumes the serial
    /// connection uses the ComedyComm protocol. Received messages are echoed
    // to the other connection so that the full system is aware of the state.
    SerialAndZmq {
        serial_path: PathBuf, // the location of the serial port
        baud: usize, // the baud rate of the serial port
        zmq_send_path: PathBuf, // the location of the ZMQ sender
        zmq_recv_path: PathBuf, // the location of the ZMQ receiver
    },
}

// Implement key connection type features
impl ConnectionType {
    
    /// An internal method to create a Live Connection from this Connection
    /// Type. This method estahblishes the connection to the underlying system.
    /// If the connection fails, it will return None.
    ///
    fn initialize(&self) -> Option<LiveConnection> {
    
        // Switch between the different connection types
        match self {
            
            // Connect a dummy live version
            &ConnectionType::Dummy => Some(LiveConnection::Dummy),
            
            // Connect to a live version of the comedy serial port
            &ConnectionType::ComedySerial { ref path, ref baud } => {
            
                // Create the new comedy connection
                let connection = match ComedyComm::new(path, baud.clone(), POLLING_RATE) {
                    Some(conn) => conn,
                    None => return None,
                };
                
                // Return a new live version
                Some(LiveConnection::ComedySerial { connection })
            },
            
            // Connect to a live version of the zmq port
            &ConnectionType::ZmqConnection { ref send_path, ref recv_path } => {
                
                // Create the new ZMQ sending socket
                let context = Context::new();
                let zmq_send = match context.socket(zmq::PUB) {
                    Ok(socket) => socket,
                    _ => return None,
                };
                
                // Bind to a new ZMQ send path
                if let Err(_) = zmq_send.bind(send_path.to_str().unwrap_or("")) {
                    return None;
                }
                                
                // Create the new ZMQ receiving socket
                let zmq_recv = match context.socket(zmq::SUB) {
                    Ok(socket) => socket,
                    _ => return None,
                };
                
                // Set the socket timeout and subscribe to all messages
                if let Err(_) = zmq_recv.set_rcvtimeo(POLLING_RATE as i32) {
                    return None;
                }
                if let Err(_) = zmq_recv.set_subscribe(&[]) {
                    return None;
                }
                
                // Bind to a new ZMQ receive path
                if let Err(_) = zmq_recv.bind(recv_path.to_str().unwrap_or("")) {
                    return None;
                }
                
                // Return a new live version
                Some(LiveConnection::ZmqConnection { zmq_send, zmq_recv })
            },
            
            // Connect to a live version of the zmq port
            &ConnectionType::ZmqToMain { ref send_path, ref recv_path } => {
                
                // Create the new ZMQ sending socket
                let context = Context::new();
                let zmq_send = match context.socket(zmq::PUB) {
                    Ok(socket) => socket,
                    _ => return None,
                };
                
                // Connect to the existing ZMQ send path
                if let Err(_) = zmq_send.connect(send_path.to_str().unwrap_or("")) {
                    return None;
                }
                                
                // Create the new ZMQ receiving socket
                let zmq_recv = match context.socket(zmq::SUB) {
                    Ok(socket) => socket,
                    _ => return None,
                };
                
                // Set the socket timeout and subscribe to all messages
                if let Err(_) = zmq_recv.set_rcvtimeo(POLLING_RATE as i32) {
                    return None;
                }
                if let Err(_) = zmq_recv.set_subscribe(&[]) {
                    return None;
                }
                
                // Connect to the existing ZMQ receive path
                if let Err(_) = zmq_recv.connect(recv_path.to_str().unwrap_or("")) {
                    return None;
                }
                
                // Return a new live version
                Some(LiveConnection::ZmqToMain { zmq_send, zmq_recv, filter_events: Vec::new() })
            },
            
            // Connect to a live version of the comedy serial port and zmq port
            &ConnectionType::SerialAndZmq { ref serial_path, ref baud, ref zmq_send_path, ref zmq_recv_path } => {
            
                // Create the new comedy connection
                let connection = match ComedyComm::new(serial_path, baud.clone(), POLLING_RATE) {
                    Some(conn) => conn,
                    None => return None,
                };
                
                // Create the new ZMQ sending socket
                let context = Context::new();
                let zmq_send = match context.socket(zmq::PUB) {
                    Ok(socket) => socket,
                    _ => return None,
                };
                
                // Bind to the ZMQ path
                if let Err(_) = zmq_send.bind(zmq_send_path.to_str().unwrap_or("")) {
                    return None;
                }
                
                // Create the new ZMQ receiving socket
                let zmq_recv = match context.socket(zmq::SUB) {
                    Ok(socket) => socket,
                    _ => return None,
                };
                
                // Set the socket timeout and subscribe to all messages
                if let Err(_) = zmq_recv.set_rcvtimeo(POLLING_RATE as i32) {
                    return None;
                }
                if let Err(_) = zmq_recv.set_subscribe(&[]) {
                    return None;
                }
                
                // Bind to the ZMQ recv path
                if let Err(_) = zmq_recv.bind(zmq_recv_path.to_str().unwrap_or("")) {
                    return None;
                }
                
                // Return a new live version
                Some(LiveConnection::SerialAndZmq { connection, zmq_send, zmq_recv })
            },
        }
    }
}


/// An internal enum to hold the different types of a system connection.
/// Unlike the Connection Type, this structure holds a fully initialized
/// connection to the underlying system.
///
enum LiveConnection {
    
    /// A dummy variant to use for debugging
    Dummy,
    
    /// A variant to connect with a ComedyComm serial port. This implementation
    /// assumes the serial connection uses the ComedyComm protocol.
    ComedySerial {
        connection: ComedyComm, // the comedy connection
    },
    
    /// A variant to connect with a ZeroMQ connection. The connection type allows
    /// messages to be the sent and received. Received messages are echoed back
    /// to the send line so that all recipients will see the message
    ZmqConnection {
        zmq_send: Socket, // the zmq send connection
        zmq_recv: Socket, // the zmq receive connection
    },
    
    /// A variant to connect to the main server system over a ZeroMQ connection.
    /// This connection presumes that a fully-functioning Minerva instance is
    /// is operating at the other end of the connection.
    ZmqToMain {
        zmq_send: Socket, // the zmq send connection
        zmq_recv: Socket, // the zmq receive connection
        filter_events: Vec<(ItemId, u32, u32)>, // events to filter out
    },
    
    /// A variant to connect with a ComedyComm serial port in combination with
    /// a ZeroMQ sender and receiver. The implementation assumes the serial
    /// connection uses the ComedyComm protocol. Received messages are echoed
    // to the other connection so that the full system is aware of the state.
    SerialAndZmq {
        connection: ComedyComm, // the comedy connection
        zmq_send: Socket, // the zmq send connection
        zmq_recv: Socket, // the zmq receive connection
    },
}

// A helper function to read a single event from the zmq connection
fn read_from_zmq(zmq_recv: &zmq::Socket) -> Option<(ItemId, u32, u32)> {

    // Read the first component of the message
    let id;
    let data1;
    let data2;
    if let Ok(message) = zmq_recv.recv_msg(0) {
        
        // Try to convert the message
        id = match message.as_str().unwrap_or("").parse::<u32>() {
            Ok(new_data) => new_data,
            _ => return Some((ItemId::new_unchecked(READ_ERROR), 0, 0)),
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
            _ => return Some((ItemId::new_unchecked(READ_ERROR), 0, 0)),
        };
    
    // Notify the system of a read error
    } else {
        return Some((ItemId::new_unchecked(READ_ERROR), 0, 0));
    }
    
    // Read the third component of the message
    if let Ok(message) = zmq_recv.recv_msg(0) {
        
        // Try to convert the message
        data2 = match message.as_str().unwrap_or("").parse::<u32>() {
            Ok(new_data) => new_data,
            _ => return Some((ItemId::new_unchecked(READ_ERROR), 0, 0)),
        };
    
    // Notify the system of a read error
    } else {
        return Some((ItemId::new_unchecked(READ_ERROR), 0, 0));
    }
    
    // Return the received id
    return Some((ItemId::new_unchecked(id), data1, data2));
}

// Implement read event for all of the connection types
impl ReadEvents for LiveConnection {
    fn read_events(&mut self) -> Vec<(ItemId, u32, u32)> {
        
        // Switch between the different connection types
        match self {
            
            // For a dummy connection, do nothing except wait
            &mut LiveConnection::Dummy => {
                thread::sleep(Duration::from_millis(POLLING_RATE));
                Vec::new()
            },
        
            // For a usb serial connection, read from the connection
            &mut LiveConnection::ComedySerial { ref mut connection } => {
                connection.read_events()
            },
            
            // For a zmq connection, read incoming messages and echo them
            &mut LiveConnection::ZmqConnection { ref mut zmq_recv, ref mut zmq_send} => {
                
                // Read any events from the zmq connection
                let mut events = Vec::new();
                while let Some(event) = read_from_zmq(zmq_recv) {
                    events.push(event);
                }
                
                // Echo the incoming events to the rest of the system
                for &(ref id, ref data1, ref data2) in events.iter() {
                
                    // Send a multipart ZMQ message, formatted as strings (fail silently)
                    zmq_send.send_str(&id.id().to_string(), zmq::SNDMORE).unwrap_or(());
                    zmq_send.send_str(&data1.to_string(), zmq::SNDMORE).unwrap_or(());
                    zmq_send.send_str(&data2.to_string(), 0).unwrap_or(());
                }
                
                // Return the list of events
                events
            },
            
            // For a zmq to main connection, filter incoming messages for an echo
            &mut LiveConnection::ZmqToMain { ref mut zmq_recv, ref mut filter_events, .. } => {
            
                // Read any events from the zmq connection
                let mut events = Vec::new();
                while let Some((id, data1, data2)) = read_from_zmq(zmq_recv) {
                
                    // Filter each event before adding it to the list
                    let mut count = 0;
                    for &(ref filter_id, ref filter_data1, ref filter_data2) in filter_events.iter() {
                        
                        // If the event matches an event in the filter
                        if (id == *filter_id) && (data1 == *filter_data1) && (data2 == *filter_data2) {
                            break; // exit with the found event count
                        }
                        
                        // Increment the count
                        count = count + 1;
                    }
                    
                    // Filter the event and remove it from the filter
                    if count < filter_events.len() {
                        
                        // Remove that events from the filter
                        filter_events.remove(count);
                    
                    // Otherwise, add the event to the list
                    } else {
                    
                        // Add the new event to the list
                        events.push((id, data1, data2));
                    }
                }
                
                // Return the list of events
                events
            }
            
            // For a usb serial and zmq sender connection, read from the serial connection
            &mut LiveConnection::SerialAndZmq { ref mut connection, ref mut zmq_send, ref mut zmq_recv } => {
                
                // Read any events from the usb connection
                let mut events = connection.read_events();
                
                // Read any events from the zmq connection
                while let Some(event) = read_from_zmq(zmq_recv) {
                    events.push(event);
                }
                
                // Echo the incoming events to the rest of the system
                for &(ref id, ref data1, ref data2) in events.iter() {
                
                    // Send a multipart ZMQ message, formatted as strings (fail silently)
                    zmq_send.send_str(&id.id().to_string(), zmq::SNDMORE).unwrap_or(());
                    zmq_send.send_str(&data1.to_string(), zmq::SNDMORE).unwrap_or(());
                    zmq_send.send_str(&data2.to_string(), 0).unwrap_or(());
                    
                    // Send an event to the USB connection
                    connection.write_event(id.clone(), data1.clone(), data2.clone()).unwrap_or(());
                }
                
                // Return the list of events
                events
            },
        }
    }
}

// Implement write event for all of the connection types
impl WriteEvent for LiveConnection {
    
    // Implement the write function
    fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), ()> {
    
        // Switch between the different connection types
        match self {
            
            // For the dummy connection, do nothing
            &mut LiveConnection::Dummy => Ok(()),
          
            // For a usb serial connection, write to the connection
            &mut LiveConnection::ComedySerial { ref mut connection } => connection.write_event(id, data1, data2),
            
            // For a zmq connection, write to the zmq port
            &mut LiveConnection::ZmqConnection { ref mut zmq_send, .. } => {
                
                // Send a multipart ZMQ message, formatted as strings
                if let Err(_) = zmq_send.send_str(&id.id().to_string(), zmq::SNDMORE) { return Err(()); }
                if let Err(_) = zmq_send.send_str(&data1.to_string(), zmq::SNDMORE) { return Err(()); }
                if let Err(_) = zmq_send.send_str(&data2.to_string(), 0) { return Err(()); }
                
                // Indicate success
                Ok(())
            },
            
            // For a zmq to main connection, note outgoing messages to filter later
            &mut LiveConnection::ZmqToMain { ref mut zmq_send, ref mut filter_events, .. } => {
                
                // Send a multipart ZMQ message, formatted as strings
                if let Err(_) = zmq_send.send_str(&id.id().to_string(), zmq::SNDMORE) { return Err(()); }
                if let Err(_) = zmq_send.send_str(&data1.to_string(), zmq::SNDMORE) { return Err(()); }
                if let Err(_) = zmq_send.send_str(&data2.to_string(), 0) { return Err(()); }
                
                // Add the event to the filter
                filter_events.push((id, data1, data2));
                
                // Indicate success
                Ok(())
            }, 
            
            // For a usb serial and zmq sender connection, write to both sinks
            &mut LiveConnection::SerialAndZmq { ref mut connection, ref mut zmq_send, .. } => {
                
                // Send the serial connection
                connection.write_event(id, data1, data2)?;
                
                // Send a multipart ZMQ message, formatted as strings
                if let Err(_) = zmq_send.send_str(&id.id().to_string(), zmq::SNDMORE) { return Err(()); }
                if let Err(_) = zmq_send.send_str(&data1.to_string(), zmq::SNDMORE) { return Err(()); }
                if let Err(_) = zmq_send.send_str(&data2.to_string(), 0) { return Err(()); }
                
                // Indicate success
                Ok(())
            },
        }
    }
}


/// An private enum to send broadcast events to the system connection
///
enum ConnectionUpdate {
    
    /// A variant to indicate an event should be broadcast
    ///
    Broadcast(ItemId),
    
    /// A variant to indicate that the connection process should stop
    Stop,
}


/// A structure to handle all the input and output with the rest of the system.
///
pub struct SystemConnection {
    general_update: GeneralUpdate, // sending structure for new events from the system
    connection_send: Option<mpsc::Sender<ConnectionUpdate>> // receiving structure for new events from the program
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
    pub fn new(general_update: GeneralUpdate, connection: Option<(ConnectionType, ItemId)>) -> SystemConnection {

        // Check to see if there is a provided connection type
        match connection {
        
            // Try to process the provided connection type
            Some((conn_type, identifier)) => {
                
                // Try to initialize the system connection
                let connection = match conn_type.initialize() {
                    Some(conn) => conn,
                    
                    // If it fails, warn the user
                    None => {
                        update!(err &general_update => "Unable To Connect To The Underlying System.");
                        return SystemConnection { general_update, connection_send: None };
                    },
                };
                
                // Span a new thread with the connection
                let (conn_send, conn_recv) = mpsc::channel();
                let gen_update = general_update.clone();
                thread::spawn(move || {
                
                    // Loop indefinitely
                    SystemConnection::run_loop(connection, gen_update, conn_recv, identifier);
                });
                
                // Return the new system connection
                SystemConnection { general_update, connection_send: Some(conn_send) }
            },
            
            // Otherwise, return the default system connection
            None => {
                SystemConnection { general_update, connection_send: None }
            }
        }
    }
    
    /// A method to update the system connection type. This method returns false
    /// if it was unable to connect to the underlying system and warns the user.
    ///
    pub fn update_system_connection(&mut self, connection: Option<(ConnectionType, ItemId)>) -> bool {
    
        // Close the existing connection, if it exists
        if let Some(ref conn_send) = self.connection_send {
            conn_send.send(ConnectionUpdate::Stop).unwrap_or(());
        }
        
        // If there is a provided new connection type
        if let Some((conn_type, identifier)) = connection {
        
            // Try to initialize the system connection
            if let Some(connection) = conn_type.initialize() {
            
                // Span a new thread with the connection
                let (conn_send, conn_recv) = mpsc::channel();
                let gen_update = self.general_update.clone();
                thread::spawn(move || {
                
                    // Loop indefinitely
                    SystemConnection::run_loop(connection, gen_update, conn_recv, identifier);
                });
            
                // Update the connection send and indicate success
                self.connection_send = Some(conn_send);
                return true;
            
            // Otherwise, indicate a failure to connect
            } else {
                
                // Update the connection and indicate failure
                self.connection_send = None;
                update!(err &self.general_update => "Unable To Connect To Underlying System.");
                return false;
            }
        
        // Otherwise, leave the system disconnected
        } else {
            self.connection_send = None;
            return true;
        }
    }
    
    /// A method to send messages between the underlying system and the program.
    ///
    pub fn broadcast(&mut self, new_event: ItemId) {

        // Extract the connection, if it exists
        if let Some(ref mut conn) = self.connection_send {
            
            // Send the new event
            if let Err(_) = conn.send(ConnectionUpdate::Broadcast(new_event)) {
                update!(err &self.general_update => "Unable To Contact The Underlying System.");
            }
        }
    }
    
    /// An internal function to run a loop of the system connection
    ///
    fn run_loop(mut connection: LiveConnection, gen_update: GeneralUpdate, conn_recv: mpsc::Receiver<ConnectionUpdate>, identifier: ItemId) {
    
        // Run the loop until there is an error or instructed to quit
        loop {
    
            // Read from the system connection, if possible
            let mut events = connection.read_events();
            
            // Read all the events from the list
            for (id, game_id, _data2) in events.drain(..) {

                // If there was a read error, notify the system
                if id == ItemId::new_unchecked(READ_ERROR) {
                    gen_update.send_update(EventUpdate::Error(String::from("There Was A Read Error.")));
                    
                    // Wait the normal polling rate (to prevent eating the processor)
                    thread::sleep(Duration::from_millis(POLLING_RATE));

                // If there was a communication error on the network, notify the system
                } else if id == ItemId::new_unchecked(COMM_ERROR) {
                    gen_update.send_update(EventUpdate::Error(String::from("There Was A Communication Error.")));

                // Verify the game id is correct                
                } else if identifier.id() == game_id {
            
                    // Create a new id and send it
                    gen_update.send_nobroadcast(id);
                }
            }
            
            // Send any new events to the system
            match conn_recv.try_recv() {
                
                // Send the new event
                Ok(ConnectionUpdate::Broadcast(id)) => { 
                    
                    // Try to send the new event
                    if let Err(_) = connection.write_event(id, identifier.id(), 0) {
                    
                        // Wait a little bit and try again
                        thread::sleep(Duration::from_millis(POLLING_RATE));
                        if let Err(_) = connection.write_event(id, identifier.id(), 0) {
                            
                            // If failed twice in a row, notify the system
                            gen_update.send_update(EventUpdate::Error(String::from("Unable To Contact The Underlying System.")));
                        }
                    }
                },
                
                // Quit when instructed or when there is an error
                Ok(ConnectionUpdate::Stop) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
                
                // Otherwise continue
                _ => (),
            }
        }
    }
}


/// Define the ReadEvent Trait
///
/// This is a convience trait to standardize reading from the system connection
/// across all system connection types.
pub trait ReadEvents {
    fn read_events(&mut self) -> Vec<(ItemId, u32, u32)>;
}


/// Define the WriteEvent Trait
///
/// This is a convience trait to standardize reading from the system connection
/// across all system connection types.
pub trait WriteEvent {
    fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), ()>;
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

