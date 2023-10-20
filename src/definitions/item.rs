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

//! This module defines a basic identifier system (ItemId, ItemDescription,
//! and ItemPair) to allow robust identification of all events,
//! scenes, and other items in the program.

// Import standard library modules
use std::cmp::Ordering;
use std::fmt;
use std::hash;

/// Define the All Stop command (a.k.a. emergency stop)
const ALL_STOP: u32 = 0;

/// This structure is a generic identifier for a configuration element (e.g. event, scene, status).
///
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct ItemId {
    id: u32,
}

// Implement key ItemId struct features
impl ItemId {
    /// A function to create an item id from u32, CAN-compliant version.
    ///
    /// When program is built with CAN checking ON (default), this function will cap valid
    /// IDs at 29 bits (i.e. 2^29 or roughly half a million IDs). If you need
    /// additional IDs (unlikely) and are not using a CAN bus, you can
    /// disable this checking by rebuilding the program with the no_can_limit
    /// feature.
    ///
    /// # Disabling CAN Checking
    ///
    /// Your program was likely compiled with CAN checking ON. Rebuild your
    /// program without CAN checking with
    ///
    /// ```
    /// cargo build --features no_can_limit
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// let item = ItemId::new(1);
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return None if the address exceeds the 29-bit
    /// address limit or conflicts with the ALL_STOP id.
    ///
    #[cfg(not(feature = "no_can_limit"))]
    pub fn new(id: u32) -> Option<ItemId> {
        // Verify id does not conflict with all stop
        if id == ALL_STOP {
            return None;
        }

        // Verify 29 bit limit for CAN bus
        if id >= 0x1FFFFFFF {
            return None;
        }

        // Return the new item id
        Some(ItemId { id })
    }

    /// A function to create an item id from u32, no CAN version.
    ///
    /// When the program is built with CAN checking OFF, this function will not
    /// check IDs validity for the CAN range. This should not be used in
    /// conjunction with a CAN bus.
    ///
    /// # Enabling CAN Checking
    ///
    /// Your program was likely compiled with CAN checking OFF. Rebuild
    /// your program with CAN checking (enabled by default) with
    ///
    /// ```
    /// cargo build
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// let item = ItemId::new(0xFFFFFFFF);
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return None if the address conflicts with the
    /// ALL_STOP id.
    ///
    #[cfg(feature = "no_can_limit")]
    pub fn new(id: u32) -> Option<ItemId> {
        // Verify id does not conflict with all stop
        if id == ALL_STOP {
            return None;
        }

        // Return the new item id
        Some(ItemId { id })
    }

    /// A function to create an item id from u32, unchecked version.
    ///
    /// This function does not verify that the new ItemId complies with the CAN
    /// limit and does not check that the new ItemId does not collide with the
    /// all stop item id. This is useful when either (or both) of these cases
    /// are possible and desired or if the valid range is checked elsewhere.
    ///
    pub fn new_unchecked(id: u32) -> ItemId {
        ItemId { id }
    }

    /// A method to return the id of the item.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = ItemId::new(5);
    /// assert_eq!(5, id.id());
    /// ```
    ///
    pub fn id(&self) -> u32 {
        self.id
    }

    /// A function to return a new all stop item id. This is a reserved id for
    /// halting all active processes returning operations to static state.
    ///
    pub fn all_stop() -> ItemId {
        ItemId { id: ALL_STOP }
    }
}

// Implement displaying that shows the ID
impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

/// This structure is a human-readable description and formatting parameters
/// to be paired with an ItemId.
///
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ItemDescription {
    pub description: String,
}

// Implement key ItemDescription struct features
impl ItemDescription {
    /// A function to create an item description from &str
    ///
    pub fn new(description: &str) -> ItemDescription {
        ItemDescription {
            description: description.to_string(),
        }
    }

    /// A function to create a default item description.
    ///
    #[allow(dead_code)]
    pub fn new_default() -> ItemDescription {
        ItemDescription {
            description: "No Description".to_string(),
        }
    }
}

// Implement displaying that shows the description but not the formatting
impl fmt::Display for ItemDescription {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

/// This structure is paried combination of an ItemId and ItemDescription.
///
#[derive(Clone, Eq, Debug, Serialize, Deserialize)]
pub struct ItemPair {
    id: u32,
    pub description: String,
}

// Implement key ItemPair features
impl ItemPair {
    /// A function to create an item pair from u32, CAN-compliant version.
    ///
    /// When program is built with CAN checking ON, this function will cap valid
    /// IDs at 29 bits (i.e. 2^29 or over half a million IDs). If you need
    /// additional IDs (unlikely) and are not using a CAN bus, you can
    /// disable this checking by rebuilding the program with the no_can_limit
    /// feature.
    ///
    /// # Disabling CAN Checking
    ///
    /// Your program was likely compiled with CAN checking ON. Rebuild your
    /// program without CAN checking with
    ///
    /// ```
    /// cargo build --features no_can_limit
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// let item = ItemPair::new(1, "Example Description");
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return None if the address exceeds the 29-bit
    /// address limit or conflicts with the ALL_STOP id.
    ///
    #[cfg(not(feature = "no_can_limit"))]
    pub fn new(id: u32, description: &str) -> Option<ItemPair> {
        // Verify id does not conflict with all stop
        if id == ALL_STOP {
            return None;
        }

        // Verify 29 bit limit for CAN bus
        if id >= 0x1FFFFFFF {
            return None;
        }

        // Return the new item pair
        Some(ItemPair {
            id,
            description: description.to_string(),
        })
    }

