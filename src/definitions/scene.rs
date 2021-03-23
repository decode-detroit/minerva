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

//! This module implements the scene structure for grouping events.

// Import crate definitions
use crate::definitions::*;

// Import FNV HashMap and HashSet
use fnv::{FnvHashMap, FnvHashSet};

/// Define the itemid and itempair definition of a key map
///
type KeyMapId = FnvHashMap<u32, ItemId>;
pub type KeyMap = FnvHashMap<u32, ItemPair>;

/// A structure to define the parameters of a scene
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    pub events: FnvHashSet<ItemId>, // hash set of the events in this scene
    pub key_map: Option<KeyMapId>,  // an optional mapping of key codes to events
}

/// A structure to define the parameters of a scene, storing the ItemPairs
/// as opposed to only ItemIds
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct DescriptiveScene {
    pub events: Vec<ItemPair>,     // a vector of the events and descriptions in this scene
    pub key_map: Option<FnvHashMap<u32, ItemPair>>,  // an optional mapping of event ids to key codes
}
