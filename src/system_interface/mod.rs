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
mod logging;
#[macro_use]
mod event_handler;
mod system_connection;

// Import crate definitions
use crate::definitions::*;

// Import other definitions
use self::event_handler::EventHandler;
use self::logging::Logger;
use self::system_connection::SystemConnection;

// Import standard library features
use std::env;
use std::fs::DirBuilder;
use std::path::PathBuf;

// Import Tokio features
use tokio::sync::mpsc;

// Import FNV HashMap
use fnv::FnvHashMap;

// Import the failure features
use failure::Error as FailureError;

// Define module constants
const POLLING_RATE: u64 = 1; // the polling rate for the system in ms
const DEFAULT_FILE: &str = "default"; // the default configuration filename
const LOG_FOLDER: &str = "log/"; // the default log folder
const ERROR_LOG: &str = "debug_log.txt"; // the default logging filename

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
    logger: Logger,                      // the logging instance for the program
    system_connection: SystemConnection, // the system connection instance for the program
    index_access: IndexAccess,           // the access point for the item index
    interface_send: InterfaceSend, // a sending line to pass interface updates
    gtk_receive: mpsc::Receiver<GtkRequest>, // the receiving line for gtk requests
    web_receive: mpsc::Receiver<WebRequest>, // the receiving line for web requests
    internal_receive: mpsc::Receiver<InternalUpdate>, // a receiving line to receive internal updates
    internal_send: InternalSend,                      // a sending line to pass internal updates
    is_debug_mode: bool,                              // a flag to indicate debug mode
}

