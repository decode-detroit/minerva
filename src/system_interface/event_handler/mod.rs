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

//! A module that holds the state machine of the program. This module loads
//! the configuration from a file, holds the configuration in memory, and
//! and handles all changes to the current state as well as queuing upcoming
//! events. This module also processes the events and sends updtes to the rest
//! of the program.

// Define private submodules
mod backup_handler;
mod config;
mod dmx_interface;
mod media_interface;
mod queue;

// Import crate definitions
use crate::definitions::*;

// Import other definitions
use self::backup_handler::BackupHandler;
use self::config::Config;
use self::dmx_interface::DmxInterface;
use self::media_interface::MediaInterface;
use self::queue::Queue;

// Import standard library features
use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::Duration;

// Import Chrono features
use chrono::NaiveDateTime;

// Import Tokio features
use tokio::fs::File;
use tokio::time::sleep;

// Import tracing features
use tracing::{error, info};

// Import anyhow features
use anyhow::Result;

/// A structure to manage all event triggering and internal event operations
/// inside the program. This structure allows the main program to be agnostic
/// to the current configuration of the program and the available events.
///
pub struct EventHandler {
    queue: Queue,                          // current event queue
    dmx_interface: Option<DmxInterface>,   // the dmx interface, if available
    media_interfaces: Vec<MediaInterface>, // list of available media interfaces
    config: Config,                        // current configuration
    config_path: PathBuf,                  // current configuration path
    index_access: IndexAccess,             // access point to the item index
    backup: BackupHandler,                 // current backup server
}

// Implement the event handler functions
impl EventHandler {
    /// A funtion to create a new event handler.
    ///
    /// This function takes a configuration filename and a general send line.
    /// This general send line will receive all updates from the event handler
    /// module including errors, warnings, and normal system updates (like
    /// currently playing events). The main program is expected to parse these
    /// updates appropriately. See the event::EventUpdate enum for more detail
    /// on the possible update types.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the provided configuration
    /// filename was impossible to find, impossible to parse, or raised a fatal
    /// error. If there is an inconsistency in the configuration file, this
    /// function will only raise a warning.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line returning None.
    /// If the log failures flag is set to false, this function will not notify
    /// of a failure to connect to the configuration file.
    ///
    pub async fn new(
        config_path: Option<PathBuf>,
        index_access: IndexAccess,
        style_access: StyleAccess,
        internal_send: InternalSend,
        interface_send: InterfaceSend,
        limited_send: LimitedSend,
        log_failure: bool,
    ) -> Result<Self> {
        // If a file was specified
        let mut config;
        let mut resolved_path;
        if let Some(path) = config_path {
            // Save the resolved path
            resolved_path = path.clone();

            // Attempt to open the configuration file
            let config_file = match File::open(path.clone()).await {
                Ok(file) => file,
                Err(_) => {
                    // Only log failure if the flag is set
                    if log_failure {
                        error!("Unable to open configuration file.");
                    }
                    return Err(anyhow!("Unable to open configuration file."));
                }
            };

            // Attempt to process the configuration file
            config = Config::from_config(
                index_access.clone(),
                style_access,
                interface_send.clone(),
                limited_send.clone(),
                config_file,
            )
            .await?;

        // Otherwise, create an empty configuration
        } else {
            config = Config::new(
                index_access.clone(),
                style_access,
                interface_send.clone(),
                limited_send.clone(),
            )
            .await;

            // Set the path to "default.yaml" in the current directory
            resolved_path = env::current_dir().unwrap_or(PathBuf::new());
            resolved_path.push("default.yaml");
        }

        // Attempt to create the dmx interface, if specified
        let mut dmx_interface = None;
        if let Some(path) = config.get_dmx_path() {
            // Try to connect to the interface
            if let Ok(interface) = DmxInterface::new(path.as_path()) {
                dmx_interface = Some(interface);

            // Otherwise, report the error
            } else {
                error!("Unable to initialize the DMX interface.");
            }
        }

        // Attempt to create any media interfaces
        let mut media_interfaces = Vec::new();
        for details in config.get_media_players() {
            media_interfaces.push(
                MediaInterface::new(
                    details.channel_map,
                    details.window_map,
                    details.apollo_params,
                )
                .await,
            );
        }

        // Create an empty event queue
        let mut queue = Queue::new(internal_send.clone());

        // Attempt to create the backup handler
        let mut backup =
            BackupHandler::new(config.get_identifier(), config.get_server_location()).await;

        // Check for existing data from the backup handler
        if let Some((current_scene, status_pairs, queued_events, dmx_universe, media_playlist)) =
            backup.reload_backup(config.get_status_ids())
        {
            // Change the current scene silently (i.e. do not trigger the reset event)
            info!(
                "Changing current scene: {}.",
                index_access.get_pair(&current_scene).await
            );
            config.choose_scene(current_scene).await.unwrap_or(());

            // Update the current status states based on the backup
            config.load_backup_status(status_pairs).await;

            // Restore the existing dmx values
            if let Some(ref interface) = dmx_interface {
                interface.restore_universe(dmx_universe).await;
            }

            // If there is a media playlist
            if media_playlist.len() > 0 {
                // Restore the media playlist
                for interface in media_interfaces.iter_mut() {
                    interface.restore_playlist(media_playlist.clone()).await;
                }
            }

            // Update the queue with the found events
            for event in queued_events {
                queue
                    .add_event(EventDelay::new(Some(event.remaining), event.event_id))
                    .await;
            }

            // Wait 20 nanoseconds for the queued events to process
            sleep(Duration::from_nanos(20)).await;

            // Trigger a redraw of the window and timeline
            interface_send.send(InterfaceUpdate::RefreshAll).await;

        // If there was no existing data in the backup, trigger the scene reset event
        } else {
            queue
                .add_event(EventDelay::new(None, config.get_current_scene()))
                .await;
        }

        // Load the current scene into the backup (to detect any crash after this point)
        backup
            .backup_current_scene(&config.get_current_scene())
            .await;

        // Return the completed EventHandler with a new queue
        Ok(Self {
            queue,
            dmx_interface,
            media_interfaces,
            config,
            config_path: resolved_path,
            index_access,
            backup,
        })
    }

