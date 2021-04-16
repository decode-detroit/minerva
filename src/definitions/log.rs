// Copyright (c) 2017-2021 Decode Detroit
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

//! This module implements the internal send communication for the system
//! interface and the log! macro for easy logging and notifications.

// Import crate definitions
use crate::definitions::*;

// Import standard library modules
use std::fmt;

// Import Chrono features
use chrono::NaiveDateTime;

// Import Tokio features
use tokio::sync::mpsc;

/// An enum to provide and receive updates from the various internal
/// components of the system interface and external updates from the interface.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InternalUpdate {
    /// A variant that broadcasts an event with the given item id. This event id
    /// is not processed or otherwise checked for validity. If data is provided,
    /// it will be broadcast with the event.
    BroadcastEvent(ItemId, Option<u32>),

    /// A variant that notifies the system of a change in the coming events
    ComingEvents(Vec<ComingEvent>),

    /// A variant that solicites a string of data from the user to send to the
    /// system. The string will be sent as a series of events with the same
    /// item id. TODO Make this more generic for other user input
    GetUserString(ItemId),

    /// A variant to pass a new video stream to the user interface
    #[cfg(feature = "media-out")]
    NewVideo(Option<VideoStream>),

    /// A variant that processes a new event with the given item id. If the
    /// check_scene flag is not set, the system will not check if the event is
    /// listed in the current scene. If broadcast is set to true, the event
    /// will be broadcast to the system
    ProcessEvent {
        event: ItemId,
        check_scene: bool,
        broadcast: bool,
    },

    /// A variant to trigger a refresh of the user interface
    /// FIXME Reconsider this arrangement
    RefreshInterface,

    /// A variant to log updates
    Update(LogUpdate),
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

    /// A method to broadcast an event via the system interface (with data,
    /// if it is provided)
    ///
    pub async fn send_broadcast(&self, event_id: ItemId, data: Option<u32>) {
        self.internal_send
            .send(InternalUpdate::BroadcastEvent(event_id, data))
            .await
            .unwrap_or(());
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
    pub async fn send_event(&self, event: ItemId, check_scene: bool, broadcast: bool) {
        self.internal_send
            .send(InternalUpdate::ProcessEvent {
                event,
                check_scene,
                broadcast,
            })
            .await
            .unwrap_or(());
    }

    /// A method to request a string from the user FIXME make this more generic
    /// for other types of data
    ///
    pub async fn send_get_user_string(&self, event: ItemId) {
        self.internal_send
            .send(InternalUpdate::GetUserString(event))
            .await
            .unwrap_or(());
    }

    /// A method to pass a new video stream to the user interface
    ///
    #[cfg(feature = "media-out")]
    pub async fn send_new_video(&self, video_stream: VideoStream) {
        self.internal_send
            .send(InternalUpdate::NewVideo(Some(video_stream)))
            .await
            .unwrap_or(());
    }

    /// A method to clear all video streams from the user interface
    ///
    #[cfg(feature = "media-out")]
    pub async fn send_clear_videos(&self) {
        self.internal_send
            .send(InternalUpdate::NewVideo(None))
            .await
            .unwrap_or(());
    }

    // A method to trigger a refresh of the user interface
    //
    pub async fn send_refresh(&self) {
        self.internal_send
            .send(InternalUpdate::RefreshInterface)
            .await
            .unwrap_or(());
    }

    /// A method to send an event update to the system interface.
    ///
    pub async fn send_update(&self, update: LogUpdate) {
        self.internal_send
            .send(InternalUpdate::Update(update))
            .await
            .unwrap_or(());
    }
}

/// An enum for logging changes to the game. Most changes in the system interface
/// should go through this enum (and through the logging module).
///
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum LogUpdate {
    /// A variant that notifies the rest of the system to broadcast this
    /// event with optional data.
    Broadcast(ItemId, Option<u32>),

    /// A variant that notifies the rest of the system of a currently playing
    /// event.
    Current(ItemId),

    // Define several event update types, in order of decreasing priority
    /// A variant which passes unrecoverable errors generated by the system.
    Error(String, Option<ItemId>),

    /// A variant that notifies the system logger to log data to the game log
    Save(String), // the data to save, formatted as a string

    /// A variant that notifies the rest of the system of the new state of the status
    Status(ItemId, ItemId), // first field is the status id, second is the new state

    /// A variant which can send any other type of update to the system.
    Update(String),

    /// A variant which passes recoverable warnings generated by the system.
    Warning(String, Option<ItemId>),
}

