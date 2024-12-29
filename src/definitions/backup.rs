// Copyright (c) 2023 Decode Detroit
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

//! This module implements structures shared from the backup handler

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::time::Duration;

// Import FNV HashMap
use fnv::FnvHashMap;

/// A structure to store queued events in a backup-safe format
///
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct QueuedEvent {
    pub remaining: Duration, // the remaining time before the event is triggered
    pub event_id: ItemId,    // id of the event to launch
}

/// Implement time updates for the QueuedEvent
impl QueuedEvent {
    /// A method to add time to the time_since field
    ///
    pub fn update(&mut self, additional_time: Duration) {
        self.remaining = self
            .remaining
            .checked_sub(additional_time)
            .unwrap_or(Duration::from_secs(0)); // default to zero it none left
    }
}

/// A structure to save a media cue with timing information
///
#[derive(Clone, Serialize, Deserialize)]
pub struct MediaPlayback {
    pub media_cue: MediaCue,  // the media information that was cued
    pub time_since: Duration, // the minimum time since the media was cued
}

/// Implement time updates for the MediaPlayback
impl MediaPlayback {
    /// A method to add time to the time_since field
    ///
    pub fn update(&mut self, additional_time: Duration) {
        self.time_since = self
            .time_since
            .checked_add(additional_time)
            .unwrap_or(self.time_since); // keep current time if overflow
    }
}

/// A structure to store the media playbacks in a playlist
pub type MediaPlaylist = FnvHashMap<u32, MediaPlayback>;

/// A structure to store the dmx universes in a set
pub type DmxUniverses = FnvHashMap<u32, DmxUniverse>;
