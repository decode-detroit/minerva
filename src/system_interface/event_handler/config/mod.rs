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
//!
//! # TODO
//!
//! * Write full configuration file examples
//!


// Reexport the key structures and types
pub use self::status::{StatusDescription, FullStatus};

// Define private submodules
mod status;

// Import the relevant structures into the correct namespace
use self::status::{StatusHandler, StatusMap};
use super::item::{ItemId, ItemDescription, ItemPair, Hidden};
use super::event::{EventDetail, NewScene, ModifyStatus, TriggerEvents, SaveData, GroupedEvent, EventUpdate};
use super::super::system_connection::ConnectionType;
use super::super::GeneralUpdate;

// Import standard library features
use std::fs::File;
use std::io::Read;
use std::io::Write;

// Import FNV HashMap
extern crate fnv;
use self::fnv::{FnvHashMap, FnvHashSet};

// Import YAML processing library
extern crate serde_yaml;


/// A type definition to clarify the variety of the scene set.
///
type Scene = FnvHashSet<ItemId>; // hash set of the events in this scene

/// A special configuration struct that is designed to allow simple
/// serialization and deserialization for the program configuration file.
/// Only used internally.
///
#[derive(Serialize, Deserialize)]
struct YamlConfig {
    version: String, // a version tag to warn the user of incompatible versions
    identifier: ItemId, // unique identifier for the program instance
    server_location: Option<String>, // the location of the backup server, if specified
    system_connection: ConnectionType, // the type of connection to the underlying system
    default_scene: Option<ItemId>, // the starting scene for the configuration
    all_scenes: FnvHashMap<ItemId, Scene>, // hash map of all availble scenes
    status_map: StatusMap, // hash map of the default game status
    event_set: FnvHashMap<ItemPair, Option<EventDetail>>, // hash map of all the item pairs and event details
} // Private struct to allow deserialization of the configuration


/// A structure to hold the whole configuration for current instantiation of the
/// program. As part of this configuration, this structure holds the description
/// lookup for all event, group, and scene ids. This structure also holds the
/// current active and modifyable scene of the program.
///
pub struct Config {
    identifier: ItemId, // unique identifier for the program instance
    system_connection: ConnectionType, // the type of connection to the underlying system
    server_location: Option<String>, // the location of the backup server, if specified
    current_scene: ItemId, // identifier for the current scene
    all_scenes: FnvHashMap<ItemId, Scene>, // hash map of all availble scenes
    status_handler: StatusHandler, // status handler for the current game status
    lookup: FnvHashMap<ItemId, ItemDescription>, // hash map of all the item descriptions
    events: FnvHashMap<ItemId, EventDetail>, // hash map of all the item details
    update_line: GeneralUpdate // line to provide updates to the higher-level system
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
    pub fn from_config(update_line: GeneralUpdate, mut config_file: &File) -> Option<Config> {
        
        // Try to read from the configuration file
        let mut config_string = String::new();
        match config_file.read_to_string(&mut config_string) {
            Ok(_) => (),
            Err(error) => {
                update!(err update_line => "Invalid Configuration File: {}", error);
                return None;
            },
        }
        
        // Try to parse the configuration file
        let yaml_config: YamlConfig = match serde_yaml::from_str(config_string.as_str()) {
            Ok(config) => config,
            Err(error) => {
                update!(err update_line => "Unable To Parse Configuration File: {}", error);
                return None;
            },
        };
        
        // Check the version id and warn the user if they differ
        let version = env!("CARGO_PKG_VERSION");
        if &yaml_config.version != version {
            update!(warn update_line => "Version Of Configuration ({}) Does Not Match Software Version ({})", &yaml_config.version, version);
        }
        
        // Turn the ItemPairs in to the lookup and event set
        let mut lookup = FnvHashMap::default();
        let mut events = FnvHashMap::default();
        for (item_pair, possible_detail) in yaml_config.event_set.iter() {
            
            // Insert the event description into the lookup
            match lookup.insert(item_pair.get_id(), item_pair.get_description()) {
            
                // Warn of events defined multiple times
                Some(_) => update!(warn update_line => "Item {} Has Multiple Definitions In Lookup.", &item_pair.id()),
                None => (),
            }
            
            // If the event detail is specified
            if let &Some(ref event_detail) = possible_detail {
            
                // Insert the event detail into the events hash map
                match events.insert(item_pair.get_id(), event_detail.clone()) {
                
                    // Warn of an event detail defined multiple times
                    Some(_) => update!(warn update_line => "Item {} Has Multiple Definitions In Event List.", &item_pair.id()),
                    None => (),
                }    
            }
        }
        
        // Verify the configuration is defined correctly
        let all_scenes = yaml_config.all_scenes;
        let status_map = yaml_config.status_map;
        Config::verify_config(&update_line, &all_scenes, &status_map, &lookup, &events);
        
        // Create the new status handler
        let status_handler = StatusHandler::new(update_line.clone(), status_map);
        
        // Try to load the default scene
        let mut current_scene = ItemId::all_stop(); // an invalid scene id
        if let Some(scene_id) = yaml_config.default_scene {
        
            // Check to see if the scene_id is valid and fail silently
            if let Some(..) = all_scenes.get(&scene_id) {
            
                // Update the current scene id
                current_scene = scene_id;
            }
        }
        
        // Return the new configuration
        Some(Config {
            identifier: yaml_config.identifier,
            system_connection: yaml_config.system_connection,
            server_location: yaml_config.server_location,
            current_scene,
            all_scenes,
            status_handler,
            lookup,
            events,
            update_line,
        })
    }
    
