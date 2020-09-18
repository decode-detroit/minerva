// Copyright (c) 2017 Decode Detroit
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

//! This module implements low level event structs and associated enums which
//! facilitate the passing and monitoring of events.

// Import the relevant structures into the correct namespace
use super::item::{ItemId, ItemPair};

// Import standard library modules
use std::fmt;
use std::time::{Duration, Instant};

// Import FNV HashMap
use fnv::FnvHashMap;

/// A small struct that holds and event id and the corresponding delay until the
/// event should be triggered. This delay is an Option<delay> to allow the
/// possibility for events to trigger immediately.
///
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct EventDelay {
    delay: Option<Duration>, // delay between now and the time for the event
    event_id: ItemId,        // id of the event to launch
}

// Implement the event delay functions
impl EventDelay {
    /// A function to return a new EventDelay by consuming and Duration and
    /// ItemId.
    ///
    pub fn new(delay: Option<Duration>, event_id: ItemId) -> EventDelay {
        EventDelay { delay, event_id }
    }

    /// A method to return a copy of the event id
    ///
    pub fn id(&self) -> ItemId {
        self.event_id.clone()
    }

    /// A method to return a Duration which indicates the delay between now
    /// and the moment when the event should be triggered.
    ///
    pub fn delay(&self) -> Option<Duration> {
        self.delay.clone()
    }
}

/// A small struct that holds and event item pair and the corresponding delay
/// until the event should be triggered. Designed for passing events to the
/// user interface.
///
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UpcomingEvent {
    pub event: ItemPair,     // id and description of the event to launch
    pub start_time: Instant, // the original start time of the event
    pub delay: Duration,     // delay between now and the time for the event
}

/// An enum with the types of data available to be saved and sent
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum DataType {
    /// A variant for time until an event
    TimeUntil {
        event_id: ItemId, // the event of interest
    },

    /// A variant for time passed until an event is triggered
    TimePassedUntil {
        event_id: ItemId,     // the event of interest
        total_time: Duration, // the total duration until the event is normally triggered
    },

    /// A variant for a predetermined string
    StaticString {
        string: String, // the string, hardcoded into the data
    },

    /// A variant for a string collected from the user
    UserString,
}

/// An enum with various action options for each event.
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum EventAction {
    /// A variant indicating a complete change in scene.
    NewScene { new_scene: ItemId },

    /// A variant used to change current status of the target status.
    ModifyStatus {
        status_id: ItemId,
        new_state: ItemId,
    },

    /// A variant that links to one event to add to the queue These events may
    /// be triggered immediately when delay is None, or after a delay if delay
    /// is Some(delay).
    CueEvent { event: EventDelay },

    /// A variant that links to one or more events to cancel. All upcoming
    /// events that match the specified id(s) will be cancelled.
    CancelEvent { event: ItemId },

    /// A variant which contains a vector of data to save in the current game
    /// logging file.
    SaveData { data: DataType },

    /// A variant which contains a type of data to include with the event
    /// when broadcast to the system
    SendData { data: DataType },

    /// A variant which selects an event based on the state of the indicated
    /// status.
    SelectEvent {
        status_id: ItemId,
        event_map: FnvHashMap<ItemId, ItemId>,
    },
}

/// A convenient type definition to specify each event
///
pub type Event = Vec<EventAction>;

// Reexport the event action type variants
pub use self::EventAction::{
    CancelEvent, SelectEvent, ModifyStatus, NewScene, CueEvent, SaveData, SendData,
};

/// An enum for updating the rest of the system on changes to the scene and
/// to the current events.
///
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum EventUpdate {
    /// A variant that notifies the rest of the system to broadcast this
    /// event with optional data.
    Broadcast(ItemPair, Option<u32>),

    /// A variant that notifies the rest of the system of a currently playing
    /// event.
    Current(ItemPair),

    // Define several event update types, in order of decreasing priority
    /// A variant which passes unrecoverable errors generated by the system.
    Error(String, Option<ItemPair>),

    /// A variant that notifies the system logger to log data to the game log
    Save(String), // the data to save, formatted as a string

    /// A variant that notifies the rest of the system of the new status of a group
    Status(ItemPair, ItemPair), // first field is the status id, second is the new state

    /// A variant which can send any other type of update to the system.
    Update(String),

    /// A variant which passes recoverable warnings generated by the system.
    Warning(String, Option<ItemPair>),
}

// Reexport the event update type variants
pub use self::EventUpdate::{Broadcast, Current, Error, Save, Status, Update, Warning};

// Implement displaying that shows detail of the event update
impl fmt::Display for EventUpdate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // If there is a broadcast event, write the formatted ID
            &Broadcast(ref event, ..) => write!(f, "Broadcast: {}", event),

            // If there is a current event, write the formatted ID
            &Current(ref current_event) => write!(f, "Now Playing: {}", current_event),

            // If there is an error, simply write the string
            &Error(ref error, ..) => write!(f, "ERROR: {}", error),

            // If there is data to save, write it
            &Save(ref data) => write!(f, "Got Data: {:?}", data),

            // If there is a status change, copy it
            &Status(ref group_id, ref status) => write!(f, "Status: {} Now {}", group_id, status),

            // If there is a system update, simply write the string
            &Update(ref update) => write!(f, "Update: {}", update),

            // If there is a warning, simply write the string
            &Warning(ref warning, ..) => write!(f, "WARNING: {}", warning),
        }
    }
}

