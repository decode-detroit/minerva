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

//! This module defines the event structure and associated types.

// Import crate definitions
use crate::definitions::*;

// Import standard library modules
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
