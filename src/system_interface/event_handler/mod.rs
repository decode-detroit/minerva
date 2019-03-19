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
pub use self::config::{FullStatus, StatusDescription};

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
    EventDelay, EventDetail, EventUpdate, GroupedEvent, ModifyStatus, NewScene, SaveData,
    TriggerEvents, UpcomingEvent,
};
use self::item::{ItemDescription, ItemId, ItemPair};
use self::queue::{ComingEvent, Queue};
use super::system_connection::ConnectionType;
use super::GeneralUpdate;

// Import standard library modules
use std::fs::File;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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
        log_failure: bool,
    ) -> Option<EventHandler> {
        // Attempt to open the configuration file
        let config_file = match File::open(config_path) {
            Ok(file) => file,
            Err(_) => {
                // Only log failure if the flag is set
                if log_failure {
                    update!(err &general_update => "Unable To Open Configuration File.");
                }
                return None;
            }
        };

        // Attempt to process the configuration file
        let mut config = match Config::from_config(general_update.clone(), &config_file) {
            Some(c) => c,
            None => return None,
        };

        // Attempt to create the backup handler
        let backup = match BackupHandler::new(
            general_update.clone(),
            config.identifier(),
            config.server_location(),
        ) {
            Some(b) => b,
            None => return None,
        };

        // Create an empty event queue
        let queue = Queue::new(general_update.clone());

        // Check for existing data from the backup handler
        if let Some((current_scene, status_pairs)) = backup.reload_backup(config.get_status_ids()) {
            // Notify that existing data was found
            update!(err &general_update => "Detected Lingering Backup Data. Reloading ...");

            // Change the current scene silently (i.e. do not trigger the reset event)
            config.choose_scene(current_scene).unwrap_or(());

            // Update the current status states based on the backup
            config.load_backup_status(status_pairs);

        // FIXME Add silent queue changes

        // If there was no existing data in the backup, trigger the scene reset event
        } else {
            queue.add_event(EventDelay::new(None, config.get_current_scene().get_id()));
        }

        // Load the current scene into the backup (to detect any crash after this point)
        backup.backup_current_scene(&config.get_current_scene().get_id());

        // Return the completed EventHandler with a new queue
        Some(EventHandler {
            general_update: general_update,
            queue,
            config,
            backup,
        })
    }

    /// A method to return the configured system connection type.
    ///
    pub fn system_connection(&self) -> (ConnectionType, ItemId) {
        self.config.system_connection()
    }

    /// A method to process a new event in the event handler.
    ///
    /// # Errors
    ///
    /// This method will not raise any errors. If the module encounters an
    /// inconsistency when trying to process an event, it will raise a warning
    /// both otherwise continue processing events.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line.
    ///
    pub fn process_event(&mut self, event_id: &ItemId, broadcast: bool) {
        // Process the event
        self.retrieve_event(event_id, broadcast);
    }

    /// A method to clear the existing events in the timed queue.
    ///
    /// This method clears all the events in the timed queue, effective
    /// retroactively. This means that any events that have not been processed
    /// (even if internally their delay has already expired) will not be
    /// processed.
    ///
    pub fn clear_events(&mut self) {
        self.queue.clear();
    }

    /// A method to return a list upcoming events.
    ///
    /// This method returns an Option depending on when the last update update
    /// occured to the upcoming events. If the upcoming events have changed
    /// since this method was last called, the method returns Some with a vector
    /// of UpcomingEvent inside which correspond to upcoming delayed events
    /// in the event handler. Otherwise, the method returns None.
    ///
    /// # Notes
    ///
    /// The order of the provided list does correspond to the order they events
    /// will occur (last event first). In addition, the method may return
    /// Some with an empty vector inside (i.e. the status changed to have no
    /// upcoming events).
    ///
    pub fn upcoming_events(&self) -> Option<Vec<UpcomingEvent>> {
        // If there are upcoming events, repackage them
        if let Some(mut events) = self.queue.list_events() {
            // Repackage the list as upcoming events
            let mut coming_events = Vec::new();
            for event in events.drain(..) {
                // Find the description and add it
                coming_events.push(UpcomingEvent {
                    start_time: event.start_time,
                    delay: event.delay,
                    event: ItemPair::from_item(event.id(), self.get_description(&event.id())),
                });
            }

            // Return the completed list
            return Some(coming_events);

        // Otherwise, return none
        } else {
            return None;
        }
    }

    /// A method to return a copy of the detail of the provided event id.
    ///
    /// # Errors
    ///
    /// This method will raise an error if the provided id was not found in
    /// configuration. This usually indicates that the provided id was incorrect
    //  or that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning None.
    ///
    pub fn get_detail(&mut self, event_id: &ItemId) -> Option<EventDetail> {
        // Try to retrieve the event detail
        self.config.try_event(event_id)
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

    /// A method to return an itempair of all available events in the current
    /// scene. This method will always return the events from lowest to
    /// highest id.
    ///
    /// # Errors
    ///
    /// This method will raise an error if one of the event ids was not found in
    /// lookup. This indicates that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning an
    /// empty ItemDescription for that scene.
    ///
    pub fn get_events(&self) -> Vec<ItemPair> {
        // Return a list of available events in the current scene
        self.config.get_events()
    }

    /// A method to return true if the provided id corresponds to a status and
    /// false otherwise.
    ///
    /// # Errors
    ///
    /// This method does not return any errors.
    ///
    pub fn is_status(&self, status_id: &ItemId) -> bool {
        // Return whether or not the id corresponds to a status
        self.config.is_status(status_id)
    }

    /// A method to return the current state of the provided status id.
    ///
    /// # Errors
    ///
    /// This method will raise an error if the provided id was not found in
    /// lookup. This usually indicates that the provided id was incorrect or
    /// that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning None.
    ///
    pub fn get_state(&self, status_id: &ItemId) -> Option<ItemId> {
        // Return a new copy of the status state
        self.config.get_state(status_id)
    }

    /// A method to return the current state description of the provided
    /// status id.
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
    pub fn get_state_description(&self, status_id: &ItemId) -> Option<ItemPair> {
        // Return the state combined with its description, if it exists
        if let Some(state) = self.get_state(status_id) {
            return Some(ItemPair::from_item(
                state.clone(),
                self.get_description(&state),
            ));
        }

        // Return on failure
        None
    }

    /// A method to return the allowed states for the provided status id,
    /// ar and empty vector if it was not found.
    ///
    /// # Errors
    ///
    /// This method will raise an error if the provided id was not found in
    /// lookup. This usually indicates that the provided id was incorrect or
    /// that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning an
    /// empty vector.
    ///
    pub fn get_allowed_states(&self, status_id: &ItemId) -> Vec<ItemPair> {
        // Return a vector of the allowed states
        self.config.get_allowed_states(status_id)
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
        if self.config.modify_status(status_id, new_state).is_ok() {
            // Backup the status change
            self.backup.backup_status(status_id, new_state);

            // Run the change event for the new state
            self.queue
                .add_event(EventDelay::new(None, new_state.clone()));
        }
    }

    /// A method to add or modify the event detail within the current configuration.
    ///
    /// # Errors
    ///
    /// This method will raise a warning if the new event detail creates an
    /// inconsistency within the configuration.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and leaving the
    /// current configuration unmodified.
    ///
    pub fn edit_event(&mut self, event_pair: &ItemPair, new_detail: &EventDetail) {
        // Modify the underlying event
        self.config.edit_event(event_pair, new_detail);
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

            // Run the reset event for the new scene
            self.queue.add_event(EventDelay::new(None, scene_id));
        }
    }

    /// A method to change the remaining delay for the provided event currently
    /// in the queue.
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
    pub fn adjust_event(&self, event_id: ItemId, start_time: Instant, new_delay: Duration) {
        // Try to modify the provided event in the current queue
        self.queue.adjust_event(ComingEvent {
            event_id,
            start_time,
            delay: new_delay,
        });
    }

    /// A method to save the current configuration to the provided file.
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

    /// An internal function to try retrieve the event details and act upon them.
    ///
    fn retrieve_event(&mut self, event_id: &ItemId, broadcast: bool) {
        // Try to retrieve the event details and unpack the event
        let event_detail = match self.config.try_event(event_id) {
            // Process a valid event
            Some(event_detail) => event_detail,

            // Ignore an invalid event
            None => return,
        };

        // Process and update the system about the event
        self.unpack_event(event_detail);
        if broadcast {
            // Broadcast the event
            let pair = ItemPair::from_item(event_id.clone(), self.get_description(&event_id));
            update!(broadcast &self.general_update => pair);

        // Update the system about the event
        } else {
            let pair = ItemPair::from_item(event_id.clone(), self.get_description(&event_id));
            update!(now &self.general_update => pair);
        }
    }

    /// An internal function to unpack the event detail and act on it.
    ///
    fn unpack_event(&mut self, event_detail: EventDetail) {
        // Unpack the event
        match event_detail {
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

            // If there are triggered events to load, load them into the queue
            TriggerEvents { events } => {
                // Unpackage each coming event and add it to the queue
                for event_delay in events {
                    // Add the triggered events to the queue
                    self.queue.add_event(event_delay);
                }
            }

            // If there is data to save, save it
            SaveData { data } => {
                // Save the data to the game log
                update!(save &self.general_update => data);
            }

            // If there is a grouped event, trigger the corresponding event
            GroupedEvent {
                status_id,
                event_map,
            } => {
                // Try to retrieve the group status state
                if let Some(state) = self.config.get_state(&status_id) {
                    // Try to find the corresponding event in the event_map
                    match event_map.get(&state) {
                        // Trigger the event if it was found
                        Some(event_id) => self.retrieve_event(event_id, true),

                        // Otherwise warn the system the event was not found
                        None => {
                            update!(warn &self.general_update => "Unable To Find State In Grouped Event: {}", state)
                        }
                    }
                }
            }
        }
    }
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
                        priority: None,
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
                        priority: None,
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
                        priority: None,
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
