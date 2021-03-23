// Copyright (c) 2019-21 Decode Detroit
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

//! This module implements the connection to a Redis backup server to maintain
//! a backup of the system state. This handler syncs the system status, current
//! scene, and queue to the server. This module does nothing if a Redis server
//! is not connected.
//!
//! WARNING: This module assumes no authorized systems/operators are compromised.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::time::Duration;

// Import the failure features
use failure::Error;

// Imprt redis client library
use redis::{Commands, RedisResult};

// Import FNV HashSet
use fnv::FnvHashSet;

// Import YAML processing library
use serde_yaml;

/// An internal structure to store queued events
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct QueuedEvent {
    pub remaining: Duration, // the remaining time before the event is triggered
    pub event_id: ItemId,    // id of the event to launch
}

/// A structure which holds a reference to the Redis server (if it exists) and
/// syncronizes local data to and from the server.
///
/// # Notes
///
/// When created, the status handler will attempt to connect to the requested
/// redis server. If the status handler cannot make the connection, the status
/// handler will raise an error and return none.
///
pub struct BackupHandler {
    identifier: Identifier, // the identifier for this instance of the controller, if specified
    connection: Option<redis::Connection>, // the Redis connection, if it exists
    internal_send: InternalSend, // the update line for posting any warnings
    backup_items: FnvHashSet<ItemId>, // items currently backed up in the system
}

// Implement key features for the status handler
impl BackupHandler {
    /// A function to create and return a new backup handler.
    ///
    /// # Errors
    ///
    /// This function will raise an error if it is unable to connect to the
    /// Redis server provided.
    ///
    /// Like all BackupHandler functions and methods, this function will fail
    /// gracefully by notifying of any errors on the update line and returning
    /// None.
    ///
    pub async fn new(
        internal_send: InternalSend,
        identifier: Identifier,
        server_location: Option<String>,
    ) -> Result<BackupHandler, Error> {
        // If a server location was specified
        if let Some(location) = server_location {
            // Try to connect to the Redis server
            if let Ok(client) = redis::Client::open(location.as_str()) {
                // Try to get a copy of the Redis connection
                if let Ok(connection) = client.get_connection() {
                    // Return the new backup handler
                    return Ok(BackupHandler {
                        identifier,
                        connection: Some(connection),
                        internal_send,
                        backup_items: FnvHashSet::default(),
                    });

                // Indicate that there was a failure to connect to the server
                } else {
                    update!(err &internal_send => "Unable To Connect To Backup Server: {}.", location);
                }

            // Indicate that there was a failure to connect to the server
            } else {
                update!(err &internal_send => "Unable To Connect To Backup Server: {}.", location);
            }

            // Indicate failure
            return Err(format_err!(
                "Unable To Connect To Backup Server: {}.",
                location
            ));

        // If a location was not specified
        } else {
            // Return the new Backup handler without a redis connection
            return Ok(BackupHandler {
                identifier,
                connection: None,
                internal_send,
                backup_items: FnvHashSet::default(),
            });
        }
    }

    /// A method to backup the current scene of the system
    ///
    /// # Errors
    ///
    /// This function will raise an error if it is unable to connect to the
    /// Redis server.
    ///
    /// Like all BackupHandler functions and methods, this function will fail
    /// gracefully by notifying of any errors on the update line.
    ///
    pub async fn backup_current_scene(&mut self, current_scene: &ItemId) {
        // If the redis connection exists
        if let Some(mut connection) = self.connection.take() {
            // Try to copy the current scene to the server
            let result: RedisResult<bool> = connection.set(
                &format!("{}:current", self.identifier),
                &format!("{}", current_scene.id()),
            );

            // Unpack the result from the operation
            if let Err(..) = result {
                // Warn that it wasn't possible to update the current scene
                update!(err self.internal_send => "Unable To Backup Current Scene Onto Backup Server.");
            }
            
            // Put the connection back
            self.connection = Some(connection);
        }
    }

