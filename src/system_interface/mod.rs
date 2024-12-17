// Copyright (c) 2019-20 Decode Detroit
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

//! A module to create and monitor the user interface and the system inputs.
//! This module links directly to the event handler and sends any updates
//! to the application window.

// Define private submodules
#[macro_use]
mod event_handler;
mod system_connection;

// Import crate definitions
use crate::definitions::*;

// Import other definitions
use self::event_handler::EventHandler;
use self::system_connection::SystemConnection;

// Import standard library features
use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;

// Import FNV HashSet
use fnv::FnvHashSet;

// Import Tokio features
use tokio::sync::mpsc;

// Import tracing features
use tracing::{error, info, warn};

/// A structure to contain the system interface and handle all updates to the
/// to the interface.
///
/// # Note
///
/// This structure is still under rapid development and may change operation
/// in the near future.
///
pub struct SystemInterface {
    event_handler: Option<EventHandler>, // the event handler instance for the program, if it exists
    system_connection: SystemConnection, // the system connection instance for the program
    index_access: IndexAccess,           // the access point for the item index
    style_access: StyleAccess,           // the access point for the style sheet
    interface_send: InterfaceSend,       // a sending line to pass interface updates
    limited_send: LimitedSend,           // a sending line to pass limited updates
    web_receive: mpsc::Receiver<WebRequest>, // the receiving line for web requests
    internal_receive: mpsc::Receiver<InternalUpdate>, // a receiving line to receive internal updates
    internal_send: InternalSend,                      // a sending line to pass internal updates
}

// Implement key SystemInterface functionality
impl SystemInterface {
    /// A function to create a new, blank instance of the system interface.
    ///
    pub async fn new(
        index_access: IndexAccess,
        style_access: StyleAccess,
        interface_send: InterfaceSend,
        limited_send: LimitedSend,
        config_file: String,
    ) -> (Self, WebSend) {
        // Create the new general update structure and receive channel
        let (internal_send, internal_receive) = InternalSend::new();

        // Create a new system connection instance
        let system_connection = SystemConnection::new(internal_send.clone(), None).await;

        // Create the web send for the web interface
        let (web_send, web_receive) = WebSend::new();

        // Create the new system interface instance
        let mut sys_interface = Self {
            event_handler: None,
            system_connection,
            index_access,
            style_access,
            interface_send,
            limited_send,
            web_receive,
            internal_receive,
            internal_send,
        };

        // Try to load a default configuration, if it exists
        if let Ok(mut path) = env::current_dir() {
            // Add the default filename FIXME only allows for relative filepaths
            path.push(config_file.as_str());

            // Try the file, if it exists
            if path.exists() {
                sys_interface.load_config(Some(path), false).await;

            // Otherwise, create an empty config
            } else {
                sys_interface.load_config(None, false).await;
            }
        }

        // Regardless, return the new SystemInterface and general send line
        (sys_interface, web_send)
    }

