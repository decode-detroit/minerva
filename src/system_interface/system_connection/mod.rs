// Copyright (c) 2019-2024 Decode Detroit
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
#[cfg(not(target_os = "windows"))]
mod mercury;
#[cfg(target_os = "windows")]
mod mercury_windows;
mod zmq;

// Import crate definitions
use crate::definitions::*;

// Import other definitions
#[cfg(not(target_os = "windows"))]
use self::mercury::Mercury;
#[cfg(target_os = "windows")]
use self::mercury_windows::Mercury;
use self::zmq::{ZmqBind, ZmqConnect};

// Import standard library features
use std::time::Duration;

// Import the tokio and tokio serial features
use tokio::sync::mpsc;
use tokio::time::sleep;

// Import tracing features
use tracing::{error, info, warn};

// Import anyhow features
use anyhow::Result;

// Define modeule constants
const RETRY_DELAY: u64 = 100; // the write retry delay for the connections in ms

// Define the a helper type for returning events
type EventWithData = (ItemId, u32, u32);

// A helper trait to consistently define checksums for an event with data
trait Checksum {
    fn checksum(&self) -> u32;
}

// Implement simple checksum functionality for the basic type
impl Checksum for (u32, u32, u32) {
    fn checksum(&self) -> u32 {
        self.0 ^ self.1 ^ self.2
    }
}

// Implement simple checksum functionality for the helper type
impl Checksum for EventWithData {
    fn checksum(&self) -> u32 {
        (self.0.id(), self.1, self.2).checksum()
    }
}

// Implement key connection type features
impl ConnectionType {
    /// An internal method to create a Live Connection from this Connection
    /// Type. This method estahblishes the connection to the underlying system.
    /// If it succeeds, it will return the live connection and a short description
    /// of the connection. If the connection fails, it will return the Error.
    ///
    async fn initialize(&self) -> Result<(LiveConnection, String)> {
        // Switch between the different connection types
        match self {
            // Connect to a live version of the Mercury port
            &ConnectionType::Mercury {
                ref path,
                ref baud,
                ref use_checksum,
                ref allowed_events,
            } => {
                // Create the new Mercury connection
                let connection = Mercury::new(
                    path,
                    baud.clone(),
                    use_checksum.clone(),
                    allowed_events.clone(),
                    RETRY_DELAY,
                )?;
                Ok((
                    LiveConnection::Mercury { connection },
                    format!("Mercury Connection at {:?}", path),
                ))
            }

            // Connect to a live version of the zmq port
            &ConnectionType::ZmqPrimary {
                ref send_path,
                ref recv_path,
            } => {
                // Create the new zmq connection
                let connection = ZmqBind::new(send_path, recv_path).await?;
                Ok((
                    LiveConnection::ZmqPrimary { connection },
                    format!("ZMQ Primary at {:?} and {:?}", send_path, recv_path),
                ))
            }

            // Connect to a live version of the zmq port
            &ConnectionType::ZmqSecondary {
                ref send_path,
                ref recv_path,
            } => {
                // Create a new zmq to main connection
                let connection = ZmqConnect::new(send_path, recv_path).await?;
                Ok((
                    LiveConnection::ZmqSecondary { connection },
                    format!("ZMQ Secondary at {:?} and {:?}", send_path, recv_path),
                ))
            }
        }
    }
}

/// An internal enum to hold the different types of a system connection.
/// Unlike the Connection Type, this structure holds a fully initialized
/// connection to the underlying system.
///
enum LiveConnection {
    /// A variant to connect with a Mercury serial port. This implementation
    /// assumes the serial connection uses the Mercury protocol.
    Mercury {
        connection: Mercury, // the Mercury serial connection
    },

    /// A variant to create a ZeroMQ connection. This connection type allows
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
}

// Implement event connection for LiveConnection
impl EventConnection for LiveConnection {
    /// The read event method
    async fn read_event(&mut self) -> Option<EventWithData> {
        // Read from the interior connection
        match self {
            &mut LiveConnection::Mercury { ref mut connection } => connection.read_event().await,
            &mut LiveConnection::ZmqPrimary { ref mut connection } => connection.read_event().await,
            &mut LiveConnection::ZmqSecondary { ref mut connection } => {
                connection.read_event().await
            }
        }
    }

    /// The write event method (does not check duplicates)
    async fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()> {
        // Write to the interior connection
        match self {
            &mut LiveConnection::Mercury { ref mut connection } => {
                connection.write_event(id, data1, data2).await
            }
            &mut LiveConnection::ZmqPrimary { ref mut connection } => {
                connection.write_event(id, data1, data2).await
            }
            &mut LiveConnection::ZmqSecondary { ref mut connection } => {
                connection.write_event(id, data1, data2).await
            }
        }
    }

    /// The echo event method (checks for duplicates from recently read events)
    async fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()> {
        // Echo events to the interior connection
        match self {
            &mut LiveConnection::Mercury { ref mut connection } => {
                connection.echo_event(id, data1, data2).await
            }
            &mut LiveConnection::ZmqPrimary { ref mut connection } => {
                connection.echo_event(id, data1, data2).await
            }
            &mut LiveConnection::ZmqSecondary { ref mut connection } => {
                connection.echo_event(id, data1, data2).await
            }
        }
    }

