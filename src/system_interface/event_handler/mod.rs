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
mod backup;
mod config;
mod queue;

// Import crate definitions
use crate::definitions::*;

// Import other definitions
use self::backup::BackupHandler;
use self::config::Config;
use self::queue::Queue;
use super::system_connection::ConnectionSet;

// Import standard library features
use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

// Import Tokio features
use tokio::time::sleep;

// Import the failure features
use failure::Error;

/// A structure to manage all event triggering and internal event operations
/// inside the program. This structure allows the main program to be agnostic
/// to the current configuration of the program and the available events.
///
pub struct EventHandler {
    internal_send: InternalSend,    // sending line for event updates and timed events
    queue: Queue,                   // current event queue
    config: Config,                 // current configuration
    backup: BackupHandler,          // current backup server
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
        config_path: PathBuf,
        index_access: IndexAccess,
        internal_send: InternalSend,
        interface_send: mpsc::Sender<InterfaceUpdate>,
        log_failure: bool,
    ) -> Result<EventHandler, Error> {
        // Attempt to open the configuration file
        let config_file = match File::open(config_path) {
            Ok(file) => file,
            Err(_) => {
                // Only log failure if the flag is set
                if log_failure {
                    update!(err &internal_send => "Unable To Open Configuration File.");
                }
                return Err(format_err!("Unable to open configuration file."));
            }
        };

        // Attempt to process the configuration file
        let mut config = Config::from_config(index_access, internal_send.clone(), interface_send, &config_file).await?;

        // Attempt to create the backup handler
        let mut backup = BackupHandler::new(
            internal_send.clone(),
            config.identifier(),
            config.server_location(),
        ).await?;

        // Create an empty event queue
        let mut queue = Queue::new(internal_send.clone());

        // Check for existing data from the backup handler
        let possible_backup = backup.reload_backup(config.get_status_ids());
        if let Some((current_scene, status_pairs, queued_events)) = possible_backup {
            // Notify that existing data was found
            update!(err &internal_send => "Detected Lingering Backup Data. Reloading ...");

            // Change the current scene silently (i.e. do not trigger the reset event)
            config.choose_scene(current_scene).await.unwrap_or(());

            // Update the current status states based on the backup
            config.load_backup_status(status_pairs).await;

            // Update the queue with the found events
            for event in queued_events {
                queue.add_event(EventDelay::new(Some(event.remaining), event.event_id)).await;
            }

            // Wait 10 nanoseconds for the queued events to process
            sleep(Duration::new(0, 20)).await;

            // Trigger a redraw of the window and timeline
            internal_send.send_refresh().await;

        // If there was no existing data in the backup, trigger the scene reset event
        } else {
            queue.add_event(EventDelay::new(None, config.get_current_scene())).await;
        }

        // Load the current scene into the backup (to detect any crash after this point)
        backup.backup_current_scene(&config.get_current_scene()).await;

        // Return the completed EventHandler with a new queue
        Ok(EventHandler {
            internal_send: internal_send,
            queue,
            config,
            backup,
        })
    }

    /// A method to return the configured system connection type.
    ///
    pub fn system_connection(&self) -> (ConnectionSet, Identifier) {
        self.config.system_connection()
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

    /// A method to return a copy of the event for the provided id.
    ///
    /// # Errors
    ///
    /// This method will raise an error if the provided id was not found in
    /// configuration. This usually indicates that the provided id was incorrect
    /// or that the configuration file is incomplete.
    ///
    pub async fn get_event(&mut self, event_id: &ItemId) -> Option<Event> {
        // Try to retrieve the event
        self.config.try_event(event_id, false).await // do not check the scene
    }

    /// A method to return a copy of the status detail of the provided item id.
    ///
    /// # Errors
    ///
    /// This method will return None if the provided id was not found in
    /// configuration. This usually indicates that the provided id was incorrect
    ///  or that the configuration file is incomplete.
    ///
    pub fn get_status(&mut self, item_id: &ItemId) -> Option<Status> {
        // Try to retrieve the status
        self.config.get_status(item_id)
    }

    /// A method to return a hashmap of the statuses available in this
    /// configuration.
    ///
    pub fn get_statuses(&mut self) -> PartialStatus {
        // Compile a hashmap of the statuses
        self.config.get_statuses()
    }

    /// A method to return a list of all available scenes in this
    /// configuration. This method will always return the scenes from lowest to
    /// highest id.
    ///
    pub fn get_scenes(&self) -> Vec<ItemId> {
        // Return a list of available scenes
        self.config.get_scenes()
    }

    /// A method to return a scene with available events and optional keymap, given
    /// an item id
    pub async fn get_scene(&self, item_id: ItemId) -> Option<DescriptiveScene> {
        // Return a scene corresponding to the id, or None if none
        self.config.get_scene(item_id).await
    }

    /// A method to return a list of all available items in the current scene.
    /// This method will always return the items from lowest to highest id.
    ///
    pub fn get_current_items(&self) -> Vec<ItemId> {
        // Return a list of available items in the current scene
        self.config.get_current_items()
    }

    /// A method to return an key map for the current scene, with all items
    /// as an itempair.
    ///
    pub async fn get_key_map(&self) -> KeyMap {
        // Return the key mapping for this scene
        self.config.get_key_map().await
    }

    /// A method to return the current scene.
    ///
    pub fn get_current_scene(&self) -> ItemId {
        // Return an item pair for the current scene
        self.config.get_current_scene()
    }

    /// A method to change the selected status within the current configuration.
    ///
    /// # Errors
    ///
    /// This method will raise an error if the provided id was not found as
    /// a status in the configuration. This usually indicates that the provided
    /// id was incorrect or that the configuration file is incorrect.
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
        update!(update &self.internal_send => "Changing Current Scene ...");

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
    pub async fn adjust_event(&mut self, event_id: ItemId, start_time: Instant, new_delay: Option<Duration>) {
        // Check to see if a delay was specified
        match new_delay {
            // If a delay was specified
            Some(delay) => {
                // Try to modify the provided event in the current queue
                self.queue.adjust_event(ComingEvent {
                    event_id,
                    start_time,
                    delay,
                }).await;
            }

            // Otherwise
            None => {
                // Try to cancel the event
                self.queue.cancel_event(ComingEvent {
                    event_id,
                    start_time,
                    delay: Duration::from_secs(0),
                }).await;
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
        let config_file = match File::create(config_path) {
            Ok(file) => file,
            Err(_) => {
                update!(err &self.internal_send => "Unable To Open Configuration File.");
                return;
            }
        };

        // Save the configuration to the provided file
        self.config.to_config(&config_file).await;
    }

    /// A method to process a new event in the event handler. If the event was
    /// processed successfully, it returns true.
    ///
    /// # Errors
    ///
    /// This method will not raise any errors. If the module encounters an
    /// inconsistency when trying to process an event, it will raise a warning
    /// and otherwise continue processing events.
    ///
    pub async fn process_event(&mut self, event_id: &ItemId, checkscene: bool, broadcast: bool) -> bool {
        // Try to retrieve the event and unpack the event
        let event = match self.config.try_event(event_id, checkscene).await {
            // Process a valid event
            Some(event) => event,

            // Return false on failure
            None => return false,
        };

        // Unpack and process each action of the event
        let mut was_broadcast = false;
        for action in event {
            // Switch based on the result of unpacking the action
            match self.unpack_action(action).await {
                // No additional action required
                UnpackResult::None => (),

                // Send data to the system
                UnpackResult::Data(mut data) => {
                    // Save that the event has been broadcast
                    was_broadcast = true;

                    // If we should broadcast the event
                    if broadcast {
                        // Broadcast the event and each piece of data
                        for number in data.drain(..) {
                            update!(broadcast &self.internal_send => event_id.clone(), Some(number));
                        }

                    // Otherwise just update the system about the event
                    } else {
                        update!(now &self.internal_send => event_id.clone());
                    }
                }

                // Solicit a string from the user
                UnpackResult::String => {
                    // Save that the event was broadcast
                    was_broadcast = true;

                    // Solicit a string
                    self.internal_send.send_get_user_string(event_id.clone()).await;
                }
            }
        }

        // Broadcast the event (if it hasn't been broadcast yet)
        if !was_broadcast {
            // If we should broadcast the event
            if broadcast {
                // Send it to the system
                update!(broadcast &self.internal_send => event_id.clone(), None);

            // Otherwise just update the system about the event
            } else {
                update!(now &self.internal_send => event_id.clone());
            }
        }

        // Indicate success
        true
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

            // If there is a queued event to load, load it into the queue
            CueEvent { event } => {
                // Add the event to the queue
                self.queue.add_event(event).await;
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
                            update!(save &self.internal_send => data_string);
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
                                update!(save &self.internal_send => data_string);
                            }
                        }
                    }

                    // Send the static string to the event
                    DataType::StaticString { string } => {
                        // Save the string to the game log
                        update!(save &self.internal_send => string);
                    }

                    // Solicit a string from the user
                    DataType::UserString => {
                        // Error that this is not yet implemented
                        update!(err &self.internal_send => "Saving a User String is not yet implemented.");
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

                    // Solicit a string from the user
                    DataType::UserString => return UnpackResult::String,
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
                        self.queue.add_event(EventDelay::new(None, event_id.clone())).await;
                        
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

    /// A variant indicating that a string should be solicited from the user.
    String,
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
