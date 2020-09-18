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

//! A module to load the configuration from a file and maintain the state
//! machine. This module handles any changes to the current state of the
//! program.

// Reexport the key structures and types
pub use self::status::{FullStatus, StatusDescription, Status};

// Define private submodules
mod status;

// Import the relevant structures into the correct namespace
use self::status::{StatusHandler, StatusMap};
use super::super::system_connection::ConnectionSet;
use super::super::{ChangeSettings, DisplaySetting, GeneralUpdate, InterfaceUpdate};
use super::event::{
    CancelEvent, Event, EventUpdate, SelectEvent, ModifyStatus, NewScene, CueEvent,
    SaveData, SendData,
};
use super::item::{Hidden, ItemDescription, ItemId, ItemPair};

// Import standard library features
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc;

// Import tokio features
use tokio::process::Command;
use tokio::runtime::Handle;

// Import the failure crate
use failure::Error;

// Import FNV HashMap and HashSet
use fnv::{FnvHashMap, FnvHashSet};

// Import YAML processing library
use serde_yaml;

/// Define the instance identifier. Instances with the same identifier will trigger
/// events with one another; instances with different identifiers will not.
/// If no identifier is specified, this instance will accept all events and
/// produce events with the identifier 0.
///
#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Identifier {
  pub id: Option<u32>,  // An optionally-specified identifier for this instance
}

// Implement display for identifier
impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.id {
            &Some(ref id) => write!(f, "{}", id),
            _ => write!(f, "default"),
        }
    }
}

/// Define the itemid and itempair definition of a key map
///
type KeyMapId = FnvHashMap<u32, ItemId>;
pub type KeyMap = FnvHashMap<u32, ItemPair>;

/// A structure to define the parameters of a scene
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    pub events: FnvHashSet<ItemId>, // hash set of the events in this scene
    pub key_map: Option<KeyMapId>,  // an optional mapping of key codes to events
}

/// A structure to define the parameters of a scene, storing the ItemPairs
/// as opposed to only ItemIds
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DescriptiveScene {
    pub events: Vec<ItemPair>,     // a vector of the events and descriptions in this scene
    pub key_map: Option<FnvHashMap<u32, ItemPair>>,  // an optional mapping of event ids to key codes
}

/// A struct to define the elements of a background process
///
#[derive(Clone, Serialize, Deserialize)]
struct BackgroundProcess {
    process: PathBuf,       // the location (relative or absolute) of the process to run
    arguments: Vec<String>, // any arguments to pass to the process
    keepalive: bool, // a flag to indicate if the process should be restarted if it stops/fails
}

/// A simple structure to hold and manage the background process
///
struct BackgroundThread {
    background_process: BackgroundProcess, // a copy of the background process info
}

// Implement the BackgroundThread Functions
impl BackgroundThread {
    /// Spawn the monitoring thread
    async fn new(
        handle: Handle,
        background_process: BackgroundProcess,
        general_update: GeneralUpdate,
    ) -> Option<BackgroundThread> {
        // Check to see if the file is valid
        if let Ok(path) = background_process.process.canonicalize() {
            // Notify that the background process is starting
            update!(update general_update => "Starting Background Process ...");

            // Create the child process
            let child = match Command::new(path.clone())
                .args(background_process.arguments.clone())
                .kill_on_drop(true)
                .spawn()
            {
                // If the child process was created, return it
                Ok(child) => child,

                // Otherwise, warn of the error and return
                _ => {
                    update!(err general_update => "Unable To Start Background Process.");
                    return None;
                }
            };

            // Extract the arguments
            let arguments = background_process.arguments.clone();
            let keepalive = background_process.keepalive;

            // Spawn a background thread to monitor the process
            handle.spawn(async move {
                // Run indefinitely or until the process fails
                loop {
                    // Wait for the process to finish
                    match child.await {
                        // If the process has terminated
                        Ok(status) => {
                            // Notify that the process was a success and restart
                            if status.success() {
                                update!(update general_update => "Background Process Finished Normally.");

                            // Otherwise, notify of a failed process
                            } else {
                                update!(err general_update => "Background Process Finished Abnormally.");
                            }
                        }

                        // If the process failed to run
                        _ => {
                            update!(err general_update => "Unable To Run Background Process.");
                            break;
                        }
                    }

                    // If the process has finished, and we want to keep it alive
                    if keepalive {
                        // Notify that the background process is restarting
                        update!(update general_update => "Restarting Background Process ...");

                        // Start the process again
                        child = match Command::new(path.clone()).args(arguments.clone()).spawn()
                        {
                            // If the child process was created, return it
                            Ok(child) => child,

                            // Otherwise, warn of the error and end the thread
                            _ => {
                                update!(err general_update => "Unable To Run Background Process.");
                                break;
                            }
                        };

                    // Otherwise, exit the loop and finish the thread
                    } else {
                        break;
                    }
                }
            });

            // Return the completed background thread
            Some(BackgroundThread {
                background_process,
            })

        // Warn that the process wasn't found
        } else {
            update!(err general_update => "Unable To Find Background Process.");
            None
        }
    }

