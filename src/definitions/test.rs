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

//! A testing module that implements useful tools for the testing the program.

/// Test_Vec Macro
///
/// A macro that allows easier comparison of two vectors (one the test vector
/// and the other generated by the test itself).
///
/// Note: This macro is depreciated and will be retired
///
macro_rules! _test_vec {
    // Compare the test vector with the messages received (order matters)
    (=$line:expr, $test:expr) => {{
        // Import libraries for testing
        use std::time::Duration;
        use tokio::time::sleep;

        // Print and check the messages received (wait at most half a second)
        let mut recv = Vec::new();
        loop {
            tokio::select! {
                // Try to find the test updates
                Some(message) = $line.recv() => {
                    // Log the new addition and check for all of them
                    recv.push(message);

                    // Check that the received vector matches the test vector
                    if $test == recv {
                        return;
                    }
                }

                // Only wait half a second
                _ = sleep(Duration::from_millis(500)) => {
                    // Check to see if the vectors are both empty
                    if $test == recv {
                        return;
                    }

                    // Otherwise break
                    break;
                }
            }
        }

        // Print debugging help if the script failed
        println!(
            "===================DEBUG==================\n\nEXPECTED\n{:?}\n\nOUTPUT\n{:?}",
            $test, recv
        );

        // If they were not found, fail the test
        panic!("Failed test vector comparison.");
    }};
}

// Imports for the test structure below
use crate::definitions::{DescriptionMap, IndexUpdate, ItemDescription, ItemId, ItemPair};
use std::sync::{Mutex, MutexGuard};
use tokio::sync::mpsc;

/// A helper structure to easily test modules that need index access
///
pub struct IndexAccess {
    index: Mutex<DescriptionMap>, // a database of ItemId/ItemDescription pairs
}

// Create a panic version of clone for this test module
impl Clone for IndexAccess {
    fn clone(&self) -> Self {
        panic!("Test index access cannot be cloned!")
    }
}

// Implement the key features of the test index access
impl IndexAccess {
    /// A function to create a new test Index Access
    ///
    /// The function returns the test Index Access structure which will
    /// process updates like a real index, but locally.
    ///
    pub fn new() -> (IndexAccess, mpsc::Receiver<IndexUpdate>) {
        // Create the fake channel
        let (_tx, receive) = mpsc::channel(8);

        // Return the new test access point
        (
            IndexAccess {
                index: Mutex::new(DescriptionMap::default()),
            },
            receive,
        )
    }

    /// A method to send a new index to the item index
    ///
    pub async fn send_index(&self, new_index: DescriptionMap) {
        // Lock access and swap the index
        if let Ok(mut index) = self.index.lock() {
            *index = new_index;
        }
    }

    /// A method to remove an item from the index
    /// Returns true if the item was found and false otherwise.
    ///
    pub async fn remove_item(&self, item_id: ItemId) -> bool {
        // Lock access
        if let Ok(index) = self.index.lock() {
            return IndexAccess::modify_item(index, item_id, None);
        }

        // Fallback
        false
    }

    /// A method to add or update the description in the item index
    /// Returns true if the item was not previously defined and false otherwise.
    ///
    pub async fn update_description(
        &self,
        item_id: ItemId,
        new_description: ItemDescription,
    ) -> bool {
        // Lock access
        if let Ok(index) = self.index.lock() {
            return IndexAccess::modify_item(index, item_id, Some(new_description));
        }

        // Fallback
        false
    }

    /// A method to see if an item is listed in the index
    ///
    pub async fn is_listed(&self, item_id: &ItemId) -> bool {
        // Lock access
        if let Ok(index) = self.index.lock() {
            // Check to see if the key exists
            return index.contains_key(item_id);
        }

        // Otherwise, return false
        false
    }

    /// A method to get the description from the item index
    ///
    pub async fn get_description(&self, item_id: &ItemId) -> ItemDescription {
        // Lock access
        if let Ok(index) = self.index.lock() {
            // Return an item description based on the provided item id
            if let Some(description) = index.get(&item_id) {
                // Return the description
                return description.clone();
            }
        }

        // Otherwise, return the default
        ItemDescription::new_default()
    }

    /// A method to get the item pair from the item index
    ///
    pub async fn get_pair(&self, item_id: &ItemId) -> ItemPair {
        // Lock access
        if let Ok(index) = self.index.lock() {
            // Return an item description based on the provided item id
            if let Some(description) = index.get(&item_id) {
                // Return the pair
                return ItemPair::from_item(item_id.clone(), description.clone());
            }
        }

        // Otherwise, return the default
        ItemPair::new_default(item_id.id())
    }

    /// A method to get all ids from the item index
    ///
    pub async fn get_all(&self) -> Vec<ItemId> {
        // Create an empty items vector
        let mut items = Vec::new();

        // Lock access
        if let Ok(index) = self.index.lock() {
            for item in index.keys() {
                items.push(item.clone());
            }
        }

        // Sort the items by item id
        items.sort_unstable();

        // Return the result
        items
    }

    /// A method to get all pairs from the item index
    ///
    pub async fn get_all_pairs(&self) -> Vec<ItemPair> {
        // Create an empty items vector
        let mut items = Vec::new();

        // Lock access
        if let Ok(index) = self.index.lock() {
            for (item, description) in index.iter() {
                items.push(ItemPair::from_item(item.clone(), description.clone()));
            }
        }

        // Sort the items by item id
        items.sort_unstable();

        // Return the result
        items
    }

