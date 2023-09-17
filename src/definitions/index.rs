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

//! This module implements shared communication structures for managing
//! access to the item index.

// Import crate definitions
use crate::definitions::*;

// Import Tokio features
#[cfg(not(test))]
use tokio::sync::mpsc;
use tokio::sync::oneshot;

// Import FNV HashMap
use fnv::FnvHashMap;

/// A type to store a hashmap of item ids and item descriptions
///
pub type DescriptionMap = FnvHashMap<ItemId, ItemDescription>; // a hash map of item ids and item descriptions

/// An enum to provide and receive updates from the item index
///
#[derive(Debug)]
pub enum IndexUpdate {
    /// A variant to pass a new index the item index
    NewIndex { new_index: DescriptionMap },

    /// A variant to add, update, or remove a description in the index
    UpdateDescription {
        item_id: ItemId,
        new_description: Option<ItemDescription>,
        reply_line: oneshot::Sender<bool>,
    },

    /// A variant to receive an existence test from the item index
    GetExistence {
        item_id: ItemId,
        reply_line: oneshot::Sender<bool>,
    },

    /// A variant to receive a description from the item index
    GetDescription {
        item_id: ItemId,
        reply_line: oneshot::Sender<ItemDescription>,
    },

    /// A variant to receive a pair from the item index
    GetPair {
        item_id: ItemId,
        reply_line: oneshot::Sender<ItemPair>,
    },

    /// A variant to receive all the ids from the item index
    GetAll {
        reply_line: oneshot::Sender<Vec<ItemId>>,
    },

    /// A variant to receive all the pairs from the item index
    GetAllPairs {
        reply_line: oneshot::Sender<Vec<ItemPair>>,
    },
}

/// The stucture and methods to send requests to the item index.
///
#[cfg(not(test))]
#[derive(Clone, Debug)]
pub struct IndexAccess {
    index_send: mpsc::Sender<IndexUpdate>, // the line to pass requests to the item index
}

// Implement the key features of the index access
#[cfg(not(test))]
impl IndexAccess {
    /// A function to create a new Index Access
    ///
    /// The function returns the Index Access structure and the index
    /// receive channel which will return the provided updates.
    ///
    pub fn new() -> (Self, mpsc::Receiver<IndexUpdate>) {
        // Create the new channel
        let (index_send, receive) = mpsc::channel(128);

        // Create and return both new items
        (IndexAccess { index_send }, receive)
    }

    /// A method to send a new index to the item index
    ///
    pub async fn send_index(&self, new_index: DescriptionMap) {
        self.index_send
            .send(IndexUpdate::NewIndex { new_index })
            .await
            .unwrap_or(());
    }

    /// A method to remove an item from the index
    /// Returns true if the item was found and false otherwise.
    ///
    pub async fn remove_item(&self, item_id: ItemId) -> bool {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .index_send
            .send(IndexUpdate::UpdateDescription {
                item_id: item_id.clone(),
                new_description: None,
                reply_line,
            })
            .await
        {
            // On failure, return false
            return false;
        }

        // Wait for the reply
        rx.await.unwrap_or(false)
    }

    /// A method to add or update the description in the item index
    /// Returns true if the item was not previously defined and false otherwise.
    /// 
    /// # Note
    /// This is a misleading return value and may be changed in future versions.
    ///
    pub async fn update_description(
        &self,
        item_id: ItemId,
        new_description: ItemDescription,
    ) -> bool {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .index_send
            .send(IndexUpdate::UpdateDescription {
                item_id: item_id.clone(),
                new_description: Some(new_description.clone()),
                reply_line,
            })
            .await
        {
            // On failure, return false
            return false;
        }

        // Wait for the reply
        rx.await.unwrap_or(false)
    }

    /// A method to see if an item exists in the index
    ///
    pub async fn is_listed(&self, item_id: &ItemId) -> bool {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .index_send
            .send(IndexUpdate::GetExistence {
                item_id: item_id.clone(),
                reply_line,
            })
            .await
        {
            // On failure, return false
            return false;
        }

        // Wait for the reply
        rx.await.unwrap_or(false)
    }

    /// A method to get the description from the item index
    ///
    pub async fn get_description(&self, item_id: &ItemId) -> ItemDescription {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .index_send
            .send(IndexUpdate::GetDescription {
                item_id: item_id.clone(),
                reply_line,
            })
            .await
        {
            // On failure, return default
            return ItemDescription::new_default();
        }

        // Wait for the reply
        rx.await.unwrap_or(ItemDescription::new_default())
    }

    /// A method to get the item pair from the item index
    ///
    pub async fn get_pair(&self, item_id: &ItemId) -> ItemPair {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .index_send
            .send(IndexUpdate::GetPair {
                item_id: item_id.clone(),
                reply_line,
            })
            .await
        {
            // On failure, return default
            return ItemPair::new_default(item_id.id());
        }

        // Wait for the reply
        rx.await.unwrap_or(ItemPair::new_default(item_id.id()))
    }

    /// A method to get all ids from the item index
    ///
    pub async fn get_all(&self) -> Vec<ItemId> {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .index_send
            .send(IndexUpdate::GetAll { reply_line })
            .await
        {
            // On failure, return none
            return Vec::new();
        }

        // Wait for the reply
        rx.await.unwrap_or(Vec::new())
    }

    /// A method to get all pairs from the item index
    ///
    pub async fn get_all_pairs(&self) -> Vec<ItemPair> {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .index_send
            .send(IndexUpdate::GetAllPairs { reply_line })
            .await
        {
            // On failure, return none
            return Vec::new();
        }

        // Wait for the reply
        rx.await.unwrap_or(Vec::new())
    }
}