    /// A method to run one iteration of the system interface to update the user
    /// and underlying system of any event changes.
    ///
    async fn run_once(&mut self) -> bool {
        // Check for updates on any line
        tokio::select! {
            // Updates from the Internal System
            Some(update) = self.internal_receive.recv() => {
                self.unpack_internal_update(update).await;
            }

            // Updates from the Web Interface
            Some(request) = self.web_receive.recv() => {
                // Unpack the request
                match self.unpack_request(request.request).await {
                    // The unpacking was a success
                    UnpackResult::Success => {
                        request.reply_to.send(WebReply::success()).unwrap_or(());
                    }

                    // The unpacking yielded a current scene and status
                    UnpackResult::SuccessWithCurrentSceneAndStatus((scene_id, status)) => {
                        request.reply_to.send(WebReply { is_valid: true, data: WebReplyData::CurrentSceneAndStatus((scene_id, status)) } ).unwrap_or(());
                    }

                    // The unpacking yielded an event
                    UnpackResult::SuccessWithEvent(event) => {
                        request.reply_to.send(WebReply { is_valid: true, data: WebReplyData::Event(Some(event.into())) } ).unwrap_or(());
                    }

                    // The unpacking yielded items
                    UnpackResult::SuccessWithItems(items) => {
                        request.reply_to.send(WebReply { is_valid: true, data: WebReplyData::Items(items) }).unwrap_or(());
                    }

                    // The unpacking yielded a group
                    UnpackResult::SuccessWithGroup(group) => {
                        request.reply_to.send(WebReply { is_valid: true, data: WebReplyData::Group(Some(group)) }).unwrap_or(());
                    }

                    // The unpacking yielded a message
                    UnpackResult::SuccessWithMessage(message) => {
                        request.reply_to.send(WebReply { is_valid: true, data: WebReplyData::Message(message) }).unwrap_or(());
                    }

                    // The unpacking yielded parameters
                    UnpackResult::SuccessWithParameters(parameters) => {
                        request.reply_to.send(WebReply { is_valid: true, data: WebReplyData::Parameters(parameters) }).unwrap_or(());
                    }

                    // The unpacking yielded a path
                    UnpackResult::SuccessWithPath(path) => {
                        request.reply_to.send(WebReply { is_valid: true, data: WebReplyData::Path {
                            filename: path.file_stem().unwrap_or_else(|| { OsStr::new("") }).to_str().unwrap_or("").to_string(),
                            path: path.to_str().unwrap_or("").to_string(),
                        } }).unwrap_or(());
                    }

                    // The unpacking yielded a status
                    UnpackResult::SuccessWithStatus(status) => {
                        request.reply_to.send(WebReply { is_valid: true, data: WebReplyData::Status(Some(status)) }).unwrap_or(());
                    }

                    // The unpacking yielded a scene
                    UnpackResult::SuccessWithScene(scene) => {
                        request.reply_to.send(WebReply { is_valid: true, data: WebReplyData::Scene(Some(scene)) }).unwrap_or(());
                    }

                    // The unpacking was a failure
                    UnpackResult::Failure(reason) => {
                        request.reply_to.send(WebReply::failure(&reason)).unwrap_or(());
                    }

                    // The unpacking indicated the program should close
                    UnpackResult::Close => {
                        request.reply_to.send(WebReply::success()).unwrap_or(());
                        return false;
                    }
                }
            }
        }

        // In most cases, indicate to continue normally
        true
    }

    /// A method to run an infinite number of interations of the system
    /// interface to update the user and underlying system of any event changes.
    ///
    /// When this loop completes, it will consume the system interface and drop
    /// all associated data.
    ///
    pub async fn run(mut self) {
        // Loop the structure indefinitely
        loop {
            // Repeat endlessly until run_once reaches close
            if !self.run_once().await {
                break;
            }
        }

        // Drop all associated data in system interface
        drop(self);
    }

    /// A method to unpack internal updates from the main program thread.
    ///
    async fn unpack_internal_update(&mut self, update: InternalUpdate) {
        // Unpack the different variant types
        match update {
            // Update the timeline with the new list of coming events
            InternalUpdate::ComingEvents(mut events) => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Backup the coming events
                    handler.backup_events(events.clone()).await;
                }

                // Repackage the list as upcoming events
                let mut upcoming_events = Vec::new();
                for event in events.drain(..) {
                    // Find the description and add it
                    upcoming_events.push(UpcomingEvent {
                        start_time: event.start_time,
                        delay: event.delay,
                        event: self.index_access.get_pair(&event.id()).await,
                    });
                }

                // Send the upcoming events to the interface
                self.interface_send
                    .send(InterfaceUpdate::UpdateTimeline {
                        events: upcoming_events,
                    })
                    .await;
            }

            // Pass an event to the event_handler
            InternalUpdate::ProcessEvent {
                event_id,
                check_scene,
                send_to_connections,
            } => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Try to process the event, and collect any events to broadcast
                    let mut is_first = true;
                    for (event_id, data) in handler.process_event(&event_id, check_scene).await {
                        // If not the first event (i.e. this event), or if we should send the first event to the connections, send it
                        if !is_first || send_to_connections {
                            self.system_connection.broadcast(event_id, data).await;
                        }
                        is_first = false;

                        // Notify the user interface of the event
                        let description = self.index_access.get_description(&event_id).await;
                        self.interface_send
                            .send(InterfaceUpdate::Notify {
                                message: description.description,
                            })
                            .await;

                        // Pass the event as a limited update
                        self.limited_send
                            .send(LimitedUpdate::CurrentEvent {
                                event: event_id.clone(),
                            })
                            .await;
                    }

