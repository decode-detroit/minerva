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

//! A module to create and monitor the user interface and the system inputs.
//! This module links directly to the event handler and sends any updates
//! to the application window.

// Reexport the key structures and types
pub use self::event_handler::event::{
    DataType, EventAction, EventDelay, Event, EventUpdate, UpcomingEvent,
};
pub use self::event_handler::{
    DescriptiveScene, FullStatus, KeyMap, Scene, StatusDescription, Status
};
pub use self::logging::{Current, Error, Logger, Notification, Update, Warning};
#[cfg(feature = "media-out")]
pub use self::system_connection::VideoStream;

// Define private submodules
#[macro_use]
mod test;
mod logging;
#[macro_use]
mod event_handler;
mod system_connection;

// Import the relevant structures into the correct namespace
use crate::definitions::{
    DisplayControl, DisplayDebug, DisplayType, DisplayWith, Hidden, ItemDescription,
    ItemId, ItemPair, LabelControl, LabelHidden,
};
use self::event_handler::{ComingEvent, EventHandler};
use self::system_connection::SystemConnection;

// Import standard library features
use std::env;
use std::fs::DirBuilder;
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant};

// Import tokio features
use tokio::sync::{mpsc, oneshot};

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
    interface_send: std_mpsc::Sender<InterfaceUpdate>, // a sending line to pass interface updates
    general_receive: mpsc::Receiver<GeneralUpdateType>, // a receiving line for all system updates
    general_update: GeneralUpdate, // a sending structure to pass new general updates
    is_debug_mode: bool,           // a flag to indicate debug mode
}

// Implement key SystemInterface functionality
impl SystemInterface {
    /// A function to create a new, blank instance of the system interface.
    ///
    pub async fn new(
        interface_send: std_mpsc::Sender<InterfaceUpdate>,
    ) -> Result<(SystemInterface, SystemSend), FailureError> {
        // Create the new general update structure and receive channel
        let (general_update, general_receive) = GeneralUpdate::new();

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
            general_update.clone(),
            interface_send.clone(),
        )?;

        // Create a new system connection instance
        let system_connection = SystemConnection::new(general_update.clone(), None).await;

        // Create the sytem send for the user interface
        let system_send = SystemSend::from_general(&general_update);

        // Create the new system interface instance
        let mut sys_interface = SystemInterface {
            event_handler: None,
            logger,
            system_connection,
            interface_send,
            general_receive,
            general_update: general_update,
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
        // Check for any of the updates
        match self.general_receive.recv().await {
            // Broadcast the event via the system connection
            Some(GeneralUpdateType::BroadcastEvent(event_id, data)) => {
                self.system_connection.broadcast(event_id, data).await;
            }

            // Update the timeline with the new list of coming events
            Some(GeneralUpdateType::ComingEvents(events)) => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Repackage the coming events into upcoming events
                    let upcoming_events = handler.repackage_events(events).await;

                    // Send the new events to the interface
                    self.interface_send
                        .send(UpdateTimeline {
                            events: upcoming_events,
                        })
                        .unwrap_or(());
                }
            }

            // Solicit a string from the user
            Some(GeneralUpdateType::GetUserString(event)) => {
                // Request the information from the user interface
                self.interface_send
                    .send(LaunchWindow {
                        window_type: WindowType::PromptString(event),
                    })
                    .unwrap_or(());
            }
            
            // Pass a video stream to the user interface
            #[cfg(feature = "media-out")]
            Some(GeneralUpdateType::NewVideo(video_stream)) => {
                // Pass the stream to the user interface
                self.interface_send
                    .send(LaunchWindow {
                        window_type: WindowType::Video(video_stream),
                    })
                    .unwrap_or(());
            }

            // Process the system update
            Some(GeneralUpdateType::System(update)) => {
                // Return the result of the update
                return self.unpack_system_update(update).await;
            }

            // Pass the information update to the logger
            Some(GeneralUpdateType::Update(event_update)) => {
                // Find the most recent notifications
                let notifications = self.logger.update(event_update).await;

                // Send a notification update to the system
                self.interface_send
                    .send(UpdateNotifications { notifications })
                    .unwrap_or(());
            }

            // Close on a communication error with the user interface thread
            _ => return false,
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