    /// A method to add, update, or remove an item and description.
    ///
    /// # Note
    ///
    /// If the item was not already in the index, this method will
    /// return false.
    ///
    fn modify_item(
        mut index: MutexGuard<DescriptionMap>,
        item_id: ItemId,
        possible_description: Option<ItemDescription>,
    ) -> bool {
        // If the request is to modify the description
        if let Some(new_description) = possible_description {
            // If the item is already in the index, update the description
            if let Some(description) = index.get_mut(&item_id) {
                // Update the description and notify the system
                *description = new_description;
                return false;
            }

            // Otherwise create a new item in the lookup
            index.insert(item_id, new_description);
            return true;
        }

        // Otherwise, try to remove the item
        if index.remove(&item_id).is_some() {
            // If the item exists
            return true;
        }

        // Otherwise, return false
        false
    }
}

// Imports for the test structure below
use crate::definitions::{StyleMap, StyleUpdate};

/// A helper structure to easily test modules that need style access
///
pub struct StyleAccess {
    style_map: Mutex<StyleMap>, // a database of selector/rule pairs
}

// Create a panic version of clone for this test module
impl Clone for StyleAccess {
    fn clone(&self) -> Self {
        panic!("Test index access cannot be cloned!")
    }
}

// Implement the key features of the test style access
impl StyleAccess {
    /// A function to create a new test Style Access
    ///
    /// The function returns the test Style Access structure which will
    /// process updates like a real style sheet, but locally.
    ///
    pub fn new() -> (StyleAccess, mpsc::Receiver<StyleUpdate>) {
        // Create the fake channel
        let (_tx, receive) = mpsc::channel(8);

        // Return the new test access point
        (
            StyleAccess {
                style_map: Mutex::new(StyleMap::default()),
            },
            receive,
        )
    }

    /// A method to send new styles to the style sheet
    ///
    pub async fn send_styles(&self, new_styles: StyleMap) {
        // Lock access and swap the map
        if let Ok(mut map) = self.style_map.lock() {
            *map = new_styles;
        }
    }

    /// A method to send new styles to the style sheet
    ///
    pub async fn add_styles(&self, mut new_styles: StyleMap) {
        // Lock access and add each style to the map
        if let Ok(mut map) = self.style_map.lock() {
            for (selector, rule) in new_styles.drain() {
                map.insert(selector, rule);
            }
        }
    }

    /// A method to remove an item from the map
    /// Returns true if the item was found and false otherwise.
    ///
    pub async fn remove_rule(&self, selector: String) -> bool {
        // Lock access
        if let Ok(map) = self.style_map.lock() {
            return StyleAccess::modify_rule(map, selector, None);
        }

        // Fallback
        false
    }

    /// A method to add or update the rule in the map
    /// Returns true if the operation was a success.
    ///
    pub async fn update_rule(&self, selector: String, new_rule: String) -> bool {
        // Lock access
        if let Ok(map) = self.style_map.lock() {
            return StyleAccess::modify_rule(map, selector, Some(new_rule));
        }

        // Fallback
        false
    }

    /// A method to see if a selector is listed in the style sheet
    ///
    pub async fn is_listed(&self, selector: &String) -> bool {
        // Lock access
        if let Ok(map) = self.style_map.lock() {
            // Check to see if the key exists
            return map.contains_key(selector);
        }

        // Otherwise, return false
        false
    }

    /// A method to get the rule from the style sheet
    ///
    pub async fn get_rule(&self, selector: &String) -> String {
        // Lock access
        if let Ok(map) = self.style_map.lock() {
            // Return a rule based on the provided selector
            if let Some(rule) = map.get(selector) {
                // Return the description
                return rule.clone();
            }
        }

        // Otherwise, return the default
        String::new()
    }

    /// A method to get all selectors from the style sheet
    ///
    pub async fn get_all_selectors(&self) -> Vec<String> {
        // Create an empty selectors vector
        let mut selectors = Vec::new();

        // Lock access
        if let Ok(map) = self.style_map.lock() {
            for selector in map.keys() {
                selectors.push(selector.clone());
            }
        }

        // Sort the selectors
        selectors.sort_unstable();

        // Return the result
        selectors
    }

    /// A method to get all rules from the style sheet
    ///
    pub async fn get_all_rules(&self) -> StyleMap {
        // Return a copy of the style map
        let mut map_copy = StyleMap::default();

        // Lock access
        if let Ok(map) = self.style_map.lock() {
            map_copy = map.clone();
        }

        // Return the result
        map_copy
    }

    /// A method to add, update, or remove a selector and rule
    ///
    /// # Note
    ///
    /// If the operation was a success, returns true.
    ///
    fn modify_rule(
        mut map: MutexGuard<StyleMap>,
        selector: String,
        possible_rule: Option<String>,
    ) -> bool {
        // If the request is to modify the rule
        if let Some(new_rule) = possible_rule {
            // Update or create a new rule in the lookup
            map.insert(selector, new_rule);
            return true;
        }

        // Otherwise, try to remove the item
        if map.remove(&selector).is_some() {
            // If the item exists
            return true;
        }

        // Otherwise, return false
        false
    }
}
