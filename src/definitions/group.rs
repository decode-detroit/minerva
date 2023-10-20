// Copyright (c) 2017-2023 Decode Detroit
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

//! This module implements the group structure for grouping events.
//! Groups do not limit access and are useful for organizing events.

// Import crate definitions
use crate::definitions::*;

// Import FNV HashSet
use fnv::FnvHashSet;

/// A structure to define the parameters of a group
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct Group {
    pub items: FnvHashSet<ItemId>, // hash set of the items in this scene
    pub is_hidden: bool,           // a flag to indicate whether the items in the group are visible
}

/// A structure to define the parameters of a group, web version
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebGroup {
    pub items: FnvHashSet<ItemId>, // hash set of the items in this scene
    pub is_hidden: bool,           // a flag to indicate whether the items in the group are visible
}

// Implement conversion to and from Group and WebGroup
impl From<WebGroup> for Group {
    fn from(group: WebGroup) -> Self {
        // Recompose as a Group
        Self {
            items: group.items,
            is_hidden: group.is_hidden,
        }
    }
}
impl From<Group> for WebGroup {
    fn from(group: Group) -> Self {
        // Recompose as a WebGroup
        Self {
            items: group.items,
            is_hidden: group.is_hidden,
        }
    }
}
