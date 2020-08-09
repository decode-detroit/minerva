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

//! A module that holds the state machine of the program. This module loads
//! the configuration from a file, holds the configuration in memory, and
//! and handles all changes to the current state as well as queuing upcoming
//! events. This module also processes the events and sends updtes to the rest
//! of the program.

// Reexport the key structures and types
pub use self::config::{
    DescriptiveScene, FullStatus, KeyMap, Scene, StatusDescription, Status
};
pub use self::queue::ComingEvent;

// Define public submodules
pub mod item;
#[macro_use]
pub mod event;

// Define private submodules
mod backup;
mod config;
mod queue;

// Import the relevant structures into the correct namespace
use self::backup::BackupHandler;
use self::config::Config;
use self::event::{
    CancelEvent, DataType, EventAction, EventDelay, Event, EventUpdate, GroupedEvent,
    ModifyStatus, NewScene, QueueEvent, SaveData, SendData, UpcomingEvent,
};
use self::item::{ItemDescription, ItemId, ItemPair};
use self::queue::Queue;
use super::system_connection::ConnectionSet;
use super::{GeneralUpdate, InterfaceUpdate};

// Import standard library modules
use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

// Import the failure features
use failure::Error;

/// A structure to manage all event triggering and internal event operations
/// inside the program. This structure allows the main program to be agnostic
/// to the current configuration of the program and the available events.
///
pub struct EventHandler {
    general_update: GeneralUpdate, // sending line for event updates and timed events
    queue: Queue,                  // current event queue
    config: Config,                // current configuration
    backup: BackupHandler,         // current backup server
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
    pub fn new(
        config_path: PathBuf,
        general_update: GeneralUpdate,
        interface_send: mpsc::Sender<InterfaceUpdate>,
        log_failure: bool,
    ) -> Result<EventHandler, Error> {
        // Attempt to open the configuration file
        let config_file = match File::open(config_path) {
            Ok(file) => file,
            Err(_) => {
                // Only log failure if the flag is set
                if log_failure {
                    update!(err &general_update => "Unable To Open Configuration File.");
                }
                return Err(format_err!("Unable to open configuration file."));
            }
        };

        // Attempt to process the configuration file
        let mut config = Config::from_config(general_update.clone(), interface_send, &config_file)?;

        // Attempt to create the backup handler
        let backup = BackupHandler::new(
            general_update.clone(),
            config.identifier(),
            config.server_location(),
        )?;

        // Create an empty event queue
        let queue = Queue::new(general_update.clone());

        // Check for existing data from the backup handler
        if let Some((current_scene, status_pairs, queued_events)) =
            backup.reload_backup(config.get_status_ids())
        {
            // Notify that existing data was found
            update!(err &general_update => "Detected Lingering Backup Data. Reloading ...");

            // Change the current scene silently (i.e. do not trigger the reset event)
            config.choose_scene(current_scene).unwrap_or(());

            // Update the current status states based on the backup
            config.load_backup_status(status_pairs);

            // Update the queue with the found events
            for event in queued_events {
                queue.add_event(EventDelay::new(Some(event.remaining), event.event_id));
            }

            // Wait 10 nanoseconds for the queued events to process
            thread::sleep(Duration::new(0, 20));

            // Trigger a redraw of the window and timeline
            general_update.send_redraw();

        // If there was no existing data in the backup, trigger the scene reset event
        } else {
            queue.add_event(EventDelay::new(None, config.get_current_scene().get_id()));
        }

        // Load the current scene into the backup (to detect any crash after this point)
        backup.backup_current_scene(&config.get_current_scene().get_id());

        // Return the completed EventHandler with a new queue
        Ok(EventHandler {
            general_update: general_update,
            queue,
            config,
            backup,
        })
    }

    /// A method to return the configured system connection type.
    ///
    pub fn system_connection(&self) -> (ConnectionSet, ItemId) {
        self.config.system_connection()
    }

    /// A method to add an event to the timed queue.
    ///
    pub fn add_event(&mut self, event_delay: EventDelay) {
        self.queue.add_event(event_delay);
    }

    /// A method to clear the existing events in the timed queue.
    ///
    /// This method clears all the events in the timed queue, effective
    /// immediately. This means that any events that have not been processed
    /// (even if their delay has already expired) will not be processed.
    ///
    pub fn clear_events(&mut self) {
        self.queue.clear();
    }

