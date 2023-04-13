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

//! A module to monitor the program status line and sort updates.
//!
//! Current event updates are sent to the provided broadcast line. Error updates
//! are logged to the provided log file. Other updates are converted to a human
//! readable format and returned to higher-level modules.

// Import crate definitions
use crate::definitions::*;

// Import the chrono library
use chrono::{Duration, Local};

// Import tracing features
use tracing::info;

/// A structure to handle all logging and update processing for the program.
///
pub struct Logger {
    old_notifications: Vec<Notification>, // internal list of notifications less than 1 minute old
    index_access: IndexAccess,            // the item index access point
    internal_send: InternalSend,          // broadcast channel for current events
    interface_send: InterfaceSend, // an update line for passing updates to the user interface
}

// Implement key Logger struct features
impl Logger {
    /// A function to create a new logger instance.
    ///
    /// This function takes a log file name to log program-wide errors and
    /// a broadcast line to braodcast current event ids to the rest of the
    /// system (the broadcast line implementation is usually platform-dependant).
    ///
    /// # Errors
    ///
    /// This function will raise an error if it was unable to open the provided
    /// log file. Like all system interface functions and methods, this function
    /// will fail gracefully by returning None.
    ///
    pub fn new(
        index_access: IndexAccess,
        internal_send: InternalSend,
        interface_send: InterfaceSend,
    ) -> Self {
        // Return the new logger
        Self {
            old_notifications: Vec::new(),
            index_access,
            internal_send,
            interface_send,
        }
    }

    /// A method to process an update from the system interface and return a
    /// vector of notification strings. Returns None if there are no new
    /// notifications.
    ///
    /// # Notes
    ///
    /// The returned notification string is designed to provide a series of
    /// updates to be returned to the user interface. The notifications returned
    /// from this method are the newest notifications as well as notifications
    /// from the last minute of operation.
    ///
    pub async fn update(&mut self, update: LogUpdate, is_debug: bool) -> Vec<Notification> {
        // Unpack the new update into a notification
        let mut notifications = vec![self.unpack_update(update).await];

        // If debug mode, print the update to the command line
        if is_debug {
            println!("{}", &notifications[0]);
        }

        // Iterate through the old notifications
        for old_note in self.old_notifications.drain(..) {
            // If the notification is younger than one minute, add it back
            if Local::now().naive_local() < old_note.time() + Duration::minutes(1) {
                notifications.push(old_note);
            }
        }

        // Update the old notifications list
        self.old_notifications = notifications.clone();

        // Return the refreshed list of notifications
        return notifications;
    }

    /// An internal method to unpack any event updates.
    ///
    /// This method sorts event updates into their various types and applies
    /// the changes to the existing events, broadcasts events, and logs any
    /// errors.
    ///
    ///
    async fn unpack_update(&mut self, update: LogUpdate) -> Notification {
        // Note the current time
        let now = Local::now().naive_local();

        // Unpack the event update based on its subtype
        match update {
            // Broadcast events and display them
            LogUpdate::Broadcast(id, data) => {
                // Broadcast the event and data, if specified
                self.internal_send.send_broadcast(id, data).await;

                // Send a current update with the item pair
                Notification::Current {
                    message: format!("{}", self.index_access.get_pair(&id).await),
                    time: now,
                }
            }

            // Notify of current events and display them
            LogUpdate::Current(id) => {
                // Send a current update with the item pair
                Notification::Current {
                    message: format!("{}", self.index_access.get_pair(&id).await),
                    time: now,
                }
            }

            // Update the state of a status
            LogUpdate::Status(status_id, new_state) => {
                // Get the item pairs for the status and state
                let status_pair = self.index_access.get_pair(&status_id).await;
                let state_pair = self.index_access.get_pair(&new_state).await;

                // Send the change to the interface
                self.interface_send
                    .send(InterfaceUpdate::UpdateStatus {
                        status_id: status_pair.clone(),
                        new_state: state_pair.clone(),
                    })
                    .await;

                // Return the notification
                Notification::Update {
                    message: format!(
                        "Changing {} To {}.",
                        status_pair.description, state_pair.description
                    ),
                    time: now,
                }
            }

            // Save data to the system
            LogUpdate::Save(data) => {
                // Send an info update
                info!("Game data: {}", data);

                // Print the full data to the notification area
                Notification::Update {
                    message: format!("Recorded: {}", data),
                    time: now,
                }
            }
        }
    }
}

// Tests of the logging module
#[cfg(test)]
mod tests {
    use super::*;

    // Test the logging module
    #[tokio::test]
    async fn logging() {
        // Import crate definitions
        use crate::definitions::*;

        // Create the communication lines
        let (internal_send, _internal_recv) = InternalSend::new();
        let (interface_send, _web_interface_recv) = InterfaceSend::new();

        // Create a test index access and load the index
        let (index_access, _rx) = IndexAccess::new();
        let mut index = DescriptionMap::default();
        index.insert(
            ItemId::new_unchecked(3),
            ItemDescription::new("Test Broadcast"),
        );
        index.insert(ItemId::new_unchecked(4), ItemDescription::new("Test Event"));
        index_access.send_index(index).await;

        // Create a new logger instance
        let mut logger =
            Logger::new(index_access, internal_send, interface_send);

        // Pass a series of updates to the logger and verify the output
        let mut result = logger
            .update(LogUpdate::Broadcast(ItemId::new_unchecked(3), None), false)
            .await;
        assert_eq!(result[0].message(), "Test Broadcast (3)".to_string());
        result = logger
            .update(LogUpdate::Current(ItemId::new_unchecked(4)), false)
            .await;
        assert_eq!(result[0].message(), "Test Event (4)".to_string());
        assert_eq!(result[1].message(), "Test Broadcast (3)".to_string());
    }
}