// Implement displaying that shows detail of the log update
impl fmt::Display for LogUpdate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // If there is a broadcast event, write the formatted ID
            &LogUpdate::Broadcast(ref event, ..) => write!(f, "Broadcast: {}", event),

            // If there is a current event, write the formatted ID
            &LogUpdate::Current(ref current_event) => write!(f, "Now Playing: {}", current_event),

            // If there is an error, simply write the string
            &LogUpdate::Error(ref error, ..) => write!(f, "ERROR: {}", error),

            // If there is data to save, write it
            &LogUpdate::Save(ref data) => write!(f, "Got Data: {:?}", data),

            // If there is a status change, copy it
            &LogUpdate::Status(ref status_id, ref state) => {
                write!(f, "Status: {} Now {}", status_id, state)
            }

            // If there is a system update, simply write the string
            &LogUpdate::Update(ref update) => write!(f, "Update: {}", update),

            // If there is a warning, simply write the string
            &LogUpdate::Warning(ref warning, ..) => write!(f, "WARNING: {}", warning),
        }
    }
}

/// A macro that allows the user to quickly and easily send status updates over
/// the update line to the rest of the system.
///
macro_rules! log {

    // Take a mpsc line and error type of LogUpdate
    (err $line:expr => $($arg:tt)*) => ({
        // Import necessary features
        use std::fmt::Write;
        use crate::definitions::LogUpdate;

        // Attempt to format the string
        let mut s = String::new();
        s.write_fmt(format_args!($($arg)*)).unwrap_or(());

        // Send the error to the mpsc line
        $line.send_update(LogUpdate::Error(s, None)).await;
    });

    // Take a mpsc line and error type of LogUpdate with an event id
    (errevent $line:expr => $event:expr => $($arg:tt)*) => ({
        // Import necessary features
        use std::fmt::Write;
        use crate::definitions::LogUpdate;

        // Attempt to format the string
        let mut s = String::new();
        s.write_fmt(format_args!($($arg)*)).unwrap_or(());

        // Send the error to the mpsc line
        $line.send_update(LogUpdate::Error(s, Some($event))).await;
    });

    // Take a mpsc line and warning type of LogUpdate
    (warn $line:expr => $($arg:tt)*) => ({
        // Import necessary features
        use std::fmt::Write;
        use crate::definitions::LogUpdate;

        // Attempt to format the string
        let mut s = String::new();
        s.write_fmt(format_args!($($arg)*)).unwrap_or(());

        // Send the warning to the mpsc line
        $line.send_update(LogUpdate::Warning(s, None)).await;
    });

    // Take a mpsc line and warning type of LogUpdate with an event id
        // Take a mpsc line and warning type of LogUpdate
    (warnevent $line:expr => $event:expr => $($arg:tt)*) => ({
        // Import necessary features
        use std::fmt::Write;
        use crate::definitions::LogUpdate;

        // Attempt to format the string
        let mut s = String::new();
        s.write_fmt(format_args!($($arg)*)).unwrap_or(());

        // Send the warning to the mpsc line
        $line.send_update(LogUpdate::Warning(s, Some($event))).await;
    });

    // Take a mpsc line and broadcast type of event update
    (broadcast $line:expr => $event:expr, $data:expr) => ({
        // Import necessary features
        use crate::definitions::LogUpdate;

        // Send an update to the mpsc line
        $line.send_update(LogUpdate::Broadcast($event, $data)).await;
    });

    // Take a mpsc line and current type of event update
    (now $line:expr => $event:expr) => ({
        // Import necessary features
        use crate::definitions::LogUpdate;

        // Send an update to the mpsc line
        $line.send_update(LogUpdate::Current($event)).await;
    });

    // Take a mpsc line and status type of event update
    (status $line:expr => $group_id:expr, $status:expr) => ({
        // Import necessary features
        use crate::definitions::LogUpdate;

        // Send an update to the mpsc line
        $line.send_update(LogUpdate::Status($group_id, $status)).await;
    });

    // Take a mpsc line and save type of event update
    (save $line:expr => $data:expr) => ({
        // Import necessary features
        use crate::definitions::LogUpdate;

        // Send an update to the mpsc line
        $line.send_update(LogUpdate::Save($data)).await;
    });

    // Take a mpsc line and update type of event update
    (update $line:expr => $($arg:tt)*) => ({
        // Import necessary features
        use crate::definitions::LogUpdate;

        // Import the standard library
        use std::fmt::Write;

        // Attempt to format the string
        let mut s = String::new();
        s.write_fmt(format_args!($($arg)*)).unwrap_or(());

        // Send the update to the mpsc line
        $line.send_update(LogUpdate::Update(s)).await;
    });
}

