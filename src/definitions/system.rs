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

/// The stucture and methods to send GtkRequests to the system interface
///
#[derive(Clone, Debug)]
pub struct GtkSend {
    gtk_send: mpsc::Sender<GtkRequest>, // the mpsc sending line to pass gtk requests
}

// Implement the key features of the gtk send struct
impl GtkSend {
    /// A function to create a new GtkSend
    ///
    /// The function returns the the GtkSend structure and the system
    /// receive channel which will return the requests.
    ///
    pub fn new() -> (Self, mpsc::Receiver<GtkRequest>) {
        // Create the new channel
        let (gtk_send, receive) = mpsc::channel(128);

        // Create and return both new items
        (GtkSend { gtk_send }, receive)
    }

    /// A method to send a gtk request in a sync setting. This method fails
    /// silently.
    ///
    pub fn send(&self, request: UserRequest) {
        self.gtk_send.blocking_send(request.into()).unwrap_or(());
    }
}

/// A structure for carrying requests from the gtk interface
///
pub struct GtkRequest {
    request: UserRequest,
}

/// Implement from and to UserRequest for GtkRequest
impl From<UserRequest> for GtkRequest {
    fn from(request: UserRequest) -> Self {
        GtkRequest { request }
    }
}
impl From<GtkRequest> for UserRequest {
    fn from(gtk_request: GtkRequest) -> Self {
        gtk_request.request
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
        let (web_send, receive) = mpsc::channel(128);

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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
        event: Option<Event>,
    },

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
        scene: Option<Scene>,
    },
}

/// An enum to specify the type of information request
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    SelectEventDescription {
        position: Option<usize>,
        is_event: bool,
    },

    /// A variant for the select event states
    SelectEventStates,
}

/// An enum to specify which Edit Item subcomponent has requested the information
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayComponent {
    /// A variant for the edit item window
    EditItemOverview {
        is_left: bool,
        variant: EditItemElement,
    },

    /// A variant for the edit action element
    EditActionElement {
        is_left: bool,
        variant: EditActionElement,
    },

    /// A variant for the edit multistate status element
    EditMultiStateStatus {
        is_left: bool,
        position: Option<usize>,
    },

    /// A variant for the edit counted state status element
    EditCountedStateStatus { is_left: bool, state_type: String },

    /// A variant for the item list panel
    ItemList,

    /// A variant for the trigger dialog
    TriggerDialog,
}

/// An enum to carry requests from the user
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserRequest {
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
    BroadcastEvent { event_id: ItemId, data: Option<u32> },

    /// A variant to trigger all the queued events to clear
    ClearQueue,

    /// A special variant to close the program and unload all the data.
    Close,

    /// A variant that provides a new configuration file for the system interface.
    /// If None is provided as the filepath, no configuration will be loaded.
    ConfigFile { filepath: Option<PathBuf> },

    /// A variant that cues a new event with the given item id. The event
    /// will trigger after the specified delay has passed.
    CueEvent { event_delay: EventDelay },

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
        start_time: NaiveDateTime, // the start time of the event, for unambiguous identification
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
    StatusChange { status: ItemId, state: ItemId },
}

/// A type to cover all web replies
///
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WebReply {
    // A variant for replies with no specific content
    #[serde(rename_all = "camelCase")]
    Generic {
        is_valid: bool,  // a flag to indicate the result of the request
        message: String, // a message describing the success or failure
    },

    // A variant that contains the complete item list
    #[serde(rename_all = "camelCase")]
    AllItems {
        is_valid: bool,                   // a flag to indicate the result of the request
        all_items: Option<Vec<ItemPair>>, // the list of all items, if found
    },

    // A variant that contains scene detail
    #[serde(rename_all = "camelCase")]
    Scene {
        is_valid: bool,                  // a flag to indicate the result of the request
        scene: Option<DescriptiveScene>, // the scene detail, if found
    },

    // A variant that contains status detail
    #[serde(rename_all = "camelCase")]
    Status {
        is_valid: bool,         // a flag to indicate the result of the request
        status: Option<Status>, // the status detail, if found
    },

    // A variant that contains event detail
    #[serde(rename_all = "camelCase")]
    Event {
        is_valid: bool,       // a flag to indicate the result of the request
        event: Option<Event>, // the event detail, if found
    },

    // A variant that contains item detail
    #[serde(rename_all = "camelCase")]
    Item {
        is_valid: bool,      // a flag to indicate the result of the request
        item_pair: ItemPair, // the item pair
    },
}

