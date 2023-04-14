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

// Define the DMX constants
pub const DMX_MAX: u32 = 512; // the highest channel of DMX, exclusive

/// A type definition for one set of Dmx channels
///
#[derive(Clone, Serialize, Deserialize)]
pub struct DmxUniverse {
    values: Vec<u8>, // Internal representation of the channel values
                     // NOTE: the chennels are internally zero-indexed, rather than the one-indexed standard of DMX
}

/// Implement key features for the DmxUniverse
impl DmxUniverse {
    /// Function to create a new, initialized list of the dmx channels
    ///
    pub fn new() -> Self {
        Self {
            values: vec![0; DMX_MAX as usize],
        }
    }

    /// Method to get the value of a particular channel
    ///
    pub fn get(&self, channel: u32) -> u8 {
        // Check the bounds
        if (channel > DMX_MAX) | (channel < 1) {
            return 0; // default to zero
        }

        // Otherwise, convert to zero-indexed and return the value
        return self.values[channel as usize - 1];
    }

    /// Method to set the value of a paticular channel
    ///
    pub fn set(&mut self, channel: u32, value: u8) {
        // Check the bounds
        if (channel <= DMX_MAX) & (channel > 0) {
            // Convert to zero-indexed and set the value
            self.values[channel as usize - 1] = value;
        } // Otherwise, do nothing
    }

    /// Method to export the universe as a set of bytes
    ///
    /// CAUTION: These bytes are zero-indexed!
    ///
    pub fn as_bytes(&self) -> Vec<u8> {
        // Return the array
        self.values.clone()
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
            .unwrap_or(self.time_since) // keep current time if overflow
    }
}

/// A structure to store the media playbacks in a playlist
pub type MediaPlaylist = FnvHashMap<u32, MediaPlayback>;
