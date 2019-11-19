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
pub use self::event_handler::event::{EventDelay, EventDetail, EventUpdate, UpcomingEvent};
pub use self::event_handler::item::{
    DisplayControl, DisplayDebug, DisplayType, DisplayWith, Hidden, ItemDescription, ItemId,
    ItemPair, LabelControl, LabelHidden,
};
pub use self::event_handler::{FullStatus, StatusDescription};
pub use self::logging::{Current, Error, Logger, Notification, Update, Warning};

// Define private submodules
#[macro_use]
mod test;
mod logging;
#[macro_use]
mod event_handler;
mod system_connection;

// Import the relevant structures into the correct namespace
use self::event_handler::EventHandler;
use self::system_connection::SystemConnection;

// Import standard library features
use std::env;
use std::fs::DirBuilder;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

// Import the failure features
use failure::Error as FailureError;

// Import GTK library
extern crate gtk;

// Define module constants
const POLLING_RATE: u64 = 1; // the polling rate for the system in ms
const DEFAULT_FILE: &str = "default.mnv"; // the default configuration filename
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
    interface_send: mpsc::Sender<InterfaceUpdate>, // a sending line to pass interface updates to the main program
    general_receive: mpsc::Receiver<GeneralUpdateType>, // a receiving line for all system updates
    general_update: GeneralUpdate, // a sending structure to pass new general updates
    is_debug_mode: bool,           // a flag to indicate debug mode
    is_edit_mode: bool,            // a flag to indicate edit mode
}

// Implement key SystemInterface functionality
impl SystemInterface {
    /// A function to create a new, blank instance of the system interface.
    ///
    pub fn new(
        interface_send: mpsc::Sender<InterfaceUpdate>,
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
        let system_connection = SystemConnection::new(general_update.clone(), None);

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
            is_edit_mode: false,
        };

        // Try to load a default configuration, if it exists
        if let Ok(mut path) = env::current_dir() {
            path.push(DEFAULT_FILE); // append the default filename
            sys_interface.load_config(path, false);
        }

