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

//! This module implements shared communication structures for managing
//! access to the style sheet.

// Import Tokio features
#[cfg(not(test))]
use tokio::sync::mpsc;
use tokio::sync::oneshot;

// Import FNV HashMap
use fnv::FnvHashMap;

/// A type to store a hashmap of CSS selectors and styling rules
///
pub type StyleMap = FnvHashMap<String, String>; // a hash map of selectors and rules (not verified)

// Implement conversion to a string for the whole stylemap
pub fn style_to_string(mut style_map: StyleMap) -> String {
    // Create an empty string
    let mut string = String::new();

    // Iterate through all the key/value pairs
    for (selector, rule) in style_map.drain() {
        string += &(selector + " " + &rule + "\n");
    }

    // Return the result
    string
}

/// An enum to provide and receive updates from the style sheet
///
#[derive(Debug)]
pub enum StyleUpdate {
    /// A variant to pass a new sheet the style sheet
    NewStyles { new_styles: StyleMap },

    /// A variant to add multiple styles at once
    AddStyles { new_styles: StyleMap },

    /// A variant to add, update, or remove a rule in the sheet
    UpdateStyle {
        selector: String,
        new_rule: Option<String>,
        reply_line: oneshot::Sender<bool>,
    },

    /// A variant to receive an existence test from the sheet
    GetExistence {
        selector: String,
        reply_line: oneshot::Sender<bool>,
    },

    /// A variant to receive a rule from the style sheet
    GetRule {
        selector: String,
        reply_line: oneshot::Sender<String>,
    },

    /// A variant to receive all the selectors from the style sheet
    GetAllSelectors {
        reply_line: oneshot::Sender<Vec<String>>,
    },

    /// A variant to receive all the selectors and rules from the style sheet
    GetAllRules {
        reply_line: oneshot::Sender<StyleMap>,
    },
}

/// The stucture and methods to send requests to the style sheet.
///
#[cfg(not(test))]
#[derive(Clone, Debug)]
pub struct StyleAccess {
    style_send: mpsc::Sender<StyleUpdate>, // the line to pass requests to the style sheet
}

// Implement the key features of the style access
#[cfg(not(test))]
impl StyleAccess {
    /// A function to create a new Style Access
    ///
    /// The function returns the Style Access structure and the style
    /// receive channel which will return the provided updates.
    ///
    pub fn new() -> (Self, mpsc::Receiver<StyleUpdate>) {
        // Create the new channel
        let (style_send, receive) = mpsc::channel(512);

        // Create and return both new items
        (StyleAccess { style_send }, receive)
    }

    /// A method to send a new styles to the style sheet
    ///
    pub async fn send_styles(&self, new_styles: StyleMap) {
        self.style_send
            .send(StyleUpdate::NewStyles { new_styles })
            .await
            .unwrap_or(());
    }

    /// A method to send a new styles to the style sheet
    ///
    pub async fn add_styles(&self, new_styles: StyleMap) {
        self.style_send
            .send(StyleUpdate::AddStyles { new_styles })
            .await
            .unwrap_or(());
    }

    /// A method to remove a rule from the stylesheet
    /// Returns true if the item was updated and false otherwise.
    ///
    pub async fn remove_rule(&self, selector: String) -> bool {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .style_send
            .send(StyleUpdate::UpdateStyle {
                selector: selector.clone(),
                new_rule: None,
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

    /// A method to add or update the rule in the style sheet
    /// Returns true if the item was not previously defined and false otherwise.
    /// FIXME This is a misleading return value.
    ///
    pub async fn update_rule(&self, selector: String, new_rule: String) -> bool {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .style_send
            .send(StyleUpdate::UpdateStyle {
                selector,
                new_rule: Some(new_rule),
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

    /// A method to see if an style rule exists in the style sheet
    ///
    pub async fn is_listed(&self, selector: &String) -> bool {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .style_send
            .send(StyleUpdate::GetExistence {
                selector: selector.clone(),
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

    /// A method to get the rule from the style sheet
    /// Returns an empty string if the selector is not found
    ///
    pub async fn get_rule(&self, selector: &String) -> String {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .style_send
            .send(StyleUpdate::GetRule {
                selector: selector.clone(),
                reply_line,
            })
            .await
        {
            // On failure, return an empty String
            return String::new();
        }

        // Wait for the reply
        rx.await.unwrap_or(String::new())
    }

    /// A method to get all the selectors from the style sheet
    ///
    pub async fn get_all_selectors(&self) -> Vec<String> {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .style_send
            .send(StyleUpdate::GetAllSelectors { reply_line })
            .await
        {
            // On failure, return none
            return Vec::new();
        }

        // Wait for the reply
        rx.await.unwrap_or(Vec::new())
    }

    /// A method to get all selectors and rules from the style sheet
    ///
    pub async fn get_all_rules(&self) -> StyleMap {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        if let Err(_) = self
            .style_send
            .send(StyleUpdate::GetAllRules { reply_line })
            .await
        {
            // On failure, return none
            return FnvHashMap::default();
        }

        // Wait for the reply
        rx.await.unwrap_or(FnvHashMap::default())
    }
}