                // Otherwise notify the user that a configuration faild to load
                } else {
                    error!("Event {} could not be processed.", event_id);
                }
            }

            // Echo an event to the system connections
            InternalUpdate::EchoEvent {
                event_id,
                data1,
                data2,
            } => {
                // Echo events to the system connections
                self.system_connection.echo(event_id, data1, data2).await;
            }
        }
    }

    /// A method to unpack user requests from the main program thread.
    ///
    /// When the update is the Close variant, the function will return false,
    /// indicating that the thread should close.
    ///
    async fn unpack_request(&mut self, request: UserRequest) -> UnpackResult {
        // Unpack the different variant types
        match request {
            // Change the delay for all events in the queue
            UserRequest::AllEventChange {
                adjustment,
                is_negative,
            } => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Adjust the current time of the event
                    handler.adjust_all_events(adjustment, is_negative).await;

                // Otherwise, return a failure
                } else {
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Handle the All Stop command which clears the queue and sends the "all stop" (a.k.a. emergency stop) command.
            UserRequest::AllStop => {
                // Try to clear all the events in the queue
                if let Some(ref mut handler) = self.event_handler {
                    handler.clear_events().await;
                }

                // Place an note in the debug log
                error!("An All Stop was triggered by the operator.");

                // Pass the all stop event to the system connection
                self.system_connection
                    .broadcast(ItemId::all_stop(), None)
                    .await;

                // Notify the user interface of the event
                self.interface_send
                    .send(InterfaceUpdate::Notify {
                        message: "ALL STOP. Upcoming events have been cleared.".to_string(),
                    })
                    .await;

                // Pass the event as a limited update
                self.limited_send
                    .send(LimitedUpdate::CurrentEvent {
                        event: ItemId::all_stop(),
                    })
                    .await;
            }

            // Clear the events currently in the queue
            UserRequest::ClearQueue => {
                // Try to clear all the events in the queue
                if let Some(ref mut handler) = self.event_handler {
                    handler.clear_events().await;

                // Otherwise, return a failure
                } else {
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Close the system interface thread.
            UserRequest::Close => return UnpackResult::Close,

            // Update the configuration provided to the underlying system
            UserRequest::ConfigFile { filepath } => {
                // Try to clear all the events in the queue
                if let Some(mut handler) = self.event_handler.take() {
                    handler.clear_events().await;
                } // old handler is dropped

                // Load the specified filepath, or create a new config if none specified
                self.load_config(filepath, true).await;
            }

            // Return the name of the current configuration file, if available
            UserRequest::ConfigParameters => {
                // Collect all the configuration parameters
                if let Some(ref handler) = self.event_handler {
                    return UnpackResult::SuccessWithParameters(ConfigParameters {
                        identifier: handler.get_identifier(),
                        server_location: handler.get_server_location(),
                        dmx_path: handler.get_dmx_path(),
                        media_players: handler.get_media_players(),
                        system_connections: handler.get_connections(),
                        background_process: handler.get_background_process(),
                        default_scene: handler.get_default_scene(),
                    });

                // Otherwise, return a failure
                } else {
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Return the name of the current configuration file, if available
            UserRequest::ConfigPath => {
                // Try to get the config name
                if let Some(ref handler) = self.event_handler {
                    return UnpackResult::SuccessWithPath(handler.get_config_path());

                // Otherwise, return a failure
                } else {
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Cue an event
            UserRequest::CueEvent { event_delay } => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Cue the event
                    handler.add_event(event_delay).await;

                // Otherwise noity the user that a configuration failed to load
                } else {
                    error!("Event couldn't be cued. No active configuration.");
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Respond with the current scene and status
            UserRequest::CurrentSceneAndStatus => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Get the current scene
                    let scene_id = handler.get_current_scene();

                    // Get a copy of the status
                    let mut status = handler.get_statuses();

                    // Repackage into a current status (drop allowed states)
                    let mut current_status = CurrentStatus::default();
                    for (status_id, status_description) in status.drain() {
                        current_status.insert(status_id.id(), status_description.current.id());
                    }

                    // Return the completed information
                    return UnpackResult::SuccessWithCurrentSceneAndStatus((
                        scene_id,
                        current_status,
                    ));

                // Otherwise notify the user that a configuration failed to load
                } else {
                    warn!("Information unavailable. No active configuration.");
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Reqpond with a detail about a particular item
            UserRequest::Detail { detail_type } => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Placeholder for the final result
                    let result;

                    // Match the type of information request
                    match detail_type {
                        // Reply to a request for all items in the current scene
                        DetailType::AllCurrentItems => {
                            // Get the scene list
                            result = UnpackResult::SuccessWithItems(handler.get_current_items());
                        }

                        // Reply to a request for the groups
                        DetailType::AllGroups => {
                            // Get the scene list
                            result = UnpackResult::SuccessWithItems(handler.get_groups());
                        }

                        // Reply to a request for the scenes
                        DetailType::AllScenes => {
                            // Get the scene list
                            result = UnpackResult::SuccessWithItems(handler.get_scenes());
                        }

                        // Reply to a request for the event
                        DetailType::Event { item_id } => {
                            // Try to get the event
                            if let Some(event) = handler.get_event(&item_id) {
                                result = UnpackResult::SuccessWithEvent(event);
                            } else {
                                result = UnpackResult::Failure("Event Not Found.".into());
                            }
                        }

                        // Reply to a request for the group
                        DetailType::Group { item_id } => {
                            // Try to get the event
                            if let Some(group) = handler.get_group(&item_id) {
                                result = UnpackResult::SuccessWithGroup(group.into());
                            } else {
                                result = UnpackResult::Failure("Group Not Found.".into());
                            }
                        }

                        // Reply to a request for the status
                        DetailType::Status { item_id } => {
                            // Try to get the scene
                            if let Some(status) = handler.get_status(&item_id) {
                                result = UnpackResult::SuccessWithStatus(status);
                            } else {
                                result = UnpackResult::Failure("Status Not Found.".into());
                            }
                        }

                        // Reply to a request for the scene
                        DetailType::Scene { item_id } => {
                            // Try to get the scene
                            if let Some(scene) = handler.get_scene(&item_id) {
                                result = UnpackResult::SuccessWithScene(scene.into());
                            } else {
                                result = UnpackResult::Failure("Scene Not Found.".into());
                            }
                        }

                        // Reply to a request for item type
                        DetailType::Type { item_id } => {
                            // Check to see if there is a scene
                            if handler.get_scene(&item_id).is_some() {
                                result = UnpackResult::SuccessWithMessage("scene".into());
                            // Check to see if there is a status
                            } else if handler.get_status(&item_id).is_some() {
                                result = UnpackResult::SuccessWithMessage("status".into());
                            // Check to see if there is an event
                            } else if handler.get_event(&item_id).is_some() {
                                result = UnpackResult::SuccessWithMessage("event".into());
                            // Check to see if there is a group
                            } else if handler.get_group(&item_id).is_some() {
                                result = UnpackResult::SuccessWithMessage("group".into());
                            // Otherwise, return none
                            } else {
                                result = UnpackResult::SuccessWithMessage("none".into());
                            }
                        }
                    }

                    // Return the result
                    return result;

                // Otherwise notify the user that a configuration failed to load
                } else {
                    warn!("Information unavailable. No active configuration.");
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Modify the underlying configuration
            UserRequest::Edit { mut modifications } => {
                // Check to see if there is an active configuration
                if let Some(ref mut handler) = self.event_handler {
                    // Process each modification in order
                    for modification in modifications.drain(..) {
                        // Match the specified moficiation
                        match modification {
                            // Add or modify the item
                            Modification::ModifyItem { item_pair } => {
                                // Pass the update and see if it's a new item
                                if self
                                    .index_access
                                    .update_description(
                                        item_pair.get_id(),
                                        item_pair.get_description(),
                                    )
                                    .await
                                {
                                    info!("Item description added: {}.", item_pair.description());

                                // If not, notify that the item was updated
                                } else {
                                    info!("Item description updated: {}.", item_pair.description());
                                }
                            }

                            // Add or modify the event
                            Modification::ModifyEvent { item_id, event } => {
                                handler.edit_event(item_id, event.map(|e| e.into())).await;
                            }

                            // Add or modify the group
                            Modification::ModifyGroup { item_id, group } => {
                                // Recompose the web group into a group
                                let new_group = group.map(|group| group.into());
                                handler.edit_group(item_id, new_group).await;
                            }

                            // Update the configuration parameters
                            Modification::ModifyParameters { parameters } => {
                                // Save the configuration parameters
                                handler.save_parameters(parameters).await;
                            }

                            // Add or modify the status
                            Modification::ModifyStatus { item_id, status } => {
                                handler.edit_status(item_id, status).await;
                            }

                            // Add or modify the scene
                            Modification::ModifyScene { item_id, scene } => {
                                // Recompose the web scene into a scene
                                let new_scene = match scene {
                                    Some(scene) => {
                                        // Extract the groups from the item list
                                        let mut items = FnvHashSet::default();
                                        let mut groups = FnvHashSet::default();
                                        for item_id in scene.items.iter() {
                                            // If it's a group, add it to the group list
                                            if handler.get_group(item_id).is_some() {
                                                groups.insert(item_id.clone());

                                            // Otherwise, save it to the item list
                                            } else {
                                                items.insert(item_id.clone());
                                            }
                                        }

                                        // Return the new scene
                                        Some(Scene {
                                            items,
                                            groups,
                                            key_map: scene.key_map,
                                        })
                                    }

                                    // Do nothing if empty
                                    None => None,
                                };

                                // Update the scene
                                handler.edit_scene(item_id, new_scene).await;
                            }

                            // Remove an item and its event, status, or scene
                            Modification::RemoveItem { item_id } => {
                                // Remove any event, status, group, or scene
                                handler.edit_event(item_id, None).await;
                                handler.edit_status(item_id, None).await;
                                handler.edit_group(item_id, None).await;
                                handler.edit_scene(item_id, None).await;

                                // Remove the item from any scene, group, or status it's a part of
                                handler.remove_item(item_id).await;

                                // Get the description
                                let description = self.index_access.get_description(&item_id).await;

                                // Remove the entry in the item index
                                if self.index_access.remove_item(item_id).await {
                                    info!("Item deleted: {}.", description);
                                } // ignore errors
                            }
                        }
                    }

                // Raise a warning that there is no active configuration
                } else {
                    warn!("Change not saved: There is no active configuration.");
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Change the remaining delay for an existing event in the queue
            UserRequest::EventChange {
                event_id,
                start_time,
                new_delay,
            } => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Adjust the current time of the event
                    handler.adjust_event(event_id, start_time, new_delay).await;

                // Otherwise, return a failure
                } else {
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Save the current configuration to the provided file
            UserRequest::SaveConfig { filepath } => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Save the current configuration
                    handler.save_config(filepath).await;

                // Otherwise, return a failure
                } else {
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Change the current scene based on the provided id and get a list of available events
            UserRequest::SceneChange { scene } => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Change the current scene (automatically triggers a redraw)
                    handler.choose_scene(scene).await;

                // Otherwise, return a failure
                } else {
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Change the state of a particular status
            UserRequest::StatusChange { status, state } => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Change the state of the indicated status
                    handler.modify_status(&status, &state).await;

                // Otherwise, return a failure
                } else {
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }
        }
        UnpackResult::Success // indicate to continue and no errors
    }

    /// An internal method to try to load the provided configuration into the
    /// system interface. If no filepath is provided, the function will create
    /// a new empty configuration.
    ///
    /// # Errors
    ///
    /// When the log failure flag is false, the function will not post an error
    /// about failing to locate the configuration file. Regardless of the flag,
    /// all other types of errors will be logged.
    ///
    async fn load_config(&mut self, filepath: Option<PathBuf>, log_failure: bool) {
        // Create a new event handler
        let event_handler = match EventHandler::new(
            filepath,
            self.index_access.clone(),
            self.style_access.clone(),
            self.internal_send.clone(),
            self.interface_send.clone(),
            self.limited_send.clone(),
            log_failure,
        )
        .await
        {
            Ok(evnt_hdlr) => evnt_hdlr,
            Err(_) => return, // errors will be logged separately if log_failure is true
        };

        // Create a new connection to the hardware system
        self.system_connection
            .update_system_connections(Some((
                event_handler.get_connections(),
                event_handler.get_identifier(),
            )))
            .await;

        // Trigger a redraw of the system
        self.interface_send.send(InterfaceUpdate::RefreshAll).await;

        // Update the event handler
        self.event_handler = Some(event_handler);
    }
}

// A helper enum to indicate the result of unpacking a request
enum UnpackResult {
    // A variant for successful unpacking
    Success,

    // A variant for successful unpacking with current scene and status
    SuccessWithCurrentSceneAndStatus((ItemId, CurrentStatus)),

    // A variant for successful unpacking with an event
    SuccessWithEvent(Event),

    // A variant for successful unpacking with items
    SuccessWithItems(Vec<ItemId>),

    // A variant for successful unpacking with a group
    SuccessWithGroup(WebGroup),

    // A variant for successful unpacking with message
    SuccessWithMessage(String),

    // A variant for successful unpacking with system parameters
    SuccessWithParameters(ConfigParameters),

    // A variant for successful unpacking with config path
    SuccessWithPath(PathBuf),

    // A variant for successful unpacking with a scene
    SuccessWithScene(WebScene),

    // A variant for successful unpacking with a status
    SuccessWithStatus(Status),

    // A variant for unsuccessful unpacking
    Failure(String),

    // A variant to indicate the program should close
    Close,
}

// Tests of the system_interface module
#[cfg(test)]
mod tests {
    //use super::*;

    // FIXME Define tests of this module
    #[test]
    fn missing_tests() {
        // FIXME: Implement this
        unimplemented!();
    }
}