    /// A helper method to return a copy of the background process info
    fn background_process(&self) -> BackgroundProcess {
        self.background_process.clone()
    }
}

/// A special configuration struct that is designed to allow simple
/// serialization and deserialization for the program configuration file.
/// Only used internally.
///
#[derive(Serialize, Deserialize)]
struct YamlConfig {
    version: String,    // a version tag to warn the user of incompatible versions
    identifier: Identifier, // unique identifier for the controller instance, if specified
    server_location: Option<String>, // the location of the backup server, if specified
    system_connection: ConnectionSet, // the type of connection(s) to the underlying system
    background_process: Option<BackgroundProcess>, // an option background process to run
    default_scene: Option<ItemId>, // the starting scene for the configuration
    fullscreen: Option<bool>, // whether the interface should begin fullscreen
    all_scenes: FnvHashMap<ItemId, Scene>, // hash map of all availble scenes
    status_map: StatusMap, // hash map of the default game status
    event_set: FnvHashMap<ItemPair, Option<Event>>, // hash map of all the item pairs and events
} // Private struct to allow deserialization of the configuration

/// A structure to hold the whole configuration for current instantiation of the
/// program. As part of this configuration, this structure holds the description
/// lookup for all event, group, and scene ids. This structure also holds the
/// current active and modifyable scene of the program.
///
pub struct Config {
    identifier: Identifier,           // unique identifier for the controller instance
    system_connection: ConnectionSet, // the type of connection(s) to the underlying system
    server_location: Option<String>,  // the location of the backup server, if specified
    background_thread: Option<BackgroundThread>, // a copy of the background process info
    current_scene: ItemId,            // identifier for the current scene
    all_scenes: FnvHashMap<ItemId, Scene>, // hash map of all availble scenes
    status_handler: StatusHandler,    // status handler for the current game status
    lookup: FnvHashMap<ItemId, ItemDescription>, // hash map of all the item descriptions
    events: FnvHashMap<ItemId, Event>, // hash map of all the events
    general_update: GeneralUpdate,    // line to provide updates to the higher-level system
}

// Implement the Config functions
impl Config {
    /// A function to create a new config from a configuration file
    ///
    /// This function uses a file to fill out the game configuration. The
    /// the format of the configuration file is YAML (http://yaml.org/) and must
    /// match the structure of the private YamlConfig structure. In addition,
    /// the configuration file must preserve a number of invarients to run
    /// properly when loaded.
    ///
    /// It is highly recommended that you use the provided configuration
    /// generation/modification tool to create the configuration file.
    ///
    /// # Errors
    ///
    /// This function will raise an error if it is unable to parse the
    /// configuration file and will raise a warning if there is an internal
    /// consistency problem with the provided configuration.
    ///
    /// Like all EventHandler functions and methods, this function will fail
    /// gracefully by notifying of any errors on the update line and returning
    /// None.
    ///
    pub async fn from_config(
        handle: Handle,
        general_update: GeneralUpdate,
        interface_send: mpsc::Sender<InterfaceUpdate>,
        mut config_file: &File,
    ) -> Result<Config, Error> {
        // Try to read from the configuration file
        let mut config_string = String::new();
        match config_file.read_to_string(&mut config_string) {
            Ok(_) => (),
            Err(error) => {
                update!(err general_update => "Invalid Configuration File: {}", error);
                return Err(format_err!("Invalid configuration file: {}", error));
            }
        }

        // Try to parse the configuration file
        let yaml_config: YamlConfig = match serde_yaml::from_str(config_string.as_str()) {
            Ok(config) => config,
            Err(error) => {
                update!(err general_update => "Unable To Parse Configuration File: {}", error);
                return Err(format_err!("Unable to parse configuration file: {}", error));
            }
        };

        // Check the version id and warn the user if they differ
        let version = env!("CARGO_PKG_VERSION");
        if &yaml_config.version != version {
            update!(warn general_update => "Version Of Configuration ({}) Does Not Match Software Version ({})", &yaml_config.version, version);
        }

        // Turn the ItemPairs in to the lookup and event set
        let mut lookup = FnvHashMap::default();
        let mut events = FnvHashMap::default();
        for (item_pair, possible_event) in yaml_config.event_set.iter() {
            // Insert the event description into the lookup
            match lookup.insert(item_pair.get_id(), item_pair.get_description()) {
                // Warn of events defined multiple times
                Some(_) => update!(warn general_update => "Item {} Has Multiple Definitions In Lookup.", &item_pair.id()),
                None => (),
            }

            // If the event is specified
            if let &Some(ref event) = possible_event {
                // Insert the event into the events hash map
                match events.insert(item_pair.get_id(), event.clone()) {
                    // Warn of an event defined multiple times
                    Some(_) => update!(warn general_update => "Item {} Has Multiple Definitions In Event List.", &item_pair.id()),
                    None => (),
                }
            }
        }

        // Verify the configuration is defined correctly
        let all_scenes = yaml_config.all_scenes;
        let status_map = yaml_config.status_map;
        Config::verify_config(&general_update, &all_scenes, &status_map, &lookup, &events).await;

        // Create the new status handler
        let status_handler = StatusHandler::new(general_update.clone(), status_map);

        // Try to load the default scene
        let mut current_scene = ItemId::all_stop(); // an invalid scene id
        if let Some(scene_id) = yaml_config.default_scene {
            // Check to see if the scene_id is valid and warn of an error
            if let Some(..) = all_scenes.get(&scene_id) {
                // Update the current scene id
                current_scene = scene_id;
            } else {
                update!(warn general_update => "Current Scene Is Not Defined.")
            }
        }

        // Try to start the background process and monitor it, if specified
        let mut background_thread = None;
        if let Some(background_process) = yaml_config.background_process.clone() {
            background_thread = BackgroundThread::new(handle, background_process, general_update.clone()).await;
        }

        // Adjust fullscreen, if specified
        if let Some(fullscreen) = yaml_config.fullscreen {
            interface_send
                .send(ChangeSettings {
                    display_setting: DisplaySetting::FullScreen(fullscreen),
                })
                .unwrap_or(());
        }

        // Return the new configuration
        Ok(Config {
            identifier: yaml_config.identifier,
            system_connection: yaml_config.system_connection,
            server_location: yaml_config.server_location,
            background_thread,
            current_scene,
            all_scenes,
            status_handler,
            lookup,
            events,
            general_update,
        })
    }

