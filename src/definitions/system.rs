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

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::path::PathBuf;
use std::time::Duration;

// Import Tokio features
use tokio::sync::{mpsc, oneshot};

// Import Chrono features
use chrono::NaiveDateTime;

/// A type to hold data to broadcast to the system
pub type BroadcastData = Vec<Option<u32>>;

/// An enum to provide and receive updates from the various internal
/// components of the system interface and external updates from the interface.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InternalUpdate {
    /// A variant that notifies the system of a change in the coming events
    ComingEvents(Vec<ComingEvent>),

    /// A variant that processes a new event with the given item id. If the
    /// check_scene flag is not set, the system will not check if the event is
    /// listed in the current scene. If broadcast is set to true, the event
    /// will be broadcast to the system
    ProcessEvent {
        event_id: ItemId,
        check_scene: bool,
        broadcast: bool,
    },
}

/// The stucture and methods to send internal updates to the system interface.
///
#[derive(Clone, Debug)]
pub struct InternalSend {
    internal_send: mpsc::Sender<InternalUpdate>, // the line to pass internal updates to the system interface
}

// Implement the key features of the internal update
impl InternalSend {
    /// A function to create a new Internal Update
    ///
    /// The function returns the the Internal Update structure and the internal
    /// receive channel which will return the provided updates.
    ///
    pub fn new() -> (InternalSend, mpsc::Receiver<InternalUpdate>) {
        // Create the new channel
        let (internal_send, receive) = mpsc::channel(128);

        // Create and return both new items
        (InternalSend { internal_send }, receive)
    }

    /// A method to send new coming events to the system
    ///
    pub async fn send_coming_events(&self, coming_events: Vec<ComingEvent>) {
        self.internal_send
            .send(InternalUpdate::ComingEvents(coming_events))
            .await
            .unwrap_or(());
    }

    // A method to process a new event. If the check_scene flag is not set,
    // the system will not check if the event is in the current scene. If
    // broadcast is set to true, the event will be broadcast to the system.
    //
    pub async fn send_event(&self, event_id: ItemId, check_scene: bool, broadcast: bool) {
        self.internal_send
            .send(InternalUpdate::ProcessEvent {
                event_id,
                check_scene,
                broadcast,
            })
            .await
            .unwrap_or(());
    }

    /// A method to send an update in an sync context
    ///
    /// # Note
    /// This method will panic if used inside an async context.
    ///
    pub fn blocking_send(&self, update: InternalUpdate) {
        self.internal_send.blocking_send(update).unwrap_or(());
    }
}

/// The stucture and methods to send WebRequests to the system interface
///
#[derive(Clone, Debug)]
pub struct WebSend {
    web_send: mpsc::Sender<WebRequest>, // the mpsc sending line to pass web requests
}

// Implement the key features of the web send struct
impl WebSend {
    /// A function to create a new WebSend
    ///
    /// The function returns the the Web Sent structure and the system
    /// receive channel which will return the provided updates.
    ///
    pub fn new() -> (Self, mpsc::Receiver<WebRequest>) {
        // Create the new channel
        let (web_send, receive) = mpsc::channel(256);

        // Create and return both new items
        (WebSend { web_send }, receive)
    }

    /// A method to send a web request. This method fails silently.
    ///
    pub async fn send(&self, reply_to: oneshot::Sender<WebReply>, request: UserRequest) {
        self.web_send
            .send(WebRequest { reply_to, request })
            .await
            .unwrap_or(());
    }
}

/// A structure for carrying requests from the web interface
///
pub struct WebRequest {
    pub reply_to: oneshot::Sender<WebReply>, // the handle for replying to the reqeust
    pub request: UserRequest,                // the request
}

/// An enum to execute one modification to the configuration
///
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Modification {
    /// A modification to add an item or modify an existing one
    #[serde(rename_all = "camelCase")]
    ModifyItem { item_pair: ItemPair },

    /// A modification to add an event, modify an existing one, or delete it
    /// (if None provided)
    #[serde(rename_all = "camelCase")]
    ModifyEvent {
        item_id: ItemId,
        event: Option<WebEvent>,
    },

    /// A modification to add a group, modify an existing one, or delete it
    /// (if None provided)
    #[serde(rename_all = "camelCase")]
    ModifyGroup {
        item_id: ItemId,
        group: Option<WebGroup>,
    },

    /// A modification to change the configuration parameters
    /// (if None provided)
    #[serde(rename_all = "camelCase")]
    ModifyParameters { parameters: ConfigParameters },

    /// A modification to add a status, modify an existing one, or delete it
    /// (if None provided)
    #[serde(rename_all = "camelCase")]
    ModifyStatus {
        item_id: ItemId,
        status: Option<Status>,
    },

    /// A modification to add a scene, modify an existing one, or delete it
    /// (if None provided)
    #[serde(rename_all = "camelCase")]
    ModifyScene {
        item_id: ItemId,
        scene: Option<WebScene>,
    },

    /// A modification to remove an item and any event, scene, or status connected to it
    ///
    /// # WARNING
    ///
    /// This does not remove dangling references to the item that may appear in other items.
    ///
    #[serde(rename_all = "camelCase")]
    RemoveItem { item_id: ItemId },
}

