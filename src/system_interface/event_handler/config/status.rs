// Copyright (c) 2019 Decode Detroit
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

//! This module implements the status handler to maintain the status of the
//! system. This handler stores the status locally (though it may also be
//! syncronized via the backup module).
//!
//! This module also implements low level status structs and enums which
//! facilitate the storage of the status and the current state of that status.

// Import the relevant structures into the correct namespace
use super::super::super::GeneralUpdate;
use super::super::event::EventUpdate;
use super::super::item::{ItemDescription, ItemId, ItemPair};

// Import FNV HashMap
extern crate fnv;
use self::fnv::FnvHashMap;

/// A type to store a hashmap of status ids and status descriptions
///
pub type StatusMap = FnvHashMap<ItemId, StatusDetail>; // a hash map of status id and status detail pairs

/// A type to store a vector of status ids and status descriptions
///
pub type FullStatus = FnvHashMap<ItemPair, StatusDescription>; // a hash map of status id pairs and status description pairs

/// A structure which holds the local status and manages any state changes.
///
/// # Notes
///
/// This module only holds and modifies the local copy of the system status. An
/// additional copy may be held in the backup module.
///
pub struct StatusHandler {
    status_map: StatusMap,      // hash map of the local status
    update_line: GeneralUpdate, // the update line for posting any warnings
}

// Implement key features for the status handler
impl StatusHandler {
    /// A function to create and return a new status handler.
    ///
    /// # Errors
    ///
    /// This function does not return any errors or warnings.
    ///
    /// Like all StatusHandler functions and methods, this function will fail
    /// gracefully by notifying of any errors on the update line and returning
    /// None.
    ///
    pub fn new(update_line: GeneralUpdate, status_map: StatusMap) -> StatusHandler {
        // Return the new status handler
        StatusHandler {
            status_map,
            update_line,
        }
    }

    /// A method to get the current state of the requested status. This
    /// method returns the state as an item id.
    ///
    /// # Errors
    ///
    /// This method will raise an error if the provided status id was not found
    /// in the configuration. This usually indicates a problem with the
    /// underlying confirguration file.
    ///
    /// Like all StatusHandler functions and methods, this function will fail
    /// gracefully by notifying of any errors on the update line and returning
    /// None.
    ///
    pub fn get_id(&self, status_id: &ItemId) -> Option<ItemId> {
        // Try to return the local status as an id
        if let Some(detail) = self.status_map.get(status_id) {
            // Return the current state
            return Some(detail.current());

        // Warn that there is an error with the provided status id
        } else {
            update!(err &self.update_line => "Unable To Locate Current State Of Status: {}.", &status_id);
            return None;
        }
    }

    /// A method to modify a status state within the current scene based
    /// on the provided status id and new state. Returns true on success.
    ///
    /// # Errors
    ///
    /// This function will raise an error if the provided id was not found in
    /// the configuration. This usually indicates a problem with the underlying
    /// configuration file.
    ///
    /// Like all StatusHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning false.
    ///
    pub fn modify_status(&mut self, status_id: &ItemId, new_state: &ItemId) -> bool {
        // Try to get a mutable reference to the status detail
        if let Some(status_detail) = self.status_map.get_mut(status_id) {
            // Try to update the status detail
            if !status_detail.update(new_state.clone()) {
                update!(warn &self.update_line => "Selected State Was Not Valid: {}", new_state);
                return false;
            }

            // Indicate success
            return true;

        // Warn the system that this is not a valid id
        } else {
            update!(warn &self.update_line => "Status ID Not Found In Config: {}", status_id);
            return false;
        }
    }

    /// A method to return a copy of the status map inside the status handler.
    ///
    /// # Errors
    ///
    /// This method does not return any errors.
    ///
    pub fn get_map(&self) -> StatusMap {
        self.status_map.clone()
    }

    /// A method to return a vector of the valid status ids in the status handler.
    ///
    /// # Errors
    ///
    /// This method does not return any errors.
    ///
    pub fn get_ids(&self) -> Vec<ItemId> {
        // Compile a list of ids from the status map
        let mut ids = Vec::new();
        for id in self.status_map.keys() {
            ids.push(id.clone());
        }

        // Return the completed list
        ids
    }