    /// A method to return the identifier for this program instance.
    ///
    pub fn identifier(&self) -> Identifier {
        self.identifier.clone()
    }

    /// A method to return a copy of the system connection type.
    ///
    pub fn system_connection(&self) -> (ConnectionSet, Identifier) {
        (self.system_connection.clone(), self.identifier())
    }

    /// A method to return the backup server location
    pub fn server_location(&self) -> Option<String> {
        self.server_location.clone()
    }

    /// A method to return the description of a particular item from the lookup.
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
    pub async fn get_description(&self, item_id: &ItemId) -> ItemDescription {
        // Return an item description based on the provided item id
        match self.lookup.get(item_id) {
            // If the item is in the lookup, return the description
            Some(description) => description.clone(),

            // Otherwise, warn the system and return the default
            None => {
                update!(warn &self.general_update => "Item Has No Description: {}", item_id);
                ItemDescription::new("No Description.", Hidden)
            }
        }
    }

    /// A method to return a status from the status handler.
    ///
    /// # Errors
    ///
    /// This method will return None if the provided id was not found in
    /// the status handler. This usually indicates that the provided id was incorrect or
    /// that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning None.
    ///
    pub fn get_status(&self, item_id: &ItemId) -> Option<Status> {
        // Return a status based on the provided item id
        self.status_handler.get_status(item_id)
    }

    /// A method to return a vector of the valid status ids.
    ///
    /// # Errors
    ///
    /// This method doesn't return any errors.
    ///
    pub fn get_status_ids(&self) -> Vec<ItemId> {
        self.status_handler.get_ids()
    }

    /// A method to silently update the status of the system based on a previous
    /// backup.
    ///
    /// # Errors
    ///
    /// This method will raise an error if one of the status ids was not found
    /// in the status map. This indicates that the configuration file is
    /// incomplete or that one of the provided pairs was was incorrect.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line.
    ///
    pub async fn load_backup_status(&mut self, mut status_pairs: Vec<(ItemId, ItemId)>) {
        // For every status in the status pairs, set the current value
        for (status_id, new_state) in status_pairs.drain(..) {
            self.status_handler.modify_status(&status_id, &new_state); // Ignore errors

            // Notify the system of the successful status change
            let status_pair =
                ItemPair::from_item(status_id.clone(), self.get_description(&status_id).await);
            let state_pair =
                ItemPair::from_item(new_state.clone(), self.get_description(&new_state).await);
            update!(status &self.general_update => status_pair, state_pair);
        }
    }

