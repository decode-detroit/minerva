// Copyright (c) 2021 Decode Detroit
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

//! A module to deliver asyncronous access to the index of item ids,
//! including item descriptions and formatting.

// Import crate definitions
use crate::definitions::*;

// Import Tokio features
use tokio::sync::mpsc;

/// A structure to store the item index and provide asyncronous responses
/// to information requests
///
pub struct ItemIndex {
    index_receive: mpsc::Receiver<IndexUpdate>,
    index: DescriptionMap, // hash map of all the item descriptions
}

// Implement key features for the item index
impl ItemIndex {
    /// A function to create a new ItemIndex
    ///
    pub fn new() -> (ItemIndex, IndexAccess) {
        // Create the new IndexAccess
        let (index_access, index_receive) = IndexAccess::new();

        // Create an empty index and the ItemIndex
        let index = DescriptionMap::default();
        let item_index = ItemIndex {
            index_receive,
            index,
        };

        // Return the new ItemIndex and IndexAccess
        (item_index, index_access)
    }

    /// A method to run the item index indefinitely
    ///
    pub async fn run(&mut self) {
        // Listen for updates indefinitely
        loop {
            self.run_once().await;
        }
    }

    /// A method to run the item index and process a single reqeust
    ///
    async fn run_once(&mut self) {
        // Listen for updates
        match self.index_receive.recv().await {
            // If there is a full index update
            Some(IndexUpdate::NewIndex { new_index }) => {
                // Replace the current index
                self.index = new_index;
            }

            // If there is an individual item update
            Some(IndexUpdate::UpdateDescription {
                item_id,
                new_description,
                reply_line,
            }) => {
                // Process the modification and return the result
                reply_line
                    .send(self.modify_item(item_id, new_description))
                    .unwrap_or(());
            }

            // If if is an existance request
            Some(IndexUpdate::GetExistence {
                item_id,
                reply_line,
            }) => {
                reply_line.send(self.index.contains_key(&item_id)).unwrap_or(());
            }

            // If it is a description request
            Some(IndexUpdate::GetDescription {
                item_id,
                reply_line,
            }) => {
                // Return the description, if found
                reply_line.send(self.get_description(item_id)).unwrap_or(());
            }

            // If it is a description request
            Some(IndexUpdate::GetPair {
                item_id,
                reply_line,
            }) => {
                // Return the description, if found
                reply_line.send(self.get_pair(item_id)).unwrap_or(());
            }

            // If it is a request for all the items
            Some(IndexUpdate::GetAll { reply_line }) => {
                // Create a copy of the item ids
                let mut items = Vec::new();
                for item in self.index.keys() {
                    items.push(item.clone());
                }

                // Sort the items by item id
                items.sort_unstable();

                // Reply with the result
                reply_line.send(items).unwrap_or(());
            }

            // If it is a request for all the item pairs
            Some(IndexUpdate::GetAllPairs { reply_line }) => {
                // Create a copy of the item ids
                let mut items = Vec::new();
                for (item, description) in self.index.iter() {
                    items.push(ItemPair::from_item(item.clone(), description.clone()));
                }

                // Sort the items by item id
                items.sort_unstable();

                // Reply with the result
                reply_line.send(items).unwrap_or(());
            }

            // Ignore failure
            _ => (),
        }
    }

    /// A method to add, update, or remove an item and description.
    ///
    /// # Note
    ///
    /// If the item was not already in the index, this method will
    /// return false.
    ///
    fn modify_item(
        &mut self,
        item_id: ItemId,
        possible_description: Option<ItemDescription>,
    ) -> bool {
        // If the request is to modify the description
        if let Some(new_description) = possible_description {
            // If the item is already in the index, update the description
            if let Some(description) = self.index.get_mut(&item_id) {
                // Update the description and notify the system
                *description = new_description;
                return false; // FIXME This is backwards. Should return true here and false down below
            }

            // Otherwise create a new item in the lookup
            self.index.insert(item_id, new_description);
            return true;
        }

        // Otherwise, try to remove the item
        if let Some(_) = self.index.remove(&item_id) {
            // If the item exists
            return true;
        }

        // Otherwise, return false
        false
    }

    /// A method to return the description of a particular item.
    ///
    /// # Note
    ///
    /// If the item is not found in the index, this function will
    /// return the default ItemDescription.
    ///
    fn get_description(&self, item_id: ItemId) -> ItemDescription {
        // Return an item description based on the provided item id
        match self.index.get(&item_id) {
            // If the item is in the index, return the description
            Some(description) => description.clone(),

            // Otherwise, return the default
            None => ItemDescription::new_default(),
        }
    }

    /// A method to repackage an ItemId into an ItemPair.
    ///
    /// # Note
    ///
    /// If the item is not found in the index, this function will
    /// return the default ItemPair.
    ///
    fn get_pair(&self, item_id: ItemId) -> ItemPair {
        // Return an item pair based on the provided item id
        match self.index.get(&item_id) {
            // If the item is in the index, return the description
            Some(description) => ItemPair::from_item(item_id, description.clone()),

            // Otherwise, return the default
            None => ItemPair::new_default(item_id.id()),
        }
    }
}

// Tests of the item index module
#[cfg(test)]
mod tests {
    use super::*;

    // Test the item index
    #[tokio::test]
    async fn call_index() {
        // Import libraries for testing
        use crate::definitions::Hidden;

        // Launch the item index
        let (mut item_index, index_access) = ItemIndex::new();

        // Spawn a new thread for the index
        tokio::spawn(async move {
            item_index.run().await;
        });

        // Create the ids and descriptions
        let id1 = ItemId::new_unchecked(10);
        let id2 = ItemId::new_unchecked(11);
        let desc1 = ItemDescription::new("Description 1", Hidden { edit_location: None });
        let desc2 = ItemDescription::new("Description 2", Hidden { edit_location: None });
        let pair1 = ItemPair::from_item(id1, desc1.clone());
        let pair2 = ItemPair::from_item(id2, desc2.clone());

        // Add a new description map to the index
        let mut new_index = DescriptionMap::default();
        new_index.insert(id1.clone(), desc1.clone());
        new_index.insert(id2.clone(), desc2.clone());
        index_access.send_index(new_index).await;

        // Verify the description and item pair from the index
        assert_eq!(desc1, index_access.get_description(&id1).await);
        assert_eq!(desc2, index_access.get_description(&id2).await);
        assert_eq!(pair1, index_access.get_pair(&id1).await);
        assert_eq!(pair2, index_access.get_pair(&id2).await);

        // Change one of the descriptions and verify the change
        assert_eq!(
            false,
            index_access.update_description(id1, desc2.clone()).await
        );
        assert_eq!(desc2, index_access.get_description(&id1).await);

        // Delete a description and verify the change
        assert_eq!(true, index_access.remove_item(id1).await);
        assert_eq!(
            false,
            index_access.is_listed(&id1).await
        );
        assert_eq!(vec!(id2)[0], index_access.get_all().await[0]);
    }
}