    /// An internal method to unpack system updates from the main program thread.
    ///
    /// When the update is the Close variant, the function will return false,
    /// indicating that the thread should close.
    ///
    async fn unpack_system_update(&mut self, update: SystemUpdate) -> bool {
        // Unpack the different variant types
        match update {
            // Change the delay for all events in the queue
            AllEventChange {
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
            AllStop => {
                // Try to clear all the events in the queue
                if let Some(mut handler) = self.event_handler.take() {
                    handler.clear_events().await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);
                }

                // Send the all stop event via the logger
                update!(broadcast &mut self.general_update => ItemPair::all_stop(), None);

                // Place an error in the debug log
                update!(err &mut self.general_update => "An All Stop was triggered by the operator.");

                // Notify the user interface of the event
                self.interface_send
                    .send(Notify {
                        message: "ALL STOP. Upcoming events have been cleared.".to_string(),
                    })
                    .unwrap_or(());
            }

            // Pass a broadcast event to the system connection (used only by
            // the user interface, not for internal messaging. See
            // GeneralUpdate::BroadcastEvent)
            BroadcastEvent { event, data } => {
                // Broadcast the event via the logger
                update!(broadcast &mut self.general_update => event.clone(), data);

                // Notify the user interface of the event
                self.interface_send
                    .send(Notify {
                        message: event.description,
                    })
                    .unwrap_or(());
            }

            // Clear the events currently in the queue
            ClearQueue => {
                // Try to clear all the events in the queue
                if let Some(mut handler) = self.event_handler.take() {
                    handler.clear_events().await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }

            // Close the system interface thread.
            Close => return false,

            // Update the configuration provided to the underlying system
            ConfigFile { filepath } => {
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

            // Swtich between normal mode and debug mode
            DebugMode(mode) => {
                // Switch the mode (redraw triggered by the user interface)
                self.is_debug_mode = mode;
            }

            // Modify the underlying configuration
            Edit { mut modifications } => {
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
                                handler.edit_item(item_pair).await;
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
                    update!(warn &mut self.general_update => "Change Not Saved: There Is No Active Configuration.");
                }
            }

            // Change the remaining delay for an existing event in the queue
            EventChange {
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
            ErrorLog { filepath } => self.logger.set_error_log(filepath),

            // Update the game log provided to the underlying system
            GameLog { filepath } => self.logger.set_game_log(filepath),

            // Pass an event to the event_handler
            ProcessEvent {
                event,
                check_scene,
                broadcast,
            } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Try to process the event
                    if handler.process_event(&event, check_scene, broadcast).await {
                        // Notify the user interface of the event
                        let description = handler.get_description(&event);
                        self.interface_send
                            .send(Notify {
                                message: description.description,
                            })
                            .unwrap_or(());
                    }
                    
                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise notify the user that a configuration faild to load
                } else {
                    update!(err &mut self.general_update => "Event Could Not Be Processed. No Active Configuration.");
                }
            }

            // Cue an event
            CueEvent { event_delay } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Cue the event
                    handler.add_event(event_delay).await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise noity the user that a configuration faild to load
                } else {
                    update!(err &mut self.general_update => "Event Could Not Be Added. No Active Configuration.");
                }
            }

            // Redraw the current window
            Redraw => {
                // Try to redraw the current window
                if let Some(ref mut handler) = self.event_handler {
                    // Compose the new event window and status items
                    let (window, statuses) = SystemInterface::sort_items(
                        handler.get_events(),
                        handler,
                        self.is_debug_mode,
                    );

                    // Send the update with the new event window
                    self.interface_send
                        .send(UpdateWindow {
                            current_scene: handler.get_current_scene(),
                            window,
                            statuses,
                            key_map: handler.get_key_map(),
                        })
                        .unwrap_or(());
                }
            }

            // Reply to the request for information
            Request { reply_to, request } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Match the type of information request
                    match request {
                        // Reply to a request for the item description
                        RequestType::Description { item_id } => {
                            // Collect the description of the item
                            let description = handler.get_description(&item_id);

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
                            let items = handler.get_items();

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
                            let scene = handler.get_scene(item_id);

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
                    update!(warn &mut self.general_update => "Information Unavailable. No Active Configuration.");
                }
            }

            // Save the current configuration to the provided file
            SaveConfig { filepath } => {
                // Extract the current event handler (if it exists)
                if let Some(mut handler) = self.event_handler.take() {
                    // Save the current configuration
                    handler.save_config(filepath).await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }

            // Change the current scene based on the provided id and get a list of available events
            SceneChange { scene } => {
                // Change the current scene, if event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Change the current scene (automatically triggers a redraw)
                    handler.choose_scene(scene).await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }

            // Change the state of a particular status
            StatusChange { status_id, state } => {
                // Change the status, if event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Change the state of the indicated status
                    handler.modify_status(&status_id, &state).await;
                    
                    // Put the handler back
                    self.event_handler = Some(handler);
                }
            }
            