/// An enum to specify the type of detail request
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetailType {
    /// A variant for the list of all items in the current scene
    AllCurrentItems,

    /// A variant for the list of all groups
    /// NOTE: Consider moving this to the index module (see AllItems)
    AllGroups,

    /// A variant for the list of all scenes
    /// NOTE: Consider moving this to the index module (see AllItems)
    AllScenes,

    /// A variant for the event associated with an item
    Event { item_id: ItemId },

    /// A variant for the list of all events in a group
    Group { item_id: ItemId },

    /// A variant for the status associated with an item
    Status { item_id: ItemId },

    /// A variant for the list of all events in a scene
    Scene { item_id: ItemId },

    /// A variant for the item type
    Type { item_id: ItemId },
}

/// An enum to carry requests from the user
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserRequest {
    /// A variant to adjust all the events in the timeline.
    /// NOTE: after the adjustment, events that would have already happened are discarded
    AllEventChange {
        adjustment: Duration, // the amount of time to add to (or subtract from) all events
        is_negative: bool,    // a flag to indicate if the delay should be subtracted
    },

    /// A special variant to send the "all stop" event which automatically
    /// is broadcast immediately and clears the event queue.
    AllStop,

    /// A variant to trigger all the queued events to clear.
    ClearQueue,

    /// A special variant to close the program and unload all the data.
    Close,

    /// A variant that provides a new configuration file for the system interface.
    /// If None is provided as the filepath, no configuration will be loaded.
    ConfigFile { filepath: Option<PathBuf> },

    /// A variant that retrieves the file path of the current configuration.
    /// If there is no active configuration, this request will throw an error.
    ConfigPath,

    /// A variant that retrieves the paramters of the current configuration
    /// including filename, system connections, default scene, etc.
    /// If there is no active configuration, this request will throw an error.
    ConfigParameters,

    /// A variant that cues a new event with the given item id. The event
    /// will trigger after the specified delay has passed.
    CueEvent { event_delay: EventDelay },

    /// A variant to provide the current scene and status
    CurrentSceneAndStatus,

    /// A variant to provide details as requested by the web interface.
    Detail { detail_type: DetailType },

    /// A variant to modify the underlying configuration.
    Edit { modifications: Vec<Modification> },

    /// A variant to change the remaining delay for an existing event in the
    /// queue.
    EventChange {
        event_id: ItemId,
        start_time: NaiveDateTime, // the start time of the event, for unambiguous identification
        new_delay: Option<Duration>, // new delay relative to the original start time, or None to cancel the event
    },

    /// A variant that provides a new configuration file to save the current
    /// configuration.
    SaveConfig { filepath: PathBuf },

    /// A variant to change the current scene.
    SceneChange { scene: ItemId },

    /// A variant to change the state of the indicated status.
    StatusChange { status: ItemId, state: ItemId },
}

/// A struct to cover all web replies
///
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebReply {
    pub is_valid: bool,
    pub data: WebReplyData,
}

/// A type to cover all web reply data
///
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WebReplyData {
    // A variant that contains current scene and status
    #[serde(rename_all = "camelCase")]
    CurrentSceneAndStatus((ItemId, CurrentStatus)),

    // A variant that contains event detail
    #[serde(rename_all = "camelCase")]
    Event(Option<WebEvent>),

    // A variant that contains item detail
    #[serde(rename_all = "camelCase")]
    Item(ItemPair),

    // A variant that contains an item list
    #[serde(rename_all = "camelCase")]
    Items(Vec<ItemId>),

    // A variant for replies with a message
    #[serde(rename_all = "camelCase")]
    Message(String),

    // A variant that contains group detail
    #[serde(rename_all = "camelCase")]
    Group(Option<WebGroup>),

    // A variant that contains configuration paramters
    #[serde(rename_all = "camelCase")]
    Parameters(ConfigParameters),

    // A variant that contains a file path
    #[serde(rename_all = "camelCase")]
    Path {
        filename: String, // the filename, including the file extension
        path: String,     // the full file path, including the filenme
    },

    // A variant that contains scene detail
    #[serde(rename_all = "camelCase")]
    Scene(Option<WebScene>),

    // A variant that contains status detail
    #[serde(rename_all = "camelCase")]
    Status(Option<Status>),
}

// Implement key features of the web reply
impl WebReply {
    /// A function to return a new, successful web reply
    ///
    pub fn success() -> WebReply {
        WebReply {
            is_valid: true,
            data: WebReplyData::Message("Request completed.".to_string()),
        }
    }

    /// A function to return a new, failed web reply
    ///
    pub fn failure<S>(reason: S) -> WebReply
    where
        S: Into<String>,
    {
        WebReply {
            is_valid: true,
            data: WebReplyData::Message(reason.into()),
        }
    }

    /// A method to check if the reply is a success
    ///
    pub fn is_success(&self) -> bool {
        self.is_valid
    }
}