    /// The process pending method
    async fn process_pending(&mut self) -> bool {
        // Process any pending writes
        match self {
            &mut LiveConnection::Mercury { ref mut connection } => {
                connection.process_pending().await
            }
            &mut LiveConnection::ZmqPrimary { ref mut connection } => {
                connection.process_pending().await
            }
            &mut LiveConnection::ZmqSecondary { ref mut connection } => {
                connection.process_pending().await
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

    /// A variant to indicate an event should be echoed
    ///
    Echo(ItemId, u32, u32),

    /// A variant to indicate that the connection process should stop
    Stop,
}

/// A structure to handle all the input and output with the rest of the system.
///
pub struct SystemConnection {
    internal_send: InternalSend, // structure to send events from the connections
    connection_senders: Vec<mpsc::Sender<ConnectionUpdate>>, // structure to forward events from the main program
}

// Implement key Logger struct features
impl SystemConnection {
    /// A function to create a new system connection instance.
    ///
    /// This function requires a general update line for passing events from the
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
    pub async fn new(
        internal_send: InternalSend,
        connections: Option<(ConnectionSet, Identifier)>,
    ) -> SystemConnection {
        // Create an empty system connection
        let mut system_connection = SystemConnection {
            internal_send,
            connection_senders: Vec::new(),
        };

        // Try to update the system connection using the provided connection type(s)
        system_connection
            .update_system_connections(connections)
            .await;

        // Return the system connection
        system_connection
    }

    /// A method to update and replace all the system connections
    ///
    pub async fn update_system_connections(
        &mut self,
        connections: Option<(ConnectionSet, Identifier)>,
    ) {
        // Close the existing connections, if they exists
        for conn_send in self.connection_senders.iter() {
            conn_send.send(ConnectionUpdate::Stop).await.unwrap_or(());
        }

        // Reset the connections
        self.connection_senders = Vec::new();

        // Check to see if there is a provided connection set
        if let Some((connection_set, identifier)) = connections {
            // Initialize each of the system connections
            for possible_connection in connection_set {
                // Create the connecting mpscs
                let (conn_send, conn_recv) = mpsc::channel(128);
                let internal_send = self.internal_send.clone();
                let identifier_clone = identifier.clone();

                // Save the sender
                self.connection_senders.push(conn_send);

                // Spin off a thread for each connection
                tokio::spawn(async move {
                    // Try to initialize the connection
                    match possible_connection.initialize().await {
                        // If successful, wait on the thread
                        Ok((connection, description)) => {
                            // Note the new connection
                            info!("System connection established: {}.", description);

                            // Run the connection loop
                            SystemConnection::run_loop(
                                connection,
                                internal_send,
                                conn_recv,
                                identifier_clone,
                            )
                            .await;
                        }

                        // If it fails, warn the user
                        Err(e) => error!("System connection error: {}.", e),
                    }
                });
            }
        } // Otherwise, leave the system disconnected
    }

    /// A method to send events to the system connections
    ///
    pub async fn broadcast(&mut self, new_event: ItemId, data: Option<u32>) {
        // Iterate through the connnections, if they exist
        for ref sender in self.connection_senders.iter() {
            // Send the new event
            if let Err(error) = sender
                .send(ConnectionUpdate::Broadcast(new_event, data))
                .await
            {
                error!("Unable to connect: {}.", error);
            }
        }
    }

    /// A method to echo events to the system connections
    ///
    pub async fn echo(&mut self, new_event: ItemId, data1: u32, data2: u32) {
        // Iterate through the connnections, if they exist
        for ref sender in self.connection_senders.iter() {
            // Send the echoed event
            if let Err(error) = sender
                .send(ConnectionUpdate::Echo(new_event, data1, data2))
                .await
            {
                error!("Unable to connect: {}.", error);
            }
        }
    }

    /// An internal function to run a loop of the system connection
    ///
    async fn run_loop(
        mut connection: LiveConnection,
        internal_send: InternalSend,
        mut conn_recv: mpsc::Receiver<ConnectionUpdate>,
        identifier: Identifier,
    ) {
        // Run the loop until there is an error or instructed to quit
        loop {
            // If there are still pending events on the connection
            if connection.process_pending().await {
                // Only wait <retry delay> for any updates
                tokio::select! {
                    // If there are new events received
                    possible_event = connection.read_event() => {
                        // See if we got an event
                        if let Some((id, game_id, data2)) = possible_event {
                            // Echo the event to all the connections
                            internal_send.send_echo(id, game_id, data2).await;

                            // If an identifier was specified
                            if let Some(identity) = identifier.id {
                                // Verify the game id is correct
                                if identity == game_id {
                                    // Send the event to the program FIXME Handle incoming data
                                    internal_send.send_event(id, true, false).await; // don't broadcast

                                // Otherwise send a notification of an incorrect game number
                                } else {
                                    // Format the warning string
                                    warn!("Game Id does not match. Event ignored ({}).", id);
                                }

                            // Otherwise, send the event to the program
                            } else {
                                internal_send.send_event(id, true, false).await; // don't broadcast
                            }
                        }
                    }

                    // Process any new events from the system
                    update = conn_recv.recv() => {
                        match update {
                            // Send the new event
                            Some(ConnectionUpdate::Broadcast(id, data)) => {
                                // Use the identifier or zero for the game id
                                let game_id = identifier.id.unwrap_or(0);

                                // Translate the data to a placeholder, if necessary
                                let data2 = data.unwrap_or(0);

                                // Catch any write errors
                                if let Err(error1) = connection.write_event(id, game_id, data2).await {
                                    // Report the error
                                    error!("Communication error: {}", error1);

                                    // Wait a little bit and try again
                                    sleep(Duration::from_millis(RETRY_DELAY)).await;
                                    if let Err(error2) = connection.write_event(id, game_id, data2).await {
                                        // Report the error
                                        error!("Persistent communication error: {}", error2);
                                    }
                                }
                            }

                            // Send the echoed event
                            Some(ConnectionUpdate::Echo(id, data1, data2)) => {
                                // Catch any echo errors
                                if let Err(error1) = connection.echo_event(id, data1, data2).await {
                                    // Report the error
                                    error!("Communication error: {}", error1);

                                    // Wait a little bit and try again
                                    sleep(Duration::from_millis(RETRY_DELAY)).await;
                                    if let Err(error2) = connection.echo_event(id, data1, data2).await {
                                        // Report the error
                                        error!("Persistent communication error: {}", error2);
                                    }
                                }
                            }

                            // Quit when instructed or when there is an error
                            Some(ConnectionUpdate::Stop) => break,
                            None => break,
                        }
                    }

                    // Wait the appropriate polling rate between process pending updates
                    _ = sleep(Duration::from_millis(RETRY_DELAY)) => (), // loop again
                }

            // Otherwise, if there are no pending events
            } else {
                // Wait indefinitely
                tokio::select! {
                    // If there is a new event received
                    possible_event = connection.read_event() => {
                        // See if we got an event
                        if let Some((id, game_id, data2)) = possible_event {
                            // Echo the event to all the connections
                            internal_send.send_echo(id, game_id, data2).await;

                            // If an identifier was specified
                            if let Some(identity) = identifier.id {
                                // Verify the game id is correct
                                if identity == game_id {
                                    // Send the event to the program FIXME Handle incoming data
                                    internal_send.send_event(id, true, false).await; // don't broadcast

                                // Otherwise send a notification of an incorrect game number
                                } else {
                                    // Format the warning string
                                    warn!("Game Id does not match. Event ignored ({}).", id);
                                }

                            // Otherwise, send the event to the program
                            } else {
                                internal_send.send_event(id, true, false).await; // don't broadcast
                            }
                        }
                    }

                    // Process any new events from the system
                    update = conn_recv.recv() => {
                        match update {
                            // Send the new event
                            Some(ConnectionUpdate::Broadcast(id, data)) => {
                                // Use the identifier or zero for the game id
                                let game_id = identifier.id.unwrap_or(0);

                                // Translate the data to a placeholder, if necessary
                                let data2 = data.unwrap_or(0);

                                // Catch any write errors
                                if let Err(error1) = connection.write_event(id, game_id, data2).await {
                                    // Report the error
                                    error!("Communication error: {}", error1);

                                    // Wait a little bit and try again
                                    sleep(Duration::from_millis(RETRY_DELAY)).await;
                                    if let Err(error2) = connection.write_event(id, game_id, data2).await {
                                        // Report the error
                                        error!("Persistent communication error: {}", error2);
                                    }
                                }
                            }

                            // Send the echoed event
                            Some(ConnectionUpdate::Echo(id, data1, data2)) => {
                                // Catch any echo errors
                                if let Err(error1) = connection.echo_event(id, data1, data2).await {
                                    // Report the error
                                    error!("Communication error: {}", error1);

                                    // Wait a little bit and try again
                                    sleep(Duration::from_millis(RETRY_DELAY)).await;
                                    if let Err(error2) = connection.echo_event(id, data1, data2).await {
                                        // Report the error
                                        error!("Persistent communication error: {}", error2);
                                    }
                                }
                            }

                            // Quit when instructed or when there is an error
                            Some(ConnectionUpdate::Stop) => break,
                            None => break,
                        }
                    }
                }
            }
        }
    }
}

/// Define the EventConnection Trait
///
/// This is a convience trait to standardize reading from and writing to the
/// event connection across all event connection types.
///
trait EventConnection {
    /// A method to read any new events from the connection. This implementation
    /// should await until new information is available and return an event with
    /// data if one was found.
    async fn read_event(&mut self) -> Option<EventWithData>;

    /// A method to write events to the connection. This implementation should
    /// not check duplicate messages received on this connection.
    async fn write_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()>;

    /// A method to echo events to this connection. This method should ensure that
    /// recently-read events are removed from the queue before sending. This method
    /// can assume that read events will be echoed exactly once.
    async fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<()>;

    /// A method to check for pending writes and process them if they exist.
    /// This method returns true if there are still pending writes.
    async fn process_pending(&mut self) -> bool;
}
