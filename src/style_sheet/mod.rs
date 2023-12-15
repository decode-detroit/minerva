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

//! A module to deliver asyncronous access to the index of user styles
//! including CSS selectors and formatting rules.
//!
//! # Note
//!
//! This stylesheet does not preserve order of the rules, so it should
//! not be used in any case where order of the rules matters.

// Import crate definitions
use crate::definitions::*;

// Import Tokio features
use tokio::sync::mpsc;

/// A structure to store the style sheet and provide asyncronous responses
/// to information requests
///
pub struct StyleSheet {
    style_receive: mpsc::Receiver<StyleUpdate>,
    style_map: StyleMap, // hash map of all the style rules
}

// Implement key features for the style sheet
impl StyleSheet {
    /// A function to create a new StyleSheet
    ///
    pub fn new() -> (StyleSheet, StyleAccess) {
        // Create the new StyleAccess
        let (style_access, style_receive) = StyleAccess::new();

        // Create an empty style map and the StyleSheet
        let style_map = StyleMap::default();
        let style_sheet = StyleSheet {
            style_receive,
            style_map,
        };

        // Return the new StyleSheet and StyleAccess
        (style_sheet, style_access)
    }

    /// A method to run the style sheet indefinitely
    ///
    pub async fn run(&mut self) {
        // Listen for updates indefinitely
        loop {
            self.run_once().await;
        }
    }

    /// A method to run the style sheet and process a single reqeust
    ///
    async fn run_once(&mut self) {
        // Listen for updates
        match self.style_receive.recv().await {
            // If there is a full style update
            Some(StyleUpdate::NewStyles { new_styles }) => {
                // Replace the current map
                self.style_map = new_styles;
            }

            // If there is a partial style update
            Some(StyleUpdate::AddStyles { mut new_styles }) => {
                // Merge the two style sheets, with preference to the new styles
                for (selector, rule) in new_styles.drain() {
                    self.style_map.insert(selector, rule);
                }
            }

            // If there is an individual style update
            Some(StyleUpdate::UpdateStyle {
                selector,
                new_rule,
                reply_line,
            }) => {
                // Process the modification and return the result
                reply_line
                    .send(self.modify_rule(selector, new_rule))
                    .unwrap_or(());
            }

            // If if is an existence request
            Some(StyleUpdate::GetExistence {
                selector,
                reply_line,
            }) => {
                reply_line
                    .send(self.style_map.contains_key(&selector))
                    .unwrap_or(());
            }

            // If it is a rule request
            Some(StyleUpdate::GetRule {
                selector,
                reply_line,
            }) => {
                // Return the rule, if found
                reply_line.send(self.get_rule(selector)).unwrap_or(());
            }

            // If it is a request for all the selectors
            Some(StyleUpdate::GetAllSelectors { reply_line }) => {
                // Create a copy of the selectors
                let mut selectors = Vec::new();
                for selector in self.style_map.keys() {
                    selectors.push(selector.clone());
                }

                // Sort the selectors by alphabet
                selectors.sort_unstable();

                // Reply with the result
                reply_line.send(selectors).unwrap_or(());
            }

            // If it is a request for all the style rules
            Some(StyleUpdate::GetAllRules { reply_line }) => {
                // Reply with a copy of the style map
                reply_line.send(self.style_map.clone()).unwrap_or(());
            }

            // Ignore failure
            None => (),
        }
    }

    /// A method to add, update, or remove a selector and rule
    ///
    /// # Note
    ///
    /// If the update was successful, this method will return true.
    ///
    fn modify_rule(&mut self, selector: String, possible_rule: Option<String>) -> bool {
        // If the request is to modify the rule
        if let Some(new_rule) = possible_rule {
            // Update or create a new item in the lookup
            self.style_map.insert(selector, new_rule);
            return true;
        }

        // Otherwise, try to remove the item
        if self.style_map.remove(&selector).is_some() {
            // If the item exists
            return true;
        }

        // Otherwise, return false
        false
    }

    /// A method to return the rule for the particular selector.
    ///
    /// # Note
    ///
    /// If the item is not found in the style sheet, this function will
    /// return an empty string.
    ///
    fn get_rule(&self, selector: String) -> String {
        // Return a rule based on the provided selector
        match self.style_map.get(&selector) {
            // If the rule is in the style sheet, return the rule
            Some(rule) => rule.clone(),

            // Otherwise, return the default
            None => String::new(),
        }
    }
}

// Tests of the item index module
#[cfg(test)]
mod tests {
    use super::*;

    // Test the style sheet
    #[tokio::test]
    async fn call_styles() {
        // Launch the style sheet
        let (mut style_sheet, style_access) = StyleSheet::new();

        // Spawn a new thread for the index
        tokio::spawn(async move {
            style_sheet.run().await;
        });

        // Create the selectors and rules
        let selector1 = "#aSelector".to_string();
        let selector2 = "#anotherSelector".to_string();
        let rule1 = "{rule: yes}".to_string();
        let rule2 = "{anotherRule: great!}".to_string();

        // Add a new style map to the style sheet
        let mut new_styles = StyleMap::default();
        new_styles.insert(selector1.clone(), rule1.clone());
        new_styles.insert(selector2.clone(), rule2.clone());
        style_access.send_styles(new_styles).await;

        // Verify the selectors and rules from the style sheet
        assert_eq!(rule1, style_access.get_rule(&selector1).await);
        assert_eq!(rule2, style_access.get_rule(&selector2).await);

        // Change one of the rules and verify the change
        assert_eq!(
            true,
            style_access
                .update_rule(selector1.clone(), rule2.clone())
                .await
        );
        assert_eq!(rule2, style_access.get_rule(&selector1).await);

        // Delete a rule and verify the change
        assert_eq!(true, style_access.remove_rule(selector1.clone()).await);
        assert_eq!(false, style_access.is_listed(&selector1).await);
        assert_eq!("".to_string(), style_access.get_rule(&selector1).await);
    }
}
