// Copyright (c) 2024 Decode Detroit
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

//! This module implements structures shared from the dmx interface

// Import standard library features
use std::path::PathBuf;

// Import FNV HashSet
use fnv::FnvHashMap;

// Define the DMX constants
pub const DMX_MAX: u32 = 512; // the highest channel of DMX, exclusive

/// A type definition for one complete set of DMX channels
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn as_bytes(&self) -> Vec<u8> {
        // Return the array
        self.values.clone()
    }
}

// A helper struct to define a one comlete set of DMX channels.
// This version is serialized with camelCase to allow compatability with Vulcan.
//
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DmxUniverseHelper {
    pub values: Vec<u8>,
}

// Implement conversion to DmxUniverseHelper
impl From<DmxUniverse> for DmxUniverseHelper {
    fn from(dmx_universe: DmxUniverse) -> Self {
        // Recompose as a media cue helper
        Self {
            values: dmx_universe.values.clone(),
        }
    }
}

/// A struct to hold parameters for Vulcan DMX controller
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VulcanParams {
    pub spawn: bool, // a flag if Minerva should spawn and manage the Vulcan process
    pub path: Option<PathBuf>, // the hardware location where the DMX signals will be sent, required if spawn is true
    pub address: Option<String>, // the address where Vulcan will listen for instructions, defaults to Vulcan on localhost
}

/// A type to hold multiple dmx controllers with a universe number
///
pub type DmxControllers = FnvHashMap<u32, VulcanParams>;