// Implement key SystemInterface functionality
impl SystemInterface {
    /// A function to create a new, blank instance of the system interface.
    ///
    pub async fn new(
        index_access: IndexAccess,
        interface_send: InterfaceSend,
    ) -> Result<(Self, GtkSend, WebSend), FailureError> {
        // Create the new general update structure and receive channel
        let (internal_send, internal_receive) = InternalSend::new();

        // Try to load the default logging file
        let (log_folder, error_log) = match env::current_dir() {
            // If the path loads
            Ok(mut path) => {
                // Create the log folder path
                path.push(LOG_FOLDER); // append the log folder

                // Make sure the log folder exists
                let builder = DirBuilder::new();
                builder.create(path.clone()).unwrap_or(()); // ignore if it already exits

                // Create the error log path
                let mut error_path = path.clone();
                error_path.push(ERROR_LOG); // append the dafault error log filename
                (Some(path), Some(error_path))
            }
            _ => (None, None),
        };

        // Try to create a new logger instance
        let logger = Logger::new(
            log_folder,
            error_log,
            index_access.clone(),
            internal_send.clone(),
            interface_send.clone(),
        )?;

        // Create a new system connection instance
        let system_connection = SystemConnection::new(internal_send.clone(), None).await;

        // Create the gtk send for the gtk interface
        let (gtk_send, gtk_receive) = GtkSend::new();

        // Create the web send for the web interface
        let (web_send, web_receive) = WebSend::new();

        // Create the new system interface instance
        let mut sys_interface = SystemInterface {
            event_handler: None,
            logger,
            system_connection,
            index_access,
            interface_send,
            gtk_receive,
            web_receive,
            internal_receive,
            internal_send,
            is_debug_mode: false,
        };

        // Try to load a default configuration, if it exists
        if let Ok(mut path) = env::current_dir() {
            // Add the default filename
            path.push(DEFAULT_FILE);

            // Add the mnv filetype
            path.set_extension("mnv");

            // Try the file, if it exists
            if path.exists() {
                sys_interface.load_config(path, false).await;

            // Otherwise, try the yaml path
            } else {
                path.set_extension("yaml");
                sys_interface.load_config(path, false).await;
            }
        }

        // Regardless, return the new SystemInterface and general send line
        Ok((sys_interface, gtk_send, web_send))
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

            // Requests from the Gtk Interface
            Some(request) = self.gtk_receive.recv() => {
                // If the program should close
                if let UnpackResult::Close = self.unpack_request(request.into()).await {
                    return false;
                }
            }

            // Updates from the Web Interface
            Some(request) = self.web_receive.recv() => {
                // Unpack the request
                match self.unpack_request(request.request).await {
                    // The unpacking was a success
                    UnpackResult::Success => {
                        request.reply_to.send(WebReply::success()).unwrap_or(());
                    }

                    // The unpacking yielded an event
                    UnpackResult::SuccessWithEvent(event) => {
                        request.reply_to.send(WebReply::Event { is_valid: true, event: Some(event) }).unwrap_or(());
                    }

                    // The unpacking yielded items
                    UnpackResult::SuccessWithItems(items) => {
                        request.reply_to.send(WebReply::Items { is_valid: true, items }).unwrap_or(());
                    }

                    // The unpacking yielded a message
                    UnpackResult::SuccessWithMessage(message) => {
                        request.reply_to.send(WebReply::Generic { is_valid: true, message }).unwrap_or(());
                    }

                    // The unpacking yielded a status
                    UnpackResult::SuccessWithStatus(status) => {
                        request.reply_to.send(WebReply::Status { is_valid: true, status: Some(status) }).unwrap_or(());
                    }

                    // The unpacking yielded a scene
                    UnpackResult::SuccessWithScene(scene) => {
                        request.reply_to.send(WebReply::Scene { is_valid: true, scene: Some(scene) }).unwrap_or(());
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
            // Broadcast the event via the system connection
            InternalUpdate::BroadcastEvent(event_id, data) => {
                self.system_connection.broadcast(event_id, data).await;
            }

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
                    }).await;
            }

            // Solicit a string from the user
            InternalUpdate::GetUserString(event) => {
                // Get the item pair for the event
                let pair = self.index_access.get_pair(&event).await;

                // Request the information from the user interface
                self.interface_send
                    .send(InterfaceUpdate::LaunchWindow {
                        window_type: WindowType::PromptString(pair),
                    }).await;
            }

            // Pass a video stream to the user interface
            #[cfg(feature = "media-out")]
            InternalUpdate::NewVideo(video_stream) => {
                // Pass the stream to the user interface
                self.interface_send
                    .send(InterfaceUpdate::LaunchWindow {
                        window_type: WindowType::Video(video_stream),
                    }).await;
            }

            // Pass an event to the event_handler
            InternalUpdate::ProcessEvent {
                event,
                check_scene,
                broadcast,
            } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Try to process the event
                    if handler.process_event(&event, check_scene, broadcast).await {
                        // Notify the user interface of the event
                        let description = self.index_access.get_description(&event).await;
                        self.interface_send
                            .send(InterfaceUpdate::Notify {
                                message: description.description,
                            }).await;
                    }

                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise notify the user that a configuration faild to load
                } else {
                    log!(err &mut self.internal_send => "Event Could Not Be Processed. No Active Configuration.");
                }
            }

            // Refresh the interface
            InternalUpdate::RefreshInterface => {
                // Try to redraw the current window
                if let Some(ref mut handler) = self.event_handler {
                    // Repackage the items in the current scene
                    // FIXME The user interface should get this information separately
                    let item_ids = handler.get_current_items();
                    let mut current_items = Vec::new();
                    for item_id in item_ids {
                        // Combine the item pair
                        current_items.push(self.index_access.get_pair(&item_id).await);
                    }

                    // Compose the new event window and status items
                    let (window, statuses) = SystemInterface::sort_items(
                        current_items,
                        self.index_access.clone(),
                        self.is_debug_mode,
                    )
                    .await;

                    // Get the current scene and key map
                    let current_scene = self
                        .index_access
                        .get_pair(&handler.get_current_scene())
                        .await;
                    let key_map = handler.get_key_map().await;

                    // Send the update with the new event window
                    self.interface_send
                        .send(InterfaceUpdate::UpdateWindow {
                            current_scene,
                            window,
                            statuses,
                            key_map,
                        }).await;
                }
            }

            // Pass the information update to the logger
            InternalUpdate::Update(log_update) => {
                // Find the most recent notifications
                let notifications = self.logger.update(log_update).await;

                // Send a notification update to the system
                self.interface_send
                    .send(InterfaceUpdate::UpdateNotifications { notifications }).await;
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
                if let Some(mut handler) = self.event_handler.take() {
                    // Adjust the current time of the event
                    handler.adjust_all_events(adjustment, is_negative).await;

                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise, return a failure
                } else {
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Handle the All Stop command which clears the queue and sends the "all stop" (a.k.a. emergency stop) command.
            UserRequest::AllStop => {
                // Try to clear all the events in the queue
                if let Some(mut handler) = self.event_handler.take() {
                    handler.clear_events().await;

                    // Put the handler back
                    self.event_handler = Some(handler);
                }

                // Send the all stop event via the logger
                log!(broadcast &mut self.internal_send => ItemId::all_stop(), None);

                // Place an error in the debug log
                log!(err &mut self.internal_send => "An All Stop was triggered by the operator.");

                // Notify the user interface of the event
                self.interface_send
                    .send(InterfaceUpdate::Notify {
                        message: "ALL STOP. Upcoming events have been cleared.".to_string(),
                    }).await;
            }

            // Pass a broadcast event to the system connection (used only by
            // the user interface, not for internal messaging. See
            // GeneralUpdate::BroadcastEvent)
            UserRequest::BroadcastEvent { event_id, data } => {
                // Broadcast the event via the logger
                log!(broadcast &mut self.internal_send => event_id, data);

                // Get the event description
                let message = self
                    .index_access
                    .get_description(&event_id)
                    .await
                    .description;

                // Notify the user interface of the event
                self.interface_send
                    .send(InterfaceUpdate::Notify { message }).await;
            }

            // Clear the events currently in the queue
            UserRequest::ClearQueue => {
                // Try to clear all the events in the queue
                if let Some(mut handler) = self.event_handler.take() {
                    handler.clear_events().await;

                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise, return a failure
                } else {
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Close the system interface thread.
            UserRequest::Close => return UnpackResult::Close, // FIXME doesn't work when selected from the web interface

            // Update the configuration provided to the underlying system
            UserRequest::ConfigFile { filepath } => {
                // Try to clear all the events in the queue
                if let Some(mut handler) = self.event_handler.take() {
                    handler.clear_events().await;
                } // old handler is dropped

                // Check to see if a new filepath was specified
                if let Some(path) = filepath {
                    // If so, try to load it
                    self.load_config(path, true).await;
                }
            }

            // Cue an event
            UserRequest::CueEvent { event_delay } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Cue the event
                    handler.add_event(event_delay).await;

                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise noity the user that a configuration failed to load
                } else {
                    log!(err &mut self.internal_send => "Event couldn't be cued. No active configuration.");
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Swtich between normal mode and debug mode
            UserRequest::DebugMode(mode) => {
                // Switch the mode (redraw triggered by the user interface)
                self.is_debug_mode = mode;
            }

            // Reqpond with a detail about a particular item
            UserRequest::Detail { detail_type } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Placeholder for the final result
                    let mut result = UnpackResult::Failure("Invalid Web Request.".into());

                    // Match the type of information request
                    match detail_type {
                        // Reply to a request for the scenes FIXME Consider moving this to the index
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
                                result = UnpackResult::SuccessWithScene(scene);
                            } else {
                                result = UnpackResult::Failure("Scene Not Found.".into());
                            }
                        }

                        // Reply to a request for item type
                        DetailType::Type { item_id } => {
                            // Check to see if there is a scene
                            if let Some(_) = handler.get_scene(&item_id) {
                                result = UnpackResult::SuccessWithMessage("scene".into());
                            // Check to see if there is a status
                            } else if let Some(_) = handler.get_status(&item_id) {
                                result = UnpackResult::SuccessWithMessage("status".into());
                            // Check to see if there is an event
                            } else if let Some(_) = handler.get_event(&item_id) {
                                result = UnpackResult::SuccessWithMessage("event".into());
                            // Check to see if the item has a description
                            } else if self.index_access.is_listed(&item_id).await {
                                result = UnpackResult::SuccessWithMessage("label".into());
                            // Otherwise, return none
                            } else {
                                result = UnpackResult::SuccessWithMessage("none".into());
                            }
                        }

                        // For other types, warn of an internal error
                        _ => log!(warn &mut self.internal_send => "Invalid Web Request."), // FIXME to  be removed
                    }

                    // Put the handler back
                    self.event_handler = Some(handler);

                    // Return the result
                    return result;

                // Otherwise noity the user that a configuration failed to load
                } else {
                    log!(warn &mut self.internal_send => "Information Unavailable. No Active Configuration.");
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Modify the underlying configuration
            UserRequest::Edit { mut modifications } => {
                // Check to see if there is an active configuration
                if let Some(mut handler) = self.event_handler.take() {
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
                                    log!(update &self.internal_send => "Item Description Updated: {}", item_pair.description());

                                // If not, notify that the item was updated
                                } else {
                                    log!(update &self.internal_send => "Item Description Added: {}", item_pair.description());
                                }
                            }

                            // Add or modify the event
                            Modification::ModifyEvent { item_id, event } => {
                                handler.edit_event(item_id, event).await;
                            }

                            // Add or modify the status
                            Modification::ModifyStatus { item_id, status } => {
                                handler.edit_status(item_id, status).await;
                            }

                            // Add or modify the scene
                            Modification::ModifyScene { item_id, scene } => {
                                handler.edit_scene(item_id, scene).await;
                            }
                        }
                    }

                    // Put the handler back
                    self.event_handler = Some(handler);

                // Raise a warning that there is no active configuration
                } else {
                    log!(warn &mut self.internal_send => "Change Not Saved: There Is No Active Configuration.");
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
                if let Some(mut handler) = self.event_handler.take() {
                    // Adjust the current time of the event
                    handler.adjust_event(event_id, start_time, new_delay).await;

                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }

            // Update the system log provided to the underlying system
            UserRequest::ErrorLog { filepath } => self.logger.set_error_log(filepath),

            // Update the game log provided to the underlying system
            UserRequest::GameLog { filepath } => self.logger.set_game_log(filepath),

            // Pass an event to the event_handler
            UserRequest::ProcessEvent {
                event,
                check_scene,
                broadcast,
            } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Try to process the event
                    if handler.process_event(&event, check_scene, broadcast).await {
                        // Notify the user interface of the event
                        let description = self.index_access.get_description(&event).await;
                        self.interface_send
                            .send(InterfaceUpdate::Notify {
                                message: description.description,
                            }).await;
                    }

                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise notify the user that a configuration faild to load
                } else {
                    log!(err &mut self.internal_send => "Event Could Not Be Processed. No Active Configuration.");
                    return UnpackResult::Failure("No active configuration.".into());
                }
            }

            // Redraw the current window
            UserRequest::Redraw => {
                // Try to redraw the current window
                if let Some(ref mut handler) = self.event_handler {
                    // Repackage the items in the current scene
                    // FIXME The user interface should get this information separately
                    let item_ids = handler.get_current_items();
                    let mut current_items = Vec::new();
                    for item_id in item_ids {
                        // Combine the item pair
                        current_items.push(self.index_access.get_pair(&item_id).await);
                    }

                    // Compose the new event window and status items
                    let (window, statuses) = SystemInterface::sort_items(
                        current_items,
                        self.index_access.clone(),
                        self.is_debug_mode,
                    )
                    .await;

                    // Get the current scene and key map
                    let current_scene = self
                        .index_access
                        .get_pair(&handler.get_current_scene())
                        .await;
                    let key_map = handler.get_key_map().await;

                    // Send the update with the new event window
                    self.interface_send
                        .send(InterfaceUpdate::UpdateWindow {
                            current_scene,
                            window,
                            statuses,
                            key_map,
                        }).await;
                }
            }

            // Reply to the request for information
            UserRequest::GtkRequest { reply_to, request } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Match the type of information request
                    match request {
                        // Reply to a request for the item description
                        DetailType::Description { item_id } => {
                            // Collect the description of the item
                            let description = self.index_access.get_description(&item_id).await;

                            // Create the item pair
                            let item_pair = ItemPair::from_item(item_id, description);

                            // Send it back to the user interface
                            self.interface_send
                                .send(InterfaceUpdate::Reply {
                                    reply_to, // echo to display component
                                    reply: ReplyType::Description {
                                        description: item_pair,
                                    },
                                }).await;
                        }

                        // Reply to a request for the event
                        DetailType::Event { item_id } => {
                            // Try to get the event
                            let event = handler.get_event(&item_id);

                            // Send an update with the event (or None)
                            self.interface_send
                                .send(InterfaceUpdate::Reply {
                                    reply_to, // echo the display component
                                    reply: ReplyType::Event { event },
                                }).await;
                        }

                        // Reply to a request for all the configuration items
                        DetailType::Items => {
                            // Collect all the items from the configuration
                            let items = self.index_access.get_all_pairs().await;

                            // Send it back to the user interface
                            self.interface_send
                                .send(InterfaceUpdate::Reply {
                                    reply_to,
                                    reply: ReplyType::Items { items },
                                }).await;
                        }

                        // Reply to a request for all the events in a scene
                        DetailType::Scene { item_id } => {
                            // Collect all the items from the configuration
                            let scene = match handler.get_scene(&item_id) {

                                // If it doesn't correspond to a scene, return none
                                None => None,

                                // If it does match, get the items and optional key map
                                Some(scene) => {
                                    // Compile a list of the events as item pairs
                                    let mut events = Vec::new();
                                    for item_id in scene.events.iter() {
                                        events.push(self.index_access.get_pair(&item_id).await);
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
                                                map.insert(key.clone(), self.index_access.get_pair(&item_id).await);
                                            }
                                            // Return the Scene with the reversed key map
                                            Some(DescriptiveScene {
                                                events,
                                                key_map: Some(map),
                                            })
                                        }

                                        // Otherwise, return the DescriptiveScene with no key map
                                        None => Some(DescriptiveScene {
                                            events,
                                            key_map: None,
                                        }),
                                    }
                                }
                            };

                            // Send it back to the user interface
                            self.interface_send
                                .send(InterfaceUpdate::Reply {
                                    reply_to,
                                    reply: ReplyType::Scene { scene },
                                }).await;
                        }

                        // Reply to a request for the status
                        DetailType::Status { item_id } => {
                            // Try to get the status
                            let status = handler.get_status(&item_id);

                            // Send an update with the status (or None)
                            self.interface_send
                                .send(InterfaceUpdate::Reply {
                                    reply_to, // echo the display component
                                    reply: ReplyType::Status { status },
                                }).await;
                        }

                        // Ignore other requests
                        _ => (),
                    }

                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise noity the user that a configuration failed to load
                } else {
                    log!(warn &mut self.internal_send => "Information Unavailable. No Active Configuration.");
                }
            }

            // Save the current configuration to the provided file
            UserRequest::SaveConfig { filepath } => {
                // Extract the current event handler (if it exists)
                if let Some(mut handler) = self.event_handler.take() {
                    // Save the current configuration
                    handler.save_config(filepath).await;

                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }

            // Change the current scene based on the provided id and get a list of available events
            UserRequest::SceneChange { scene } => {
                // Change the current scene, if event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Change the current scene (automatically triggers a redraw)
                    handler.choose_scene(scene).await;

                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }

            // Change the state of a particular status
            UserRequest::StatusChange { status, state } => {
                // Change the status, if event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Change the state of the indicated status
                    handler.modify_status(&status, &state).await;

                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }
        }
        UnpackResult::Success // indicate to continue and no errors
    }

    /// An internal method to try to load the provided configuration into the
    /// system interface.
    ///
    /// # Errors
    ///
    /// When the log failure flag is false, the function will not post an error
    /// about failing to locate the configuration file. Regardless of the flag,
    /// all other types of errors will be logged.
    ///
    async fn load_config(&mut self, filepath: PathBuf, log_failure: bool) {
        // Clone the interface send
        let interface_send = self.interface_send.clone();

        // Create a new event handler
        let mut event_handler = match EventHandler::new(
            filepath,
            self.index_access.clone(),
            self.internal_send.clone(),
            interface_send,
            log_failure,
        )
        .await
        {
            Ok(evnt_hdlr) => evnt_hdlr,
            Err(_) => return, // errors will be logged separately if log_failure is true
        };

        // Create a new connection to the underlying system
        let system_connection = event_handler.system_connection();
        if !self
            .system_connection
            .update_system_connection(Some(system_connection))
            .await
        {
            return;
        }

        // Get the scenes and full status
        let scene_ids = event_handler.get_scenes();
        let mut partial_status = event_handler.get_statuses();

        // Repackage the scenes with their descriptions
        // FIXME Should be pulled separately by the user interface
        let mut scenes = Vec::new();
        for scene_id in scene_ids {
            scenes.push(self.index_access.get_pair(&scene_id).await);
        }

        // Repackage the statuses with their descriptions
        // FIXME Should be pulled separately by the user interface
        let mut full_status = FullStatus::default();

        // For each status id and status
        for (status_id, mut status) in partial_status.drain() {
            // Create a new status pair from the status id
            let status_pair = self.index_access.get_pair(&status_id).await;

            // Create the new current pair from the status description
            let current = self.index_access.get_pair(&status.current).await;

            // Repackage the allowed states
            let mut allowed = Vec::new();
            for state in status.allowed.drain(..) {
                allowed.push(self.index_access.get_pair(&state).await);
            }

            // Add the new key/value to the full status
            full_status.insert(status_pair, StatusDescription { current, allowed });
        }

        // Send the newly available scenes and full status to the user interface
        self.interface_send
            .send(InterfaceUpdate::UpdateConfig {
                scenes,
                full_status,
            }).await;

        // Trigger a redraw of the system
        self.internal_send.send_refresh().await;

        // Update the event handler
        self.event_handler = Some(event_handler);
    }

    /// An internal to sort the available events in this current scene
    /// into an Event Window.
    ///
    async fn sort_items(
        mut items: Vec<ItemPair>,
        index_access: IndexAccess,
        is_debug_mode: bool,
    ) -> (EventWindow, Vec<ItemPair>) {
        // Iterate through the items and group them
        let mut groups = Vec::new();
        let mut general_group = Vec::new();
        let mut statuses = Vec::new();
        for item in items.drain(..) {
            // Unpack the items
            match item.display {
                // Add display control events to the general control group
                DisplayControl { .. } => general_group.push(item),

                // Add display with events to the matching event group
                DisplayWith { group_id, .. } => {
                    let group_pair = index_access.get_pair(&group_id).await;
                    SystemInterface::sort_groups(&mut groups, group_pair, item);
                }

                // Add display debug events to the matching event group
                DisplayDebug { group_id, .. } => {
                    // If the system is in debug mode
                    if is_debug_mode {
                        // If a group id is specified, add it to the correct group
                        if let Some(id) = group_id {
                            let group_pair = index_access.get_pair(&id).await;
                            SystemInterface::sort_groups(&mut groups, group_pair, item);

                        // Otherwise add it to the general group
                        } else {
                            general_group.push(item);
                        }
                    }
                }

                // Add label control items to the statuses list
                LabelControl { .. } => statuses.push(item),

                // Ignore label hidden and hidden items
                _ => (),
            }
        }

        // Add the general group to the rest of the groups and return the packaged result
        groups.push(EventGroup {
            group_id: None,
            group_events: general_group,
        });
        (groups, statuses)
    }

    /// An internal function to sort through the groups currently in the provided
    /// vector, add the provided event if it matches one of the groups, and
    /// create a new group if it does not.
    ///
    fn sort_groups(groups: &mut Vec<EventGroup>, event_group: ItemPair, event: ItemPair) {
        // Look through the existing groups for a group match
        let mut found = false; // flag for if a matching group was found
        for group in groups.iter_mut() {
            // Check for a real group id
            if let Some(ref id) = group.group_id {
                // If the id is a match, add the current event
                if id == &event_group {
                    group.group_events.push(event.clone());
                    found = true;
                    break;
                }
            }
        }

        // If a matching id was not found, add a new group
        if !found {
            // Check to see if the group id has a corresponding status
            groups.push(EventGroup {
                group_id: Some(event_group),
                group_events: vec![event],
            });
        }
    }
}

// A helper enum to indicate the result of unpacking a request
enum UnpackResult {
    // A variant for successful unpacking
    Success,

    // A variant for successful unpacking with an event
    SuccessWithEvent(Event),

    // A variant for successful unpacking with items
    SuccessWithItems(Vec<ItemId>),

    // A variant for successful unpacking with message
    SuccessWithMessage(String),

    // A variant for successful unpacking with a scene
    SuccessWithScene(Scene),

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