/// An enum to contain system notifications in different types.
///
/// This notification type mirrors the log update type, but is only allowed
/// to contain strings for display to the user and the system time of the
/// notification (no other types, as in event update). This type also omits
/// several of the variants described in the event update as they should not
/// be displayed to the user.
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Notification {
    /// An error type of notification
    Error {
        message: String,
        time: NaiveDateTime,
        event: Option<ItemPair>,
    },

    /// A warning type of notification
    Warning {
        message: String,
        time: NaiveDateTime,
        event: Option<ItemPair>,
    },

    /// A current event type of notification
    Current {
        message: String,
        time: NaiveDateTime,
    },

    /// Any other type of internal update
    Update {
        message: String,
        time: NaiveDateTime,
    },
}

// Implement key features for the Notification type
impl Notification {
    /// A function to return a copy of the message inside the notification,
    /// regardless of variant.
    ///
    #[allow(dead_code)]
    pub fn message(&self) -> String {
        match self {
            // For every variant type, return a copy of the message
            &Notification::Error { ref message, .. } => message.clone(),
            &Notification::Warning { ref message, .. } => message.clone(),
            &Notification::Current { ref message, .. } => message.clone(),
            &Notification::Update { ref message, .. } => message.clone(),
        }
    }

    /// A function to return a copy of the time inside the notification,
    /// regardless of variant.
    ///
    pub fn time(&self) -> NaiveDateTime {
        match self {
            // For every variant type, return a copy of the message
            &Notification::Error { ref time, .. } => time.clone(),
            &Notification::Warning { ref time, .. } => time.clone(),
            &Notification::Current { ref time, .. } => time.clone(),
            &Notification::Update { ref time, .. } => time.clone(),
        }
    }
}

// Implement the display formatting for notifications.
impl fmt::Display for Notification {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // For every variant type, combine the message and notification time
            &Notification::Error {
                ref message,
                ref time,
                ..
            } => write!(f, "{}: {}", time.format("%F %T"), message),
            &Notification::Warning {
                ref message,
                ref time,
                ..
            } => write!(f, "{}: {}", time.format("%F %T"), message),
            &Notification::Current {
                ref message,
                ref time,
            } => write!(f, "{}: {}", time.format("%F %T"), message),
            &Notification::Update {
                ref message,
                ref time,
            } => write!(f, "{}: {}", time.format("%F %T"), message),
        }
    }
}

// Tests of the update module
#[cfg(test)]
mod tests {
    use super::*;

    // Test the log! macro
    #[tokio::test]
    async fn log_macro() {
        // Import libraries for testing
        use crate::definitions::{InternalSend, InternalUpdate};

        // Create the receiving line
        let (tx, mut rx) = InternalSend::new();

        // Generate a few messages
        log!(err tx => "Test Error {}", 1);
        log!(warn tx => "Test Warning {}", 2);
        log!(broadcast tx => ItemId::new_unchecked(3), None);
        log!(now tx => ItemId::new_unchecked(4));
        log!(update tx => "Test Update {}", "5");

        // Create the test vector
        let test = vec![
            InternalUpdate::Update(LogUpdate::Error("Test Error 1".to_string(), None)),
            InternalUpdate::Update(LogUpdate::Warning("Test Warning 2".to_string(), None)),
            InternalUpdate::Update(LogUpdate::Broadcast(ItemId::new_unchecked(3), None)),
            InternalUpdate::Update(LogUpdate::Current(ItemId::new_unchecked(4))),
            InternalUpdate::Update(LogUpdate::Update("Test Update 5".to_string())),
        ];

        // Print and check the messages received (wait at most half a second)
        test_vec!(=rx, test);
    }
}