    /// A function to create an item pair from u32, no CAN version.
    ///
    /// When program is built with CAN checking OFF, this function will not
    /// check IDs validity for the CAN range. This should not be used in
    /// conjunction with a CAN bus.
    ///
    /// # Enabling CAN Checking
    ///
    /// Your program was likely compiled with CAN checking OFF. Rebuild
    /// your program with CAN checking (enabled by default) with
    ///
    /// ```
    /// cargo build
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// let item = ItemId::new(0xFFFFFFFF, "Example Description");
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return None if the address conflicts with the
    /// ALL_STOP id.
    ///
    #[cfg(feature = "no_can_limit")]
    pub fn new(id: u32, description: &str) -> Option<ItemPair> {
        // Verify id does not conflict with all stop
        if id == ALL_STOP {
            return None;
        }

        // Return the new item pair
        Some(ItemPair {
            id,
            description: description.to_string(),
        })
    }

    /// A function to create an item id from u32, unchecked version.
    ///
    /// This function does not verify that the new ItemPair complies with the
    /// CAN limit and does not check that the new ItemId does not collide with
    /// the all stop item id. This is useful when either (or both) of these
    /// cases are possible and desired or if the id is checked elsewhere.
    ///
    pub fn new_unchecked(id: u32, description: &str) -> ItemPair {
        ItemPair {
            id,
            description: description.to_string(),
        }
    }

    /// A function to create an item id from u32 with default fields for the other entries.
    ///
    /// This function does not verify that the new ItemPair complies with the
    /// CAN limit and does not check that the new ItemId does not collide with
    /// the all stop item id. This is useful when either (or both) of these
    /// cases are possible and desired or if it was checked elsewhere.
    ///
    pub fn new_default(id: u32) -> ItemPair {
        ItemPair::from_item(ItemId::new_unchecked(id), ItemDescription::new_default())
    }

    /// A function to create an item pair from ItemId and ItemDescription.
    ///
    /// This function takes a valid ItemId and ItemDescription and combines them
    /// into a single ItemPair. Both inputs are consumed in the creation of the
    /// ItemPair.
    ///
    pub fn from_item(id: ItemId, description: ItemDescription) -> ItemPair {
        ItemPair {
            id: id.id,
            description: description.description,
        }
    }

    /// A method to return the id of the itempair.
    ///
    /// # Examples
    ///
    /// ```
    /// let id_pair = ItemPair::new_default(5);
    /// assert_eq!(5, id_pair.id());
    /// ```
    ///
    pub fn id(&self) -> u32 {
        self.id
    }

    /// A method to produce a new ItemID from the ItemPair.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = ItemId::new(5);
    /// let id_pair = ItemPair::new_default(5);
    /// assert_eq!(id, id_pair.get_id());
    /// ```
    ///
    pub fn get_id(&self) -> ItemId {
        ItemId { id: self.id }
    }

    /// A method to return the cloned description of the ItemPair.
    ///
    /// # Examples
    ///
    /// ```
    /// let description = String::new("A Description");
    /// let id_pair = ItemPair::new(5, "A Description");
    /// assert_eq!(description, id_pair.description());
    /// ```
    ///
    pub fn description(&self) -> String {
        self.description.clone()
    }

    /// A method to produce a new ItemDescription from the ItemPair.
    ///
    /// # Examples
    ///
    /// ```
    /// let id_description = ItemDescription:new("A Description");
    /// let id_pair = ItemPair::new(5, "A Description");
    /// assert_eq!(id_description, id_pair.get_description());
    /// ```
    ///
    pub fn get_description(&self) -> ItemDescription {
        ItemDescription {
            description: self.description.clone(),
        }
    }

    /// A function to return a new all stop item pair. This is a reserved id for
    /// halting all active processes and returning operations to a static state.
    ///
    pub fn all_stop() -> ItemPair {
        ItemPair {
            id: ALL_STOP,
            description: "ALL STOP".to_string(),
        }
    }
}

// Implement hashing which ignores the description
impl hash::Hash for ItemPair {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

// Implement partialEq which ignores the description
/// The default equality operation will only compare the ids of the ItemPair.
///
impl PartialEq for ItemPair {
    fn eq(&self, rhs: &ItemPair) -> bool {
        self.id == rhs.id
    }
}

// Implement Ord which ignores the description
impl Ord for ItemPair {
    fn cmp(&self, other: &ItemPair) -> Ordering {
        self.id.cmp(&other.id)
    }
}

// Implement partialOrd which ignores the description, based off cmp
impl PartialOrd for ItemPair {
    fn partial_cmp(&self, other: &ItemPair) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Implement displaying that shows both ID and description
impl fmt::Display for ItemPair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({})", self.description, self.id)
    }
}

// Tests of the item module
#[cfg(test)]
mod tests {
    use super::*;

    // Test id with CAN bus limiter
    #[test]
    #[cfg_attr(not(feature = "no_can_limit"), should_panic)]
    fn create_id() {
        // Try to create an id out of CAN range
        let _e = ItemId::new(0xFFFFFFFF).unwrap();
    }

    // Test comparison of simple item pairs
    #[test]
    fn compare_ids() {
        // Create several events
        let id = ItemPair::new(1, "One Event").unwrap();
        let same_id = id.clone();
        let different_description = ItemPair::new(1, "Different Description").unwrap();
        let different_id = ItemPair::new(2, "Two Event").unwrap();

        // Compare the events
        assert_eq!(id, same_id);
        assert_eq!(id, different_description);
        assert_ne!(id, different_id);
        assert_ne!(same_id, different_id);
    }
}