    /// A method to add an event to the timed queue.
    ///
    pub async fn add_event(&mut self, event_delay: EventDelay) {
        self.queue.add_event(event_delay).await;
    }

    /// A method to clear the existing events in the timed queue.
    ///
    /// This method clears all the events in the timed queue, effective
    /// immediately. This means that any events that have not been processed
    /// (even if their delay has already expired) will not be processed.
    ///
    pub async fn clear_events(&mut self) {
        self.queue.clear().await;
    }

    /// A method to backup a list of coming events.
    ///
    pub async fn backup_events(&mut self, events: Vec<ComingEvent>) {
        // Backup the coming events
        self.backup.backup_events(events).await;
    }

    /// A method to return a copy of the background process
    ///
    pub fn get_background_process(&self) -> Option<BackgroundProcess> {
        // Return a copy of the background process
        self.config.get_background_process()
    }

    /// A method to return a copy of the current path for the configuration
    ///
    pub fn get_config_path(&self) -> PathBuf {
        // Return a copy of the pathbuf
        self.config_path.clone()
    }

    /// A method to return a copy of the system connections
    ///
    pub fn get_connections(&self) -> ConnectionSet {
        self.config.get_connections()
    }

    /// A method to return a list of all available items in the current scene.
    /// This method will always return the items from lowest to highest id.
    ///
    pub fn get_current_items(&self) -> Vec<ItemId> {
        // Return a list of available items in the current scene
        self.config.get_current_items()
    }

    /// A method to return the current scene.
    ///
    pub fn get_current_scene(&self) -> ItemId {
        // Return an item pair for the current scene
        self.config.get_current_scene()
    }

    /// A method to return the default scene
    ///
    pub fn get_default_scene(&self) -> ItemId {
        self.config.get_default_scene()
    }

    /// A method to return a copy of the dmx path
    ///
    pub fn get_dmx_path(&self) -> Option<PathBuf> {
        self.config.get_dmx_path()
    }

    /// A method to return the identifier number.
    ///
    pub fn get_identifier(&self) -> Identifier {
        self.config.get_identifier()
    }

    /// A method to return a copy of the event for the provided id.
    ///
    pub fn get_event(&mut self, event_id: &ItemId) -> Option<Event> {
        // Try to get a copy of the event
        self.config.get_event(event_id)
    }

    /// A method to return a group with available events given
    /// an item id
    pub fn get_group(&self, item_id: &ItemId) -> Option<Group> {
        // Return a group corresponding to the id, or None if none
        self.config.get_group(item_id)
    }