// Implement key features of the web reply
impl WebReply {
    /// A function to return a new, successful web reply
    ///
    pub fn success() -> WebReply {
        WebReply::Generic {
            is_valid: true,
            message: "Request completed.".to_string(),
        }
    }

    /// A function to return a new, failed web reply
    ///
    pub fn failure<S>(reason: S) -> WebReply
    where
        S: Into<String>,
    {
        WebReply::Generic {
            is_valid: false,
            message: reason.into(),
        }
    }

    /// A method to check if the reply is a success
    ///
    pub fn is_success(&self) -> bool {
        match self {
            &WebReply::Generic { ref is_valid, .. } => is_valid.clone(),
            &WebReply::AllItems { ref is_valid, .. } => is_valid.clone(),
            &WebReply::Scene { ref is_valid, .. } => is_valid.clone(),
            &WebReply::Status { ref is_valid, .. } => is_valid.clone(),
            &WebReply::Event { ref is_valid, .. } => is_valid.clone(),
            &WebReply::Item { ref is_valid, .. } => is_valid.clone(),
        }
    }
}

/*
/// The stucture and methods to send updates to the user interface.
///
#[cfg(not(test))]
#[derive(Clone, Debug)]
pub struct IndexAccess {
    index_send: mpsc::Sender<IndexUpdate>, // the line to pass internal updates to the system interface
}

// Implement the key features of the index access
#[cfg(not(test))]
impl IndexAccess {
    /// A function to create a new Index Access
    ///
    /// The function returns the Index Access structure and the index
    /// receive channel which will return the provided updates.
    ///
    pub fn new() -> (IndexAccess, mpsc::Receiver<IndexUpdate>) {
        // Create the new channel
        let (index_send, receive) = mpsc::channel(256);

        // Create and return both new items
        (IndexAccess { index_send }, receive)
    }

    /// A method to send a new index to the item index
    ///
    pub async fn send_index(&self, new_index: DescriptionMap) {
        self.index_send
            .send(IndexUpdate::NewIndex { new_index })
            .await
            .unwrap_or(());
    }

    /// A method to remove an item from the index
    /// Returns true if the item was found and false otherwise.
    ///
    pub async fn _remove_item(&self, item_id: ItemId) -> bool {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .index_send
            .send(IndexUpdate::UpdateDescription {
                item_id: item_id.clone(),
                new_description: None,
                reply_line,
            })
            .await
        {
            // On failure, return false
            return false;
        }

        // Wait for the reply
        rx.await.unwrap_or(false)
    }

    /// A method to add or update the description in the item index
    /// Returns true if the item was not previously defined and false otherwise.
    ///
    pub async fn update_description(
        &self,
        item_id: ItemId,
        new_description: ItemDescription,
    ) -> bool {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .index_send
            .send(IndexUpdate::UpdateDescription {
                item_id: item_id.clone(),
                new_description: Some(new_description.clone()),
                reply_line,
            })
            .await
        {
            // On failure, return false
            return false;
        }

        // Wait for the reply
        rx.await.unwrap_or(false)
    }

    /// A method to get the description from the item index
    ///
    pub async fn get_description(&self, item_id: &ItemId) -> ItemDescription {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .index_send
            .send(IndexUpdate::GetDescription {
                item_id: item_id.clone(),
                reply_line,
            })
            .await
        {
            // On failure, return default
            return ItemDescription::new_default();
        }

        // Wait for the reply
        rx.await.unwrap_or(ItemDescription::new_default())
    }

    /// A method to get the item pair from the item index
    ///
    pub async fn get_pair(&self, item_id: &ItemId) -> ItemPair {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .index_send
            .send(IndexUpdate::GetPair {
                item_id: item_id.clone(),
                reply_line,
            })
            .await
        {
            // On failure, return default
            return ItemPair::new_default(item_id.id());
        }

        // Wait for the reply
        rx.await.unwrap_or(ItemPair::new_default(item_id.id()))
    }

    /// A method to get all pairs from the item index
    ///
    pub async fn get_all(&self) -> Vec<ItemPair> {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .index_send
            .send(IndexUpdate::GetAll { reply_line })
            .await
        {
            // On failure, return none
            return Vec::new();
        }

        // Wait for the reply
        rx.await.unwrap_or(Vec::new())
    }
}*/
