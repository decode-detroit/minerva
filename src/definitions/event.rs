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

// Import standard library features
use std::time::Duration;

// Import Chrono features
use chrono::{NaiveDateTime, Local};

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

/// A struct to allow easier manipulation of queued events.
/// 
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ComingEvent {
    pub start_time: NaiveDateTime,   // the start time of the event
    pub delay: Duration,        // delay between the start time and the trigger time for the event
    pub event_id: ItemId,       // id of the event to launch
}

// Implement the Coming Event features
impl ComingEvent {
    /// A function to return a new ComingEvent by consuming Duration and
    /// ItemId.
    ///
    pub fn new(delay: Duration, event_id: ItemId) -> ComingEvent {
        ComingEvent {
            start_time: Local::now().naive_local(),
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
        // Calculate the time since the event was queued
        let elapsed = Local::now().naive_local().signed_duration_since(self.start_time);
        
        // Compare the durations, or default to playing the event immediately
        match elapsed.to_std().ok() {
            // If the conversion was a success, perform the calculation
            Some(duration) => self.delay.checked_sub(duration),

            // Default to zero
            None => None,
        }
    }

    /// A method to compare the start time and event id of two coming events.
    /// The method returns true iff both values are equal.
    /// 
    pub fn compare_with(&self, other: &ComingEvent) -> bool {
        (self.event_id == other.event_id) & (self.start_time == other.start_time)
    }
}

/// A small struct that holds and event item pair and the corresponding delay
/// until the event should be triggered. Designed for passing events to the
/// user interface.
///
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpcomingEvent {
    pub event: ItemPair,            // id and description of the event to launch
    pub start_time: NaiveDateTime,  // the original start time of the event
    pub delay: Duration,            // delay between now and the time for the event
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

/// A struct to define a single fade of a DMX channel
///
/// # Note
///
/// Assumes the channels are one-indexed (the DMX standard) rather than
/// zero-indexed.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmxFade {
    pub channel: u32,               // the dmx channel to fade
    pub value: u8,                  // the final value at the end of the fade
    pub duration: Option<Duration>, // the duration of the fade (None if instantaneous)
}

/// A struct to define a single media track to play
///
/// # Note
///
/// The uri format must follow the URI syntax rules. This means local files must
/// by specified like "file:///absolute/path/to/file.mp4".
///
/// If a file is specified in the loop media field, the channel will loop this
/// media when this media completes. This takes priority over the channel loop
/// media field.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaCue {
    pub uri: String, // the location of the video or audio file to play
    pub channel: u32, // the channel of the video or audio. New media sent to the same channel will replace the old media, starting instantly
    pub loop_media: Option<String>, // the location of media to loop after this media is complete
}

// A helper struct to define a single media cue.
// This version is serialized with camelCase to allow compatability with Apollo.
//
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaCueHelper {
    pub uri: String,
    pub channel: u32,
    pub loop_media: Option<String>,
}

// Implement conversion to MediaCueHelper
impl From<MediaCue> for MediaCueHelper {
    fn from(media_cue: MediaCue) -> Self {
        // Recompose as a media cue helper
        MediaCueHelper {
            uri: media_cue.uri,
            channel: media_cue.channel,
            loop_media: media_cue.loop_media,
        }
    }
}

/// An enum to define the adjustment directions
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdjustmentDirection {
    Up,
    Down,
    Left,
    Right,
}

/// An enum to define the adjustment directions
// This version is serialized with camelCase to allow compatability with Apollo.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AdjustmentDirectionHelper {
    Up,
    Down,
    Left,
    Right,
}

// Implement conversion to AdjustmentDirectionHelper
impl From<AdjustmentDirection> for AdjustmentDirectionHelper {
    fn from(direction: AdjustmentDirection) -> Self {
        // Map the enums
        match direction {
            AdjustmentDirection::Up => AdjustmentDirectionHelper::Up,
            AdjustmentDirection::Down => AdjustmentDirectionHelper::Down,
            AdjustmentDirection::Left => AdjustmentDirectionHelper::Left,
            AdjustmentDirection::Right => AdjustmentDirectionHelper::Right,
        }
    }
}

/// A struct to define an adjustment for a media channel
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaAdjustment {
    pub channel: u32, // the channel of the video
    pub direction: AdjustmentDirection, // the direction to adjust the video frame
}


// A helper struct to define a media adjustment.
// This version is serialized with camelCase to allow compatability with Apollo.
//
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAdjustmentHelper {
    pub channel: u32, // the channel of the video
    pub direction: AdjustmentDirectionHelper, // the direction to adjust the video frame
}

// Implement conversion to MediaAdjustmentHelper
impl From<MediaAdjustment> for MediaAdjustmentHelper {
    fn from(adjustment: MediaAdjustment) -> Self {
        // Recompose as a media cue helper
        MediaAdjustmentHelper {
            channel: adjustment.channel,
            direction: adjustment.direction.into(),
        }
    }
}

/// An enum with various action options for each event.
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum EventAction {
    /// A variant to adjust media on one of the media channels.
    AdjustMedia { adjustment: MediaAdjustment },

    /// A variant that links to one or more events to cancel. All upcoming
    /// events that match the specified id(s) will be cancelled.
    CancelEvent { event: ItemId },

    // A variant to cue a DMX fade on one of the channels
    CueDmx { fade: DmxFade },

    /// A variant that links to one event to add to the queue. These events may
    /// be triggered immediately when delay is None, or after a delay if delay
    /// is Some(delay).
    CueEvent { event: EventDelay },

    /// A variant to cue media on one of the media channels.
    CueMedia { cue: MediaCue },

    /// A variant used to change current status of the target status.
    ModifyStatus {
        status_id: ItemId,
        new_state: ItemId,
    },

    /// A variant indicating a complete change in scene.
    NewScene { new_scene: ItemId },

    /// A variant which contains a vector of data to save in the current game
    /// logging file.
    SaveData { data: DataType },

    /// A variant which selects an event based on the state of the indicated
    /// status.
    SelectEvent {
        status_id: ItemId,
        event_map: FnvHashMap<ItemId, ItemId>,
    },

    /// A variant which contains a type of data to include with the event
    /// when broadcast to the system
    SendData { data: DataType },
}

