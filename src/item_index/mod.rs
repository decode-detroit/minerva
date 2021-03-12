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

// Import the relevant structures into the correct namespace
use crate::definitions::{ItemId, ItemDescription, DescriptionMap, IndexSend, IndexUpdate};

// Import standard library features
use std::sync::{Arc, Mutex};

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
    pub fn new() -> (ItemIndex, IndexSend) {
        // Create the new IndexSend
        let (index_send, index_receive) = IndexSend::new();

        // Create an empty index and the ItemIndex
        let index = DescriptionMap::default();
        let item_index = ItemIndex{ index_receive, index };

        // Return the new ItemIndex and IndexSend
        (item_index, index_send)
    }

    /// A method to run the item index indefinitely and spawn a new thread
    /// for every request.
    /// 
    pub async fn run_loop() {

    }
    
    /// A method to return the description of a particular item.
    ///
    /// # Errors
    ///
    /// This method will raise an error if the provided id was not found in
    /// the index. This usually indicates that the provided id was incorrect
    /// or that the configuration file is incomplete.
    /// 
    /// On an error, the method will send an error message to the interface
    /// and return an empty ItemDescription.
    ///
    fn get_description(&self, item_id: &ItemId) -> ItemDescription {
        // Return an item description based on the provided item id
        match self.index.get(item_id) {
            // If the item is in the index, return the description
            Some(description) => description.clone(),

            // Otherwise, return the default
            None => {
                ItemDescription::new_default()
            }
        }
    }
}