    /// A method to return a list of all available groups in this
    /// configuration. This method will always return the groups from lowest to
    /// highest id.
    ///
    pub fn get_groups(&self) -> Vec<ItemId> {
        // Return a list of available groups
        self.config.get_groups()
    }

    /// A method to return an key map for the current scene, with all items
    /// as an item id.
    ///
    #[allow(dead_code)]
    pub async fn get_key_map(&self) -> Option<KeyMap> {
        // Return the key mapping for this scene
        self.config.get_key_map().await
    }

    /// A method to return a copy of the media players
    ///
    pub fn get_media_players(&self) -> Vec<MediaPlayer> {
        self.config.get_media_players()
    }

    /// A method to return a scene with available events and optional keymap, given
    /// an item id
    ///
    pub fn get_scene(&self, item_id: &ItemId) -> Option<Scene> {
        // Return a scene corresponding to the id, or None if none
        self.config.get_scene(item_id)
    }

    /// A method to return a list of all available scenes in this
    /// configuration. This method will always return the scenes from lowest to
    /// highest id.
    ///
    pub fn get_scenes(&self) -> Vec<ItemId> {
        // Return a list of available scenes
        self.config.get_scenes()
    }

    /// A method to return a copy of the status detail of the provided item id.
    ///
    pub fn get_status(&mut self, item_id: &ItemId) -> Option<Status> {
        // Try to retrieve the status
        self.config.get_status(item_id)
    }

    /// A method to return a hashmap of the statuses available in this
    /// configuration.
    ///
    pub fn get_statuses(&self) -> PartialStatus {
        // Compile a hashmap of the statuses
        self.config.get_statuses()
    }

    /// A method to return the backup server location
    pub fn get_server_location(&self) -> Option<String> {
        self.config.get_server_location()
    }

    /// A method to save the new configuration parameters
    pub async fn save_parameters(&mut self, parameters: ConfigParameters) {
        self.config.save_parameters(parameters).await;
    }

    /// A method to change the selected status within the current configuration.
    ///
    pub async fn modify_status(&mut self, status_id: &ItemId, new_state: &ItemId) {
        // Try to modify the underlying status
        if let Some(new_id) = self.config.modify_status(status_id, new_state).await {
            // Backup the status change
            self.backup.backup_status(status_id, &new_id).await;

            // Run the change event for the new state (no backup necessary)
            self.queue.add_event(EventDelay::new(None, new_id)).await;
        }
    }

    /// A method to add or modify an event within the current configuration.
    ///
    pub async fn edit_event(&mut self, event_id: ItemId, new_event: Option<Event>) {
        self.config.edit_event(event_id, new_event).await;
    }

    /// A method to add or modify a group within the current configuration.
    ///
    pub async fn edit_group(&mut self, group_id: ItemId, new_group: Option<Group>) {
        self.config.edit_group(group_id, new_group).await;
    }

    /// A method to add or modify a status within the current configuration.
    ///
    pub async fn edit_status(&mut self, status_id: ItemId, new_status: Option<Status>) {
        self.config.edit_status(status_id, new_status).await;
    }

    /// A method to add or modify a scene within the current configuration.
    ///
    pub async fn edit_scene(&mut self, scene_id: ItemId, new_scene: Option<Scene>) {
        self.config.edit_scene(scene_id, new_scene).await;
    }

    /// A method to remove all references to an item from the current configuration.
    ///
    pub async fn remove_item(&mut self, item_id: ItemId) {
        self.config.remove_item(item_id).await;
    }

    /// A method to change the selected scene within the current configuration.
    ///
    /// # Errors
    ///
    /// This method will raise an error if the provided id was not found as an
    /// available scene. This usually indicates that the provided id was
    /// incorrect or that the configuration file is incorrect.
    ///
    pub async fn choose_scene(&mut self, scene_id: ItemId) {
        // Send an update to the rest of the system (will preceed error if there is one)
        info!(
            "Changing current scene: {}.",
            self.index_access.get_pair(&scene_id).await
        );

        // Try to change the underlying scene
        if self.config.choose_scene(scene_id).await.is_ok() {
            // Backup the current scene change
            self.backup.backup_current_scene(&scene_id).await;

            // Run the reset event for the new scene (no backup necessary)
            self.queue.add_event(EventDelay::new(None, scene_id)).await;
        }
    }