    /// A method to backup a status state on the backup server based on the
    /// provided status id and new state.
    ///
    /// # Note
    ///
    /// As the backup handler does not hold a copy of the status map, this
    /// method does not verify the validity of the new state in any way.
    /// It is expected that the calling module will perform this check.
    ///
    /// # Errors
    ///
    /// This function will raise an error if it is unable to connect to the
    /// Redis server.
    ///
    /// Like all BackupHandler functions and methods, this function will fail
    /// gracefully by notifying of any errors on the update line.
    ///
    pub async fn backup_status(&mut self, status_id: &ItemId, new_state: &ItemId) {
        // If the redis connection exists
        if let Some(mut connection) = self.connection.take() {
            // Try to copy the state to the server
            let result: RedisResult<bool>;
            result = connection.set(
                &format!("{}:{}", self.identifier, status_id),
                &format!("{}", new_state.id()),
            );

            // Warn that the particular status was not set
            if let Err(..) = result {
                update!(warn &self.internal_send => "Unable To Backup Status Onto Backup Server: {}.", status_id);

            // Otherwise, add the id to the backup items
            } else {
                self.backup_items.insert(status_id.clone());
            }
            
            // Put the connection back
            self.connection = Some(connection);
        }
    }

    /// A method to backup the event queue on the backup server based on the
    /// provided coming events
    ///
    /// # Note
    ///
    /// As the backup handler does not hold a copy of the configuration, this
    /// method does not verify the validity of the event queue in any way.
    /// It is expected that the calling module will perform this check.
    ///
    /// # Errors
    ///
    /// This function will raise an error if it is unable to connect to the
    /// Redis server.
    ///
    /// Like all BackupHandler functions and methods, this function will fail
    /// gracefully by notifying of any errors on the update line.
    ///
    pub async fn backup_events(&mut self, coming_events: Vec<ComingEvent>) {
        // If the redis connection exists
        if let Some(mut connection) = self.connection.take() {
            // Covert the coming events to queued events
            let mut queued_events = Vec::new();
            for event in coming_events {
                // Convert each event to a queued event
                if let Some(remaining) = event.remaining() {
                    queued_events.push(QueuedEvent {
                        remaining,
                        event_id: event.id(),
                    });
                }
            }

            // Try to serialize the coming events
            let event_string = match serde_yaml::to_string(&queued_events) {
                Ok(string) => string,
                Err(error) => {
                    update!(err &self.internal_send => "Unable To Parse Coming Events: {}", error);
                    
                    // Put the connection back
                    self.connection = Some(connection);
                    return;
                }
            };

            // Try to copy the event to the server
            let result: RedisResult<bool>;
            result = connection.set(&format!("{}:queue", self.identifier), &event_string);

            // Warn that the event queue was not set
            if let Err(..) = result {
                update!(warn &self.internal_send => "Unable To Backup Events Onto Backup Server.");
            }
            
            // Put the connection back
            self.connection = Some(connection);
        }
    }

