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

// Import Tokio and warp features
use tokio::sync::mpsc;
use warp::ws::Message;

// Import FNV HashMap
use fnv::FnvHashMap;

/// An enum to change one of the display settings of the user interface
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplaySetting {
    /// A variant to change the debug mode of the display
    DebugMode(bool),

    /// A variant to change the font size of the display
    LargeFont(bool),

    /// A variant to change the color mode of the display
    HighContrast(bool),
}

/// A structure to hold the parameters of the configuration file
///
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigParameters {
    pub identifier: Identifier,
    pub server_location: Option<String>,
    pub dmx_controllers: DmxControllers,
    pub media_players: Vec<MediaPlayer>,
    pub system_connections: ConnectionSet,
    pub background_process: Option<BackgroundProcess>,
    pub default_scene: ItemId,
}

/// An enum type to provide updates to the web interface
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InterfaceUpdate {
    /// A variant to provide the current scene and state of all statuses
    #[serde(rename_all = "camelCase")]
    CurrentSceneAndStatus {
        current_scene: ItemId,
        current_status: CurrentStatus,
    },

    /// A variant to post a current event to the status bar
    #[serde(rename_all = "camelCase")]
    Notify { message: String },

    /// A variant to indicate that the entire interface should be refreshed
    #[serde(rename_all = "camelCase")]
    RefreshAll,

    /// A variant indicating the current scene should be refreshed with
    /// the new scene.
    #[serde(rename_all = "camelCase")]
    UpdateScene { current_scene: ItemId },

    /// A variant to update the state of a partiular status.
    #[serde(rename_all = "camelCase")]
    UpdateStatus {
        status_id: ItemPair, // the status to update
        new_state: ItemPair, // the new state of the status
    },

    /// A variant indicating that the system notifications should be updated.
    #[serde(rename_all = "camelCase")]
    UpdateNotifications { notifications: Vec<String> },

    /// A variant indicating that the event timeline should be updated.
    #[serde(rename_all = "camelCase")]
    UpdateTimeline { events: Vec<UpcomingEvent> },
}

// Implement from<InterfaceUpdate> for Message)
impl From<InterfaceUpdate> for Result<Message, warp::Error> {
    fn from(update: InterfaceUpdate) -> Self {
        // Try to serialize the update
        match serde_json::to_string(&update) {
            Ok(string) => Ok(Message::text(string)),

            // On failure, return an empty string (unable to convert the error)
            _ => Ok(Message::text("")),
        }
    }
}

/// The stucture and methods to send updates to the user interface.
///
#[derive(Clone, Debug)]
pub struct InterfaceSend {
    interface_send: mpsc::Sender<InterfaceUpdate>, // the line to pass updates to the web user interface
}

// Implement the key features of interface send
impl InterfaceSend {
    /// A function to create a new InterfaceSend
    ///
    /// The function returns the InterfaceSend structure and the interface
    /// receive channels which will return the provided updates.
    ///
    pub fn new() -> (Self, mpsc::Receiver<InterfaceUpdate>) {
        // Create one or two new channels
        let (interface_send, interface_recv) = mpsc::channel(512);

        // Create and return the new items
        (InterfaceSend { interface_send }, interface_recv)
    }

    /// A method to send an interface update. This method fails silently.
    ///
    pub async fn send(&self, update: InterfaceUpdate) {
        self.interface_send.send(update).await.unwrap_or(());
    }
}

/// A type to store a hashmap of status ids and current state ids
///
pub type CurrentStatus = FnvHashMap<u32, u32>;

/// An enum type to provide updates to the limited interface.
/// These updates contain only the minimal information needed
/// to follow operations as they progress.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LimitedUpdate {
    /// A variant to send a current event id
    #[serde(rename_all = "camelCase")]
    CurrentEvent {
        event: ItemId, // current event id
    },

    /// A variant to provide the current scene and state of all statuses
    #[serde(rename_all = "camelCase")]
    CurrentSceneAndStatus {
        current_scene: ItemId,
        current_status: CurrentStatus,
    },

    /// A variant indicating the current scene should be refreshed with
    /// the new scene.
    #[serde(rename_all = "camelCase")]
    UpdateScene { current_scene: ItemId },

    /// A variant to update the state of a partiular status
    #[serde(rename_all = "camelCase")]
    UpdateStatus {
        status_id: ItemId, // the status to update
        new_state: ItemId, // the new state of the status
    },
}

// Implement from<LimitedUpdate> for Message)
impl From<LimitedUpdate> for Result<Message, warp::Error> {
    fn from(update: LimitedUpdate) -> Self {
        // Try to serialize the update
        match serde_json::to_string(&update) {
            Ok(string) => Ok(Message::text(string)),

            // On failure, return an empty string (unable to convert the error)
            _ => Ok(Message::text("")),
        }
    }
}

/// The stucture and methods to send updates to the limited interface.
///
#[derive(Clone, Debug)]
pub struct LimitedSend {
    limited_send: mpsc::Sender<LimitedUpdate>, // the line to pass updates to the limited interface
}

// Implement the key features of limited send
impl LimitedSend {
    /// A function to create a new LimitedSend
    ///
    /// The function returns the LimitedSend structure and the limited
    /// receive channels which will return the provided updates.
    ///
    pub fn new() -> (Self, mpsc::Receiver<LimitedUpdate>) {
        // Create one or two new channels
        let (limited_send, limited_recv) = mpsc::channel(512);

        // Create and return the new items
        (LimitedSend { limited_send }, limited_recv)
    }

    /// A method to send a limited update. This method fails silently.
    ///
    pub async fn send(&self, update: LimitedUpdate) {
        self.limited_send.send(update).await.unwrap_or(());
    }
}