    /// A method to change the remaining delay for the provided event currently
    /// in the queue, or to cancel the event.
    ///
    /// # Errors
    ///
    /// This method will fail silently if the provided id was not found in the
    /// queue. This usually indicates that the event has been triggered and that
    /// the user tried to modify an event just a few moments before the time
    /// expired.
    ///
    pub async fn adjust_event(
        &mut self,
        event_id: ItemId,
        start_time: NaiveDateTime,
        new_delay: Option<Duration>,
    ) {
        // Check to see if a delay was specified
        match new_delay {
            // If a delay was specified
            Some(delay) => {
                // Try to modify the provided event in the current queue
                self.queue
                    .adjust_event(ComingEvent {
                        event_id,
                        start_time,
                        delay,
                    })
                    .await;
            }

            // Otherwise
            None => {
                // Try to cancel the event
                self.queue
                    .cancel_event(ComingEvent {
                        event_id,
                        start_time,
                        delay: Duration::from_secs(0),
                    })
                    .await;
            }
        }
    }

    /// A method to change the remaining delay for all the events in the queue.
    ///
    /// # Note
    ///
    /// This method will drop any events that should have happened in the past.
    /// In other words, if is_negative is true and the adjustment is longer
    /// than the last event in the queue, this function is equivalent to
    /// clearing the queue (none of the events will be triggered).
    ///
    pub async fn adjust_all_events(&mut self, adjustment: Duration, is_negative: bool) {
        // Modify the remaining delay for all events in the queue
        self.queue.adjust_all(adjustment, is_negative).await;
    }

    /// A method to save the current configuration to the provided file.
    ///
    /// # Errors
    ///
    /// This method will fail silently if it was unable to create the desired
    /// file. This usually indicates that there is an underlying file system
    /// error.
    ///
    pub async fn save_config(&mut self, config_path: PathBuf) {
        // Attempt to open the new configuration file
        let config_file = match File::create(&config_path).await {
            Ok(file) => file,
            Err(_) => {
                error!("Unable to open configuration file.");
                return;
            }
        };

        // Notify success
        info!(
            "Writing configuration to file: {}.",
            config_path
                .file_name()
                .unwrap_or(OsStr::new("filename unavailable"))
                .to_str()
                .unwrap_or("filename unavailable")
        );

        // Save the configuration to the provided file
        self.config.to_config(config_file).await;

        // Update the current config path
        self.config_path = config_path;
    }

    /// A method to process a new event in the event handler. If the event was
    /// processed successfully, it will return any events that should be broadcast
    /// to the system (including their associated data, if applicable).
    ///
    /// # Errors
    ///
    /// This method will raise an error if the event was not found. If the
    /// module encounters an inconsistency when trying to process an event,
    /// it will raise a warning and otherwise continue processing events.
    ///
    pub async fn process_event(
        &mut self,
        event_id: &ItemId,
        checkscene: bool,
        broadcast: bool,
    ) -> Result<BroadcastData, ()> {
        // Try to retrieve the event and unpack the event
        let event = match self.config.try_event(event_id, checkscene).await {
            // Process a valid event
            Some(event) => event,

            // Return Err on failure
            None => return Err(()),
        };

        // Unpack and process each action of the event
        let mut broadcast_data = BroadcastData::new();
        for action in event {
            // Switch based on the result of unpacking the action
            match self.unpack_action(action).await {
                // No additional action required
                UnpackResult::None => (),

                // Send data to the system
                UnpackResult::Data(mut data) => {
                    // If we should broadcast the event
                    if broadcast {
                        // Return the event and each piece of data
                        for number in data.drain(..) {
                            broadcast_data.push(Some(number));
                        }
                    }
                }
            }
        }

        // If data hasn't been saved yet for broadcast
        if broadcast_data.is_empty() {
            // And if we should broadcast the event
            if broadcast {
                // Save empty data to the list
                broadcast_data.push(None);
            }
        }

        // Indicate success
        Ok(broadcast_data)
    }

