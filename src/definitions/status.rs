// Copyright (c) 2019-2021 Decode Detroit
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

//! This module also implements low level status structs and enums to store
//! the current status and consistently update the status.

// Import crate definitions
use crate::definitions::*;

// Import FNV HashMap
use fnv::FnvHashMap;

/// A type to store a hashmap of status ids and status descriptions
///
pub type StatusMap = FnvHashMap<ItemId, Status>; // a hash map of status id and status pairs

/// A type to store a hashmap of status ids and status descriptions
///
/// # FIXME
/// This intermediary should be eliminated and the UI should call for this
/// information as needed.
///
pub type PartialStatus = FnvHashMap<ItemId, StatusPartialDescription>; // a hash map of status ids and status descriptions

/// A type to store a vector of status ids and status descriptions
///
pub type FullStatus = FnvHashMap<ItemPair, StatusDescription>; // a hash map of status id pairs and status description pairs

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

    // Test creation and modification of a MultiState status
    #[test]
    fn multistate() {
        // Create a new multistate
        let id1 = ItemId::new_unchecked(10);
        let id2 = ItemId::new_unchecked(11);
        let id3 = ItemId::new_unchecked(12);
        let id4 = ItemId::new_unchecked(13);
        let valid_states = vec![id1, id2, id3];
        let mut status = MultiState {
            current: id1,
            allowed: valid_states.clone(),
            no_change_silent: false,
        };

        // Check the current state
        assert_eq!(id1, status.current());

        // Check the allowed states
        assert_eq!(valid_states, status.allowed());

        // Check changing the state
        assert_eq!(Some(id2), status.update(id2));
        assert_eq!(id2, status.current());

        // Check changing the state to an invalid option
        assert_eq!(None, status.update(id4));
        assert_eq!(id2, status.current());
    }

    // Test creation and modification of a CountedState status
    #[test]
    fn countedstate() {
        // Create a new countedstate
        let id1 = ItemId::new_unchecked(10);
        let id2 = ItemId::new_unchecked(11);
        let id3 = ItemId::new_unchecked(12);
        let id4 = ItemId::new_unchecked(13);
        let valid_states = vec![id1, id2, id3];
        let mut status = CountedState {
            current: id2,
            trigger: id1,
            anti_trigger: id2,
            reset: id3,
            count: 1,
            default_count: 1,
            no_change_silent: false,
        };

        // Check the current state
        assert_eq!(id2, status.current());

        // Check the allowed states
        assert_eq!(valid_states, status.allowed());

        // Check changing the state
        assert_eq!(Some(id1), status.update(id1));
        assert_eq!(id1, status.current());

        // Check resetting the state
        assert_eq!(Some(id2), status.update(id3));
        assert_eq!(id2, status.current());

        // Check changing the state to and from
        assert_eq!(Some(id1), status.update(id1));
        assert_eq!(Some(id2), status.update(id2));
        assert_eq!(id2, status.current());

        // Check changing the state to an invalid option
        assert_eq!(None, status.update(id4));
        assert_eq!(id2, status.current());
    }
}