/// An web-safe (JSON readable) enum with various action options for each event.
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum WebEventAction {
    /// A variant to adjust media on one of the media channels.
    AdjustMedia { adjustment: MediaAdjustment },

    /// A variant that links to one or more events to cancel. All upcoming
    /// events that match the specified id(s) will be cancelled.
    CancelEvent { event: ItemId },

    // A variant to cue a DMX fade on one of the channels
    CueDmx { fade: DmxFade },

    /// A variant that links to one event to add to the queue These events may
    /// be triggered immediately when delay is None, or after a delay if delay
    /// is Some(delay).
    CueEvent { event: EventDelay },

    /// A variant to cue media on one of the media channels.
    CueMedia { cue: MediaCue },

    /// A variant used to change current status of the target status.
    ModifyStatus {
        status_id: ItemId,
        new_state: ItemId,
    },

    /// A variant indicating a complete change in scene.
    NewScene { new_scene: ItemId },

    /// A variant which contains a vector of data to save in the current game
    /// logging file.
    SaveData { data: DataType },

    /// A variant which selects an event based on the state of the indicated
    /// status.
    SelectEvent {
        status_id: ItemId,
        event_map: FnvHashMap<u32, ItemId>,
    },

    /// A variant which contains a type of data to include with the event
    /// when broadcast to the system
    SendData { data: DataType },
}

// Implement conversions to and from WebEventAction
impl From<EventAction> for WebEventAction {
    fn from(event_action: EventAction) -> Self {
        match event_action {
            // Convert keys to u32 for Select Event
            EventAction::SelectEvent { status_id, mut event_map } => {
                // Remap the ItemIds as u32
                let mut new_event_map = FnvHashMap::default();
                for (key, value) in event_map.drain() {
                    new_event_map.insert(key.id(), value);
                }

                // Return the completed select event
                WebEventAction::SelectEvent { status_id, event_map: new_event_map }
            }

            // Leave the rest untouched
            EventAction::AdjustMedia { adjustment } => WebEventAction::AdjustMedia { adjustment },
            EventAction::CancelEvent { event } => WebEventAction::CancelEvent { event }, 
            EventAction::CueDmx { fade } => WebEventAction::CueDmx { fade },
            EventAction::CueEvent { event } => WebEventAction::CueEvent { event },
            EventAction::CueMedia { cue } => WebEventAction::CueMedia { cue },
            EventAction::ModifyStatus { status_id, new_state } => WebEventAction::ModifyStatus { status_id, new_state },
            EventAction::NewScene { new_scene } => WebEventAction::NewScene { new_scene },
            EventAction::SaveData { data } => WebEventAction::SaveData { data },
            EventAction::SendData { data } => WebEventAction::SendData { data },
        }
    }
}
impl From<WebEventAction> for EventAction {
    fn from(web_event_action: WebEventAction) -> Self {
        match web_event_action {
            // Convert keys to ItemId for Select Event
            WebEventAction::SelectEvent { status_id, mut event_map } => {
                // Remap the ItemIds as u32
                let mut new_event_map = FnvHashMap::default();
                for (key, value) in event_map.drain() {
                    new_event_map.insert(ItemId::new_unchecked(key), value); // Possible injection attack surface (i.e. an id over 29 bits). Minimal consequenses of this attack
                }

                // Return the completed select event
                EventAction::SelectEvent { status_id, event_map: new_event_map }
            }

            // Leave the rest untouched
            WebEventAction::AdjustMedia { adjustment } => EventAction::AdjustMedia { adjustment },
            WebEventAction::CancelEvent { event } => EventAction::CancelEvent { event },
            WebEventAction::CueDmx { fade } => EventAction::CueDmx { fade },
            WebEventAction::CueEvent { event } => EventAction::CueEvent { event },
            WebEventAction::CueMedia { cue } => EventAction::CueMedia { cue },
            WebEventAction::ModifyStatus { status_id, new_state } => EventAction::ModifyStatus { status_id, new_state },
            WebEventAction::NewScene { new_scene } => EventAction::NewScene { new_scene },
            WebEventAction::SaveData { data } => EventAction::SaveData { data },
            WebEventAction::SendData { data } => EventAction::SendData { data },
        }
    }
}


/// A convenient type definition to specify each event
///
pub type Event = Vec<EventAction>;

/// A convenient web-safe structure definition to specify each event
/// (Rust doesn't allow trait definitions for two implicit types)
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct WebEvent {
    actions: Vec<WebEventAction>,
}

// Implement conversions to and from WebEvent
impl From<Event> for WebEvent {
    fn from(mut event: Event) -> Self {
        // Convert each action to the opposite type and return the result
        let mut actions = Vec::new();
        for action in event.drain(..) {
            actions.push(action.into());
        }
        WebEvent { actions }
    }
}
impl From<WebEvent> for Event {
    fn from(mut web_event: WebEvent) -> Self {
        // Convert each action to the opposite type and return the result
        let mut event = Event::new();
        for action in web_event.actions.drain(..) {
            event.push(action.into());
        }
        event
    }
}

// Reexport the event action type variants
pub use self::EventAction::{
    AdjustMedia, CancelEvent, CueDmx, CueEvent, CueMedia, ModifyStatus, NewScene, SaveData, SelectEvent, SendData,
};