            // Trigger and event from the web FIXME mostly a duplicate of process event
            WebRequest { item_id, reply_line } => {
                // If the event handler exists
                if let Some(mut handler) = self.event_handler.take() {
                    // Try to process the event
                    if handler.process_event(&item_id, true, true).await {
                        // Notify the user of success
                        reply_line.send(item_id).unwrap_or(());
                    }
                    
                    // Put the handler back
                    self.event_handler = Some(handler);

                // Otherwise, notify the user of the failure FIXME better reply format
                } else {
                    reply_line.send(ItemId::all_stop()).unwrap_or(());
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
    /// all other types of errors will be logged on the general_send line.
    ///
    async fn load_config(&mut self, filepath: PathBuf, log_failure: bool) {
        // Clone the interface send
        let interface_send = self.interface_send.clone();
        
        // Create a new event handler
        let mut event_handler = match EventHandler::new(
            filepath,
            self.general_update.clone(),
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

        // Send the newly available scenes and full status to the user interface
        self.interface_send
            .send(UpdateConfig {
                scenes: event_handler.get_scenes(),
                full_status: event_handler.get_full_status(),
            })
            .unwrap_or(());

        // Trigger a redraw of the system
        self.general_update.send_redraw().await;

        // Update the event handler
        self.event_handler = Some(event_handler);
    }

    /// An internal to sort the available events in this current scene
    /// into an Event Window.
    ///
    fn sort_items(
        mut items: Vec<ItemPair>,
        event_handler: &mut EventHandler,
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
                    let group_pair =
                        ItemPair::from_item(group_id, event_handler.get_description(&group_id));
                    SystemInterface::sort_groups(&mut groups, group_pair, item);
                }

                // Add display debug events to the matching event group
                DisplayDebug { group_id, .. } => {
                    // If the system is in debug mode
                    if is_debug_mode {
                        // If a group id is specified, add it to the correct group
                        if let Some(id) = group_id {
                            let group_pair =
                                ItemPair::from_item(id, event_handler.get_description(&id));
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

/// An private enum to provide and receive updates from the various internal
/// components of the system interface and external updates from the interface.
///
#[derive(Debug)]
enum GeneralUpdateType {
    /// A variant that broadcasts an event with the given item id. This event id
    /// is not processed or otherwise checked for validity. If data is provided,
    /// it will be broadcast with the event.
    BroadcastEvent(ItemId, Option<u32>),

    /// A variant that notifies the system of a change in the coming events
    ComingEvents(Vec<ComingEvent>),
    
    /// A variant that solicies a string of data from the user to send to the
    /// system. The string will be sent as a series of events with the same
    /// item id. TODO Make this more generic for other user input
    GetUserString(ItemPair),
    
    /// A variant to pass a new video stream to the user interface
    #[cfg(feature = "media-out")]
    NewVideo(Option<VideoStream>),

    /// A variant to notify the system of an update from the user interface
    System(SystemUpdate),

    /// A variant to notify the system of informational update
    Update(EventUpdate),
}

/// The public stucture and methods to send updates to the system interface.
///
#[derive(Clone, Debug)]
pub struct GeneralUpdate {
    general_send: mpsc::Sender<GeneralUpdateType>, // the mpsc sending line to pass updates to the system interface
}

// Implement the key features of the general update struct
impl GeneralUpdate {
    /// A function to create the new General Update structure.
    ///
    /// The function returns the the General Update structure and the general
    /// receive channel which will return the provided updates.
    ///
    fn new() -> (GeneralUpdate, mpsc::Receiver<GeneralUpdateType>) {
        // Create the new channel
        let (general_send, receive) = mpsc::channel(128);

        // Create and return both new items
        (GeneralUpdate { general_send }, receive)
    }

    /// A method to broadcast an event via the system interface (with data,
    /// if it is provided)
    ///
    async fn send_broadcast(&self, event_id: ItemId, data: Option<u32>) {
        self.general_send.clone()
            .send(GeneralUpdateType::BroadcastEvent(event_id, data))
            .await.unwrap_or(());
    }

    /// A method to send new coming events to the system
    ///
    async fn send_coming_events(&self, coming_events: Vec<ComingEvent>) {
        self.general_send.clone()
            .send(GeneralUpdateType::ComingEvents(coming_events))
            .await.unwrap_or(());
    }

    /// A method to process a new event. If the check_scene flag is not set,
    /// the system will not check if the event is in the current scene. If
    /// broadcast is set to true, the event will be broadcast to the system.
    ///
    async fn send_event(&self, event: ItemId, check_scene: bool, broadcast: bool) {
        self.send_system(ProcessEvent {
            event,
            check_scene,
            broadcast,
        }).await;
    }

    /// A method to request a string from the user FIXME make this more generic
    /// for other types of data
    ///
    async fn send_get_user_string(&self, event: ItemPair) {
        self.general_send.clone()
            .send(GeneralUpdateType::GetUserString(event))
            .await.unwrap_or(());
    }
     
    /// A method to pass a new video stream to the user interface
    ///
    #[cfg(feature = "media-out")]
    async fn send_new_video(&self, video_stream: VideoStream) {
        self.general_send.clone()
            .send(GeneralUpdateType::NewVideo(Some(video_stream)))
            .await.unwrap_or(());
    }

    /// A method to clear all video streams from the user interface
    ///
    #[cfg(feature = "media-out")]
    async fn send_clear_videos(&self) {
        self.general_send.clone()
            .send(GeneralUpdateType::NewVideo(None))
            .await.unwrap_or(());
    }

    /// A method to trigger a redraw of the current window
    ///
    async fn send_redraw(&self) {
        self.send_system(Redraw).await;
    }

    /// A method to pass a system update to the system interface.
    ///
    async fn send_system(&self, update: SystemUpdate) {
        self.general_send.clone()
            .send(GeneralUpdateType::System(update))
            .await.unwrap_or(());
    }

    /// A method to send an event update to the system interface.
    ///
    async fn send_update(&self, update: EventUpdate) {
        self.general_send.clone()
            .send(GeneralUpdateType::Update(update))
            .await.unwrap_or(());
    }
}

/// A special, public version of the general update which only allows for a
/// system send (without other types of updates).
///
#[derive(Clone, Debug)]
pub struct SystemSend {
    general_send: mpsc::Sender<GeneralUpdateType>, // the mpsc sending line to pass system updates to the interface
}

// Implement the key features of the system send struct
impl SystemSend {
    /// A function to create a new system send from a general update.
    ///
    fn from_general(general_update: &GeneralUpdate) -> SystemSend {
        SystemSend {
            general_send: general_update.general_send.clone(),
        }
    }

    /// A method to send a system update. This version of the method fails
    /// silently.
    ///
    pub async fn send(&self, update: SystemUpdate) {
        self.general_send.send(GeneralUpdateType::System(update)).await.unwrap_or(());
    }

    /// A method to send a system update in a sync setting. This method fails
    /// silently.
    ///
    pub fn blocking_send(&self, update: SystemUpdate) {
        self.general_send.blocking_send(GeneralUpdateType::System(update)).unwrap_or(());
    }
}

/// A special, public version of the general update which only allows for a
/// system send (without other types of updates).
///
/// # Note
///
/// This version is depreciated. It should not be perpetuated.
///
#[derive(Clone, Debug)]
pub struct SyncSystemSend {
    system_send: SystemSend, // the mpsc sending line to pass system updates to the interface
}

// Implement the key features of the system send struct
impl SyncSystemSend {
    /// A function to create a new sync system send from a system send
    ///
    pub fn from_async(system_send: &SystemSend) -> SyncSystemSend {
        // Return the completed SyncSystemSend
        SyncSystemSend {
            system_send: system_send.clone(),
        }
    }

    /// A method to send a system update. This version of the method fails
    /// silently.
    ///
    pub fn send(&self, update: SystemUpdate) {
        // Send the message
        self.system_send.blocking_send(update);
    }
}

/// An enum to execute one modification to the configuration
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Modification {
    /// A modification to add an item or modify an existing one
    ModifyItem {
        item_pair: ItemPair,
    },

    /// A modification to add an event, modify an existing one, or delete it
    /// (if None provided)
    ModifyEvent {
        item_id: ItemId,
        event: Option<Event>,
    },

    /// A modification to add a status, modify an existing one, or delete it
    /// (if None provided)
    ModifyStatus {
        item_id: ItemId,
        status: Option<Status>,
    },

    /// A modification to add a scene, modify an existing one, or delete it
    /// (if None provided)
    ModifyScene {
        item_id: ItemId,
        scene: Option<Scene>,
    },
}

/// An enum to specify the type of information request
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestType {
    /// A variant for the description of an item
    Description { item_id: ItemId },

    /// A variant for the event associated with an item
    Event { item_id: ItemId },

    /// A variant for the status associated with an item
    Status { item_id: ItemId },

    /// A variant for the list of all events in a scene
    Scene { item_id: ItemId },

    /// A variant for the list of all items
    Items,
}

/// An enum to specify which Edit Action subcomponent has requested the information
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditActionElement {
    /// A variant for the edit new scene action
    EditNewScene,

    /// A variant for the edit modify status
    EditModifyStatus { is_status: bool },

    /// A variant for the edit cue event
    EditCueEvent,

    /// A variant for the edit cancel event
    EditCancelEvent,

    /// A variant for the edit save data
    EditSaveData,

    /// A variant for the edit send data
    EditSendData,

    /// A variant for the select event status description
    SelectEventDescription { position: Option<usize>, is_event: bool },

    /// A variant for the select event states
    SelectEventStates,
}

/// An enum to specify which Edit Item subcomponent has requested the information
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditItemElement {
    /// A variant for the item description
    ItemDescription,

    /// A variant for the group field
    Group,

    /// A variant for the status field
    Status { state: Option<ItemId> },

    /// A variant for the state dropdown
    State,

    /// A variant for the different edit item details
    Details,
}

/// An enum to specify which display component has requested the information
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisplayComponent {
    /// A variant for the edit item window
    EditItemOverview { is_left: bool, variant: EditItemElement },

    /// A variant for the edit action element
    EditActionElement { is_left: bool, variant: EditActionElement},

    /// A variant for the edit multistate status element
    EditMultiStateStatus { is_left: bool, position: Option<usize> },

    /// A variant for the edit counted state status element
    EditCountedStateStatus { is_left: bool, state_type: String },

    /// A variant for the item list panel
    ItemList,

    /// A variant for the trigger dialog
    TriggerDialog,
}

/// An enum to provide updates from the main thread to the system interface,
/// listed in order of increasing usage.
///
#[derive(Debug)]
pub enum SystemUpdate {
    /// A variant to adjust all the events in the timeline
    /// NOTE: after the adjustment, events that would have already happened are discarded
    AllEventChange {
        adjustment: Duration, // the amount of time to add to (or subtract from) all events
        is_negative: bool,    // a flag to indicate if the delay should be subtracted
    },

    /// A special variant to send the "all stop" event which automatically
    /// is broadcast immediately and clears the event queue.
    AllStop,

    /// A variant that broadcasts an event with the given item id. This event id
    /// is not processed or otherwise checked for validity. If data is provided
    /// it will be broadcast with the event.
    BroadcastEvent { event: ItemPair, data: Option<u32> },

    /// A variant to trigger all the queued events to clear
    ClearQueue,

    /// A special variant to close the program and unload all the data.
    Close,

    /// A variant that provides a new configuration file for the system interface.
    /// If None is provided as the filepath, no configuration will be loaded.
    ConfigFile { filepath: Option<PathBuf> },

    /// A special variant to switch to or from debug mode for the program.
    DebugMode(bool),

    /// A variant to modify the underlying configuration
    Edit { modifications: Vec<Modification> },

    /// A variant that provides a new error log file for the system interface.
    ErrorLog { filepath: PathBuf },

    /// A variant to change the remaining delay for an existing event in the
    /// queue.
    EventChange {
        event_id: ItemId,
        start_time: Instant, // the start time of the event, for unambiguous identification
        new_delay: Option<Duration>, // new delay relative to the original start time, or None to cancel the event
    },

    /// A variant that provides a new game log file for the system interface.
    GameLog { filepath: PathBuf },

    /// A variant that processes a new event with the given item id. If the
    /// check_scene flag is not set, the system will not check if the event is
    /// listed in the current scene. If broadcast is set to true, the event
    /// will be broadcast to the system
    ProcessEvent {
        event: ItemId,
        check_scene: bool,
        broadcast: bool,
    },

    /// A variant that queues a new event with the given item id. The event
    /// will trigger after the specified delay has passed.
    CueEvent { event_delay: EventDelay },

    /// A variant that triggers a redraw of the user interface window
    Redraw,

    /// A variant that requests information from the system and directs it
    /// to a specific spot on the window
    Request {
        reply_to: DisplayComponent,
        request: RequestType,
    },

    /// A variant that provides a new configuration file to save the current
    /// configuration.
    SaveConfig { filepath: PathBuf },

    /// A variant to change the selected scene provided by the user interface.
    SceneChange { scene: ItemId },

    /// A variant to change the state of the indicated status.
    StatusChange { status_id: ItemId, state: ItemId },
    
    /// A variant for web requests FIXME standardize this
    WebRequest { item_id: ItemId, reply_line: oneshot::Sender<ItemId> },
}

// Reexport the system update type variants
pub use self::SystemUpdate::{
    AllEventChange, AllStop, BroadcastEvent, ClearQueue, Close, ConfigFile, DebugMode, Edit,
    ErrorLog, EventChange, GameLog, ProcessEvent, CueEvent, Redraw, Request, SaveConfig,
    SceneChange, StatusChange, WebRequest,
};

/// A structure to list a series of event buttons that are associated with one
/// event group.
///
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct EventGroup {
    pub group_id: Option<ItemPair>, // the group id identifying and describing the group or None for the general group
    pub group_events: Vec<ItemPair>, // a vector of the events that belong in this group
}

/// A type to list a series of event groups that fill the event window.
///
pub type EventWindow = Vec<EventGroup>; // a vector of event groups that belong in this window

/// An enum to launch one of the special windows for the user interface
///
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum WindowType {
    /// A variant to launch the jump dialog with an optional scene of interest
    Jump(Option<ItemPair>),
    
    /// A variant to solicit a string from the user. The string will be sent as
    /// a series of events to the system
    PromptString(ItemPair),
    
    /// A variant to show the shortcuts window
    Shortcuts,
    
    /// A variant to launch the status dialog with an optional relevant status of interest
    Status(Option<ItemPair>),
    
    /// A variant to launch the trigger dialog with an optional event of interest
    Trigger(Option<ItemPair>),
    
    /// A variant to launch a video window with a source from the video system connection
    #[cfg(feature = "media-out")]
    Video(Option<VideoStream>),
}

/// An enum to change one of the display settings of the user interface
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum DisplaySetting {
    /// A variant to change the fullscreen mode of the display
    FullScreen(bool),

    /// A variant to change the debug mode of the display
    DebugMode(bool),

    /// A variant to change the edit mode of the display
    // FIXME EditMode(bool),

    /// A variant to change the font size of the display
    LargeFont(bool),

    /// A variant to change the color mode of the display
    HighContrast(bool),
}

/// An enum to specify the type of information reply
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyType {
    /// A variant for the description of an item
    Description { description: ItemPair },

    /// A variant for the event associated with an item
    Event { event: Option<Event> },

    /// A variant for the status associated with an item
    Status { status: Option<Status> },

    /// A variant for the list of events in a scene
    Scene { scene: Option<DescriptiveScene> },

    /// A variant for the list of item pairs
    Items { items: Vec<ItemPair> },
}

/// An enum type to provide interface updates back to the user interface thread.
///
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum InterfaceUpdate {
    /// A variant to change the display settings
    ChangeSettings { display_setting: DisplaySetting },

    /// A variant to switch the interface to or from edit mode
    EditMode(bool),

    /// A variant to launch one of the special windows
    LaunchWindow { window_type: WindowType },

    /// A variant to post a current event to the status bar
    Notify { message: String },

    /// A variant to reply to an information request from the user interface
    Reply {
        reply_to: DisplayComponent,
        reply: ReplyType,
    },

    /// A variant to update the available scenes and full status in the main
    /// program window.
    UpdateConfig {
        scenes: Vec<ItemPair>,
        full_status: FullStatus,
    },

    /// A variant indicating the entire button window should be refreshed with
    /// the new provided window.
    UpdateWindow {
        current_scene: ItemPair,
        statuses: Vec<ItemPair>,
        window: EventWindow,
        key_map: KeyMap,
    },

    /// A variant to update the state of a partiular status.
    UpdateStatus {
        status_id: ItemPair, // the group to update
        new_state: ItemPair, // the new state of the group
    },

    /// A variant indicating that the system notifications should be updated.
    UpdateNotifications { notifications: Vec<Notification> },

    /// A variant indicating that the event timeline should be updated.
    UpdateTimeline { events: Vec<UpcomingEvent> },
}

// Reexport the interface update type variants
pub use self::InterfaceUpdate::{
    ChangeSettings, EditMode, LaunchWindow, Notify, Reply, UpdateConfig, UpdateNotifications,
    UpdateStatus, UpdateTimeline, UpdateWindow,
};

// Tests of the system_interface module
#[cfg(test)]
mod tests {
    use super::*;

    // FIXME Define tests of this module
    #[test]
    fn test_system_interface() {
        // FIXME: Implement this
        unimplemented!();
    }
}
