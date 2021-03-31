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
use self::system_connection::SystemConnection;
use self::logging::Logger;
#[cfg(feature = "media-out")]
use crate::definitions::LaunchWindow;

// Import standard library features
use std::env;
use std::fs::DirBuilder;
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;

// Import Tokio features
use tokio::sync::mpsc;

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
    interface_send: std_mpsc::Sender<InterfaceUpdate>, // a sending line to pass interface updates
    system_receive: mpsc::Receiver<SystemUpdate>, // a receiving line to receive system updates
    internal_receive: mpsc::Receiver<InternalUpdate>, // a receiving line to receive internal updates
    internal_send: InternalSend, // a sending line to pass internal updates
    is_debug_mode: bool,           // a flag to indicate debug mode
}

// Implement key SystemInterface functionality
impl SystemInterface {
    /// A function to create a new, blank instance of the system interface.
    ///
    pub async fn new(
        index_access: IndexAccess,
        interface_send: std_mpsc::Sender<InterfaceUpdate>,
    ) -> Result<(Self, SystemSend), FailureError> {
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

        // Create the sytem send for the user interface
        let (system_send, system_receive) = SystemSend::new();

        // Create the new system interface instance
        let mut sys_interface = SystemInterface {
            event_handler: None,
            logger,
            system_connection,
            index_access,
            interface_send,
            system_receive,
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
        Ok((sys_interface, system_send))
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

            // Updates from the User Interface
            Some(update) = self.system_receive.recv() => {
                return self.unpack_system_update(update).await;
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
            // Repeat endlessly until run_once fails.
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
                self.interface_send.send(InterfaceUpdate::UpdateTimeline {
                    events: upcoming_events,
                }).unwrap_or(());
            }

            // Solicit a string from the user
            InternalUpdate::GetUserString(event) => {
                // Get the item pair for the event
                let pair = self.index_access.get_pair(&event).await;
                
                // Request the information from the user interface
                self.interface_send
                    .send(InterfaceUpdate::LaunchWindow {
                        window_type: WindowType::PromptString(pair),
                    })
                    .unwrap_or(());
            }
            
            // Pass a video stream to the user interface
            #[cfg(feature = "media-out")]
            InternalUpdate::NewVideo(video_stream) => {
                // Pass the stream to the user interface
                self.interface_send
                    .send(LaunchWindow {
                        window_type: WindowType::Video(video_stream),
                    })
                    .unwrap_or(());
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
                            })
                            .unwrap_or(());
                    }
                    
                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise notify the user that a configuration faild to load
                } else {
                    update!(err &mut self.internal_send => "Event Could Not Be Processed. No Active Configuration.");
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
                    ).await;

                    // Get the current scene and key map
                    let current_scene = self.index_access.get_pair(&handler.get_current_scene()).await;
                    let key_map = handler.get_key_map().await;

                    // Send the update with the new event window
                    self.interface_send
                        .send(InterfaceUpdate::UpdateWindow {
                            current_scene,
                            window,
                            statuses,
                            key_map,
                        })
                        .unwrap_or(());
                }
            }

            // Pass the information update to the logger
            InternalUpdate::Update(log_update) => {
                // Find the most recent notifications
                let notifications = self.logger.update(log_update).await;

                // Send a notification update to the system
                self.interface_send
                    .send(InterfaceUpdate::UpdateNotifications { notifications })
                    .unwrap_or(());
            }
        }
    }

    /// A method to unpack system updates from the main program thread.
    ///
    /// When the update is the Close variant, the function will return false,
    /// indicating that the thread should close.
    ///
    async fn unpack_system_update(&mut self, update: SystemUpdate) -> bool {
        // Unpack the different variant types
        match update {
            // Change the delay for all events in the queue
            SystemUpdate::AllEventChange {
                adjustment,
                is_negative,
            } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Adjust the current time of the event
                    handler.adjust_all_events(adjustment, is_negative).await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }

            // Handle the All Stop command which clears the queue and sends the "all stop" (a.k.a. emergency stop) command.
            SystemUpdate::AllStop => {
                // Try to clear all the events in the queue
                if let Some(mut handler) = self.event_handler.take() {
                    handler.clear_events().await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);
                }

                // Send the all stop event via the logger
                update!(broadcast &mut self.internal_send => ItemId::all_stop(), None);

                // Place an error in the debug log
                update!(err &mut self.internal_send => "An All Stop was triggered by the operator.");

                // Notify the user interface of the event
                self.interface_send
                    .send(InterfaceUpdate::Notify {
                        message: "ALL STOP. Upcoming events have been cleared.".to_string(),
                    })
                    .unwrap_or(());
            }

            // Pass a broadcast event to the system connection (used only by
            // the user interface, not for internal messaging. See
            // GeneralUpdate::BroadcastEvent)
            SystemUpdate::BroadcastEvent { event, data } => {
                // Broadcast the event via the logger
                update!(broadcast &mut self.internal_send => event.get_id(), data);

                // Notify the user interface of the event
                self.interface_send
                    .send(InterfaceUpdate::Notify {
                        message: event.description,
                    })
                    .unwrap_or(());
            }

            // Clear the events currently in the queue
            SystemUpdate::ClearQueue => {
                // Try to clear all the events in the queue
                if let Some(mut handler) = self.event_handler.take() {
                    handler.clear_events().await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }

            // Close the system interface thread.
            SystemUpdate::Close => return false,

            // Update the configuration provided to the underlying system
            SystemUpdate::ConfigFile { filepath } => {
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
            SystemUpdate::CueEvent { event_delay } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Cue the event
                    handler.add_event(event_delay).await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise noity the user that a configuration faild to load
                } else {
                    update!(err &mut self.internal_send => "Event Could Not Be Added. No Active Configuration.");
                }
            }

            // Swtich between normal mode and debug mode
            SystemUpdate::DebugMode(mode) => {
                // Switch the mode (redraw triggered by the user interface)
                self.is_debug_mode = mode;
            }

            // Modify the underlying configuration
            SystemUpdate::Edit { mut modifications } => {
                // Check to see if there is an active configuration
                if let Some(mut handler) = self.event_handler.take() {
                    // Process each modification in order
                    for modification in modifications.drain(..) {
                        // Match the specified moficiation
                        match modification {
                            // Add or modify the item
                            Modification::ModifyItem {
                                item_pair,
                            } => {
                                // Pass the update and see if it's a new item
                                if self.index_access.update_description(item_pair.get_id(), item_pair.get_description()).await {
                                    update!(update &self.internal_send => "Item Description Updated: {}", item_pair.description());
                                
                                // If not, notify that the item was updated
                                } else {
                                    update!(update &self.internal_send => "Item Description Added: {}", item_pair.description());
                                }
                            }

                            // Add or modify the event
                            Modification::ModifyEvent {
                                item_id,
                                event,
                            } => {
                                handler.edit_event(item_id, event).await;
                            }

                            // Add or modify the status
                            Modification::ModifyStatus {
                                item_id,
                                status,
                            } => {
                                handler.edit_status(item_id, status).await;
                            }

                            // Add or modify the scene
                            Modification::ModifyScene {
                                item_id,
                                scene,
                            } => {
                                handler.edit_scene(item_id, scene).await;
                            }
                        }
                    }
                    
                    // Put the handler back
                    self.event_handler = Some(handler);

                // Raise a warning that there is no active configuration
                } else {
                    update!(warn &mut self.internal_send => "Change Not Saved: There Is No Active Configuration.");
                }
            }

            // Change the remaining delay for an existing event in the queue
            SystemUpdate::EventChange {
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
            SystemUpdate::ErrorLog { filepath } => self.logger.set_error_log(filepath),

            // Update the game log provided to the underlying system
            SystemUpdate::GameLog { filepath } => self.logger.set_game_log(filepath),

            // Pass an event to the event_handler
            SystemUpdate::ProcessEvent {
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
                            })
                            .unwrap_or(());
                    }
                    
                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise notify the user that a configuration faild to load
                } else {
                    update!(err &mut self.internal_send => "Event Could Not Be Processed. No Active Configuration.");
                }
            }

            // Redraw the current window
            SystemUpdate::Redraw => {
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
                    ).await;

                    // Get the current scene and key map
                    let current_scene = self.index_access.get_pair(&handler.get_current_scene()).await;
                    let key_map = handler.get_key_map().await;

                    // Send the update with the new event window
                    self.interface_send
                        .send(InterfaceUpdate::UpdateWindow {
                            current_scene,
                            window,
                            statuses,
                            key_map,
                        })
                        .unwrap_or(());
                }
            }

            // Reply to the request for information
            SystemUpdate::Request { reply_to, request } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Match the type of information request
                    match request {
                        // Reply to a request for the item description
                        RequestType::Description { item_id } => {
                            // Collect the description of the item
                            let description = self.index_access.get_description(&item_id).await;

                            // Create the item pair
                            let item_pair = ItemPair::from_item(item_id, description);

                            // Send it back to the user interface
                            self.interface_send
                                .send(Reply {
                                    reply_to, // echo to display component
                                    reply: ReplyType::Description { description: item_pair },
                                })
                                .unwrap_or(());
                        }

                        // Reply to a request for the event
                        RequestType::Event { item_id } => {
                            // Try to get the event
                            let event = handler.get_event(&item_id).await;

                            // Send an update with the event (or None)
                            self.interface_send
                                .send(Reply {
                                    reply_to, // echo the display component
                                    reply: ReplyType::Event { event },
                                })
                                .unwrap_or(());
                        }

                        // Reply to a request for all the configuration items
                        RequestType::Items => {
                            // Collect all the items from the configuration
                            let items = self.index_access.get_all().await;

                            // Send it back to the user interface
                            self.interface_send
                                .send(Reply {
                                    reply_to,
                                    reply: ReplyType::Items { items },
                                })
                                .unwrap_or(());
                        }

                        // Reply to a request for all the events in a scene
                        RequestType::Scene { item_id } => {
                            // Collect all the items from the configuration
                            let scene = handler.get_scene(item_id).await;

                            // Send it back to the user interface
                            self.interface_send
                                .send(Reply {
                                    reply_to,
                                    reply: ReplyType::Scene { scene },
                                })
                                .unwrap_or(());
                        }
                        // Reply to a request for the status
                        RequestType::Status { item_id } => {
                            // Try to get the status
                            let status = handler.get_status(&item_id);

                            // Send an update with the status (or None)
                            self.interface_send
                                .send(Reply {
                                    reply_to, // echo the display component
                                    reply: ReplyType::Status { status },
                                })
                                .unwrap_or(());
                        }
                    }
                    
                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise noity the user that a configuration failed to load
                } else {
                    update!(warn &mut self.internal_send => "Information Unavailable. No Active Configuration.");
                }
            }

            // Save the current configuration to the provided file
            SystemUpdate::SaveConfig { filepath } => {
                // Extract the current event handler (if it exists)
                if let Some(mut handler) = self.event_handler.take() {
                    // Save the current configuration
                    handler.save_config(filepath).await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }

            // Change the current scene based on the provided id and get a list of available events
            SystemUpdate::SceneChange { scene } => {
                // Change the current scene, if event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Change the current scene (automatically triggers a redraw)
                    handler.choose_scene(scene).await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }

            // Change the state of a particular status
            SystemUpdate::StatusChange { status_id, state } => {
                // Change the status, if event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Change the state of the indicated status
                    handler.modify_status(&status_id, &state).await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }
            
            // Trigger and event from the web FIXME mostly a duplicate of process event
            SystemUpdate::Web { reply_to, request } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Match the request type
                    match request {
                        // Cue an event
                        WebRequest::CueEvent { event_id } => {
                            // Try to process the event
                            if handler.process_event(&event_id, true, true).await {
                                // Notify the user of success
                                reply_to.send(WebReply::success()).unwrap_or(());

                            // Note if there was a failure
                            } else {
                                reply_to.send(WebReply::failure("Unable to cue the event.")).unwrap_or(());
                            }
                        }

                        // Ignore other cases
                        _ => {
                            reply_to.send(WebReply::failure("Other requests have not been implemented.")).unwrap_or(());
                        }
                    }
                    
                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise, notify the user of the failure
                } else {
                    reply_to.send(WebReply::failure("No active configuration.")).unwrap_or(());
                }
            }
        }
        true // indicate to continue
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
        ).await {
            Ok(evnt_hdlr) => evnt_hdlr,
            Err(_) => return, // errors will be logged separately if log_failure is true
        };

        // Create a new connection to the underlying system
        let system_connection = event_handler.system_connection();
        if !self.system_connection
            .update_system_connection(Some(system_connection)).await
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
            })
            .unwrap_or(());

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