    /// A method to repackage a list of coming events as upcoming events.
    ///
    /// # Notes
    ///
    /// The order of the provided list does correspond to the order the events
    /// will occur (last event first). The coming events are backed up (if
    /// the backup feature is active).
    ///
    pub fn repackage_events(&self, mut events: Vec<ComingEvent>) -> Vec<UpcomingEvent> {
        // Backup the coming events
        self.backup.backup_events(events.clone());

        // Repackage the list as upcoming events
        let mut upcoming_events = Vec::new();
        for event in events.drain(..) {
            // Find the description and add it
            upcoming_events.push(UpcomingEvent {
                start_time: event.start_time,
                delay: event.delay,
                event: ItemPair::from_item(event.id(), self.get_description(&event.id())),
            });
        }

        // Return the completed list
        upcoming_events
    }

    /// A method to return a copy of the event for the provided id.
    ///
    /// # Errors
    ///
    /// This method will raise an error if the provided id was not found in
    /// configuration. This usually indicates that the provided id was incorrect
    ///  or that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning None.
    ///
    pub fn get_event(&mut self, event_id: &ItemId) -> Option<Event> {
        // Try to retrieve the event
        self.config.try_event(event_id, false) // do not check the scene
    }

    /// A method to return a copy of the description of the provided id.
    ///
    /// # Errors
    ///
    /// This method will raise an error if the provided id was not found in
    /// lookup. This usually indicates that the provided id was incorrect or
    /// that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning an
    /// empty ItemDescription.
    ///
    pub fn get_description(&self, item_id: &ItemId) -> ItemDescription {
        // Return a new copy of the event description
        self.config.get_description(item_id)
    }

    /// A method to return a copy of the status detail of the provided item id.
    ///
    /// # Errors
    ///
    /// This method will return None if the provided id was not found in
    /// configuration. This usually indicates that the provided id was incorrect
    ///  or that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning None.
    ///
    pub fn get_status(&mut self, item_id: &ItemId) -> Option<Status> {
        // Try to retrieve the status
        self.config.get_status(item_id)
    }

    /// A method to return a hashmap of the full status available in this
    /// configuration.
    ///
    /// # Errors
    ///
    /// This method will raise an error if one of the status ids was not found in
    /// the lookup. This indicates that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning an
    /// empty ItemDescription for that status.
    ///
    pub fn get_full_status(&mut self) -> FullStatus {
        // Compile a hashmap of the full status
        self.config.get_full_status()
    }

    /// A method to return an itempair of all available scenes in this
    /// configuration. This method will always return the scenes from lowest to
    /// highest id.
    ///
    /// # Errors
    ///
    /// This method will raise an error if one of the scene ids was not found in
    /// lookup. This indicates that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning an
    /// empty ItemDescription for that scene.
    ///
    pub fn get_scenes(&self) -> Vec<ItemPair> {
        // Return a list of available scenes
        self.config.get_scenes()
    }

    /// A method to return a scene with available events and optional keymap, given
    /// an item id
    pub fn get_scene(&self, item_id: ItemId) -> Option<DescriptiveScene> {
        // Return a scene corresponding to the id, or None if none
        self.config.get_scene(item_id)
    }

    /// A method to return an itempair of all available items in the configuration.
    ///
    pub fn get_items(&self) -> Vec<ItemPair> {
        // Return a list of available items in the current scene
        self.config.get_items()
    }

    /// A method to return an itempair of all available events in the current
    /// scene. This method will always return the items from lowest to
    /// highest id.
    ///
    /// # Errors
    ///
    /// This method will raise an error if one of the item ids was not found in
    /// lookup. This indicates that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning an
    /// empty ItemDescription for that item.
    ///
    pub fn get_events(&self) -> Vec<ItemPair> {
        // Return a list of available items in the current scene
        self.config.get_events()
    }

    /// A method to return an key map for the current scene, with all items
    /// as an itempair.
    ///
    /// # Errors
    ///
    /// This method will raise an error if one of the item ids was not found in
    /// lookup. This indicates that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning an
    /// empty ItemDescription for that item.
    ///
    pub fn get_key_map(&self) -> KeyMap {
        // Return the key mapping for this scene
        self.config.get_key_map()
    }

