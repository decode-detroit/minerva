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

// Import Tokio features
use tokio::sync::mpsc;

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
