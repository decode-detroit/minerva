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

/// An enum to change one of the display settings of the user interface
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// An enum type to provide interface to the web interface
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InterfaceUpdate {
    /// A variant to change the display settings
    #[serde(rename_all = "camelCase")]
    ChangeSettings { display_setting: DisplaySetting },

    /// A variant to post a current event to the status bar
    #[serde(rename_all = "camelCase")]
    Notify { message: String },

    /// A variant to update the available scenes and full status in the main
    /// program window.
    #[serde(rename_all = "camelCase")]
    UpdateConfig {
        scenes: Vec<ItemPair>,
        full_status: FullStatus,
    },

    /// A variant indicating the entire button window should be refreshed with
    /// the new provided window.
    #[serde(rename_all = "camelCase")]
    UpdateWindow {
        current_scene: ItemId,
        current_items: Vec<ItemId>,
        key_map: KeyMap,
    },

    /// A variant to update the state of a partiular status.
    #[serde(rename_all = "camelCase")]
    UpdateStatus {
        status_id: ItemPair, // the group to update
        new_state: ItemPair, // the new state of the group
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
    web_interface_send: mpsc::Sender<InterfaceUpdate>, // the line to pass updates to the web user interface
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
        let (web_interface_send, web_receive) = mpsc::channel(256);

        // Create and return the new items
        (InterfaceSend { web_interface_send }, web_receive)
    }

    /// A method to send an interface update. This method fails silently.
    ///
    pub async fn send(&self, update: InterfaceUpdate) {
        self.web_interface_send.send(update).await.unwrap_or(());
    }
}