    /// A function to reload an existing backup from the backup server. If the
    /// data exists, this function returns the existing backup data.
    ///
    /// # Errors
    ///
    /// This function will raise an error if it is unable to connect to the
    /// Redis server.
    ///
    /// Like all BackupHandler functions and methods, this function will fail
    /// gracefully by notifying of any errors on the update line and returning
    /// None.
    ///
    pub fn reload_backup(
        &mut self,
        mut status_ids: Vec<ItemId>,
    ) -> Option<(ItemId, Vec<(ItemId, ItemId)>, Vec<QueuedEvent>)> {
        // If the redis connection exists
        if let Some(mut connection) = self.connection.take() {
            // Check to see if there is an existing scene
            let result: RedisResult<String> =
                connection.get(&format!("{}:current", self.identifier));

            // If the current scene exists
            if let Ok(current_str) = result {
                // Try to read the exising event queue
                let mut queued_events: Vec<QueuedEvent> = Vec::new();
                let result: RedisResult<String> =
                    connection.get(&format!("{}:queue", self.identifier));

                // If something was received
                if let Ok(queue_string) = result {
                    // Try to parse the queue
                    if let Ok(events) = serde_yaml::from_str(queue_string.as_str()) {
                        queued_events = events;
                    }
                }

                // Compile a list of valid status pairs
                let mut status_pairs: Vec<(ItemId, ItemId)> = Vec::new();
                for status_id in status_ids.drain(..) {
                    // Try to read an existing status from the backup
                    let result: RedisResult<String> =
                        connection.get(&format!("{}:{}", self.identifier, status_id));

                    // If something was received
                    if let Ok(state_str) = result {
                        // Try to parse the current state id
                        if let Ok(state_id) = state_str.parse::<u32>() {
                            // Try to compose the id into an item
                            if let Some(new_state) = ItemId::new(state_id) {
                                // Add the status id and new state to the status pairs
                                status_pairs.push((status_id, new_state));
                            }
                        }
                    }
                }

                // Try to parse the current scene id
                if let Ok(current_id) = current_str.parse::<u32>() {
                    // Try to compose the id into an item
                    if let Some(current_scene) = ItemId::new(current_id) {
                        // Put the connection back
                        self.connection = Some(connection);
                        
                        // Return the current scene and status pairs
                        return Some((current_scene, status_pairs, queued_events));
                    }
                }
            }

            // Put the connection back
            self.connection = Some(connection);
        }

        // Silently return nothing if the connection does not exist or there was not a current scene
        None
    }
}

// Implement the drop trait for the backup handler struct.
impl Drop for BackupHandler {
    /// This method removes all the the existing statuses from the status server.
    ///
    /// # Errors
    ///
    /// This method will ignore any errors as it is called only when the backup
    /// connection is being closed.
    ///
    fn drop(&mut self) {
        // If the redis connection exists
        if let Some(mut connection) = self.connection.take() {
            // Try to delete the current scene if it exists (unable to manually specify types)
            let _: RedisResult<bool> = connection.del(&format!("{}:current", self.identifier));

            // Try to delete the queue if it exists
            let _: RedisResult<bool> = connection.del(&format!("{}:queue", self.identifier));

            // Try to delete all the items that were backed up
            for item in self.backup_items.drain() {
                let _: RedisResult<bool> = connection.del(&format!("{}:{}", self.identifier, item));
            }
        }
    }
}

// Tests of the status module
#[cfg(test)]
mod tests {
    use super::*;

    // FIXME Define tests of this module
    #[tokio::test]
    async fn backup_game() {
        // Import libraries for testing
        use crate::definitions::{Identifier, InternalSend, InternalUpdate};

        // Create a channel for receiving messages from the backup handler
        let (internal_send, mut rx) = InternalSend::new();

        // Create the backup handler
        let mut backup_handler = BackupHandler::new(
            internal_send,
            Identifier { id: None },
            Some("redis://127.0.0.1:6379".to_string()),
        ).await.unwrap();

        // Make sure there is no existing backup
        if let Some(_) = backup_handler.reload_backup(Vec::new()) {
            panic!("Backup already existed before beginning of the test.");
        }

        // Create the current scene and status pairs
        let current_scene = ItemId::new_unchecked(10);
        let status1  = ItemId::new_unchecked(11);
        let state1 = ItemId::new_unchecked(12);
        let status2 = ItemId::new_unchecked(13);
        let state2 = ItemId::new_unchecked(14);
        
        // Backup the current scene, statuses (unable to easily test coming events)
        backup_handler.backup_current_scene(&current_scene).await;
        backup_handler.backup_status(&status1, &state1).await;
        backup_handler.backup_status(&status2, &state2).await;

        // Reload the backup
        if let Some((reload_scene, statuses, _)) = backup_handler.reload_backup(vec!(status1, status2)) {
            assert_eq!(current_scene, reload_scene);
            assert_eq!(vec!((status1, state1), (status2, state2)), statuses);
        
        // If the backup doesn't exist, throw the error
        } else {
            panic!("Backup was not reloaded.");
        }

        // Make sure no messages were received (wait at most half a second)
        let empty: Vec<InternalUpdate> = Vec::new();
        test_vec!(=rx, empty);
    }
}