        // Regardless, return the new SystemInterface and general send line
        Ok((sys_interface, system_send))
    }

    /// A method to run one iteration of the system interface to update the user
    /// and underlying system of any event changes.
    ///
    pub fn run_once(&mut self) -> bool {
        // Check for any of the updates
        match self.general_receive.recv() {
            // If an event to process was recieved, pass it to the event handler
            Ok(GeneralUpdateType::Event(event_id, checkscene)) => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Process the event
                    handler.process_event(&event_id, checkscene, true);

                    // Notify the user interface of the event
                    let description = handler.get_description(&event_id);
                    self.interface_send
                        .send(Notify {
                            message: format!("Event: {}", description.description),
                        })
                        .unwrap_or(());

                    // Wait 20 nanoseconds for the queued events to process
                    thread::sleep(Duration::new(0, 20));

                    // If the upcoming events have changed, send an update
                    if let Some(events) = handler.upcoming_events() {
                        self.interface_send
                            .send(UpdateQueue { events })
                            .unwrap_or(());
                    }

                // Otherwise noity the user that a configuration faild to load
                } else {
                    update!(err &self.general_update => "Event Could Not Be Processed. No Active Configuration.");
                }
            }

            // If a no broadcast event was recieved, pass it to the event handler
            Ok(GeneralUpdateType::NoBroadcastEvent(event_id)) => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Process the event
                    handler.process_event(&event_id, true, false); // checkscene, no broadcast

                    // Notify the user interface of the event
                    let description = handler.get_description(&event_id);
                    self.interface_send
                        .send(Notify {
                            message: format!("Event: {}", description.description),
                        })
                        .unwrap_or(());

                    // Wait 10 nanoseconds for the queued events to process
                    thread::sleep(Duration::new(0, 10));

                    // If the upcoming events have changed, send an update
                    if let Some(events) = handler.upcoming_events() {
                        self.interface_send
                            .send(UpdateQueue { events })
                            .unwrap_or(());
                    }
                }
            }

            // If an event update was recieved, pass it to the logger
            Ok(GeneralUpdateType::Update(event_update)) => {
                // Find the most recent notifications
                let notifications = self.logger.update(event_update);

                // Send a notification update to the system
                self.interface_send
                    .send(UpdateNotifications { notifications })
                    .unwrap_or(());
            }

            // If a broadcast event was recieved, send it to the system connection
            Ok(GeneralUpdateType::Broadcast(event_id)) => {
                self.system_connection.broadcast(event_id, None);
            }

            // If a broadcast data event was received, send it to the system connection
            Ok(GeneralUpdateType::BroadcastData(event_id, data)) => {
                self.system_connection.broadcast(event_id, Some(data));
            }

            // If a system update was recieved, process the update
            Ok(GeneralUpdateType::System(update)) => {
                // Return the result of the update
                return self.unpack_system_update(update);
            }

            // If a redraw command was triggered, redraw the current window
            Ok(GeneralUpdateType::Redraw) => {
                // Try to redraw the current window
                if let Some(ref mut handler) = self.event_handler {
                    // Compose the new event window and status items
                    let (window, statuses) = SystemInterface::sort_items(
                        handler.get_items(),
                        handler,
                        self.is_debug_mode,
                    );

                    // Send the update with the new event window
                    self.interface_send
                        .send(UpdateWindow {
                            current_scene: handler.get_current_scene(),
                            window,
                            statuses,
                        })
                        .unwrap_or(());

                    // If the upcoming events have changed, send an update
                    if let Some(events) = handler.upcoming_events() {
                        self.interface_send
                            .send(UpdateQueue { events })
                            .unwrap_or(());
                    }
                }
            }

            // Close on a communication error with the user interface thread
            Err(mpsc::RecvError) => return false,
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
    pub fn run(mut self) {
        // Loop the structure indefinitely
        loop {
            // Repeat endlessly until run_once fails.
            if !self.run_once() {
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
    fn unpack_system_update(&mut self, update: SystemUpdate) -> bool {
        // Unpack the different variant types
        match update {
            // Close the system interface thread.
            Close => return false,

            // Handle the All Stop command which clears the queue and sends the "all stop" (a.k.a. emergency stop) command.
            AllStop => {
                // Try to clear all the events in the queue
                if let Some(ref mut handler) = self.event_handler {
                    handler.clear_events();

                    // Wait 10 nanoseconds for the queued events to process
                    thread::sleep(Duration::new(0, 10));

                    // Send an update that there are no upcoming events
                    if let Some(events) = handler.upcoming_events() {
                        self.interface_send
                            .send(UpdateQueue { events })
                            .unwrap_or(());
                    }
                }

                // Send the all stop command directly to the system
                self.general_update
                    .send_update(EventUpdate::Broadcast(ItemPair::all_stop()));
            }

            // Swtich between normal mode and debug mode
            DebugMode(mode) => {
                // Switch the mode (redraw triggered by the user interface)
                self.is_debug_mode = mode;
            }

            // Switch between run mode and edit mode
            EditMode(mode) => {
                self.is_edit_mode = mode;
            }

            // Modify or add the event detail
            EditDetail {
                event_pair,
                event_detail,
            } => {
                // Create/modify the new event detail in the configuration
                if let Some(ref mut handler) = self.event_handler {
                    // Add/modify the event detail and the description
                    handler.edit_event(&event_pair, &event_detail);

                    // Trigger a redraw of the user interface
                    self.general_update.send_redraw();

                // Raise a warning that there is no current configuration
                } else {
                    update!(warn &self.general_update => "Change Not Saved: There Is No Current Configuration.");
                }
            }

            // Update the configuration provided to the underlying system
            ConfigFile { filepath } => {
                // Try to clear all the events in the queue
                if let Some(ref mut handler) = self.event_handler {
                    handler.clear_events();

                    // Wait 10 nanoseconds for the queued events to process
                    thread::sleep(Duration::new(0, 10));

                    // Send an update that there are no upcoming events
                    if let Some(events) = handler.upcoming_events() {
                        self.interface_send
                            .send(UpdateQueue { events })
                            .unwrap_or(());
                    }
                }

                // Drop the old event handler
                self.event_handler = None;

                // Check to see if a new filepath was specified
                if let Some(path) = filepath {
                    // If so, try to load it
                    self.load_config(path, true);
                }
            }

            // Save the current configuration to the provided file
            SaveConfig { filepath } => {
                // Extract the current event handler (if it exists)
                if let Some(ref handler) = self.event_handler {
                    // Save the current configuration
                    handler.save_config(filepath);
                }
            }

            // Update the game log provided to the underlying system
            GameLog { filepath } => self.logger.set_game_log(filepath),

            // Update the system log provided to the underlying system
            ErrorLog { filepath } => self.logger.set_error_log(filepath),

            // Change the current scene based on the provided id and get a list of available events
            SceneChange { scene } => {
                // Change the current scene, if event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Change the current scene (automatically triggers a redraw)
                    handler.choose_scene(scene);
                }
            }

            // Change the state of a particular status
            StatusChange { status_id, state } => {
                // Change the status, if event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Change the state of the indicated status
                    handler.modify_status(&status_id, &state);
                }
            }

            // Change the remaining delay for an existing event in the queue
            EventChange {
                event_id,
                start_time,
                new_delay,
            } => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Adjust the current time of the event
                    handler.adjust_event(event_id, start_time, new_delay);

                    // Wait 10 nanoseconds for the queued events to process
                    thread::sleep(Duration::new(0, 10));

                    // If the upcoming events have changed, send an update
                    if let Some(events) = handler.upcoming_events() {
                        self.interface_send
                            .send(UpdateQueue { events })
                            .unwrap_or(());
                    }
                }
            }

            // Change the delay for all events in the queue
            AllEventChange {
                adjustment,
                is_negative,
            } => {
                // If the event handler exists
                if let Some(ref mut handler) = self.event_handler {
                    // Adjust the current time of the event
                    handler.adjust_all_events(adjustment, is_negative);

                    // Wait 10 nanoseconds for the queued events to process
                    thread::sleep(Duration::new(0, 10));

                    // If the upcoming events have changed, send an update
                    if let Some(events) = handler.upcoming_events() {
                        self.interface_send
                            .send(UpdateQueue { events })
                            .unwrap_or(());
                    }
                }
            }

            // Clear the events currently in the queue
            ClearQueue => {
                // Try to clear all the events in the queue
                if let Some(ref mut handler) = self.event_handler {
                    handler.clear_events();

                    // Wait 10 nanoseconds for the queued events to process
                    thread::sleep(Duration::new(0, 10));

                    // Send an update that there are no upcoming events
                    if let Some(events) = handler.upcoming_events() {
                        self.interface_send
                            .send(UpdateQueue { events })
                            .unwrap_or(());
                    }
                }
            }

            // Pass a triggered event to the event_handler, if it exists
            TriggerEvent { event, checkscene } => {
                // Operate normally when not in edit mode
                if !self.is_edit_mode {
                    self.general_update.send_event(event, checkscene);

                // Return the event detail if in edit mode
                } else {
                    if let Some(ref mut handler) = self.event_handler {
                        // Try to get the event detail
                        if let Some(detail) = handler.get_detail(&event) {
                            // Send an update with the current event detail
                            self.interface_send
                                .send(DetailToModify {
                                    event_id: ItemPair::from_item(
                                        event.clone(),
                                        handler.get_description(&event),
                                    ),
                                    event_detail: detail,
                                })
                                .unwrap_or(());

                        // Raise a warning for the system if the detail was not found
                        } else {
                            update!(warn &self.general_update => "Unable To Find Detail For Event: {}", event);
                        }
                    }
                }
            }

            // Redraw the user interface
            Redraw => self.general_update.send_redraw(),
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
    fn load_config(&mut self, filepath: PathBuf, log_failure: bool) {
        // Create a new event handler
        let mut event_handler =
            match EventHandler::new(filepath, self.general_update.clone(), log_failure) {
                Ok(evnt_hdlr) => evnt_hdlr,
                Err(_) => return, // errors will be logged separately if log_failure is true
            };

        // Create a new connection to the underlying system
        if !self
            .system_connection
            .update_system_connection(Some(event_handler.system_connection()))
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
        self.general_update.send_redraw();

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
#[derive(Clone, Debug, PartialEq, Eq)]
enum GeneralUpdateType {
    /// A variant for the event type
    Event(ItemId, bool),

    /// A variant for the no broadcast event type
    NoBroadcastEvent(ItemId),

    /// A variant for the update type
    Update(EventUpdate),

    /// A variant for the broadcast type
    Broadcast(ItemId),

    /// A variant for the broadcast data type
    BroadcastData(ItemId, u32),

    /// A variant for the system update type
    System(SystemUpdate),

    /// A variant to trigger an internal redraw of the window
    Redraw,
}

/// The public stucture and methods to send updates to the system interface.
///
#[derive(Clone, Debug)]
pub struct GeneralUpdate {
    general_send: mpsc::Sender<GeneralUpdateType>, // the mpsc sending line to pass updates to the system interface
}

// Implement the key features of the general update struct
// FIXME Consider moving this and the Interface update type to "neutral territory"
impl GeneralUpdate {
    /// A function to create the new General Update structure.
    ///
    /// The function returns the the General Update structure and the general
    /// receive channel which will return the provided updates.
    ///
    fn new() -> (GeneralUpdate, mpsc::Receiver<GeneralUpdateType>) {
        // Create the new channel
        let (general_send, receive) = mpsc::channel();

        // Create and return both new items
        (GeneralUpdate { general_send }, receive)
    }

    /// A method to trigger a new event in the system interface. If the
    /// checkscene flag is not set, the system will not check if the event
    /// is in the current scene.
    ///
    pub fn send_event(&self, event_id: ItemId, checkscene: bool) {
        self.general_send
            .send(GeneralUpdateType::Event(event_id, checkscene))
            .unwrap_or(());
    }

    /// A method to trigger a new no broadcast event in the system interface.
    ///
    pub fn send_nobroadcast(&self, event_id: ItemId) {
        self.general_send
            .send(GeneralUpdateType::NoBroadcastEvent(event_id))
            .unwrap_or(());
    }

    /// A method to send an event update to the system interface.
    ///
    pub fn send_update(&self, update: EventUpdate) {
        self.general_send
            .send(GeneralUpdateType::Update(update))
            .unwrap_or(());
    }

    /// A method to broadcast an event via the system interface.
    ///
    pub fn send_broadcast(&self, event_id: ItemId) {
        self.general_send
            .send(GeneralUpdateType::Broadcast(event_id))
            .unwrap_or(());
    }

    /// A method to broadcast an event and corresponding data via the system interface.
    ///
    pub fn send_broadcast_data(&self, event_id: ItemId, data: u32) {
        self.general_send
            .send(GeneralUpdateType::BroadcastData(event_id, data))
            .unwrap_or(());
    }

    /// A method to pass a system update to the system interface.
    ///
    pub fn send_system(&self, update: SystemUpdate) {
        self.general_send
            .send(GeneralUpdateType::System(update))
            .unwrap_or(());
    }

    /// A method to trigger a redraw of the current window
    ///
    pub fn send_redraw(&self) {
        self.general_send
            .send(GeneralUpdateType::Redraw)
            .unwrap_or(());
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
    pub fn send(&self, update: SystemUpdate) {
        self.general_send
            .send(GeneralUpdateType::System(update))
            .unwrap_or(());
    }
}

/// An enum to provide updates from the main thread to the system interface,
/// listed in order of increasing usage.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemUpdate {
    /// A special variant to send the "all stop" event which automatically
    /// is broadcast immediately and clears the event queue.
    AllStop,

    /// A special variant to close the program and unload all the data.
    Close,

    /// A special variant to switch to or from debug mode for the program.
    DebugMode(bool),

    /// A special variant to switch to or from editing mode for the program.
    EditMode(bool),

    /// A variant to modify or add the detail of the indicated event
    EditDetail {
        event_pair: ItemPair,
        event_detail: EventDetail,
    },

    /// A variant that provides a new configuration file for the system interface.
    /// If None is provided as the filepath, no configuration will be loaded.
    /// This is the correct way to deconstruct the system before shutting down.
    ConfigFile { filepath: Option<PathBuf> },

    /// A variant that provides a new configuration file to save the current
    /// configuration.
    SaveConfig { filepath: PathBuf },

    /// A variant that provides a new game log file for the system interface.
    GameLog { filepath: PathBuf },

    /// A variant that provides a new error log file for the system interface.
    ErrorLog { filepath: PathBuf },

    /// A variant to change the selected scene provided by the user interface.
    SceneChange { scene: ItemId },

    /// A variant to change the state of the indicated status.
    StatusChange { status_id: ItemId, state: ItemId },

    /// A variant to change the remaining delay for an existing event in the
    /// queue.
    EventChange {
        event_id: ItemId,
        start_time: Instant, // the start time of the event, for unambiguous identification
        new_delay: Option<Duration>, // new delay relative to the original start time, or None to cancel the event
    },

    /// A variant to adjust all the events in the timeline
    /// NOTE: after the adjustment, events that would have already happened are discarded
    AllEventChange {
        adjustment: Duration, // the amount of time to add to (or subtract from) all events
        is_negative: bool, // a flag to indicate if the delay should be subtracted from existing events
    },

    /// A variant to trigger all the queued events to clear
    ClearQueue,

    /// A variant that triggers a new event with the given item id. If the
    /// checkscene flag is no set, the system will not check if the event is
    /// listed in the current scene.
    TriggerEvent { event: ItemId, checkscene: bool },

    /// A variant that triggers a redraw of the current event window
    Redraw,
}

// Reexport the system update type variants
pub use self::SystemUpdate::{
    AllEventChange, AllStop, ClearQueue, Close, ConfigFile, DebugMode, EditDetail, EditMode,
    ErrorLog, EventChange, GameLog, Redraw, SaveConfig, SceneChange, StatusChange,
    TriggerEvent,
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
    /// A variant to launch the status dialog with an optional relevant status of interest
    Status(Option<ItemPair>),

    /// A variant to launch the jump dialog with an optional scene of interest
    Jump(Option<ItemPair>),

    /// A variant to launch the trigger dialog with an optional event of interest
    Trigger(Option<ItemPair>),
}

/// An enum to change one of the display settings of the user interface
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum DisplaySetting {
    /// A variant to change the fullscreen mode of the display
    FullScreen(bool),
    
    /// A variant to change the debug mode of the display
    DebugMode(bool),

    /// A variant to change the font size of the display
    LargeFont(bool),

    /// A variant to change the color mode of the display
    HighContrast(bool),
}

/// An enum type to provide interface updates back to the user interface thread.
///
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum InterfaceUpdate {
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
    },

    /// A variant to update the state of a partiular status.
    UpdateStatus {
        status_id: ItemPair, // the group to update
        new_state: ItemPair, // the new state of the group
    },

    /// A variant indicating that the system notifications should be updated.
    UpdateNotifications { notifications: Vec<Notification> },

    /// A variant indicating that the event queue should be updated.
    UpdateQueue { events: Vec<UpcomingEvent> },

    /// A variant to launch one of the special windows
    LaunchWindow { window_type: WindowType },

    /// A variant to change the display settings
    ChangeSettings { display_setting: DisplaySetting },

    /// A variant to post a current event to the status bar
    Notify { message: String },

    /// A variant to provide the current detail of an event to allow modification
    /// as desired.
    DetailToModify {
        event_id: ItemPair,
        event_detail: EventDetail,
    },
}

// Reexport the interface update type variants
pub use self::InterfaceUpdate::{
    ChangeSettings, DetailToModify, LaunchWindow, Notify, UpdateConfig,
    UpdateNotifications, UpdateQueue, UpdateStatus, UpdateWindow,
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