    /// A method to return a hashmap of the complete described status map in
    /// this configuration.
    ///
    /// # Errors
    ///
    /// This method will raise an error if one of the status ids was not found in
    /// the lookup. This indicates that the configuration file is incomplete.
    ///
    /// Like all StatusHandler functions and methods, this method will fail
    /// gracefully by notifying of errors on the update line and returning an
    /// empty ItemDescription for that status.
    ///
    pub fn get_full_status<F>(&self, mut get_description: F) -> FullStatus
    where
        F: FnMut(&ItemId) -> ItemDescription,
    {
        // Compile a list of the available statuses
        let mut id_vec = Vec::new();
        for key in self.status_map.keys() {
            id_vec.push(key.clone());
        }

        // Sort the status ids
        id_vec.sort_unstable();

        // Sort them in order and then pair them with their descriptions
        let mut full_status = FullStatus::default();
        for status_id in id_vec {
            // Create a new status pair from the status id
            let description = get_description(&status_id);
            let status_pair = ItemPair::from_item(status_id.clone(), description);

            // Compose the status detail into a status description
            let status_description = match self.status_map.get(&status_id) {
                // The status detail exists
                Some(detail) => {
                    // Repackage as a new status description
                    let current_pair =
                        ItemPair::from_item(detail.current(), get_description(&detail.current()));

                    // Rapackage the allowed states
                    let mut allowed_pairs = Vec::new();
                    for state in detail.allowed() {
                        allowed_pairs
                            .push(ItemPair::from_item(state.clone(), get_description(&state)));
                    }

                    // Return the new status description
                    StatusDescription {
                        current: current_pair,
                        allowed: allowed_pairs,
                    }
                }

                // The description was not found - should not be possible
                None => unreachable!(),
            };

            // Add the status description to the hashmap of statuses
            full_status.insert(status_pair, status_description);
        }

        // Return the result
        full_status
    }
}

/// An enum to hold all status detail variants.
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum StatusDetail {
    /// The MultState variant
    ///
    MultiState {
        current: ItemId,      // the current state
        allowed: Vec<ItemId>, // the allowed states
    },

    /// The CountedState variant
    ///
    CountedState {
        current: ItemId,      // the current state
        trigger: ItemId,      // the state when the status is triggered
        anti_trigger: ItemId, // the state when the status is not triggered
        reset: ItemId,        // the state to reset the status to its default value
        count: u32,           // the current count of the status
        default_count: u32,   // the starting value of the status count
    },
}

// Reexport the status detail variants
use self::StatusDetail::{CountedState, MultiState};

// Implement key features for Status Detail
impl StatusDetail {
    /// A method to return the current state of the status
    ///
    pub fn current(&self) -> ItemId {
        match self {
            &MultiState { ref current, .. } => current.clone(),
            &CountedState { ref current, .. } => current.clone(),
        }
    }

    /// A method to return the allowed states
    ///
    pub fn allowed(&self) -> Vec<ItemId> {
        match self {
            &MultiState { ref allowed, .. } => allowed.clone(),
            &CountedState {
                ref trigger,
                ref anti_trigger,
                ref reset,
                ..
            } => {
                // Create and return the allowed vector
                let mut allowed = Vec::new();
                allowed.push(trigger.clone());
                allowed.push(anti_trigger.clone());
                allowed.push(reset.clone());
                allowed
            }
        }
    }

    /// A method to verify that the specified state is allowed.
    /// This method does not change the current state.
    ///
    pub fn is_allowed(&self, new_state: &ItemId) -> bool {
        match self {
            // The multistate variant
            &MultiState { ref allowed, .. } => {
                // Check if the new state is valid
                allowed.is_empty() | allowed.contains(&new_state)
            }

            // The countedstate variant
            &CountedState {
                ref trigger,
                ref anti_trigger,
                ref reset,
                ..
            } => {
                // Check if the new state is valid
                (*new_state == *trigger) | (*new_state == *anti_trigger) | (*new_state == *reset)
            }
        }
    }

    /// A method to update the state of the status, first checking for
    /// that the new state is valid. If the operation was successful, the
    /// method returns true.
    ///
    pub fn update(&mut self, new_state: ItemId) -> bool {
        match self {
            // The multistate variant
            &mut MultiState {
                ref mut current,
                ref allowed,
                ..
            } => {
                // Check that the new state is valid
                if allowed.is_empty() | allowed.contains(&new_state) {
                    // Update the state
                    *current = new_state;
                    return true;
                }
                false // indicate failure
            }

            // The countedstate variant
            &mut CountedState {
                ref mut current,
                ref mut count,
                ref reset,
                ref default_count,
                ref trigger,
                ref anti_trigger,
                ..
            } => {
                // Reset the count and state
                if new_state == *reset {
                    *count = *default_count; // reset the count
                    *current = *anti_trigger; // reset the current state

                // Increment the count when the anti-trigger is provided
                } else if new_state == *anti_trigger {
                    *count = *count + 1; // increase the count
                    *current = *anti_trigger; // reset the current state

                // Decrement the count when the trigger is provided
                } else if new_state == *trigger {
                    // If the count is not zero, decrease it
                    if *count > 0 {
                        *count = *count - 1;
                    }

                    // If the count is now zero, change the current state
                    if *count == 0 {
                        *current = *trigger;
                    }

                // Otherwise report failure
                } else {
                    return false;
                }
                true // indicate success
            }
        }
    }
}

/// A struct which allows a limited number of possible states. This version
/// uses fully described itempairs for use with the user interface. If the
/// allowed state vector is empty, any state will be allowed.
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct StatusDescription {
    pub current: ItemPair,
    pub allowed: Vec<ItemPair>,
}

// Tests of the status module
#[cfg(test)]
mod tests {
    use super::*;

    // FIXME Define tests of this module
    #[test]
    fn test_status() {
        // FIXME: Implement this
        unimplemented!();
    }
}