    /// A method to return the identifier for this program instance.
    ///
    pub fn identifier(&self) -> ItemId {
        self.identifier.clone()
    }
    
    /// A method to return a copy of the system connection type.
    ///
    pub fn system_connection(&self) -> (ConnectionType, ItemId) {
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
    pub fn get_description(&self, item_id: &ItemId) -> ItemDescription {
        
        // Return an item description based on the provided item id
        match self.lookup.get(item_id) {
            
            // If the item is in the lookup, return the description
            Some(description) => description.clone(),
            
            // Otherwise, warn the system and return the default
            None => {
                update!(warn &self.update_line => "Item Has No Description: {}", item_id);
                ItemDescription::new("No Description.", Hidden)
            },
        }
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
    pub fn load_backup_status(&mut self, mut status_pairs: Vec<(ItemId, ItemId)>) {
    
        // For every status in the status pairs, set the current value
        for (status_id, new_state) in status_pairs.drain(..) {
            self.status_handler.modify_status(&status_id, &new_state);
            
            // Notify the system of the successful status change
            let status_pair = ItemPair::from_item(status_id.clone(), self.get_description(&status_id));
            let state_pair = ItemPair::from_item(new_state.clone(), self.get_description(&new_state));
            update!(status &self.update_line => status_pair, state_pair);
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
        self.status_handler.get_full_status(|id| {self.get_description(id)} )
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
    
        // Compile a list of the available scenes
        let mut id_vec = Vec::new();
        for scene_id in self.all_scenes.keys() {
            id_vec.push(scene_id);
        }
        
        // Sort them in order and then pair them with their descriptions
        id_vec.sort_unstable();
        let mut scenes = Vec::new();
        for scene_id in id_vec {
            let description = self.get_description(scene_id);
            scenes.push(ItemPair::from_item(scene_id.clone(), description));
        }
        
        // Return the result
        scenes        
    }
    
    /// A method to return an itempair of all available events in the current
    /// scene. This method will always return the scenes from lowest to
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
    
        // Create an empty events vector
        let mut events = Vec::new();
        
        // Try to open the current scene
        if let Some(scene) = self.all_scenes.get(&self.current_scene) {
    
            // Compile a list of the available events
            let mut id_vec = Vec::new();
            for event_id in scene.iter() {
                id_vec.push(event_id);
            }
            
            // Sort them in order and then pair them with their descriptions
            id_vec.sort_unstable();
            for event_id in id_vec {
                
                // Get the event description and add it to the events list
                let description = self.get_description(&event_id);
                
                // Prefilter the events to remove hidden events
                events.push(ItemPair::from_item(event_id.clone(), description));
            }
        }
        
        // Return the result
        events
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
        self.status_handler.is_status(status_id)
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
    pub fn get_state(&self, status_id: &ItemId) -> Option<ItemId> {
    
        // Return the internal state of the status handler
        self.status_handler.get_id(status_id)
    }
    
    /// A method to return a vector of itempairs of the allowed states for
    /// the provided status id. This vector will be empty if it was not found.
    ///
    /// # Errors
    /// 
    /// This method will raise an error the provided status id was not found in
    /// the lookup. This indicates that the configuration file is incomplete.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the status line and returning an
    /// empty vector.
    ///
    pub fn get_allowed_states(&self, status_id: &ItemId) -> Vec<ItemPair> {
    
        // Try to find the allowed states for the status
        let state_ids = self.status_handler.get_allowed(status_id);
        
        // Repackage the states with their corresponding descriptions
        let mut states = Vec::new();
        for id in state_ids.iter() {
            states.push(ItemPair::from_item(id.clone(), self.get_description(id)));
        }
        
        // Return the complete list of allowed states
        states
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
    
        // Compose an item pair for the current scene
        ItemPair::from_item(self.current_scene.clone(), self.get_description(&self.current_scene))
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
    pub fn choose_scene(&mut self, scene_id: ItemId) -> Result<(),()> {
        
        // Check to see if the scene_id is valid
        if self.all_scenes.contains_key(&scene_id) {
                
            // Update the current scene id
            self.current_scene = scene_id;
            
            // Trigger a redraw of the window
            self.update_line.send_redraw();
            
            // Indicate success
            return Ok(());
            
        // Warn the system that the selected id doesn't exist
        } else {
            
            // Warn of the error and indicate failure
            update!(warn &self.update_line => "Scene ID Not Found In Config: {}", scene_id);
            return Err(());
        }
    }
    
    /// A method to modify a status state within the current scene based
    /// on the provided status id and new state.
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
    pub fn modify_status(&mut self, status_id: &ItemId, new_state: &ItemId) -> Result<(),()> {
        
        // Try to update the underlying status
        if self.status_handler.modify_status(&status_id, &new_state) {
        
            // Notify the system of the successful status change
            let status_pair = ItemPair::from_item(status_id.clone(), self.get_description(&status_id));
            let state_pair = ItemPair::from_item(new_state.clone(), self.get_description(&new_state));
            update!(status &self.update_line => status_pair, state_pair.clone());
            
            // Indicate success
            return Ok(());
        }
        
        // Indicate failure
        return Err(());
    }
    
    /// A method to modify or add the item description within the current
    /// lookup based on the provided id and new description.
    ///
    /// # Errors
    ///
    /// This function will notify the system if it updated the description or
    /// added a new description.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and making no
    /// modifications to the current scene.
    ///
    pub fn edit_description(&mut self, item_pair: &ItemPair) {
        
        // If the item is in the lookup, update the description
        if let Some(description) = self.lookup.get_mut(&item_pair.get_id()) {
            
            // Update the description and notify the system
            *description = item_pair.get_description();
            update!(update &self.update_line => "Item Description Updated: {}", item_pair.description());
            return;
        }
        
        // Otherwise create a new item in the lookup
        self.lookup.insert(item_pair.get_id(), item_pair.get_description());
        update!(update &self.update_line => "Item Description Added: {}", item_pair.description());
    }
    
    /// A method to modify or add event detail within the current scene based
    /// on the provided event id and new detail.
    ///
    /// # Errors
    /// 
    /// This method will raise a warning if the new event detail creates an
    /// inconsistency within the configuration.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and making no
    /// modifications to the current scene.
    ///
    pub fn edit_event(&mut self, event_pair: &ItemPair, new_detail: &EventDetail) {
        
        // Update or add the event description in the lookup
        self.edit_description(event_pair);
        
        // If the event is in the event list, update the event detail
        if let Some(detail) = self.events.get_mut(&event_pair.get_id()) {
        
            // Update the detail and notify the system
            *detail = new_detail.clone();
            update!(update &self.update_line => "Event Detail Updated: {}", event_pair.description());
            return;
        }
        
        // Otherwise, add the event and event detail
        self.events.insert(event_pair.get_id(), new_detail.clone());
        update!(update &self.update_line => "Event Detail Added: {}", event_pair.description());
    }
    
    /// A method to return the event detail based on the event id.
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
    pub fn try_event(&mut self, id: &ItemId) -> Option<EventDetail> {
        
        // Try to open the current scene
        if let Some(scene) = self.all_scenes.get(&self.current_scene) {
        
            // Check to see if the event is listed in the current scene
            if scene.contains(id) {
            
                // Return the event detail for the event
                return match self.events.get(id) {
                
                    // Return the found event detail
                    Some(detail) => Some(detail.clone()),
                    
                    // Return None if the id doesn't exist in the scene
                    None => {
                         
                        // Warn that the event could not be found
                        update!(warn &self.update_line => "Event ID Not Found In Configuration: {}", &self.current_scene);
                        
                        // Return None
                        None
                    },
                };
            }
            
            // If the event is not listed in the current scene, return None
            None
        
        // Warn that there isn't a current scene
        } else {
            update!(warn &self.update_line => "Scene ID Not Found In Configuration: {}", &self.current_scene);
            return None;
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
    pub fn to_config(&self, mut config_file: &File) {
        
        // Convert the configuration to YamlConfig
        let mut lookup = Vec::new();
        for (item, name) in self.lookup.iter() {
            lookup.push(ItemPair::from_item(item.clone(), name.clone()));
        }
        
        // Assemble the event set from the events and lookup
        let mut event_set = FnvHashMap::default();
        for (item_id, description) in self.lookup.iter() {
            
            // Compose the item pair
            let item_pair = ItemPair::from_item(item_id.clone(), description.clone());
            
            // Add the event detail, if found
            match self.events.get(item_id) {
            
                // Include the event detail, when found
                Some(detail) => {
                    event_set.insert(item_pair, Some(detail.clone()));
                },
                
                // Otherwise, default to None
                None => {
                    event_set.insert(item_pair, None);
                },
            }
        }
        
        // Create a YAML config from the elements
        let yaml_config = YamlConfig {
            version: env!("CARGO_PKG_VERSION").to_string(),
            identifier: self.identifier(),
            server_location: self.server_location.clone(),
            system_connection: self.system_connection.clone(),
            default_scene: Some(self.current_scene.clone()),
            all_scenes: self.all_scenes.clone(),
            status_map: self.status_handler.get_map(),
            event_set,
        };
        
        // Try to parse the configuration
        let config_string = match serde_yaml::to_string(&yaml_config) {
            Ok(config_string) => config_string,
            Err(error) => {
                update!(err &self.update_line => "Unable To Parse Current Configuration: {}", error);
                return
            },
        };
        
        // Try to write the configuration to the file
        match config_file.write_all(config_string.as_bytes()) {
            Ok(_) => (),
            Err(error) => update!(err &self.update_line => "Unable To Write Configuration To File: {}", error),
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
    fn verify_config(update_line: &GeneralUpdate, all_scenes: &FnvHashMap<ItemId, Scene>, status_map: &StatusMap, lookup: &FnvHashMap<ItemId, ItemDescription>, events: &FnvHashMap<ItemId, EventDetail>) {
        
        // Verify each scene in the config
        for (id, scene) in all_scenes.iter() {
            if !Config::verify_scene(update_line, scene, all_scenes, status_map, lookup, events) {
                update!(warn update_line => "Broken Scene Definition: {}", id);
            }
            
            // Verify that the scene is described in the lookup
            if !lookup.contains_key(&id) {
                update!(warn update_line => "Scene Not Described In Lookup: {}", id);
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
    fn verify_scene(update_line: &GeneralUpdate, scene: &Scene, all_scenes: &FnvHashMap<ItemId, Scene>,  status_map: &StatusMap, lookup: &FnvHashMap<ItemId, ItemDescription>, events: &FnvHashMap<ItemId, EventDetail>) -> bool {
        
        // Verify that each event detail in the scene is valid
        let mut test = true;
        for id in scene.iter() {
        
            // Find the matching event detail
            if let Some(event_detail) = events.get(id) {
            
                // Verify the event detail
                if !Config::verify_detail(update_line, event_detail, scene, all_scenes, status_map, lookup, events){
                    update!(warn update_line => "Invalid Event Detail: {}", id);
                    test = false;
                }
            
            // Otherwise warn that the event detail was not found in the events
            }  else {
                update!(warn update_line => "Event Listed In Scene, But Not Found: {}", id);
                test = false;
            }
            
            // Verify that the event is described in the event lookup
            test = test & Config::verify_lookup(update_line, lookup, id);
        }
        test // return the result
    }
        
    /// An internal function to verify a particular event detail in the context
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
    fn verify_detail(update_line: &GeneralUpdate, event_detail: &EventDetail, scene: &Scene, all_scenes: &FnvHashMap<ItemId, Scene>,  status_map: &StatusMap, lookup: &FnvHashMap<ItemId, ItemDescription>, event_list: &FnvHashMap<ItemId, EventDetail>) -> bool {
        
        // Unpack the event detail
        match event_detail {
        
            // If there is a new scene, verify the id is valid
            &NewScene { ref new_scene } => {
                
                // If the desired scene does exist
                if all_scenes.contains_key(new_scene) {
                
                    // Verify that the newscene event exists in the new scene
                    if !event_list.contains_key(new_scene) {
                        update!(warn update_line => "Reset Scene Event Missing From Scene: {}", new_scene);
                        return false;
                    }
                
                // If the desired scene does not exist
                } else {
                
                    // Warn the system and indicate failure
                    update!(warn update_line => "Event Detail Contains Invalid Scene: {}", new_scene);
                    return false;
                }
                
                // If the scene exists, verify the scene is described
                return Config::verify_lookup(update_line, lookup, new_scene);
            },
            
            // If there is a status modification, verify both components of the modification
            &ModifyStatus { ref status_id, ref new_state } => {
                
                // Check that the status_id is valid
                if let Some(detail) = status_map.get(status_id) {
                    
                    // Also verify the new state
                    if !detail.is_allowed(new_state) {
                        update!(warn update_line => "Event Detail Contains Invalid New State: {}", &new_state);
                        return false;
                    }
                
                } else {
                    update!(warn update_line => "Event Detail Contains Invalid Status Id: {}", &status_id);
                    return false;
                }
                
                // If the status exists, verify the status and state are described
                return Config::verify_lookup(update_line, lookup, status_id) & Config::verify_lookup(update_line, lookup, new_state);
            },
            
            // If there are triggered events to load, verify that they exist
            &TriggerEvents { ref events } => {

                // Check that all of them exist
                for event_delay in events {
                    
                    // Verify that the event is listed in the current scene
                    if !scene.contains(&event_delay.id()) {
                        update!(warn update_line => "Event Detail Contains Unlisted Triggered Events: {}", &event_delay.id());
                        return false;
                    }
                    
                    // Return false if any event_id is incorrect
                    if !event_list.contains_key(&event_delay.id()) {
                        update!(warn update_line => "Event Detail Contains Invalid Triggered Events: {}", &event_delay.id());
                        return false;
                    } // Don't need to check lookup as all valid individual events are already checked
                }
            },
            
            // If there is data to save, assume validity
            &SaveData { .. } => (),
            
            // If there is a grouped event, verify the components of the event
            &GroupedEvent { ref status_id, ref event_map } => {
                
                // Check that the status_id is valid
                if let Some(detail) = status_map.get(status_id) {
                    
                    // Verify that the allowed states vector isn't empty
                    let allowed = detail.allowed();
                    if allowed.is_empty() {
                        update!(warn update_line => "Event Detail Contains Empty Grouped Event: {}", status_id);
                        return false;
                    }
                    
                    // Verify each state is represented in the event_map
                    for state in allowed.iter() {
                        
                        // If there is a matching event, verify that it exists
                        if let Some(target_event) = event_map.get(state) {
                        
                            // Verify that the event is listed in the current scene
                            if !scene.contains(&target_event) {
                                update!(warn update_line => "Event Group Contains Unlisted Triggered Events: {}", &target_event);
                                return false;
                            }
                            
                            // Verify that the event exists
                            if !event_list.contains_key(&target_event) {
                                update!(warn update_line => "Event Group Contains Invalid Target Event: {}", &target_event);
                                return false;
                            }
                        
                        // If there isn't a match event, raise a warning
                        } else {
                            update!(warn update_line => "Event Group Does Not Specify Target Event: {}", state);
                            return false;
                        }   
                    }
                    
                // If the status doesn't exist, raise a warning
                } else {
                    update!(warn update_line => "Event Detail Contains Invalid Status: {}", status_id);
                    return false;
                }
            },
        }
        true // If no errors were thrown
    }
    
    /// An internal function to verify that a particular id is in the lookup.
    ///
    /// Like all EventHandler functions and methods, this method will fail
    /// gracefully by notifying of warnings on the status line. This function
    /// does not raise any errors.
    ///
    fn verify_lookup(update_line: &GeneralUpdate, lookup: &FnvHashMap<ItemId, ItemDescription>, id: &ItemId) -> bool {
    
        // Check to see if the id is available in the lookup
        if !lookup.contains_key(&id) {
            update!(warn update_line => "Item Not Described In Lookup: {}", id);
            return false;
        }
        true // Otherwise indicate success
    }
}


// Tests of the scene module
#[cfg(test)]
mod tests {
    use super::*;
    
    // Write an example configuration
    #[cfg(feature = "example_configs")]
    #[test]
    fn write_example_config() {
    
        // Import features for testing
        use std::time::Duration;
        use super::super::item::DisplayWith;
        use super::super::event::EventDelay;
        
        // Write the example grouped event
        let mut one_grouped_map = FnvHashMap::default();
        one_grouped_map.insert(ItemId::new(31).unwrap(), ItemId::new(61).unwrap());
        let one_grouped_event = GroupedEvent { group_id: ItemId::new(43).unwrap(), status_map: one_grouped_map, };
        
        // Write the example triggered event
        let trigger_event = TriggerEvents { events: vec!( EventDelay::new(Some(Duration::from_millis(20)), ItemId::new(20).unwrap()), EventDelay::new(None, ItemId::new(41).unwrap())) };
        
        // Write the example scene
        let mut one_scene = scene::default();
        one_scene.insert(ItemId::new(12).unwrap(), SaveData { data: vec!(12), });
        one_scene.insert(ItemId::new(20).unwrap(), one_grouped_event);
        one_scene.insert(ItemId::new(22).unwrap(), trigger_event);
        let mut all_scenes = FnvHashMap::default();
        all_scenes.insert(ItemId::new(24).unwrap(), one_scene);
        
        // Write the example status map
        let mut status_map = StatusMap::default();
        status_map.insert(ItemId::new(51).unwrap(), MultiStatus { current: ItemId::new(13).unwrap(), allowed: vec!(ItemId::new(34).unwrap(), ItemId::new(53).unwrap()) });
        
        // Write the example config
        let yaml_config = YamlConfig { all_scenes, lookup: vec!(ItemPair::from_item(ItemId::new(12).unwrap(), ItemDescription { description: "Test Description".to_string(), display: DisplayWith(ItemId::new(53).unwrap()) }), ItemPair::from_item(ItemId::new(16).unwrap(), ItemDescription { description: "Test 2 Description".to_string(), display: Hidden })), status_map, };
        
        // Try to parse the configuration
        let config_string = match serde_yaml::to_string(&yaml_config) {
            Ok(config_string) => config_string,
            Err(_) => {
                return;
            },
        };
        
        // Attempt to open the configuration file
        let mut config_file = match File::create("examples/example_yaml.mnv") {
            Ok(file) => file,
            Err(_) => {
                return;
            },
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
            None => panic!("Unable to load configuration file: {}", rx.try_recv().unwrap()),
        }
    }

    // Test config verification
    #[test]
    fn verify_config() {
        
        // Import features for testing
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
            None => panic!("Unable to load configuration file: {}", rx.try_recv().unwrap()),
        }
        
        // Create the test vector
        let test = vec!(Warning("Event Detail Contains Invalid Status Id: |44|".to_string()),
                        Warning("Invalid Event Detail: |8|".to_string()),
                        Warning("Broken Scene Definition: |200|".to_string()),
                        Warning("Event Detail Contains Invalid Triggered Events: |8|".to_string()),
                        Warning("Invalid Event Detail: |2|".to_string()),
                        Warning("Broken Scene Definition: |100|".to_string()));
        
        // Print and check the messages received (wait at most half a second)
        test_vec!(~rx, test);
    }
}