    /// A method to return a hashmap of the full status available in this
    /// configuration.
    ///
    /// # Errors
    ///
    /// This method will raise an error if one of the status ids was not found in
    /// the status map. This indicates that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning an
    /// empty ItemDescription for that status.
    ///
    pub fn get_full_status(&self) -> FullStatus {
        // Get the full status from the status map
        self.status_handler
            .get_full_status(async |id| self.get_description(id).await)
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
    pub async fn get_scenes(&self) -> Vec<ItemPair> {
        // Compile a list of the available scenes
        let mut id_vec = Vec::new();
        for scene_id in self.all_scenes.keys() {
            id_vec.push(scene_id);
        }

        // Sort them in order and then pair them with their descriptions
        id_vec.sort_unstable();
        let mut scenes = Vec::new();
        for scene_id in id_vec {
            let description = self.get_description(scene_id).await;
            scenes.push(ItemPair::from_item(scene_id.clone(), description));
        }

        // Return the result
        scenes
    }


    /// A method to return a scene, given an ItemId. If the id corresponds to a valid scene,
    /// the method returns the scene. Otherwise, it returns None.
    ///
    pub async fn get_scene(&self, item_id: ItemId) -> Option<DescriptiveScene> {
        // Check if the given item id corresponds to a scene
        match self.all_scenes.get(&item_id) {
            // If it doesn't correspond to a scene, return none
            None => None,

            // If it does match, get the items and optional key map
            Some(scene) => {
                // Compile a list of the events as item pairs
                let mut events = Vec::new();
                for item_id in scene.events.iter() {
                    events.push(ItemPair::from_item(item_id.clone(), self.get_description(&item_id).await));
                }

                // Sort the items in order
                events.sort_unstable();

                // Check to see if a key map was specified
                match &scene.key_map {
                    // If the key map exists, reverse it
                    Some(key_map) => {
                        // Create an empty key map
                        let mut map = FnvHashMap::default();

                        // Iterate through the key map for this scene
                        for (key, item_id) in key_map.iter() {
                            // Combine the item pair and key value
                            map.insert(key.clone(), ItemPair::from_item(item_id.clone(), self.get_description(&item_id).await));
                        }
                        // Return the Scene with the reversed key map
                        Some(DescriptiveScene { events, key_map: Some(map) })
                    }

                    // Otherwise, return the DescriptiveScene with no key map
                    None => Some(DescriptiveScene { events, key_map: None }),
                }
            }
        }
    }

    /// A method to return an itempair of all items available in the lookup.
    ///
    pub fn get_items(&self) -> Vec<ItemPair> {
        // Create an empty items vector
        let mut items = Vec::new();
        for (item, description) in self.lookup.iter() {
            items.push(ItemPair::from_item(item.clone(), description.clone()));
        }
        
        // Sort the items by item id
        items.sort_unstable();

        // Return the result
        items
    }

    /// A method to return an itempair of all available events and statuses
    /// in a scene. The method takes in an option for the scene id. If None,
    /// the method will return the itempairs for the current scene.
    /// This method will always return the items from
    /// lowest to highest id.
    ///
    /// # Errors
    ///
    /// This method will raise an error if one of the item ids was not found in
    /// lookup. This indicates that the configuration file is incomplete.
    ///
    pub async fn get_events(&self) -> Vec<ItemPair> {
        // Create an empty events vector
        let mut items = Vec::new();

        // Try to open the current scene
        if let Some(scene) = self.all_scenes.get(&self.current_scene) {
            // Compile a list of the available items
            let mut id_vec = Vec::new();
            for item_id in scene.events.iter() {
                id_vec.push(item_id);
            }

            // Sort them in order and then pair them with their descriptions
            id_vec.sort_unstable();
            for item_id in id_vec {
                // Get the item description
                let description = self.get_description(&item_id).await;

                // Combine the item id and description
                items.push(ItemPair::from_item(item_id.clone(), description));
            }
        }

        // Return the result
        items
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
    pub async fn get_key_map(&self) -> KeyMap {
        // Create an empty key map
        let mut map = FnvHashMap::default();

        // Try to open the current scene
        if let Some(scene) = self.all_scenes.get(&self.current_scene) {
            // If the key map exists
            if let Some(key_map) = &scene.key_map {
                // Iterate through the key map for this scene
                for (key, id) in key_map.iter() {
                    // Get the item description
                    let description = self.get_description(&id).await;

                    // Combine the item id and description
                    map.insert(key.clone(), ItemPair::from_item(id.clone(), description));
                }
            }
        }

        // Return the result
        map
    }

    /// A method to return an item id of the current state of the provided
    /// item id, or None if it was not found.
    ///
    /// # Errors
    ///
    /// This method will raise an error the provided status id was not found in
    /// lookup. This indicates that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning an
    /// empty ItemDescription for that scene.
    ///
    pub async fn get_state(&self, status_id: &ItemId) -> Option<ItemId> {
        // Return the internal state of the status handler
        self.status_handler.get_state(status_id).await
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
    pub async fn get_current_scene(&self) -> ItemPair {
        // Compose an item pair for the current scene
        ItemPair::from_item(
            self.current_scene.clone(),
            self.get_description(&self.current_scene).await,
        )
    }

    /// A method to select a scene map from existing configuration based on the
    /// provided scene id. If successful, the method returns true. Otherwise,
    /// false.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the provided id was not found in
    /// the configuration. This usually indicates a problem with the underlying
    /// configuration file.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and making no
    /// modifications to the current scene.
    ///
    pub async fn choose_scene(&mut self, scene_id: ItemId) -> Result<(), ()> {
        // Check to see if the scene_id is valid
        if self.all_scenes.contains_key(&scene_id) {
            // Update the current scene id
            self.current_scene = scene_id;

            // Trigger a redraw of the window
            self.general_update.send_redraw();

            // Indicate success
            return Ok(());

        // Warn the system that the selected id doesn't exist
        } else {
            // Warn of the error and indicate failure
            update!(warn &self.general_update => "Scene ID Not Found In Config: {}", scene_id);
            return Err(());
        }
    }

    /// A method to modify a status state within the current scene based
    /// on the provided status id and new state. Method returns the new state or
    /// None. None is returned either because
    ///  * the status was already in this state and the status has the
    ///    no_change_silent flag set, or
    ///  * if the state failed to change because one or both ids are invalid.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the provided id was not found in
    /// the configuration. This usually indicates a problem with the underlying
    /// configuration file.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and making no
    /// modifications to the current scene.
    ///
    pub async fn modify_status(&mut self, status_id: &ItemId, new_state: &ItemId) -> Option<ItemId> {
        // Try to update the underlying status
        if let Some(new_id) = self.status_handler.modify_status(&status_id, &new_state).await {
            // Notify the system of the successful status change
            let status_pair =
                ItemPair::from_item(status_id.clone(), self.get_description(&status_id).await);
            let state_pair = ItemPair::from_item(new_id.clone(), self.get_description(&new_id).await);
            update!(status &self.general_update => status_pair, state_pair.clone());

            // Indicate status change
            return Some(new_id);
        }

        // Indicate no change
        None
    }

    /// A method to modify or add an item with provided item pair
    ///
    pub async fn edit_item(&mut self, item_pair: ItemPair) {
        // If the item is in the lookup, update the description
        if let Some(description) = self.lookup.get_mut(&item_pair.get_id()) {
            // Update the description and notify the system
            *description = item_pair.get_description();
            update!(update &self.general_update => "Item Description Updated: {}", item_pair.description());
            return;
        }

        // Otherwise create a new item in the lookup
        self.lookup
            .insert(item_pair.get_id(), item_pair.get_description());
        update!(update &self.general_update => "Item Description Added: {}", item_pair.description());
    }

    /// A method to modify or add an event with provided event id and new event.
    ///
    pub async fn edit_event(&mut self, event_id: ItemId, possible_event: Option<Event>) {
        // If a new event was specified
        if let Some(new_event) = possible_event {        
            // If the event is in the event list, update the event
            if let Some(event) = self.events.get_mut(&event_id) {
                // Update the event and notify the system
                *event = new_event;
                update!(update &self.general_update => "Event Updated: {}", self.get_description(&event_id).await);
            
            // Otherwise, add the event
            } else {
                update!(update &self.general_update => "Event Added: {}", self.get_description(&event_id).await);
                self.events.insert(event_id, new_event);
            }
        
        // If no new event was specified
        } else {
            // If the event is in the event list, remove it
            if let Some(_) = self.events.remove(&event_id) {
                // Notify the user that it was removed
                update!(update &self.general_update => "Event Removed: {}", self.get_description(&event_id).await);
            }
        }
    }
    
    /// A method to modify or add a status with provided id.
    ///
    pub async fn edit_status(&mut self, status_id: ItemId, new_status: Option<Status>) {
        // Get the item description and then pass the change to the status handler
        let description = self.get_description(&status_id).await.description;
        self.status_handler.edit_status(status_id, new_status, description);
    }

    /// A method to modify or add a scene with provided id.
    ///
    pub async fn edit_scene(&mut self, scene_id: ItemId, possible_scene: Option<Scene>) {
        // If a new scene was specified
        if let Some(new_scene) = possible_scene {
            // If the scene is in the scene list, update the scene
            if let Some(scene) = self.all_scenes.get_mut(&scene_id) {
                // Update the scene and notify the system
                *scene = new_scene;
                update!(update &self.general_update => "Scene Updated: {}", self.get_description(&scene_id).await);
            
            // Otherwise, add the scene
            } else {
                update!(update &self.general_update => "Scene Added: {}", self.get_description(&scene_id).await);
                self.all_scenes.insert(scene_id, new_scene);
            }
        
        // If no new event was specified
        } else {
            // If the scene is in the scene list, remove it
            if let Some(_) = self.all_scenes.remove(&scene_id) {
                // Notify the user that it was removed
                update!(update &self.general_update => "Scene Removed: {}", self.get_description(&scene_id).await);
            }
        }
    }
    
    /// A method to delete the item description within the lookup.
    ///
    pub async fn _delete_description(&mut self, item_id: &ItemId) {
        // Try to remove the item from the lookup
        if let Some(description) = self.lookup.remove(item_id) {
            // Notify the user that it was removed
            update!(update &self.general_update => "Item Description Removed: {}", description);
        }
    }

    /// A method to return the event based on the event id.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the provided id was not found in
    /// the configuration. This usually indicates a problem with the underlying
    /// configuration file.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning
    /// None.
    ///
    pub async fn try_event(&mut self, id: &ItemId, checkscene: bool) -> Option<Event> {
        // If the checkscene flag is set
        if checkscene {
            // Try to open the current scene
            if let Some(scene) = self.all_scenes.get(&self.current_scene) {
                // Check to see if the event is listed in the current scene
                if !scene.events.contains(id) {
                    // If the event is not listed in the current scene, notify
                    update!(warnevent &self.general_update => ItemPair::from_item(id.clone(), self.get_description(&id).await)  => "Event Not In Current Scene.");
                    return None;
                }
            // Warn that there isn't a current scene
            } else {
                update!(err &self.general_update => "Current Scene Not Found.");
                return None;
            }
        }
        
        // Try to return the event
        match self.events.get(id) {
            // Return the found event
            Some(event) => Some(event.clone()),

            // Return None if the id doesn't exist
            None => {
                // Notify of an invalid event
                update!(errevent &self.general_update => ItemPair::from_item(id.clone(), self.get_description(&id).await) => "Event Not Found.");

                // Return None
                None
            }
        }
    }

    /// A method to write the current configuration to a file.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the current configuration is broken
    /// or the provided file was not usable. This usually indicates a problem
    /// the provided file type.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and making no
    /// modifications to the file.
    ///
    pub async fn to_config(&self, mut config_file: &File) {
        // Assemble the event set from the events and lookup
        let mut event_set = FnvHashMap::default();
        for (item_id, description) in self.lookup.iter() {
            // Compose the item pair
            let item_pair = ItemPair::from_item(item_id.clone(), description.clone());

            // Add the event if found
            match self.events.get(item_id) {
                // Include the event, when found
                Some(event) => {
                    event_set.insert(item_pair, Some(event.clone()));
                }

                // Otherwise, default to None
                None => {
                    event_set.insert(item_pair, None);
                }
            }
        }

        // Try to get a copy of the background process
        let background_process = match &self.background_thread {
            &Some(ref bt) => Some(bt.background_process()),
            &None => None,
        };

        // Create a YAML config from the elements
        let yaml_config = YamlConfig {
            version: env!("CARGO_PKG_VERSION").to_string(),
            identifier: self.identifier(),
            server_location: self.server_location.clone(),
            system_connection: self.system_connection.clone(),
            background_process,
            default_scene: Some(self.current_scene.clone()),
            fullscreen: None, // default to no entry, must be manually changed
            all_scenes: self.all_scenes.clone(),
            status_map: self.status_handler.get_map(),
            event_set,
        };

        // Try to parse the configuration
        let config_string = match serde_yaml::to_string(&yaml_config) {
            Ok(config_string) => config_string,
            Err(error) => {
                update!(err &self.general_update => "Unable To Parse Current Configuration: {}", error);
                return;
            }
        };

        // Try to write the configuration to the file
        match config_file.write_all(config_string.as_bytes()) {
            Ok(_) => (),
            Err(error) => update!(err &self.general_update => "Unable To Write Configuration To File: {}", error),
        }
    }

    /// An internal function to verify the configuration.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of warnings on the update line. This function
    /// does not raise any errors.
    ///
    /// # Note
    ///
    /// This function raises a warning at the first inconsistency and is not
    /// guaranteed to catch later inconsistencies.
    ///
    async fn verify_config(
        general_update: &GeneralUpdate,
        all_scenes: &FnvHashMap<ItemId, Scene>,
        status_map: &StatusMap,
        lookup: &FnvHashMap<ItemId, ItemDescription>,
        events: &FnvHashMap<ItemId, Event>,
    ) {
        // Verify each scene in the config
        for (id, scene) in all_scenes.iter() {
            if !Config::verify_scene(
                general_update,
                scene,
                all_scenes,
                status_map,
                lookup,
                events,
            ).await {
                update!(warn general_update => "Broken Scene Definition: {}", id);
            }

            // Verify that the scene is described in the lookup
            if !lookup.contains_key(&id) {
                update!(warn general_update => "Scene Not Described In Lookup: {}", id);
            }
        }
    }

    /// An internal function to verify a particular scene in the context of config.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of warnings on the update line. This function
    /// does not raise any errors.
    ///
    /// # Note
    ///
    /// This function raises a warning at the first inconsistency and is not
    /// guaranteed to catch later inconsistencies.
    ///
    async fn verify_scene(
        general_update: &GeneralUpdate,
        scene: &Scene,
        all_scenes: &FnvHashMap<ItemId, Scene>,
        status_map: &StatusMap,
        lookup: &FnvHashMap<ItemId, ItemDescription>,
        events: &FnvHashMap<ItemId, Event>,
    ) -> bool {
        // Verify that each event in the scene is valid
        let mut test = true;
        for id in scene.events.iter() {
            // Find the matching event
            if let Some(event) = events.get(id) {
                // Verify the event
                if !Config::verify_event(
                    general_update,
                    event,
                    scene,
                    all_scenes,
                    status_map,
                    lookup,
                    events,
                ).await {
                    update!(warn general_update => "Invalid Event: {}", id);
                    test = false;
                }

            // Otherwise verify that the item id corresponds to a status
            } else if let None = status_map.get(id) {
                // Warn that an invalid event or status was listed in the scene
                update!(warn general_update => "Item Listed In Scene, But Not Found: {}", id);
                test = false;
            }

            // Verify that the event is described in the event lookup
            test = test & Config::verify_lookup(general_update, lookup, id).await;
        }

        // If the key map is specified
        if let Some(key_map) = &scene.key_map {
            // Verify that each key mapping matches a valid event
            for (_, id) in key_map.iter() {
                // Make sure the event is listed in the scene
                if !scene.events.contains(id) {
                    update!(warn general_update => "Event In Shortcuts, But Not In Scene: {}", id);
                    test = false;
                } // Events in the scene have already been tested for other validity
            }
        }
        test // return the result
    }

    /// An internal function to verify a particular event in the context
    /// of scene and config.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of warnings on the update line. This function
    /// does not raise any errors.
    ///
    /// # Note
    ///
    /// This function raises a warning at the first inconsistency and is not
    /// guaranteed to catch later inconsistencies.
    ///
    async fn verify_event(
        general_update: &GeneralUpdate,
        event: &Event,
        scene: &Scene,
        all_scenes: &FnvHashMap<ItemId, Scene>,
        status_map: &StatusMap,
        lookup: &FnvHashMap<ItemId, ItemDescription>,
        event_list: &FnvHashMap<ItemId, Event>,
    ) -> bool {
        // Unpack each action in the event
        for action in event {
            // Check each action, exiting early if any action fails the check
            match action {
                // If there is a new scene, verify the id is valid
                &NewScene { ref new_scene } => {
                    // If the desired scene does exist
                    if all_scenes.contains_key(new_scene) {
                        // Verify that the newscene event exists in the new scene
                        if !event_list.contains_key(new_scene) {
                            update!(warn general_update => "Reset Scene Event Missing From Scene: {}", new_scene);
                            return false;
                        }

                    // If the desired scene does not exist
                    } else {
                        // Warn the system and indicate failure
                        update!(warn general_update => "Event Contains Invalid Scene: {}", new_scene);
                        return false;
                    }

                    // If the scene exists, verify the scene is described
                    return Config::verify_lookup(general_update, lookup, new_scene).await;
                }

                // If there is a status modification, verify both components of the modification
                &ModifyStatus {
                    ref status_id,
                    ref new_state,
                } => {
                    // Check that the status_id is valid
                    if let Some(status) = status_map.get(status_id) {
                        // Also verify the new state
                        if !status.is_allowed(new_state) {
                            update!(warn general_update => "Event Contains Invalid New State: {}", &new_state);
                            return false;
                        }
                    } else {
                        update!(warn general_update => "Event Contains Invalid Status Id: {}", &status_id);
                        return false;
                    }

                    // If the status exists, verify the status and state are described
                    return Config::verify_lookup(general_update, lookup, status_id).await
                        & Config::verify_lookup(general_update, lookup, new_state).await;
                }

                // If there is an event to cue, verify that it exists
                &CueEvent { ref event } => {
                    // Verify that the event is listed in the current scene
                    if !scene.events.contains(&event.id()) {
                        update!(warn general_update => "Event Contains Cue Event, But Not In Scene: {}", &event.id());
                        // Do not flag as incorrect
                    }

                    // Return false if the event_id is incorrect
                    if !event_list.contains_key(&event.id()) {
                        update!(warn general_update => "Event Contains Invalid Cue Event: {}", &event.id());
                        return false;
                    } // Don't need to check lookup as all valid individual events are already checked
                }

                // If there are events to cancel, verify that they exist
                &CancelEvent { ref event } => {
                    // Return false if the event doesn't exist
                    if !event_list.contains_key(&event) {
                        update!(warn general_update => "Event Contains Invalid Cancelled Events: {}", &event);
                        return false;
                    } // Don't need to check lookup as all valid individual events are already checked. Don't need to check scene validity because cancelled events are not necessarily in the same scene.
                }

                // If there is data to save or send, assume validity
                &SaveData { .. } => (),
                &SendData { .. } => (),

                // If there is a select event, verify the components of the event
                &SelectEvent {
                    ref status_id,
                    ref event_map,
                } => {
                    // Check that the status_id is valid
                    if let Some(status) = status_map.get(status_id) {
                        // Verify that the allowed states vector isn't empty
                        let allowed = status.allowed();
                        if allowed.is_empty() {
                            update!(warn general_update => "Event Contains Empty Select Event: {}", status_id);
                            return false;
                        }

                        // Verify each event that corresponds to a state is valid
                        for state in allowed.iter() {
                            // If there is a matching event, verify that it exists
                            if let Some(target_event) = event_map.get(state) {
                                // Verify that the event is listed in the current scene
                                if !scene.events.contains(&target_event) {
                                    update!(warn general_update => "Event Group Contains Event, But It Is Not Listed In Scene: {}", &target_event);
                                    // Do not flag as incorrect
                                }

                                // Verify that the event exists
                                if !event_list.contains_key(&target_event) {
                                    update!(warn general_update => "Event Group Contains Invalid Target Event: {}", &target_event);
                                    return false;
                                }

                            // If no event is specified, nothing is triggered
                            }
                        }

                    // If the status doesn't exist, raise a warning
                    } else {
                        update!(warn general_update => "Event Contains Invalid Status: {}", status_id);
                        return false;
                    }
                }
            }
        }
        true // If no errors were thrown
    }

    /// An internal function to verify that a particular id is in the lookup.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of warnings on the status line. This function
    /// does not raise any errors.
    ///
    async fn verify_lookup(
        general_update: &GeneralUpdate,
        lookup: &FnvHashMap<ItemId, ItemDescription>,
        id: &ItemId,
    ) -> bool {
        // Check to see if the id is available in the lookup
        if !lookup.contains_key(&id) {
            update!(warn general_update => "Item Not Described In Lookup: {}", id);
            return false;
        }
        true // Otherwise indicate success
    }
}

// Tests of the scene module
#[cfg(test)]
mod tests {
    use super::*;

    // FIXME Reimplement these out of date tests
    // Write an example configuration
    /*#[cfg(feature = "example_configs")]
    #[test]
    fn write_example_config() {
        // Import features for testing
        use super::super::event::EventDelay;
        use super::super::item::DisplayWith;
        use std::time::Duration;
        use std::sync::mpsc;

        // Write the example select event
        let mut one_select_map = FnvHashMap::default();
        one_select_map.insert(ItemId::new(31).unwrap(), ItemId::new(61).unwrap());
        let one_select_event = SelectEvent {
            group_id: ItemId::new(43).unwrap(),
            status_map: one_select_map,
        };

        // Write the example triggered event
        let trigger_event = TriggerEvents {
            events: vec![
                EventDelay::new(Some(Duration::from_millis(20)), ItemId::new(20).unwrap()),
                EventDelay::new(None, ItemId::new(41).unwrap()),
            ],
        };

        // Write the example scene
        let mut one_scene = scene::default();
        one_scene.insert(ItemId::new(12).unwrap(), SaveData { data: vec![12] });
        one_scene.insert(ItemId::new(20).unwrap(), one_select_event);
        one_scene.insert(ItemId::new(22).unwrap(), trigger_event);
        let mut all_scenes = FnvHashMap::default();
        all_scenes.insert(ItemId::new(24).unwrap(), one_scene);

        // Write the example status map
        let mut status_map = StatusMap::default();
        status_map.insert(
            ItemId::new(51).unwrap(),
            MultiStatus {
                current: ItemId::new(13).unwrap(),
                allowed: vec![ItemId::new(34).unwrap(), ItemId::new(53).unwrap()],
            },
        );

        // Write the example config
        let yaml_config = YamlConfig {
            all_scenes,
            lookup: vec![
                ItemPair::from_item(
                    ItemId::new(12).unwrap(),
                    ItemDescription {
                        description: "Test Description".to_string(),
                        display: DisplayWith(ItemId::new(53).unwrap()),
                    },
                ),
                ItemPair::from_item(
                    ItemId::new(16).unwrap(),
                    ItemDescription {
                        description: "Test 2 Description".to_string(),
                        display: Hidden,
                    },
                ),
            ],
            status_map,
        };

        // Try to parse the configuration
        let config_string = match serde_yaml::to_string(&yaml_config) {
            Ok(config_string) => config_string,
            Err(_) => {
                return;
            }
        };

        // Attempt to open the configuration file
        let mut config_file = match File::create("examples/example_yaml.mnv") {
            Ok(file) => file,
            Err(_) => {
                return;
            }
        };

        // Try to write the configuration to the file
        match config_file.write_all(config_string.as_bytes()) {
            Ok(_) => (),
            Err(_) => (),
        }
    }

    // Test loading a scene from file
    #[test]
    fn load_config_from_file() {
        // Import features for testing
        use std::sync::mpsc;

        // Attempt to open the configuration file
        let config_file = match File::open("examples/testing_config.mnv") {
            Ok(file) => file,
            Err(_) => panic!("Unable to open configuration file."),
        };

        // Create the update line (ignore warning messages)
        let (tx, rx) = mpsc::channel();

        // Attempt to process the configuration file
        match Config::from_config(tx, &config_file) {
            Some(_) => (),
            None => panic!(
                "Unable to load configuration file: {}",
                rx.try_recv().unwrap()
            ),
        }
    }

    // Test config verification
    #[test]
    fn verify_config() {
        // Import features for testing
        use std::sync::mpsc;
        use super::super::event::Warning;

        // Attempt to open the configuration file
        let config_file = match File::open("examples/broken_config.mnv") {
            Ok(file) => file,
            Err(_) => panic!("Unable to open configuration file"),
        };

        // Create the update line (ignore warning messages)
        let (tx, rx) = mpsc::channel();

        // Attempt to process the configuration file
        match Config::from_config(tx, &config_file) {
            Some(_) => (),
            None => panic!(
                "Unable to load configuration file: {}",
                rx.try_recv().unwrap()
            ),
        }

        // Create the test vector
        let test = vec![
            Warning("Event Detail Contains Invalid Status Id: |44|".to_string()),
            Warning("Invalid Event Detail: |8|".to_string()),
            Warning("Broken Scene Definition: |200|".to_string()),
            Warning("Event Detail Contains Invalid Triggered Events: |8|".to_string()),
            Warning("Invalid Event Detail: |2|".to_string()),
            Warning("Broken Scene Definition: |100|".to_string()),
        ];

        // Print and check the messages received (wait at most half a second)
        test_vec!(~rx, test);
    }*/
}