    /// An internal function to unpack the event and act on it. If the
    /// event results in data to broadcast, the data will be returned.
    ///
    async fn unpack_action(&mut self, event_action: EventAction) -> UnpackResult {
        // Unpack the event
        match event_action {
            // If there is a new scene, execute the change
            NewScene { new_scene } => {
                // Try to change the scene current scene
                self.choose_scene(new_scene).await;
            }

            // If there is a status modification, execute the change
            ModifyStatus {
                status_id,
                new_state,
            } => {
                // Try to change the state of the status and trigger the event
                self.modify_status(&status_id, &new_state).await;
            }

            // If there is a fade to cue, send it to the dmx connection
            CueDmx { fade } => {
                // Send it to the dmx interface, if it exists
                if let Some(ref interface) = &self.dmx_interface {
                    if let Err(err) = interface.play_fade(fade.clone()).await {
                        error!("Error with DMX playback: {}.", err);

                    // If successful, backup the dmx fade
                    } else {
                        self.backup.backup_dmx(fade).await;
                    }

                // Warn that there is no active Dmx interface
                } else {
                    error!("Failed to play DMX fade: No DMX interface available.")
                }
            }

            // If there is a queued event to load, load it into the queue
            CueEvent { event } => {
                // Add the event to the queue
                self.queue.add_event(event).await;
            }

            // If there is media to cue, send it to the media connection
            CueMedia { cue } => {
                // Send the cue to each media interface in turn
                let mut success = false;
                for interface in self.media_interfaces.iter_mut() {
                    if let Ok(_) = interface.play_cue(cue.clone()).await {
                        success = true;
                    }
                }

                // If one of the media players played the cue, back it up
                if success {
                    self.backup.backup_media(cue).await;

                // Otherwise, report the error
                } else {
                    error!("Failed to play media cue.");
                }
            }

            // If there is media to adjust, send it to the media connection
            AdjustMedia { adjustment } => {
                // Send the cue to each media interface in turn
                let mut success = false;
                for interface in self.media_interfaces.iter_mut() {
                    if let Ok(_) = interface.adjust_media(adjustment.clone()).await {
                        success = true;
                    }
                }

                // If all media players failed to play the cue, report the error
                if !success {
                    error!("Failed to adjust media.");
                }
            }

            // If there is an event to cancel, remove it from the queue
            CancelEvent { event } => {
                // Cancel any events with the matching id in the queue
                self.queue.cancel_all(event).await;
            }

            // If there is data to save, save it
            SaveData { data } => {
                // Select for the type of data
                match data {
                    // Collect time until an event
                    DataType::TimeUntil { event_id } => {
                        // Check to see if there is time remaining for the event
                        if let Some(duration) = self.queue.event_remaining(&event_id).await {
                            // Convert the duration to minutes and seconds
                            let minutes = duration.as_secs() / 60;
                            let seconds = duration.as_secs() % 60;

                            // Compose a string for the log
                            let data_string = format!("Time {}:{}", minutes, seconds);

                            // Save the data to the game log
                            info!(target: GAME_LOG, "Game data: {}.", data_string);
                        }
                    }

                    // Collect time passed until an event
                    DataType::TimePassedUntil {
                        event_id,
                        total_time,
                    } => {
                        // Check to see if there is time remaining for the event
                        if let Some(remaining) = self.queue.event_remaining(&event_id).await {
                            // Subtract the remaining time from the total time
                            if let Some(result) = total_time.checked_sub(remaining) {
                                // Convert the duration to minutes and seconds
                                let minutes = result.as_secs() / 60;
                                let seconds = result.as_secs() % 60;

                                // Compose a string for the log
                                let data_string = format!("Time {}:{}", minutes, seconds);

                                // Save the data to the game log
                                info!(target: GAME_LOG, "Game data: {}.", data_string);
                            }
                        }
                    }

                    // Send the static string to the event
                    DataType::StaticString { string } => {
                        // Save the string to the game log
                        info!(target: GAME_LOG, "Game data: {}.", string);
                    }
                }
            }

            // If there is data to send, collect and send it
            SendData { data } => {
                // Select for the type of data
                match data {
                    // Collect time until an event
                    DataType::TimeUntil { event_id } => {
                        // Check to see if there is time remaining for the event
                        if let Some(duration) = self.queue.event_remaining(&event_id).await {
                            // Convert the data to u32 (truncated)
                            return UnpackResult::Data(vec![duration.as_secs() as u32]);

                        // Otherwise, return empty data
                        } else {
                            return UnpackResult::Data(vec![0]);
                        }
                    }

                    // Collect time passed until an event
                    DataType::TimePassedUntil {
                        event_id,
                        total_time,
                    } => {
                        // Check to see if there is time remaining for the event
                        if let Some(remaining) = self.queue.event_remaining(&event_id).await {
                            // Subtract the remaining time from the total time
                            if let Some(result) = total_time.checked_sub(remaining) {
                                // Convert the data to u32 (truncated)
                                return UnpackResult::Data(vec![result.as_secs() as u32]);

                            // Otherwise, return no duration
                            } else {
                                return UnpackResult::Data(vec![0]);
                            }

                        // Otherwise, return the full duration (truncated)
                        } else {
                            return UnpackResult::Data(vec![total_time.as_secs() as u32]);
                        }
                    }

                    // Send the static string to the event
                    DataType::StaticString { string } => {
                        // Convert the string into bytes
                        let mut bytes = string.into_bytes();

                        // Save the length of the new vector
                        let length = bytes.len() as u32;
                        let mut data = vec![length];

                        // Convert the bytes into a u32 Vec
                        let (mut first, mut second, mut third, mut fourth) = (0, 0, 0, 0);
                        for (num, byte) in bytes.drain(..).enumerate() {
                            // Repack the data efficiently
                            match num % 4 {
                                0 => first = byte as u32,
                                1 => second = byte as u32,
                                2 => third = byte as u32,
                                _ => {
                                    fourth = byte as u32;
                                    data.push(
                                        (first << 24) | (second << 16) | (third << 8) | fourth,
                                    );
                                }
                            }
                        }

                        // Save the last bit of data if the total doesn't add to 4
                        if (length % 4) != 0 {
                            data.push((first << 24) | (second << 16) | (third << 8) | fourth);
                        }

                        // Return the complete data
                        return UnpackResult::Data(data);
                    }
                }
            }

            // If there is a select event, trigger the selected event
            SelectEvent {
                status_id,
                event_map,
            } => {
                // Try to retrieve the group status state
                if let Some(state) = self.config.get_state(&status_id).await {
                    // Try to find the corresponding event in the event_map
                    if let Some(event_id) = event_map.get(&state) {
                        // Trigger the event if it was found
                        self.queue
                            .add_event(EventDelay::new(None, event_id.clone()))
                            .await;

                        // States with no matching event are ignored
                    }
                }
            }
        }

        // Return none for most cases
        UnpackResult::None
    }
}

