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

//! This module implements the scene structure for grouping events to
//! limit access to one set of events at a time.

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
    pub items: FnvHashSet<ItemId>, // hash set of the items in this scene (excluding groups)
    pub groups: FnvHashSet<ItemId>, // hash set of the groups in this scene
    pub key_map: Option<KeyMapId>, // an optional mapping of key codes to events
}

/// A structure to define the parameters of a scene, web version
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebScene {
    pub items: FnvHashSet<ItemId>, // hash set of the items in this scene (including group ids)
    pub key_map: Option<KeyMapId>, // an optional mapping of key codes to events
}

// Implement conversion from Scene to WebScene
impl From<Scene> for WebScene {
    fn from(mut scene: Scene) -> Self {
        // Combine the items and groups lists
        scene.items.extend(&scene.groups);

        // Recompose as a WebScene
        Self {
            items: scene.items,
            key_map: scene.key_map,
        }
    }
}