    /// A method to return the current scene.
    ///
    /// # Errors
    ///
    /// This method will raise an warning if the current scene is not described
    /// in the lookup. This indicates that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the status line and returning an
    /// ItemPair with an empty description.
    ///
    pub fn get_current_scene(&self) -> ItemPair {
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
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and leaving the
    /// current configuration unmodified.
    ///
    pub fn modify_status(&mut self, status_id: &ItemId, new_state: &ItemId) {
        // Try to modify the underlying status
        if let Some(new_id) = self.config.modify_status(status_id, new_state) {
            // Backup the status change
            self.backup.backup_status(status_id, &new_id);

            // Run the change event for the new state (no backup necessary)
            self.queue.add_event(EventDelay::new(None, new_id));
        }
    }

    /// A method to add or modify an item within the current configuration.
    ///
    pub fn edit_item(&mut self, item_id: ItemPair) {
        self.config.edit_item(item_id);
    }

    /// A method to add or modify an event within the current configuration.
    ///
    pub fn edit_event(&mut self, event_id: ItemId, new_event: Option<Event>) {
        self.config.edit_event(event_id, new_event);
    }
        
    /// A method to add or modify a status within the current configuration.
    ///
    pub fn edit_status(&mut self, status_id: ItemId, new_status: Option<Status>) {
        self.config.edit_status(status_id, new_status);
    }
    
    /// A method to add or modify a scene within the current configuration.
    ///
    pub fn edit_scene(&mut self, scene_id: ItemId, new_scene: Option<Scene>) {
        self.config.edit_scene(scene_id, new_scene);
    }

    /// A method to change the selected scene within the current configuration.
    ///
    /// # Errors
    ///
    /// This method will raise an error if the provided id was not found as an
    /// available scene. This usually indicates that the provided id was
    /// incorrect or that the configuration file is incorrect.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the general line and leaving the
    /// current configuration unmodified.
    ///
    pub fn choose_scene(&mut self, scene_id: ItemId) {
        // Send an update to the rest of the system (will preceed error if there is one)
        update!(update &self.general_update => "Changing Current Scene ...");

        // Try to change the underlying scene
        if self.config.choose_scene(scene_id).is_ok() {
            // Backup the current scene change
            self.backup.backup_current_scene(&scene_id);

            // Run the reset event for the new scene (no backup necessary)
            self.queue.add_event(EventDelay::new(None, scene_id));
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
    /// expired
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by ignoring this failure.
    ///
    pub fn adjust_event(&self, event_id: ItemId, start_time: Instant, new_delay: Option<Duration>) {
        // Check to see if a delay was specified
        match new_delay {
            // If a delay was specified
            Some(delay) => {
                // Try to modify the provided event in the current queue
                self.queue.adjust_event(ComingEvent {
                    event_id,
                    start_time,
                    delay,
                });
            }

            // Otherwise
            None => {
                // Try to cancel the event
                self.queue.cancel_event(ComingEvent {
                    event_id,
                    start_time,
                    delay: Duration::from_secs(0),
                });
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
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by ignoring this failure.
    ///
    pub fn adjust_all_events(&self, adjustment: Duration, is_negative: bool) {
        // Modify the remaining delay for all events in the queue
        self.queue.adjust_all(adjustment, is_negative);
    }

    /// A method to save the current configuration to the provided file.
    ///
    /// # Errors
    ///
    /// This method will fail silently if it was unable to create the desired
    /// file. This usually indicates that there is an underlying file system
    /// error.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying the user.
    ///
    pub fn save_config(&self, config_path: PathBuf) {
        // Attempt to open the new configuration file
        let config_file = match File::create(config_path) {
            Ok(file) => file,
            Err(_) => {
                update!(err &self.general_update => "Unable To Open Configuration File.");
                return;
            }
        };

        // Save the configuration to the provided file
        self.config.to_config(&config_file);
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
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line.
    ///
    pub fn process_event(&mut self, event_id: &ItemId, checkscene: bool, broadcast: bool) -> bool {
        // Try to retrieve the event and unpack the event
        let event = match self.config.try_event(event_id, checkscene) {
            // Process a valid event
            Some(event) => event,

            // Return false on failure
            None => return false,
        };

        // Compose the item into an item pair
        let pair = ItemPair::from_item(event_id.clone(), self.get_description(&event_id));

        // Unpack and process each action of the event
        let mut was_broadcast = false;
        for action in event {
            // Switch based on the result of unpacking the action
            match self.unpack_action(action) {
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
                            update!(broadcast &self.general_update => pair.clone(), Some(number));
                        }

                    // Otherwise just update the system about the event
                    } else {
                        update!(now &self.general_update => pair.clone());
                    }
                }

                // Solicit a string from the user
                UnpackResult::String => {
                    // Save that the event was broadcast
                    was_broadcast = true;

                    // Solicit a string
                    self.general_update.send_get_user_string(pair.clone());
                }
            }
        }

        // Broadcast the event (if it hasn't been broadcast yet)
        if !was_broadcast {
            // If we should broadcast the event
            if broadcast {
                // Send it to the system
                update!(broadcast &self.general_update => pair.clone(), None);

            // Otherwise just update the system about the event
            } else {
                update!(now &self.general_update => pair.clone());
            }
        }

        // Indicate success
        true
    }

    /// An internal function to unpack the event and act on it. If the
    /// event results in data to broadcast, the data will be returned.
    ///
    fn unpack_action(&mut self, event_action: EventAction) -> UnpackResult {
        // Unpack the event
        match event_action {
            // If there is a new scene, execute the change
            NewScene { new_scene } => {
                // Try to change the scene current scene
                self.choose_scene(new_scene);
            }

            // If there is a status modification, execute the change
            ModifyStatus {
                status_id,
                new_state,
            } => {
                // Try to change the state of the status and trigger the event
                self.modify_status(&status_id, &new_state);
            }

            // If there is a queued event to load, load it into the queue
            QueueEvent { event } => {
                // Add the event to the queue
                self.queue.add_event(event);
            }

            // If there is an event to cancel, remove it from the queue
            CancelEvent { event } => {
                // Cancel any events with the matching id in the queue
                self.queue.cancel_all(event);
            }

            // If there is data to save, save it
            SaveData { data } => {
                // Select for the type of data
                match data {
                    // Collect time until an event
                    DataType::TimeUntil { event_id } => {
                        // Check to see if there is time remaining for the event
                        if let Some(duration) = self.queue.event_remaining(&event_id) {
                            // Convert the duration to minutes and seconds
                            let minutes = duration.as_secs() / 60;
                            let seconds = duration.as_secs() % 60;

                            // Compose a string for the log
                            let data_string = format!("Time {}:{}", minutes, seconds);

                            // Save the data to the game log
                            update!(save &self.general_update => data_string);
                        }
                    }

                    // Collect time passed until an event
                    DataType::TimePassedUntil {
                        event_id,
                        total_time,
                    } => {
                        // Check to see if there is time remaining for the event
                        if let Some(remaining) = self.queue.event_remaining(&event_id) {
                            // Subtract the remaining time from the total time
                            if let Some(result) = total_time.checked_sub(remaining) {
                                // Convert the duration to minutes and seconds
                                let minutes = result.as_secs() / 60;
                                let seconds = result.as_secs() % 60;

                                // Compose a string for the log
                                let data_string = format!("Time {}:{}", minutes, seconds);

                                // Save the data to the game log
                                update!(save &self.general_update => data_string);
                            }
                        }
                    }

                    // Send the static string to the event
                    DataType::StaticString { string } => {
                        // Save the string to the game log
                        update!(save &self.general_update => string);
                    }

                    // Solicit a string from the user
                    DataType::UserString => {
                        // Error that this is not yet implemented
                        update!(err &self.general_update => "Saving a User String is not yet implemented.");
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
                        if let Some(duration) = self.queue.event_remaining(&event_id) {
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
                        if let Some(remaining) = self.queue.event_remaining(&event_id) {
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

            // If there is a grouped event, trigger the corresponding event
            GroupedEvent {
                status_id,
                event_map,
            } => {
                // Try to retrieve the group status state
                if let Some(state) = self.config.get_state(&status_id) {
                    // Try to find the corresponding event in the event_map
                    if let Some(event_id) = event_map.get(&state) {
                        // Trigger the event if it was found
                        self.queue.add_event(EventDelay::new(None, event_id.clone()));
                        
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
    use super::*;

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
                    "Load Events Or Save Data (Grouped Event)",
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
                    "Load Events Or Save Data (Grouped Event)",
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
                    "Load Events Or Save Data (Grouped Event)",
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
