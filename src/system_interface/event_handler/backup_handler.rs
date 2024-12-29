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
use std::time::{Duration, Instant};

// Import tracing features
use tracing::{error, info, warn};

// Imprt redis client library
use redis::{Commands, ConnectionLike, RedisResult};

// Import FNV HashSet and HashMap
use fnv::{FnvHashSet, FnvHashMap};

// Import YAML processing library
use serde_yaml;

/// A helper structure to hold the last update
///
#[derive(Clone, Debug, Serialize, Deserialize)]
struct LastUpdates {
    queue_update: Duration, // the time since the last update for the queue backup
    media_update: Duration, // the time since the last update for the media backup
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
    last_queue_update: Instant, // the time of the last update for the queue backup
    last_media_update: Instant, // the time of the last update for the media backup
    backup_items: FnvHashSet<ItemId>, // items currently backed up in the system
    dmx_universes: DmxUniverses, // the current final value of all DMX fades for each universe
    media_playlist: MediaPlaylist, // the current media playback for each channel
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
    pub async fn new(identifier: Identifier, server_location: Option<String>) -> Self {
        // If a server location was specified
        if let Some(location) = server_location {
            // Try to connect to the Redis server
            if let Ok(client) = redis::Client::open(location.as_str()) {
                // Try to get a copy of the Redis connection
                if let Ok(mut connection) = client.get_connection() {
                    // Set the snapshot settings
                    let result: RedisResult<redis::Value> = connection.req_command(
                        redis::Cmd::new()
                            .arg("CONFIG")
                            .arg("SET")
                            .arg("save")
                            .arg("60 1"),
                    );

                    // Unpack the result from the operation
                    if let Err(..) = result {
                        // Warn that it wasn't possible to update the current scene
                        error!("Unable to set Redis snapshot settings.");
                    }

                    // Return the new backup handler
                    return Self {
                        identifier,
                        connection: Some(connection),
                        last_queue_update: Instant::now(),
                        last_media_update: Instant::now(),
                        backup_items: FnvHashSet::default(),
                        dmx_universes: FnvHashMap::default(),
                        media_playlist: MediaPlaylist::default(),
                    };

                // Indicate that there was a failure to connect to the server
                } else {
                    error!("Unable to connect to backup server: {}.", location);
                }

            // Indicate that there was a failure to connect to the server
            } else {
                error!("Unable to connect to backup server: {}.", location);
            }
        }

        // If a location was not specified or the connection failed, return without a redis connection
        Self {
            identifier,
            connection: None,
            last_queue_update: Instant::now(),
            last_media_update: Instant::now(),
            backup_items: FnvHashSet::default(),
            dmx_universes: FnvHashMap::default(),
            media_playlist: MediaPlaylist::default(),
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
                error!("Unable to backup current scene onto backup server.");
            }

            // Backup the update times
            self.backup_last_update(&mut connection).await;

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
            let result: RedisResult<bool> = connection.set(
                &format!("{}:{}", self.identifier, status_id),
                &format!("{}", new_state.id()),
            );

            // Warn that the particular status was not set
            if let Err(..) = result {
                error!("Unable to backup status onto backup server: {}.", status_id);

            // Otherwise, add the id to the backup items
            } else {
                self.backup_items.insert(status_id.clone());
            }

            // Backup the update times
            self.backup_last_update(&mut connection).await;

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
                    error!("Unable to parse coming events: {}.", error);

                    // Put the connection back
                    self.connection = Some(connection);
                    return;
                }
            };

            // Try to copy the event to the server
            let result: RedisResult<bool> = connection.set(&format!("{}:queue", self.identifier), &event_string);

            // Alert that the event queue was not set
            if let Err(..) = result {
                error!("Unable to backup events onto backup server.");
            }

            // Save the new update time
            self.last_queue_update = Instant::now();

            // Backup the update times
            self.backup_last_update(&mut connection).await;

            // Put the connection back
            self.connection = Some(connection);
        }
    }

    /// A method to backup the dmx values on the backup server based on each
    /// provided dmx fade
    ///
    /// # Note
    ///
    /// As the backup handler does not hold a copy of the configuration, this
    /// method does not verify the validity of the dmx values in any way.
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
    pub async fn backup_dmx(&mut self, dmx_fade: DmxFade) {
        // If the redis connection exists
        if let Some(mut connection) = self.connection.take() {
            // Find an existing dmx universe
            if let Some(universe) = self.dmx_universes.get_mut(&dmx_fade.universe.unwrap_or(0)) {
                // Update the final value
                universe.set(dmx_fade.channel, dmx_fade.value);

            // Or create the universe if it doesn't exist
            } else {
                let mut universe = DmxUniverse::new();
                
                // Set the final value
                universe.set(dmx_fade.channel, dmx_fade.value);

                // And add it to the collection of universes
                self.dmx_universes.insert(dmx_fade.universe.unwrap_or(0), universe);
            }

            // Try to serialize the dmx universes and save them
            let dmx_string = match serde_yaml::to_string(&self.dmx_universes) {
                Ok(string) => string,
                Err(error) => {
                    error!("Unable to parse DMX universer: {}.", error);

                    // Put the connection back
                    self.connection = Some(connection);
                    return;
                }
            };

            // Try to copy the data to the server
            let result: RedisResult<bool> = connection.set(&format!("{}:dmx", self.identifier), &dmx_string);

            // Alert that the dmx status was not set
            if let Err(..) = result {
                error!("Unable to backup DMX onto backup server.");
            }

            // Backup the update times
            self.backup_last_update(&mut connection).await;

            // Put the connection back
            self.connection = Some(connection);
        }
    }

    /// A method to backup the currently playing media to the backup server.
    /// It assumes the media started playing as this function was called.
    ///
    /// # Note
    ///
    /// As the backup handler does not hold a copy of the configuration, this
    /// method does not verify the validity of the media cue values in any way.
    /// It is expected that the calling module will perform this check.
    ///
    /// The media interface only waits half a second for media to load before
    /// seeking to the corrent position of the media. This delay may not be
    /// sufficient for network-loaded media which can take several seconds
    /// to load. If the media takes too long to load, the media with resume
    /// playback from the start rather than its correct position.
    ///
    /// # Errors
    ///
    /// This function will raise an error if it is unable to connect to the
    /// Redis server.
    ///
    /// Like all BackupHandler functions and methods, this function will fail
    /// gracefully by notifying of any errors on the update line.
    ///
    pub async fn backup_media(&mut self, media_cue: MediaCue) {
        // If the redis connection exists
        if let Some(mut connection) = self.connection.take() {
            // Add the cue to the media playlist
            self.media_playlist.insert(
                media_cue.channel,
                MediaPlayback {
                    media_cue,
                    time_since: Duration::from_secs(0),
                },
            ); // replaces an existing media playback, if it exists

            // Try to serialize the media playlist
            let media_string = match serde_yaml::to_string(&self.media_playlist) {
                Ok(string) => string,
                Err(error) => {
                    error!("Unable to parse media playlist: {}.", error);

                    // Put the connection back
                    self.connection = Some(connection);
                    return;
                }
            };

            // Try to copy the data to the server
            let result: RedisResult<bool> = connection.set(&format!("{}:media", self.identifier), &media_string);

            // Alert that the media playlist was not set
            if let Err(..) = result {
                error!("Unable to backup media onto backup server.");
            }

            // Save the new update time
            self.last_media_update = Instant::now();

            // Backup the update times
            self.backup_last_update(&mut connection).await;

            // Put the connection back
            self.connection = Some(connection);
        }
    }

    /// A method to reload an existing backup from the backup server. If the
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
    ) -> Option<(
        ItemId,
        Vec<(ItemId, ItemId)>,
        Vec<QueuedEvent>,
        DmxUniverses,
        MediaPlaylist,
    )> {
        // If the redis connection exists
        if let Some(mut connection) = self.connection.take() {
            // Check to see if there is an existing scene
            let result: RedisResult<String> =
                connection.get(&format!("{}:current", self.identifier));

            // If the current scene exists
            if let Ok(current_str) = result {
                // Warn that existing data was found
                warn!("Detected lingering backup data. Reloading ...");

                // Try to read the last update times
                let mut last_updates = LastUpdates {
                    queue_update: Duration::from_secs(0),
                    media_update: Duration::from_secs(0),
                };
                let result: RedisResult<String> =
                    connection.get(&format!("{}:lastupdate", self.identifier));

                // If something was received
                if let Ok(update_string) = result {
                    // Try to parse the data
                    if let Ok(updates) = serde_yaml::from_str(update_string.as_str()) {
                        last_updates = updates;
                    }
                }

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

                // Update the timing for the media playlist
                info!(
                    "Adjusting event queue by {}.{:0>3}.",
                    last_updates.queue_update.as_secs(),
                    (last_updates.queue_update.as_millis() % 1000)
                );
                if queued_events.len() > 0 {
                    for event in queued_events.iter_mut() {
                        event.update(last_updates.queue_update);
                    }
                }

                // Try to read the existing dmx universe
                let mut dmx_universes = FnvHashMap::default();
                let result: RedisResult<String> =
                    connection.get(&format!("{}:dmx", self.identifier));

                // If something was received
                if let Ok(dmx_string) = result {
                    // Try to parse the data
                    if let Ok(universe) = serde_yaml::from_str(dmx_string.as_str()) {
                        dmx_universes = universe;
                    }
                }

                // Save the dmx universe
                self.dmx_universes = dmx_universes.clone();

                // Try to read the existing media cues
                let mut media_playlist = MediaPlaylist::default();
                let result: RedisResult<String> =
                    connection.get(&format!("{}:media", self.identifier));

                // If something was received
                if let Ok(media_string) = result {
                    // Try to parse the data
                    if let Ok(media) = serde_yaml::from_str(media_string.as_str()) {
                        media_playlist = media;
                    }
                }

                // Update the timing for the media playlist
                info!(
                    "Adjusting media playlist by {}.{:0>3}.",
                    last_updates.media_update.as_secs(),
                    (last_updates.media_update.as_millis() % 1000)
                );
                if media_playlist.len() > 0 {
                    for playback in media_playlist.values_mut() {
                        playback.update(last_updates.media_update);
                    }
                }

                // Save the media playlist
                self.media_playlist = media_playlist.clone();

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
                        return Some((
                            current_scene,
                            status_pairs,
                            queued_events,
                            dmx_universes,
                            media_playlist,
                        ));
                    }
                }
            }

            // Put the connection back
            self.connection = Some(connection);
        }

        // Silently return nothing if the connection does not exist or there was not a current scene
        None
    }

    /// A helper function to backup the last update times for the queue and media
    ///
    async fn backup_last_update(&mut self, connection: &mut redis::Connection) {
        // Create the last updates structure
        let last_updates = LastUpdates {
            queue_update: self.last_queue_update.elapsed(),
            media_update: self.last_media_update.elapsed(),
        };

        // Try to serialize the update times
        let update_string = match serde_yaml::to_string(&last_updates) {
            Ok(string) => string,
            Err(error) => {
                error!("Unable to parse update times {}.", error);
                return;
            }
        };

        // Try to copy the data to the server
        let result: RedisResult<bool> = connection.set(&format!("{}:lastupdate", self.identifier), &update_string);

        // Alert that the media playlist was not set
        if let Err(..) = result {
            error!("Unable to backup update times onto backup server.");
        }
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

            // Try to delete the last update backup if it exists
            let _: RedisResult<bool> = connection.del(&format!("{}:lastupdate", self.identifier));

            // Try to delete the queue if it exists
            let _: RedisResult<bool> = connection.del(&format!("{}:queue", self.identifier));

            // Try to delete the dmx backup if it exists
            let _: RedisResult<bool> = connection.del(&format!("{}:dmx", self.identifier));

            // Try to delete the media backup if it exists
            let _: RedisResult<bool> = connection.del(&format!("{}:media", self.identifier));

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

    // Test the backup module
    #[tokio::test]
    async fn backup_game() {
        // Import libraries for testing
        use crate::definitions::Identifier;

        // Create the backup handler
        let mut backup_handler = BackupHandler::new(
            Identifier { id: None },
            Some("redis://127.0.0.1:6379".to_string()),
        )
        .await;

        // Make sure there is no existing backup
        if backup_handler.reload_backup(Vec::new()).is_some() {
            panic!("Backup already existed before beginning of the test.");
        }

        // Create the current scene and status pairs
        let current_scene = ItemId::new_unchecked(10);
        let status1 = ItemId::new_unchecked(11);
        let state1 = ItemId::new_unchecked(12);
        let status2 = ItemId::new_unchecked(13);
        let state2 = ItemId::new_unchecked(14);

        // Backup the current scene, statuses, dmx (unable to easily test coming events)
        backup_handler.backup_current_scene(&current_scene).await;
        backup_handler.backup_status(&status1, &state1).await;
        backup_handler.backup_status(&status2, &state2).await;
        backup_handler
            .backup_dmx(DmxFade {
                universe: None,
                channel: 1,
                value: 255,
                duration: None,
            })
            .await;
        backup_handler
            .backup_dmx(DmxFade {
                universe: Some(0),
                channel: 3,
                value: 150,
                duration: None,
            })
            .await;
        backup_handler
            .backup_media(MediaCue {
                channel: 1,
                uri: "video.mp4".to_string(),
                loop_media: None,
            })
            .await;
        backup_handler
            .backup_media(MediaCue {
                channel: 1,
                uri: "new_video.mp4".to_string(),
                loop_media: None,
            })
            .await;

        // Reload the backup
        if let Some((reload_scene, statuses, _queue, dmx, media)) =
            backup_handler.reload_backup(vec![status1, status2])
        {
            assert_eq!(current_scene, reload_scene);
            assert_eq!(vec!((status1, state1), (status2, state2)), statuses);
            assert_eq!(255 as u8, dmx.get(&0).unwrap().get(1));
            assert_eq!(150 as u8, dmx.get(&0).unwrap().get(3));
            assert_eq!(
                MediaCue {
                    channel: 1,
                    uri: "new_video.mp4".to_string(),
                    loop_media: None
                },
                media.get(&1).unwrap().media_cue
            );

        // If the backup doesn't exist, throw the error
        } else {
            panic!("Backup was not reloaded.");
        }
    }
}