/// A helper enum to return the different results of unpacking an event
///
#[derive(Clone, Debug, PartialEq, Eq)]
enum UnpackResult {
    /// A variant indicating there is no additional information needed for this
    /// event (the most common result).
    None,

    /// A variant indicating that some data that should be broadcast to the system.
    Data(Vec<u32>),
}

// Tests of the event handler module
#[cfg(test)]
mod tests {
    //use super::*;

    // FIXME Define tests of this module
    #[test]
    fn missing_tests() {
        // FIXME: Implement this
        unimplemented!();
    }

    // FIXME Repair these tests
    // Simple test of running the queue module
    /*#[test]
    fn handle_events() {
        // Import libraries for testing
        use self::event::EventDelay;
        use self::item::Hidden;
        use std::thread;
        use std::time::Duration;

        // Create a new Event Handler
        let (tx, rx) = mpsc::channel();
        let (_, unused) = mpsc::channel();
        let mut event_handler =
            EventHandler::new(PathBuf::from("examples/testing_config.mnv"), tx, unused).unwrap();

        // Select the correct scene
        event_handler.config.choose_scene(ItemId::new(100).unwrap());

        // Define the first two event delays
        let event1_delay =
            EventDelay::new(Some(Duration::from_millis(100)), ItemId::new(1).unwrap());
        let event2_delay =
            EventDelay::new(Some(Duration::from_millis(200)), ItemId::new(2).unwrap());

        // Trigger the first two events
        event_handler
            .queue
            .add_events(vec![event1_delay, event2_delay]);

        // Create the test vector
        let test = vec![
            EventUpdate::Update("Got Data: 1".to_string()),
            EventUpdate::Current(ItemPair::new(1, "Save Data 1", Hidden).unwrap()),
            EventUpdate::Broadcast(ItemPair::new(2, "Load Delayed Events", Hidden).unwrap()),
            EventUpdate::Update("Got Data: 3".to_string()),
            EventUpdate::Current(ItemPair::new(3, "Save Data 3", Hidden).unwrap()),
            EventUpdate::Update("Got Data: 4".to_string()),
            EventUpdate::Current(ItemPair::new(4, "Save Data 4", Hidden).unwrap()),
        ];

        // Wait 2 seconds for all the events to process
        let mut index = 0;
        while index < 2000 {
            event_handler.run_once();
            thread::sleep(Duration::from_millis(1));
            index = index + 1;
        }

        // Wait up to half a second to find all the status updates in the correct order
        test_vec!(=rx, test);
    }

    // Test linked events with queue module
    #[test]
    fn link_events() {
        // Import libraries for testing
        use self::event::EventDelay;
        use self::item::{DisplayControl, DisplayWith, Hidden};
        use std::thread;
        use std::time::Duration;
        use std::sync::mpsc;

        // Create a new Event Handler
        let (tx, rx) = mpsc::channel();
        let (_, unused) = mpsc::channel();
        let mut event_handler =
            EventHandler::new(PathBuf::from("examples/testing_config.mnv"), tx, unused).unwrap();

        // Select the correct scene
        event_handler.config.choose_scene(ItemId::new(200).unwrap());

        // Define the first two event delays
        let event5_delay =
            EventDelay::new(Some(Duration::from_millis(200)), ItemId::new(5).unwrap());
        let event8_delay =
            EventDelay::new(Some(Duration::from_millis(500)), ItemId::new(8).unwrap());

        // Trigger the first and last event
        event_handler
            .queue
            .add_events(vec![event5_delay, event8_delay]);

        // Create the test vector
        let test = vec![
            EventUpdate::Broadcast(
                ItemPair::new(5, "Load Immediate Events (6 & 7)", Hidden).unwrap(),
            ),
            EventUpdate::Broadcast(ItemPair::new(15, "Load Delayed Events (5)", Hidden).unwrap()),
            EventUpdate::Current(
                ItemPair::new(
                    6,
                    "Load Events Or Save Data (Select Event)",
                    DisplayWith {
                        group_id: Some(ItemId::new(10).unwrap()),
                        position: None,
                        color: None,
                        highlight: None,
                    }
                )
                .unwrap(),
            ),
            EventUpdate::Update("Got Data: 7".to_string()),
            EventUpdate::Current(ItemPair::new(7, "Save Data 7", Hidden).unwrap()),
            EventUpdate::Broadcast(
                ItemPair::new(5, "Load Immediate Events (6 & 7)", Hidden).unwrap(),
            ),
            EventUpdate::Broadcast(ItemPair::new(15, "Load Delayed Events (5)", Hidden).unwrap()),
            EventUpdate::Current(
                ItemPair::new(
                    6,
                    "Load Events Or Save Data (Select Event)",
                    DisplayWith {
                        group_id: Some(ItemId::new(10).unwrap()),
                        position: None,
                        color: None,
                        highlight: None,
                    }
                )
                .unwrap(),
            ),
            EventUpdate::Update("Got Data: 7".to_string()),
            EventUpdate::Current(ItemPair::new(7, "Save Data 7", Hidden).unwrap()),
            EventUpdate::Status(
                ItemPair::new(10, "Test Event Group - Loop Or Save", Hidden).unwrap(),
                ItemDescription::new("Currently Saving Data", Hidden),
            ),
            EventUpdate::Current(ItemPair::new(8, "Modify Test Event Group", Hidden).unwrap()),
            EventUpdate::Broadcast(
                ItemPair::new(5, "Load Immediate Events (6 & 7)", Hidden).unwrap(),
            ),
            EventUpdate::Update("Got Data: 7".to_string()),
            EventUpdate::Current(ItemPair::new(7, "Save Data 7", Hidden).unwrap()),
            EventUpdate::Current(
                ItemPair::new(
                    6,
                    "Load Events Or Save Data (Select Event)",
                    DisplayWith {
                        group_id: Some(ItemId::new(10).unwrap()),
                        position: None,
                        color: None,
                        highlight: None,
                    }
                )
                .unwrap(),
            ),
            EventUpdate::Update("Got Data: 7".to_string()),
            EventUpdate::Current(ItemPair::new(7, "Save Data 7", Hidden).unwrap()),
        ];

        // Wait 2 seconds for all the events to process
        let mut index = 0;
        while index < 2000 {
            event_handler.run_once();
            thread::sleep(Duration::from_millis(1));
            index = index + 1;
        }

        // Wait up to half a second to find all the status updates in the correct order
        test_vec!(=rx, test);
    }*/
}