/// A macro that allows the user to quickly and easily send status updates over
/// the update line to the rest of the system.
///
macro_rules! update {

    // Take a mpsc line and error type of EventUpdate
    (err $line:expr => $($arg:tt)*) => ({

        // Import the standard library
        use std::fmt::Write;

        // Attempt to format the string
        let mut s = String::new();
        match s.write_fmt(format_args!($($arg)*)) {

            // Send the error to the mpsc line
            Ok(_normal) => {
                $line.send_update(EventUpdate::Error(s, None)).await;
            },

            // Send generic error to the mpsc line
            Err(_error) => {
                $line.send_update(EventUpdate::Error("Unknown Error Occured.".to_string(), None)).await;
            },
        }
    });

    // Take a mpsc line and error type of EventUpdate with an event id
    (errevent $line:expr => $event:expr => $($arg:tt)*) => ({

        // Import the standard library
        use std::fmt::Write;

        // Attempt to format the string
        let mut s = String::new();
        match s.write_fmt(format_args!($($arg)*)) {

            // Send the error to the mpsc line
            Ok(_normal) => {
                $line.send_update(EventUpdate::Error(s, Some($event))).await;
            },

            // Send generic error to the mpsc line
            Err(_error) => {
                $line.send_update(EventUpdate::Error("Unknown Error Occured.".to_string(), None)).await;
            },
        }
    });

    // Take a mpsc line and warning type of EventUpdate
    (warn $line:expr => $($arg:tt)*) => ({

        // Import the standard library
        use std::fmt::Write;

        // Attempt to format the string
        let mut s = String::new();
        match s.write_fmt(format_args!($($arg)*)) {

            // Send the warning to the mpsc line
            Ok(_normal) => {
                $line.send_update(EventUpdate::Warning(s, None)).await;
            },

            // Send generic warning to the mpsc line
            Err(_error) => {
                $line.send_update(EventUpdate::Warning("Unknown Warning Occured.".to_string(), None)).await;
            },
        }
    });

    // Take a mpsc line and warning type of EventUpdate with an event id
        // Take a mpsc line and warning type of EventUpdate
    (warnevent $line:expr => $event:expr => $($arg:tt)*) => ({

        // Import the standard library
        use std::fmt::Write;

        // Attempt to format the string
        let mut s = String::new();
        match s.write_fmt(format_args!($($arg)*)) {

            // Send the warning to the mpsc line
            Ok(_normal) => {
                $line.send_update(EventUpdate::Warning(s, Some($event))).await;
            },

            // Send generic warning to the mpsc line
            Err(_error) => {
                $line.send_update(EventUpdate::Warning("Unknown Warning Occured.".to_string(), None)).await;
            },
        }
    });

    // Take a mpsc line and broadcast type of event update
    (broadcast $line:expr => $event:expr, $data:expr) => ({

        // Send an update to the mpsc line
        $line.send_update(EventUpdate::Broadcast($event, $data)).await;
    });

    // Take a mpsc line and current type of event update
    (now $line:expr => $event:expr) => ({

        // Send an update to the mpsc line
        $line.send_update(EventUpdate::Current($event)).await;
    });

    // Take a mpsc line and status type of event update
    (status $line:expr => $group_id:expr, $status:expr) => ({

        // Send an update to the mpsc line
        $line.send_update(EventUpdate::Status($group_id, $status)).await;
    });

    // Take a mpsc line and save type of event update
    (save $line:expr => $data:expr) => ({

        // Send an update to the mpsc line
        $line.send_update(EventUpdate::Save($data)).await;
    });

    // Take a mpsc line and update type of event update
    (update $line:expr => $($arg:tt)*) => ({

        // Import the standard library
        use std::fmt::Write;

        // Attempt to format the string
        let mut s = String::new();
        match s.write_fmt(format_args!($($arg)*)) {

            // Send the update to the mpsc line
            Ok(_normal) => {
                $line.send_update(EventUpdate::Update(s)).await;
            },

            // Drop the failed update
            Err(_error) => (),
        }
    });
}

// Tests of the event module
#[cfg(test)]
mod tests {
    use super::*;

    // Test the update! macro
    #[test]
    fn test_update() {
        // Import libraries for testing
        use super::super::super::GeneralUpdate;
        use super::super::super::GeneralUpdateType;
        use super::super::item::Hidden;

        // Create the receiving line
        let (tx, rx) = GeneralUpdate::new();

        // Generate a few messages
        update!(err tx => "Test Error {}", 1);
        update!(warn tx => "Test Warning {}", 2);
        update!(broadcast tx => ItemPair::new(3, "Test Event 3", Hidden).unwrap());
        update!(now tx => ItemPair::new(4, "Test Event 4", Hidden).unwrap());
        update!(update tx => "Test Update {}", "5");

        // Create the test vector
        let test = vec![
            GeneralUpdateType::Update(Error("Test Error 1".to_string())),
            GeneralUpdateType::Update(Warning("Test Warning 2".to_string())),
            GeneralUpdateType::Update(Broadcast(ItemPair::new(3, "Test Event 3", Hidden).unwrap())),
            GeneralUpdateType::Update(Current(ItemPair::new(4, "Test Event 4", Hidden).unwrap())),
            GeneralUpdateType::Update(Update("Test Update 5".to_string())),
        ];

        // Print and check the messages received (wait at most half a second)
        test_vec!(=rx, test);
    }
}
