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
#[cfg(feature = "media-out")]
use std::sync::{Arc, Mutex, mpsc as std_mpsc};

// Import Tokio and warp features
use tokio::sync::mpsc;
use warp::ws::Message;

/// A structure to list a series of event buttons that are associated with one
/// event group.
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct EventGroup {
    pub group_id: Option<ItemPair>, // the group id identifying and describing the group or None for the general group
    pub group_events: Vec<ItemPair>, // a vector of the events that belong in this group
}

/// A type to list a series of event groups that fill the event window.
///
pub type EventWindow = Vec<EventGroup>; // a vector of event groups that belong in this window

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

/// An enum type to provide updates to the user interface thread.
///
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum InterfaceUpdate {
    /// A variant to change the display settings
    ChangeSettings { display_setting: DisplaySetting },

    /// A variant to post a current event to the status bar
    Notify { message: String },

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

    /// A variant to launch the video window
    #[cfg(feature = "media-out")]
    Video { video_stream: Option<VideoStream> },
}

/// An enum type to provide interface to the web interface (a subset of the InterfaceUpdates)
///
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WebInterfaceUpdate {
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
        current_scene: ItemPair,
        statuses: Vec<ItemPair>,
        window: EventWindow,
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
    UpdateNotifications { notifications: Vec<Notification> },

    /// A variant indicating that the event timeline should be updated.
    #[serde(rename_all = "camelCase")]
    UpdateTimeline { events: Vec<UpcomingEvent> },
}

// Implement from<WebInterfaceUpdate> for Message)
impl From<WebInterfaceUpdate> for Result<Message, warp::Error> {
    fn from(update: WebInterfaceUpdate) -> Self {
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
    #[cfg(feature = "media-out")]
    gtk_interface_send: Arc<Mutex<std_mpsc::Sender<InterfaceUpdate>>>, // the line to pass updates to the gtk user interface
    web_interface_send: mpsc::Sender<WebInterfaceUpdate>, // the line to pass updates to the web user interface
}

// Implement the key features of interface send
impl InterfaceSend {
    /// A function to create a new InterfaceSend
    ///
    /// The function returns the InterfaceSend structure and the interface
    /// receive channels which will return the provided updates.
    ///
    #[cfg(not(feature = "media-out"))]
    pub fn new() -> (Self, mpsc::Receiver<WebInterfaceUpdate>) {
        // Create one or two new channels
        let (web_interface_send, web_receive) = mpsc::channel(256);

        // Create and return the new items
        (InterfaceSend { web_interface_send }, web_receive)
    }

    /// A function to create a new InterfaceSend, Media-enabled version
    ///
    /// The function returns the InterfaceSend structure and the interface
    /// receive channels which will return the provided updates.
    ///
    #[cfg(feature = "media-out")]
    pub fn new() -> (Self, std_mpsc::Receiver<InterfaceUpdate>, mpsc::Receiver<WebInterfaceUpdate>) {
        // Create one or two new channels
        let (gtk_interface_send, gtk_receive) = std_mpsc::channel();
        let (web_interface_send, web_receive) = mpsc::channel(256);

        // Create and return the new items
        return (InterfaceSend { gtk_interface_send: Arc::new(Mutex::new(gtk_interface_send)), web_interface_send }, gtk_receive, web_receive);
    }

    /// A method to send an interface update. This method fails silently.
    ///
    pub async fn send(&self, update: InterfaceUpdate) {
        // Get a lock on the gtk send line
        #[cfg(feature = "media-out")]
        if let Ok(gtk_send) = self.gtk_interface_send.lock() {
            // Send the update to the gtk interface
            gtk_send.send(update.clone()).unwrap_or(());
        }

        // Match the update type
        match update.clone() {
            // For certain types, send the update to the web interface too
            InterfaceUpdate::ChangeSettings { display_setting } => {
                self.web_interface_send.send(WebInterfaceUpdate::ChangeSettings { display_setting }).await.unwrap_or(());
            }
            InterfaceUpdate::Notify { message } => {
                self.web_interface_send.send(WebInterfaceUpdate::Notify { message }).await.unwrap_or(());
            }
            InterfaceUpdate::UpdateConfig { scenes, full_status } => {
                self.web_interface_send.send(WebInterfaceUpdate::UpdateConfig { scenes, full_status }).await.unwrap_or(());
            }
            InterfaceUpdate::UpdateWindow { current_scene, statuses, window, key_map } => {
                self.web_interface_send.send(WebInterfaceUpdate::UpdateWindow { current_scene, statuses, window, key_map }).await.unwrap_or(());
            }
            InterfaceUpdate::UpdateStatus { status_id, new_state } => {
                self.web_interface_send.send(WebInterfaceUpdate::UpdateStatus { status_id, new_state }).await.unwrap_or(());
            }
            InterfaceUpdate::UpdateNotifications { notifications } => {
                self.web_interface_send.send(WebInterfaceUpdate::UpdateNotifications { notifications }).await.unwrap_or(());
            }
            InterfaceUpdate::UpdateTimeline { events } => {
                self.web_interface_send.send(WebInterfaceUpdate::UpdateTimeline { events }).await.unwrap_or(());
            }

            // Ignore others
            _ => (),
        }
    }

    /// A method to send an interface update in a sync setting. This method fails
    /// silently.
    ///
    pub fn sync_send(&self, update: InterfaceUpdate) {
        // Get a lock on the gtk send line
        #[cfg(feature = "media-out")]
        if let Ok(gtk_send) = self.gtk_interface_send.lock() {
            // Send the update to the gtk interface
            gtk_send.send(update.clone()).unwrap_or(());
        }

        // Match the update type
        match update.clone() {
            // For certain types, send the update to the web interface too
            InterfaceUpdate::ChangeSettings { display_setting } => {
                self.web_interface_send.blocking_send(WebInterfaceUpdate::ChangeSettings { display_setting }).unwrap_or(());
            }
            InterfaceUpdate::Notify { message } => {
                self.web_interface_send.blocking_send(WebInterfaceUpdate::Notify { message }).unwrap_or(());
            }
            InterfaceUpdate::UpdateConfig { scenes, full_status } => {
                self.web_interface_send.blocking_send(WebInterfaceUpdate::UpdateConfig { scenes, full_status }).unwrap_or(());
            }
            InterfaceUpdate::UpdateWindow { current_scene, statuses, window, key_map } => {
                self.web_interface_send.blocking_send(WebInterfaceUpdate::UpdateWindow { current_scene, statuses, window, key_map }).unwrap_or(());
            }
            InterfaceUpdate::UpdateStatus { status_id, new_state } => {
                self.web_interface_send.blocking_send(WebInterfaceUpdate::UpdateStatus { status_id, new_state }).unwrap_or(());
            }
            InterfaceUpdate::UpdateNotifications { notifications } => {
                self.web_interface_send.blocking_send(WebInterfaceUpdate::UpdateNotifications { notifications }).unwrap_or(());
            }
            InterfaceUpdate::UpdateTimeline { events } => {
                self.web_interface_send.blocking_send(WebInterfaceUpdate::UpdateTimeline { events }).unwrap_or(());
            }

            // Ignore others
            _ => (),
        }
    }
}
