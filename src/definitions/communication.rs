// Copyright (c) 2019-2021 Decode Detroit
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

//! This module implements shared communication structures for communicating
//! across the modules of the system.

// Reexport the key structures and types
use crate::definitions::{
    ItemId, ItemPair, EventDelay, Event, EventUpdate, UpcomingEvent, DescriptiveScene,
    FullStatus, KeyMap, Scene, Status
};
#[cfg(feature = "media-out")]
pub use self::system_connection::VideoStream;


// Import standard library features
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;


/// A struct to allow easier manipulation of coming events.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ComingEvent {
    pub start_time: Instant, // the original start time of the event
    pub delay: Duration,     // delay between the start time and the trigger time for the event
    pub event_id: ItemId,    // id of the event to launch
}

// Implement the Coming Event features
impl ComingEvent {
    /// A function to return a new ComingEvent by consuming Duration and
    /// ItemId.
    ///
    pub fn new(delay: Duration, event_id: ItemId) -> ComingEvent {
        ComingEvent {
            start_time: Instant::now(),
            delay,
            event_id,
        }
    }

    /// A method to return a copy of the event id.
    ///
    pub fn id(&self) -> ItemId {
        self.event_id.clone()
    }

    /// A method to calculate the amount of time remaining before the event
    /// triggers. Returns None if the event should already have occured.
    ///
    pub fn remaining(&self) -> Option<Duration> {
        self.delay.checked_sub(self.start_time.elapsed())
    }

    /// A method to compare the start time and event id of two coming events.
    /// The method returns true iff both values are equal.
    ///
    pub fn compare_with(&self, other: &ComingEvent) -> bool {
        (self.event_id == other.event_id) & (self.start_time == other.start_time)
    }
}

/// An private enum to provide and receive updates from the various internal
/// components of the system interface and external updates from the interface.
///
#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub fn new() -> (GeneralUpdate, mpsc::Receiver<GeneralUpdateType>) {
        // Create the new channel
        let (general_send, receive) = mpsc::channel();

        // Create and return both new items
        (GeneralUpdate { general_send }, receive)
    }

    /// A method to broadcast an event via the system interface (with data,
    /// if it is provided)
    ///
    pub async fn send_broadcast(&self, event_id: ItemId, data: Option<u32>) {
        self.general_send
            .send(GeneralUpdateType::BroadcastEvent(event_id, data)).await
            .unwrap_or(());
    }

    /// A method to send new coming events to the system
    ///
    pub async fn send_coming_events(&self, coming_events: Vec<ComingEvent>) {
        self.general_send
            .send(GeneralUpdateType::ComingEvents(coming_events)).await
            .unwrap_or(());
    }

    /// A method to process a new event. If the check_scene flag is not set,
    /// the system will not check if the event is in the current scene. If
    /// broadcast is set to true, the event will be broadcast to the system.
    ///
    pub async fn send_event(&self, event: ItemId, check_scene: bool, broadcast: bool) {
        self.send_system(ProcessEvent {
            event,
            check_scene,
            broadcast,
        }).await;
    }

    /// A method to request a string from the user FIXME make this more generic
    /// for other types of data
    ///
    pub async fn send_get_user_string(&self, event: ItemPair) {
        self.general_send
            .send(GeneralUpdateType::GetUserString(event)).await
            .unwrap_or(());
    }
     
    /// A method to pass a new video stream to the user interface
    ///
    #[cfg(feature = "media-out")]
    pub async fn send_new_video(&self, video_stream: VideoStream) {
        self.general_send
            .send(GeneralUpdateType::NewVideo(Some(video_stream))).await
            .unwrap_or(());
    }

    /// A method to clear all video streams from the user interface
    ///
    #[cfg(feature = "media-out")]
    pub async fn send_clear_videos(&self) {
        self.general_send
            .send(GeneralUpdateType::NewVideo(None)).await
            .unwrap_or(());
    }

    /// A method to trigger a redraw of the current window
    ///
    pub async fn send_redraw(&self) {
        self.send_system(Redraw).await;
    }

    /// A method to pass a system update to the system interface.
    ///
    pub async fn send_system(&self, update: SystemUpdate) {
        self.general_send
            .send(GeneralUpdateType::System(update)).await
            .unwrap_or(());
    }

    /// A method to send an event update to the system interface.
    ///
    pub async fn send_update(&self, update: EventUpdate) {
        self.general_send
            .send(GeneralUpdateType::Update(update)).await
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
    pub fn from_general(general_update: &GeneralUpdate) -> SystemSend {
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
}

// Reexport the system update type variants
pub use self::SystemUpdate::{
    AllEventChange, AllStop, BroadcastEvent, ClearQueue, Close, ConfigFile, DebugMode, Edit,
    ErrorLog, EventChange, GameLog, ProcessEvent, CueEvent, Redraw, Request, SaveConfig,
    SceneChange, StatusChange,
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
