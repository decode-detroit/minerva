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
use crate::definitions::{InternalSend, ItemId, Status, StatusMap, PartialStatus, StatusPartialDescription};

/// A structure which holds the local status and manages any state changes.
///
/// # Notes
///
/// This module only holds and modifies the local copy of the system status. An
/// additional copy may be held in the backup module.
///
pub struct StatusHandler {
    status_map: StatusMap,      // hash map of the local status
    update_line: InternalSend, // the update line for posting any warnings
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
    pub fn new(update_line: InternalSend, status_map: StatusMap) -> StatusHandler {
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

// Tests of the status module
#[cfg(test)]
mod tests {
    //use super::*;

    // FIXME Define tests of this module
    #[test]
    fn missing_tests() {
        // FIXME: Implement this
        unimplemented!();
    }
}
