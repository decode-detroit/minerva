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

//! A module to load the configuration from a file and maintain the state
//! machine. This module handles any changes to the current state of the
//! program.

// Import crate definitions
use crate::definitions::*;

// Define private submodules
mod status;

// Import the relevant structures into the correct namespace
use self::status::StatusHandler;

// Import standard library features
use std::path::Path;

// Import tokio features
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

// Import tracing features
use tracing::{error, info, warn};

// Import anyhow features
use anyhow::Result;

// Import FNV HashMap
use fnv::FnvHashMap;

// Import YAML processing library
use serde_yaml;

/// A simple structure to hold and manage the background process
///
struct BackgroundThread {
    background_process: BackgroundProcess, // a copy of the background process info
}

// Implement the BackgroundThread Functions
impl BackgroundThread {
    /// Spawn the monitoring thread
    async fn new(background_process: BackgroundProcess) -> Option<BackgroundThread> {
        // Check to see if the file is valid
        if let Ok(path) = background_process.process.canonicalize() {
            // Notify that the background process is starting
            info!("Starting background process ...");

            // Create the child process
            let mut child = match Command::new(path.clone())
                .args(background_process.arguments.clone())
                .kill_on_drop(true)
                .spawn()
            {
                // If the child process was created, return it
                Ok(child) => child,

                // Otherwise, warn of the error and return
                _ => {
                    error!("Unable to start background process.");
                    return None;
                }
            };

            // Extract the arguments
            let arguments = background_process.arguments.clone();
            let keepalive = background_process.keepalive;

            // Spawn a background thread to monitor the process
            tokio::spawn(async move {
                // Run indefinitely or until the process fails
                loop {
                    // Wait for the process to finish
                    match child.wait().await {
                        // If the process has terminated
                        Ok(status) => {
                            // Notify that the process was a success and restart
                            if status.success() {
                                info!("Background process finiished normally.");

                            // Otherwise, notify of a failed process
                            } else {
                                error!("Background process finished abnormally.");
                            }
                        }

                        // If the process failed to run
                        _ => {
                            error!("Unable to run background process.");
                            break;
                        }
                    }

                    // If the process has finished, and we want to keep it alive
                    if keepalive {
                        // Notify that the background process is restarting
                        info!("Restarting background process ...");

                        // Start the process again
                        child = match Command::new(path.clone())
                            .args(arguments.clone())
                            .kill_on_drop(true)
                            .spawn()
                        {
                            // If the child process was created, return it
                            Ok(child) => child,

                            // Otherwise, warn of the error and end the thread
                            _ => {
                                error!("Unable to run background process.");
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
            Some(BackgroundThread { background_process })

        // Warn that the process wasn't found
        } else {
            error!("Unable to find background process.");
            None
        }
    }

    /// A helper method to return a copy of the background process info
    fn background_process(&self) -> BackgroundProcess {
        self.background_process.clone()
    }
}

/// The configuration struct that is designed to allow simple
/// serialization and deserialization for the program configuration file.
/// This structure is saved to the external configuration file.
///
#[derive(Serialize, Deserialize)]
struct YamlConfig {
    version: String,        // a version tag to warn the user of incompatible versions
    identifier: Identifier, // unique identifier for the controller instance, if specified
    server_location: Option<String>, // the location of the backup server, if specified
    dmx_controllers: DmxControllers, // the details of the dmx controller(s)
    media_players: Vec<MediaPlayer>, // the details of the media player(s)
    system_connections: ConnectionSet, // the type of connection(s) to the underlying system
    background_process: Option<BackgroundProcess>, // an option background process to run
    default_scene: ItemId,  // the starting scene for the configuration
    group_map: FnvHashMap<ItemId, Group>, // hash map of all availble groups
    scene_map: FnvHashMap<ItemId, Scene>, // hash map of all availble scenes
    status_map: StatusMap,  // hash map of the default game status
    event_set: FnvHashMap<ItemPair, Option<Event>>, // hash map of all the item pairs and events
    user_styles: StyleMap, // A string representing arbitrary css for styling the user and edit interfaces
}

/// A structure to hold the whole configuration for current instantiation of the
/// program. As part of this configuration, this structure holds the description
/// lookup for all event, group, and scene ids. This structure also holds the
/// current active and modifyable scene of the program.
///
pub struct Config {
    identifier: Identifier, // unique identifier for the controller instance
    system_connections: ConnectionSet, // the type of connection(s) to the underlying system
    dmx_controllers: DmxControllers, // the details of the dmx controller(s)
    media_players: Vec<MediaPlayer>, // the details of the media player(s)
    server_location: Option<String>, // the location of the backup server, if specified
    background_thread: Option<BackgroundThread>, // a copy of the background process info
    default_scene: ItemId,  // the starting scene for the configuration
    current_scene: ItemId,  // identifier for the current scene
    group_map: FnvHashMap<ItemId, Group>, // hash map of all availble groups
    scene_map: FnvHashMap<ItemId, Scene>, // hash map of all availble scenes
    status_handler: StatusHandler, // status handler for the current game status
    event_set: FnvHashMap<ItemId, Event>, // hash map of all the events
    index_access: IndexAccess, // access point to the item index
    style_access: StyleAccess, // access point to the style sheet
    interface_send: InterfaceSend, // sending line for updates to the user interface
    limited_send: LimitedSend, // sending line for limited updates
}

// Implement key features for the configuration
impl Config {
    /// A function to create a new empty config with no settings.
    ///
    pub async fn new(
        index_access: IndexAccess,
        style_access: StyleAccess,
        interface_send: InterfaceSend,
        limited_send: LimitedSend,
    ) -> Config {
        // Create the new status handler
        let status_handler = StatusHandler::new(FnvHashMap::default());

        // Clear the item index
        index_access.send_index(DescriptionMap::default()).await;

        // Clear the user styles
        style_access.send_styles(StyleMap::default()).await;

        // Return a new empty configuration
        Config {
            identifier: Identifier { id: None },
            system_connections: ConnectionSet::new(),
            dmx_controllers: DmxControllers::default(),
            media_players: Vec::new(),
            server_location: None,
            background_thread: None,
            default_scene: ItemId::all_stop(),
            current_scene: ItemId::all_stop(),
            group_map: FnvHashMap::default(),
            scene_map: FnvHashMap::default(),
            status_handler,
            event_set: FnvHashMap::default(),
            index_access,
            style_access,
            interface_send,
            limited_send,
        }
    }

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
        index_access: IndexAccess,
        style_access: StyleAccess,
        interface_send: InterfaceSend,
        limited_send: LimitedSend,
        mut config_file: File,
    ) -> Result<Config> {
        // Try to read from the configuration file
        let mut config_string = String::new();
        match config_file.read_to_string(&mut config_string).await {
            Ok(_) => (),
            Err(error) => {
                error!("Invalid configuration file: {}.", error);
                return Err(anyhow!("Invalid configuration file: {}", error));
            }
        }

        // Try to parse the configuration file
        let yaml_config: YamlConfig = match serde_yaml::from_str(config_string.as_str()) {
            Ok(config) => config,
            Err(error) => {
                error!("Unable to parse configuration file: {}.", error);
                return Err(anyhow!("Unable to parse configuration file: {}", error));
            }
        };

        // Check the version id and warn the user if they differ
        let version = env!("CARGO_PKG_VERSION");
        if &yaml_config.version != version {
            warn!(
                "Version of configuration ({}) does not match software version ({}).",
                &yaml_config.version, version
            );
        }

        // Turn the ItemPairs in to the item index and event set
        let mut item_index = DescriptionMap::default();
        let mut event_set = FnvHashMap::default();
        for (item_pair, possible_event) in yaml_config.event_set.iter() {
            // Insert the event description into the lookup
            match item_index.insert(item_pair.get_id(), item_pair.get_description()) {
                // Warn of events defined multiple times
                Some(_) => {
                    warn!(
                        "Item {} has multiple definitions in lookup.",
                        &item_pair.id()
                    ) // FIXME This check doesn't work due to the deserialization process
                }
                None => (),
            }

            // If the event is specified
            if let &Some(ref event) = possible_event {
                // Insert the event into the events hash map
                match event_set.insert(item_pair.get_id(), event.clone()) {
                    // Warn of an event defined multiple times
                    Some(_) => {
                        warn!(
                            "Item {} has multiple definitions in event list.",
                            &item_pair.id()
                        )
                    }
                    None => (),
                }
            }
        }

        // Verify the configuration is defined correctly
        let scene_map = yaml_config.scene_map;
        let group_map = yaml_config.group_map;
        let status_map = yaml_config.status_map;
        Config::verify_config(&scene_map, &group_map, &status_map, &item_index, &event_set).await; // FIXME check groups as well

        // Load the item index
        index_access.send_index(item_index).await;

        // Load the user styles
        style_access.send_styles(yaml_config.user_styles).await;

        // Create the new status handler
        let status_handler = StatusHandler::new(status_map);

        // Load the default scene
        let current_scene = yaml_config.default_scene;

        // Check to see if the default scene is valid and warn if not defined
        if scene_map.get(&current_scene).is_none() {
            warn!("Current scene is not defined.");
        }

        // Try to start the background process and monitor it, if specified
        let mut background_thread = None;
        if let Some(background_process) = yaml_config.background_process {
            background_thread = BackgroundThread::new(background_process).await;
        }

        // Return the new configuration
        Ok(Config {
            identifier: yaml_config.identifier,
            system_connections: yaml_config.system_connections,
            dmx_controllers: yaml_config.dmx_controllers,
            server_location: yaml_config.server_location,
            media_players: yaml_config.media_players,
            background_thread,
            default_scene: yaml_config.default_scene,
            current_scene,
            group_map,
            scene_map,
            status_handler,
            event_set,
            index_access,
            style_access,
            interface_send,
            limited_send,
        })
    }

    /// A method to return a copy of the background process
    ///
    pub fn get_background_process(&self) -> Option<BackgroundProcess> {
        // Get a copy of the background process, if it exists
        match &self.background_thread {
            &Some(ref bt) => Some(bt.background_process()),
            &None => None,
        }
    }

    /// A method to return a copy of the system connections
    ///
    pub fn get_connections(&self) -> ConnectionSet {
        self.system_connections.clone()
    }

    /// A method to return the default scene
    ///
    pub fn get_default_scene(&self) -> ItemId {
        self.default_scene
    }

    /// A method to return a copy of the dmx contollgers
    ///
    pub fn get_dmx_controllers(&self) -> DmxControllers {
        self.dmx_controllers.clone()
    }

    /// A method to return the identifier
    ///
    pub fn get_identifier(&self) -> Identifier {
        self.identifier
    }

    /// A method to return a group, given an ItemId. If the id corresponds to a valid group,
    /// the method returns the group. Otherwise, it returns None.
    ///
    pub fn get_group(&self, item_id: &ItemId) -> Option<Group> {
        // Return the scene, if found, and return a copy
        self.group_map.get(item_id).map(|group| group.clone())
    }

    /// A method to return a list of all available scenes in this
    /// configuration. This method will always return the scenes from lowest to
    /// highest id.
    ///
    pub fn get_groups(&self) -> Vec<ItemId> {
        // Compile a list of the available groups
        let mut groups = Vec::new();
        for group_id in self.group_map.keys() {
            groups.push(group_id.clone());
        }

        // Sort them in order
        groups.sort_unstable();

        // Return the result
        groups
    }

    /// A method to return a copy of the media player details
    ///
    pub fn get_media_players(&self) -> Vec<MediaPlayer> {
        self.media_players.clone()
    }

    /// A method to return the backup server location
    ///
    pub fn get_server_location(&self) -> Option<String> {
        self.server_location.clone()
    }

    /// A method to return a status from the status handler.
    ///
    pub fn get_status(&self, item_id: &ItemId) -> Option<Status> {
        // Return a status based on the provided item id
        self.status_handler.get_status(item_id)
    }

    /// A method to return a vector of the valid status ids.
    ///
    pub fn get_status_ids(&self) -> Vec<ItemId> {
        self.status_handler.get_ids()
    }

    /// A method to silently update the status of the system based on a previous
    /// backup.
    ///
    pub async fn load_backup_status(&mut self, mut status_pairs: Vec<(ItemId, ItemId)>) {
        // For every status in the status pairs, set the current value
        for (status_id, new_state) in status_pairs.drain(..) {
            self.status_handler
                .modify_status(&status_id, &new_state)
                .await;

            // Get the item pairs
            let status_pair = self.index_access.get_pair(&status_id).await;
            let state_pair = self.index_access.get_pair(&new_state).await;

            // Send the change to the interface
            self.interface_send
                .send(InterfaceUpdate::UpdateStatus {
                    status_id: status_pair.clone(),
                    new_state: state_pair.clone(),
                })
                .await;

            // Send the change to the limited interface
            self.limited_send
                .send(LimitedUpdate::UpdateStatus {
                    status_id: status_id.clone(),
                    new_state: new_state.clone(),
                })
                .await;

            // Notify the user of the change
            info!("Changing {} to {}.", status_pair, state_pair);
        }
    }

    /// A method to return a hashmap of the statuses available in this
    /// configuration.
    ///
    pub fn get_statuses(&self) -> PartialStatus {
        // Get the statuses from the status handler
        self.status_handler.get_partial_status()
    }

    /// A method to return a scene, given an ItemId. If the id corresponds to a valid scene,
    /// the method returns the scene. Otherwise, it returns None.
    ///
    pub fn get_scene(&self, item_id: &ItemId) -> Option<Scene> {
        // Return the scene, if found, and return a copy
        self.scene_map.get(item_id).map(|scene| scene.clone())
    }

    /// A method to return a list of all available scenes in this
    /// configuration. This method will always return the scenes from lowest to
    /// highest id.
    ///
    pub fn get_scenes(&self) -> Vec<ItemId> {
        // Compile a list of the available scenes
        let mut scenes = Vec::new();
        for scene_id in self.scene_map.keys() {
            scenes.push(scene_id.clone());
        }

        // Sort them in order
        scenes.sort_unstable();

        // Return the result
        scenes
    }

    /// A method to return a list of all items in the current scene.
    /// This method will always return the items from lowest to highest id.
    ///
    /// # Note
    ///
    /// For grouped items this method only returnes the group id.
    /// Group items much be queried separately.
    ///
    pub fn get_current_items(&self) -> Vec<ItemId> {
        // Create an empty item vector
        let mut items = Vec::new();

        // Try to open the current scene
        if let Some(scene) = self.scene_map.get(&self.current_scene) {
            // Compile the list of the available items
            for item_id in scene.items.iter() {
                items.push(item_id.clone());
            }

            // Add the list of group ids
            items.extend(&scene.groups);

            // Sort them in order
            items.sort_unstable();
        }

        // Return the result
        items
    }

    /// A method to return an key map for the current scene, with all items
    /// as an item id.
    ///
    /// # Note
    /// If there is no current scene, this method returns None.
    ///
    pub async fn get_key_map(&self) -> Option<KeyMap> {
        // Try to open the current scene
        if let Some(scene) = self.scene_map.get(&self.current_scene) {
            // Return a copy of the key map
            scene.key_map.clone()

        // Otherwise, return an empty map
        } else {
            None
        }
    }

    /// A method to return an item id of the current state of the provided
    /// item id, or None if it was not found.
    ///
    pub async fn get_state(&self, status_id: &ItemId) -> Option<ItemId> {
        // Return the internal state of the status handler
        self.status_handler.get_state(status_id).await
    }

    /// A method to return the current scene.
    ///
    pub fn get_current_scene(&self) -> ItemId {
        self.current_scene.clone()
    }

    /// A method to save new parameters to the configuration
    ///
    pub async fn save_parameters(&mut self, parameters: ConfigParameters) {
        // Update the fields of the current configuration
        self.identifier = parameters.identifier;
        self.server_location = parameters.server_location;
        self.dmx_controllers = parameters.dmx_controllers;
        self.media_players = parameters.media_players;
        self.system_connections = parameters.system_connections;
        self.default_scene = parameters.default_scene;
    }

    /// A method to select a scene map from existing configuration based on the
    /// provided scene id.
    ///
    pub async fn choose_scene(&mut self, scene_id: ItemId) -> Result<(), ()> {
        // Check to see if the scene_id is valid
        if self.scene_map.contains_key(&scene_id) {
            // Update the current scene id
            self.current_scene = scene_id;

            // Send the scene change to the user interface
            self.interface_send
                .send(InterfaceUpdate::UpdateScene {
                    current_scene: scene_id,
                })
                .await;

            // Send the scene change to the limited interface
            self.limited_send
                .send(LimitedUpdate::UpdateScene {
                    current_scene: scene_id,
                })
                .await;

            // Indicate success
            return Ok(());

        // Warn the system that the selected id doesn't exist
        } else {
            // Warn of the error and indicate failure
            warn!("Scene Id not found in configuration: {}.", scene_id);
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
    /// This method will raise an error if the provided id was not found in
    /// the configuration. This usually indicates a problem with the underlying
    /// configuration file.
    ///
    pub async fn modify_status(
        &mut self,
        status_id: &ItemId,
        new_state: &ItemId,
    ) -> Option<ItemId> {
        // Try to update the underlying status
        if let Some(new_id) = self
            .status_handler
            .modify_status(&status_id, &new_state)
            .await
        {
            // Get the item pairs
            let status_pair = self.index_access.get_pair(&status_id).await;
            let state_pair = self.index_access.get_pair(&new_state).await;

            // Send the change to the interface
            self.interface_send
                .send(InterfaceUpdate::UpdateStatus {
                    status_id: status_pair.clone(),
                    new_state: state_pair.clone(),
                })
                .await;

            // Send the change to the limited interface
            self.limited_send
                .send(LimitedUpdate::UpdateStatus {
                    status_id: status_id.clone(),
                    new_state: new_state.clone(),
                })
                .await;

            // Notify the user of the change
            info!("Changing {} to {}.", status_pair, state_pair);

            // Indicate status change
            return Some(new_id);
        }

        // Indicate no change
        None
    }

    /// A method to modify or add an event with provided event id and new event.
    ///
    pub async fn edit_event(&mut self, event_id: ItemId, possible_event: Option<Event>) {
        // If a new event was specified
        if let Some(new_event) = possible_event {
            // Verify that the event is not a status
            if self.status_handler.get_status(&event_id).is_some() {
                error!(
                    "Cannot add event. Item is already a status: {}.",
                    self.index_access.get_description(&event_id).await
                );
            }

            // Verify that the event is not a group
            if self.group_map.contains_key(&event_id) {
                error!(
                    "Cannot add event. Item is already a group: {}.",
                    self.index_access.get_description(&event_id).await
                );
            }

            // If the event is in the event list, update the event
            if let Some(event) = self.event_set.get_mut(&event_id) {
                // Update the event and notify the system
                *event = new_event;
                info!(
                    "Event updated: {}.",
                    self.index_access.get_description(&event_id).await
                );

            // Otherwise, add the event
            } else {
                info!(
                    "Event added: {}.",
                    self.index_access.get_description(&event_id).await
                );
                self.event_set.insert(event_id, new_event);
            }

        // If no new event was specified
        } else {
            // If the event is in the event list, remove it
            if self.event_set.remove(&event_id).is_some() {
                // Notify the user that it was removed
                info!(
                    "Event removed: {}.",
                    self.index_access.get_description(&event_id).await
                );

                // If the event is also a scene, remove it
                if self.scene_map.remove(&event_id).is_some() {
                    // Notify the user that it was removed
                    info!(
                        "Matching scene removed: {}.",
                        self.index_access.get_description(&event_id).await
                    );
                }
            }
        }
    }

    /// A method to modify or add a group with provided id.
    ///
    pub async fn edit_group(&mut self, group_id: ItemId, possible_group: Option<Group>) {
        // If a new group was specified
        if let Some(new_group) = possible_group {
            // Verify that the group is not an event
            if self.event_set.contains_key(&group_id) {
                error!(
                    "Cannot add group. Item is already an event: {}.",
                    self.index_access.get_description(&group_id).await
                );
            }

            // Verify that the group is not a status
            if self.status_handler.get_status(&group_id).is_some() {
                error!(
                    "Cannot add group. Item is already a status: {}.",
                    self.index_access.get_description(&group_id).await
                );
            }

            // Verify that the group is not a scene
            if self.scene_map.contains_key(&group_id) {
                error!(
                    "Cannot add group. Item is already a scene: {}.",
                    self.index_access.get_description(&group_id).await
                );
            }

            // If the group is in the group list, update the group
            if let Some(group) = self.group_map.get_mut(&group_id) {
                // Update the group and notify the system
                *group = new_group;
                info!(
                    "Group updated: {}.",
                    self.index_access.get_description(&group_id).await
                );

            // Otherwise, add the group
            } else {
                info!(
                    "Group added: {}.",
                    self.index_access.get_description(&group_id).await
                );
                self.group_map.insert(group_id, new_group);

                // If the group is in a scene, move it to the correct list
                for scene in self.scene_map.values_mut() {
                    // If the group is in the item list
                    if scene.items.remove(&group_id) {
                        // Add it to the group list instead
                        scene.groups.insert(group_id);
                    }
                }
            }

        // If no new group was specified
        } else {
            // If the group is in the group list, remove it
            if self.group_map.remove(&group_id).is_some() {
                // Notify the user that it was removed
                info!(
                    "Group removed: {}.",
                    self.index_access.get_description(&group_id).await
                );

                // If the group is in a scene, move it to the correct list
                for scene in self.scene_map.values_mut() {
                    // If the group is in the group list
                    if scene.groups.remove(&group_id) {
                        // Add it to the item list instead
                        scene.items.insert(group_id);
                    }
                }
            }
        }
    }

    /// A method to modify or add a status with provided id.
    ///
    pub async fn edit_status(&mut self, status_id: ItemId, new_status: Option<Status>) {
        // If adding or modifying a status
        if new_status.is_some() {
            // Verify that the status is not an event
            if self.event_set.contains_key(&status_id) {
                error!(
                    "Cannot add status. Item is already an event: {}.",
                    self.index_access.get_description(&status_id).await
                );
            }

            // Verify that the status is not a scene
            if self.scene_map.contains_key(&status_id) {
                error!(
                    "Cannot add status. Item is already a scene: {}.",
                    self.index_access.get_description(&status_id).await
                );
            }

            // Verify that the status is not a group
            if self.group_map.contains_key(&status_id) {
                error!(
                    "Cannot add status. Item is already a group: {}.",
                    self.index_access.get_description(&status_id).await
                );
            }
        }

        // Get the item description and then pass the change to the status handler
        let description = self
            .index_access
            .get_description(&status_id)
            .await
            .description;
        self.status_handler
            .edit_status(status_id, new_status, description)
            .await;
    }

    /// A method to modify or add a scene with provided id.
    ///
    pub async fn edit_scene(&mut self, scene_id: ItemId, possible_scene: Option<Scene>) {
        // If a new scene was specified
        if let Some(new_scene) = possible_scene {
            // Verify that the scene is not a status
            if self.status_handler.get_status(&scene_id).is_some() {
                error!(
                    "Cannot add scene. Item is already a status: {}.",
                    self.index_access.get_description(&scene_id).await
                );
            }

            // Verify that the scene is not a group
            if self.group_map.contains_key(&scene_id) {
                error!(
                    "Cannot add scene. Item is already a group: {}.",
                    self.index_access.get_description(&scene_id).await
                );
            }

            // If the scene is in the scene list, update the scene
            if let Some(scene) = self.scene_map.get_mut(&scene_id) {
                // Update the scene and notify the system
                *scene = new_scene;
                info!(
                    "Scene updated: {}.",
                    self.index_access.get_description(&scene_id).await
                );

                // Make sure the scene is also an event
                if !self.event_set.contains_key(&scene_id) {
                    self.event_set.insert(scene_id, Event::new());
                }

            // Otherwise, add the scene
            } else {
                info!(
                    "Scene added: {}.",
                    self.index_access.get_description(&scene_id).await
                );
                self.scene_map.insert(scene_id, new_scene);

                // Make sure the scene is also an event
                if !self.event_set.contains_key(&scene_id) {
                    self.event_set.insert(scene_id, Event::new());
                }
            }

        // If no new scene was specified
        } else {
            // If the scene is in the scene list, remove it
            if self.scene_map.remove(&scene_id).is_some() {
                // Notify the user that it was removed
                info!(
                    "Scene removed: {}.",
                    self.index_access.get_description(&scene_id).await
                );

                // Also remove the matching event (if it exists)
                self.event_set.remove(&scene_id);
            }
        }
    }

    /// A method to remove all references to an item from the current configuration.
    ///
    pub async fn remove_item(&mut self, item_id: ItemId) {
        // Look through each scene and remove the item if it exists
        for scene in self.scene_map.values_mut() {
            // Remove the item if it exists
            scene.items.remove(&item_id);
        }

        // Look through each group and remove the item if it exists
        for group in self.group_map.values_mut() {
            // Remove the item if it exists
            group.items.remove(&item_id);
        }

        // Look through each status and check if the item exists
        let statuses = self.status_handler.get_map();
        for (status_id, status) in statuses.iter() {
            // If the item is one of the allowed states
            if status.is_allowed(&item_id) {
                // Warn the user that the status is broken
                warn!(
                    "Item appears in status {}. Status has a broken definition.",
                    self.index_access.get_description(&status_id).await
                );
            }
        }

        // Look through each event and remove the item if it exists
        for (event_id, event) in self.event_set.iter() {
            // Look through each action
            let mut is_broken = false;
            for action in event {
                // Match the action type
                match action {
                    // If the item id appears in the action, mark the event as broken
                    CancelEvent { event } => {
                        if event == &item_id {
                            is_broken = true;
                            break;
                        }
                    }

                    CueEvent { event } => {
                        if event.id() == item_id {
                            is_broken = true;
                            break;
                        }
                    }

                    ModifyStatus {
                        status_id,
                        new_state,
                    } => {
                        if status_id == &item_id || new_state == &item_id {
                            is_broken = true;
                            break;
                        }
                    }

                    NewScene { new_scene } => {
                        if new_scene == &item_id {
                            is_broken = true;
                            break;
                        }
                    }

                    SelectEvent {
                        status_id,
                        event_map,
                    } => {
                        // Check the status id
                        if status_id == &item_id {
                            is_broken = true;
                            break;
                        }

                        // Check all events in the event map
                        for (state, select_event) in event_map.iter() {
                            if state == &item_id || select_event == &item_id {
                                is_broken = true;
                                break;
                            }
                        }
                    }

                    // Ignore other action types
                    _ => (),
                }
            }

            // If the event is broken, warn the user
            if is_broken {
                warn!(
                    "Item appears in event {}. Event has a broken definition.",
                    self.index_access.get_description(&event_id).await
                );
            }
        }
    }

    /// A method to return the event based on the event id.
    ///
    /// # Errors
    ///
    /// This function will return None if the provided id was not found in
    /// the configuration. This usually indicates that the provided id was incorrect
    /// or a problem with the underlying configuration file.
    ///
    pub fn get_event(&mut self, id: &ItemId) -> Option<Event> {
        // Try to return a copy of the event
        self.event_set.get(id).map(|event| event.clone())
    }

    /// A method to return the event based on the event id.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the provided id was not found in
    /// the configuration. This usually indicates a problem with the underlying
    /// configuration file.
    ///
    pub async fn try_event(&mut self, id: &ItemId, checkscene: bool) -> Option<Event> {
        // If the checkscene flag is set and the event is not the current scene
        if checkscene && id != &self.current_scene {
            // Check if the event is the current scene, or try to open the current scene
            if let Some(scene) = self.scene_map.get(&self.current_scene) {
                // Check to see if the event is listed in the current scene
                if !scene.items.contains(id) {
                    // If not, check all the groups in the scene
                    let mut is_found = false;
                    for group_id in scene.groups.iter() {
                        if let Some(group) = self.group_map.get(&group_id) {
                            if group.items.contains(id) {
                                is_found = true;
                                break;
                            }
                        }
                    }

                    // If not found
                    if !(is_found) {
                        // If the event is not listed in the current scene or scene groups, notify
                        warn!("Event not in current scene: {}.", id);
                        return None;
                    }
                }
            // Warn that there isn't a current scene
            } else {
                error!("Current scene not found.");
                return None;
            }
        }

        // Try to return the event
        match self.get_event(id) {
            // Return the found event
            Some(event) => Some(event),

            // Return None if the id doesn't exist
            None => {
                // Notify of an invalid event
                error!("Event not found: {}", id);

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
    pub async fn to_config(&self, mut config_file: File) {
        // Assemble the event set from the item index and events
        let mut item_index = self.index_access.get_all_pairs().await;
        let mut event_set = FnvHashMap::default();
        // Look through the item index
        for item_pair in item_index.drain(..) {
            // Add the event if found
            match self.event_set.get(&item_pair.get_id()) {
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

        // Try to get a copy of the style sheet
        let user_styles = self.style_access.get_all_rules().await;

        // Create a YAML config from the elements
        let yaml_config = YamlConfig {
            version: env!("CARGO_PKG_VERSION").into(),
            identifier: self.get_identifier(),
            server_location: self.server_location.clone(),
            system_connections: self.get_connections(),
            dmx_controllers: self.dmx_controllers.clone(),
            media_players: self.media_players.clone(),
            background_process: self.get_background_process(),
            default_scene: self.default_scene,
            group_map: self.group_map.clone(),
            scene_map: self.scene_map.clone(),
            status_map: self.status_handler.get_map(),
            event_set,
            user_styles,
        };

        // Try to parse the configuration
        let config_string = match serde_yaml::to_string(&yaml_config) {
            Ok(config_string) => config_string,
            Err(error) => {
                error!("Unable to parse current configuration: {}.", error);
                return;
            }
        };

        // Try to write the configuration to the file
        match config_file.write_all(config_string.as_bytes()).await {
            Ok(_) => (),
            Err(error) => {
                error!("Unable to write configuration file: {}.", error)
            }
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
        scene_map: &FnvHashMap<ItemId, Scene>,
        group_map: &FnvHashMap<ItemId, Group>,
        status_map: &StatusMap,
        lookup: &FnvHashMap<ItemId, ItemDescription>,
        events: &FnvHashMap<ItemId, Event>,
    ) {
        // Verify each scene in the config
        for (id, scene) in scene_map.iter() {
            if !Config::verify_scene(scene, scene_map, group_map, status_map, lookup, events).await
            {
                warn!("Broken scene definition: {}.", id);
            }

            // Verify that the scene is described in the lookup
            if !lookup.contains_key(&id) {
                warn!("Scene not described in lookup: {}.", id);
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
        scene: &Scene,
        scene_map: &FnvHashMap<ItemId, Scene>,
        group_map: &FnvHashMap<ItemId, Group>,
        status_map: &StatusMap,
        lookup: &FnvHashMap<ItemId, ItemDescription>,
        events: &FnvHashMap<ItemId, Event>,
    ) -> bool {
        // Verify that each item in the scene is valid
        let mut test = true;
        for id in scene.items.iter() {
            // Find the matching event
            if let Some(event) = events.get(id) {
                // Verify the event
                if !Config::verify_event(
                    event, scene, scene_map, group_map, status_map, lookup, events,
                )
                .await
                {
                    warn!("Invalid event: {}.", id);
                    test = false;
                }

            // Otherwise verify that the item id corresponds to a group
            } else if group_map.get(id).is_some() {
                // Verify the group
                /*if !Config::verify_group(event, scene, scene_map, status_map, lookup, events).await
                {
                    warn!("Invalid event: {}.", id);
                    test = false;
                }*/
                warn!("Group verification not implemented.");

            // Otherwise verify that the item id corresponds to a status
            } else if status_map.get(id).is_none() {
                // Warn that an invalid event or status was listed in the scene
                warn!("Item listed in scene but not found: {}.", id);
                test = false;
            }

            // Verify that the event is described in the event lookup
            test = test & Config::verify_lookup(lookup, id).await;
        }

        // If the key map is specified
        if let Some(key_map) = &scene.key_map {
            // Verify that each key mapping matches a valid event
            for (_, id) in key_map.iter() {
                // Make sure the event is listed in the scene
                if !scene.items.contains(id) {
                    // If not, check all the groups in the scene
                    let mut is_found = false;
                    for group_id in scene.groups.iter() {
                        if let Some(group) = group_map.get(&group_id) {
                            if group.items.contains(id) {
                                is_found = true;
                                break;
                            }
                        }
                    }

                    // If not found
                    if !(is_found) {
                        warn!("Event in keyboard shortcuts, but not in scene: {}", id);
                        test = false;
                    }
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
        event: &Event,
        scene: &Scene,
        scene_map: &FnvHashMap<ItemId, Scene>,
        group_map: &FnvHashMap<ItemId, Group>,
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
                    if scene_map.contains_key(new_scene) {
                        // Verify that the newscene event exists in the new scene
                        if !event_list.contains_key(new_scene) {
                            warn!("Reset scene event missing from scene: {}.", new_scene);
                            return false;
                        }

                    // If the desired scene does not exist
                    } else {
                        // Warn the system and indicate failure
                        warn!("Event contains invalid scene: {}.", new_scene);
                        return false;
                    }

                    // If the scene exists, verify the scene is described
                    return Config::verify_lookup(lookup, new_scene).await;
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
                            warn!("Event contains invalid new state: {}.", &new_state);
                            return false;
                        }
                    } else {
                        warn!("Event contains invalid status id: {}.", &status_id);
                        return false;
                    }

                    // If the status exists, verify the status and state are described
                    return Config::verify_lookup(lookup, status_id).await
                        & Config::verify_lookup(lookup, new_state).await;
                }
                // If there is dmx fade to cue, assume validity
                &CueDmx { .. } => (),

                // If there is an event to cue, verify that it exists
                &CueEvent { ref event } => {
                    // Verify that the event is listed in the current scene
                    if !scene.items.contains(&event.id()) {
                        // If not, check all the groups in the scene
                        let mut is_found = false;
                        for group_id in scene.groups.iter() {
                            if let Some(group) = group_map.get(&group_id) {
                                if group.items.contains(&event.id()) {
                                    is_found = true;
                                    break;
                                }
                            }
                        }

                        // If not found
                        if !(is_found) {
                            warn!("Cued event not in scene: {}.", &event.id());
                            // Do not flag as incorrect
                        }
                    }

                    // Return false if the event_id is incorrect
                    if !event_list.contains_key(&event.id()) {
                        warn!("Event contains invalid cue event: {}.", &event.id());
                        return false;
                    } // Don't need to check lookup as all valid individual events are already checked
                }

                // If there is media to cue, check if the file exists
                &CueMedia { ref cue } => {
                    // If the cue is referencing a local file
                    if cue.uri.starts_with("file://") {
                        if !Path::new(&cue.uri[7..]).exists() {
                            warn!("Media file missing for cue media: {}.", cue.uri);
                            return false;
                        }
                    }

                    // If the loop media exists and is referencing a local file
                    if let Some(ref media) = cue.loop_media {
                        if media.starts_with("file://") {
                            if !Path::new(&media[7..]).exists() {
                                warn!("Media file missing for cue media: {}.", &media);
                                return false;
                            }
                        }
                    }
                }

                // If there is media to adjust, assume validity
                &AdjustMedia { .. } => (),

                // If there are events to cancel, verify that they exist
                &CancelEvent { ref event } => {
                    // Return false if the event doesn't exist
                    if !event_list.contains_key(&event) {
                        warn!("Event contains invalid cancelled event: {}.", &event);
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
                            warn!("Event contains empty select event: {}.", status_id);
                            return false;
                        }

                        // Verify each event that corresponds to a state is valid
                        for state in allowed.iter() {
                            // If there is a matching event, verify that it exists
                            if let Some(target_event) = event_map.get(state) {
                                // Verify that the event exists
                                if !event_list.contains_key(&target_event) {
                                    warn!(
                                        "Select event has invalid target event: {}.",
                                        &target_event
                                    );
                                    return false;
                                }
                                // If no event is specified, nothing is triggered
                            }
                        }

                    // If the status doesn't exist, raise a warning
                    } else {
                        warn!("Select event contains invalid status: {}.", status_id);
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
    async fn verify_lookup(lookup: &FnvHashMap<ItemId, ItemDescription>, id: &ItemId) -> bool {
        // Check to see if the id is available in the lookup
        if !lookup.contains_key(&id) {
            warn!("Item not described in lookup: {}.", id);
            return false;
        }
        true // Otherwise indicate success
    }
}

// Tests of the scene module
#[cfg(test)]
mod tests {
    //use super::*;

    // FIXME Define tests of this module
    #[test]
    fn missing_tests() {
        // FIXME: Implement this
        unimplemented!();
    }

    // FIXME Reimplement these out of date tests
    // Test loading a scene from file
    /* #[test]
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
