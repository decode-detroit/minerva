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
use super::super::item::{ItemId, ItemPair};

// Import FNV HashMap
use fnv::FnvHashMap;

/// A type to store a hashmap of status ids and status descriptions
///
pub type StatusMap = FnvHashMap<ItemId, Status>; // a hash map of status id and status pairs

/// A type to store a vector of status ids and status descriptions
/// 
/// # FIXME
/// This intermediary should be eliminated and the UI should call for this
/// information as needed.
///
pub type PartialStatus = FnvHashMap<ItemId, StatusPartialDescription>; // a hash map of status ids and status descriptions

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
    pub async fn get_state(&self, status_id: &ItemId) -> Option<ItemId> {
        // Try to return the local status as an id
        if let Some(status) = self.status_map.get(status_id) {
            // Return the current state
            return Some(status.current());

        // Warn that there is an error with the provided status id
        } else {
            update!(err &self.update_line => "Unable To Locate Current State Of Status: {}.", &status_id);
            return None;
        }
    }

    /// A method to get the status of the requested item id.
    ///
    pub fn get_status(&self, status_id: &ItemId) -> Option<Status> {
        // Return the status if found
        match self.status_map.get(status_id) {
            Some(status) => Some(status.clone()),
            None => None,
        }
    }
    
    /// A method to edit an existing status, add a new one, or delete the existing
    ///
    pub async fn edit_status(&mut self, status_id: ItemId, possible_status: Option<Status>, description: String) {
        // If a new status was specified
        if let Some(new_status) = possible_status {
            // If the scene is in the status_map
            if let Some(status) = self.status_map.get_mut(&status_id) {
                // Update the status and notify the system
                *status = new_status;
                update!(update &self.update_line => "Status Updated: {}", description);
            
            // Otherwise, add the status
            } else {
                update!(update &self.update_line => "Status Added: {}", description);
                self.status_map.insert(status_id, new_status);
            }
        
        // If no new status was specified
        } else {
            // If the status is in the status map, remove it
            if let Some(_) = self.status_map.remove(&status_id) {
                // Notify the user that it was removed
                update!(update &self.update_line => "Status Removed: {}", description);
            }
        }
    
    }

    /// A method to modify a status state within the current scene based
    /// on the provided status id and new state. Method returns the new state or
    /// None. None is returned either because
    ///  * the status was already in this state and the status has the
    ///    no_change_silent flag set, or
    ///  * if the state failed to change because one or both ids are invalid.
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
    pub async fn modify_status(&mut self, status_id: &ItemId, new_state: &ItemId) -> Option<ItemId> {
        // Try to get a mutable reference to the status
        if let Some(status) = self.status_map.get_mut(status_id) {
            // Try to update the status and return the result
            status.update(new_state.clone())

        // Warn the system that this is not a valid id
        } else {
            update!(warn &self.update_line => "Status ID Not Found In Config: {}", status_id);
            None
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
    /// This methos does not return any errors
    ///
    pub fn get_partial_status(&self) -> PartialStatus {
        // Compile a list of the available statuses
        let mut id_vec = Vec::new();
        for key in self.status_map.keys() {
            id_vec.push(key.clone());
        }

        // Sort the status ids
        id_vec.sort_unstable();

        // Pair them with their descriptions
        let mut partial_status = PartialStatus::default();
        for status_id in id_vec {
            // Compose the status into a status description
            let status_description = match self.status_map.get(&status_id) {
                // The status exists
                Some(status) => {
                    // Repackage as a new status description
                    StatusPartialDescription {
                        current: status.current(),
                        allowed: status.allowed(),
                    }
                }

                // The description was not found - should not be possible
                None => unreachable!(),
            };

            // Add the status description to the hashmap of statuses
            partial_status.insert(status_id, status_description);
        }

        // Return the result
        partial_status
    }
}

/// An enum to hold all status variants.
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum Status {
    /// The MultState variant
    ///
    MultiState {
        current: ItemId,        // the current state
        allowed: Vec<ItemId>,   // the allowed states
        no_change_silent: bool, // if true, events are only broadcast when the state changes
    },

    /// The CountedState variant
    ///
    CountedState {
        current: ItemId,        // the current state
        trigger: ItemId,        // the state when the status is triggered
        anti_trigger: ItemId,   // the state when the status is not triggered
        reset: ItemId,          // the state to reset the status to its default value
        count: u32,             // the current count of the status
        default_count: u32,     // the starting value of the status count
        no_change_silent: bool, // if true, events are only broadcast when the state changes
    },
}

// Reexport the status variants
use self::Status::{CountedState, MultiState};

// Implement key features for Status
impl Status {
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
    /// method returns the new state, otherwise None. // FIXME consider adding
    /// a distinction between no change and failure
    ///
    pub fn update(&mut self, new_state: ItemId) -> Option<ItemId> {
        match self {
            // The multistate variant
            &mut MultiState {
                ref mut current,
                ref allowed,
                ref no_change_silent,
            } => {
                // Check that the new state is valid
                if allowed.is_empty() | allowed.contains(&new_state) {
                    // If no_change_slient, and the states are the same
                    if *no_change_silent & (*current == new_state) {
                        return None; // Indicate no change
                    }
                    
                    // Update the state
                    *current = new_state;
                    Some(new_state)
                
                // Indicate failure
                } else {
                    None
                }
            }

            // The countedstate variant
            &mut CountedState {
                ref mut current,
                ref mut count,
                ref reset,
                ref default_count,
                ref trigger,
                ref anti_trigger,
                ref no_change_silent,
            } => {
                // Reset the count and state
                if new_state == *reset {
                    // Reset the count
                    *count = *default_count;
                    
                    // If no_change_silent and current is already anti_trigger
                    if *no_change_silent & (*current == *anti_trigger) {
                        return None; // Indicate no change
                    }
                    
                    // Reset the current state
                    *current = *anti_trigger;
                    Some(current.clone())

                // Increment the count when the anti-trigger is provided
                } else if new_state == *anti_trigger {
                    // Increase the count
                    *count = *count + 1;
                    
                    // If no_change_silent and current is already anti_trigger
                    if *no_change_silent & (*current == *anti_trigger) {
                        return None; // Indicate no change
                    }
                    
                    // Reset the current state
                    *current = *anti_trigger;
                    Some(current.clone())

                // Decrement the count when the trigger is provided
                } else if new_state == *trigger {
                    // If the count is not zero, decrease it
                    if *count > 0 {
                        *count = *count - 1;
                    
                    // If the count is already zero and no_change_silent
                    } else if *no_change_silent {
                        return None; // Indicate no change
                    }

                    // If the count is now zero, change the current state
                    if *count == 0 {
                        *current = *trigger;
                    }

                    // Return the current state
                    Some(current.clone())

                // Otherwise report failure
                } else {
                    None
                }
            }
        }
    }
}

/// A struct which allows a limited number of possible states. If the
/// allowed state vector is empty, any state will be allowed.
///
/// # FIXME
/// Reconsider this specification. Perhaps an empty allowed state vector
/// should indicate that the user cannot select a valid state.
///
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct StatusPartialDescription {
    pub current: ItemId,
    pub allowed: Vec<ItemId>,
}

/// A struct which allows a limited number of possible states. If the
/// allowed state vector is empty, any state will be allowed.
///
/// # FIXME
/// Reconsider this specification. Perhaps an empty allowed state vector
/// should indicate that the user cannot select a valid state.
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
