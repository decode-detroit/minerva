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

// Import the relevant structures into the correct namespace
use crate::definitions::{EventUpdate, InternalSend, InterfaceUpdate, UpdateStatus, Notification};

// Import standard library modules
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;

// Import the chrono library
use chrono::{Local, Duration};

// Import the failure features
use failure::Error as FailureError;

/// A structure to handle all logging and update processing for the program.
///
pub struct Logger {
    game_log: Option<File>,                        // game log file for the program
    error_log: Option<File>,                       // error log file for the program
    old_notifications: Vec<Notification>, // internal list of notifications less than 1 minute old
    internal_send: InternalSend,        // broadcast channel for current events
    interface_send: std_mpsc::Sender<InterfaceUpdate>, // an update line for passing updates to the user interface
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
        log_path: Option<PathBuf>,
        error_path: Option<PathBuf>,
        internal_send: InternalSend,
        interface_send: std_mpsc::Sender<InterfaceUpdate>,
    ) -> Result<Logger, FailureError> {
        // Attempt to open the game log file
        let game_log = match log_path {
            // If a file was specified, try to load it
            Some(mut filepath) => {
                // Use the current time for each instance
                filepath.push(format!("game_log_{}", Local::now().format("%F_%H-%M")).as_str());

                // Create the new file instance
                match File::create(filepath.to_str().unwrap_or("")) {
                    Ok(file) => Some(file),
                    Err(_) => return Err(format_err!("Unable to create game log file.")),
                }
            }

            // If a file was not specified, run without a log file
            None => None,
        };

        // Attempt to open the error log file
        let error_log = match error_path {
            // If a file was specified, try to open it first
            Some(filepath) => match OpenOptions::new().append(true).open(filepath.to_str().unwrap_or("")) {
                Ok(file) => Some(file),

                // If the file does not exist
                Err(_) => {
                    // Try to create the filepath instead
                    match File::create(filepath.to_str().unwrap_or("")) {
                        Ok(file) => Some(file),
                        Err(_) => {
                            return Err(format_err!("Unable to create error log file."));
                        }
                    }
                }
            },

            // If a file was not specified, run without a log file
            None => None,
        };

        // Return the new logger
        Ok(Logger {
            game_log,
            error_log,
            old_notifications: Vec::new(),
            internal_send,
            interface_send,
        })
    }

    /// A method to set the game log file for the logger.
    ///
    /// This function takes a log file name to log saved data.
    ///
    pub fn set_game_log(&mut self, log_path: PathBuf) {
        // Attempt to open the log file
        self.game_log = match File::create(&log_path.to_str().unwrap_or("")) {
            Ok(file) => Some(file),
            Err(_) => None,
        }
    }

    /// A method to set the error log file for the logger.
    ///
    /// This function takes a log file name to log program-wide errors.
    ///
    pub fn set_error_log(&mut self, log_path: PathBuf) {
        // Attempt to open the log file
        self.error_log = match File::create(&log_path.to_str().unwrap_or("")) {
            Ok(file) => Some(file),
            Err(_) => None,
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
    pub async fn update(&mut self, update: EventUpdate) -> Vec<Notification> {
        // Unpack the new update into a notification
        let mut notifications = vec![self.unpack_update(update).await];

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
    async fn unpack_update(&mut self, update: EventUpdate) -> Notification {
        // Note the current time
        let now = Local::now().naive_local();
        
        // Unpack the event update based on its subtype
        match update {
            // Log and display errors
            EventUpdate::Error(error, event) => {
                // Try to write it to the file
                if let Some(ref mut file) = self.error_log {
                    // Try to write to the file
                    if let Err(_) = file.write_all(
                        format!(
                            "{} — ERROR: {}\n",
                            now.format("%F %H:%M"),
                            &error
                        )
                        .as_bytes(),
                    
                    // Post a message on error
                    ) {
                        return Notification::Error {
                            message: "Unable To Write To Error Log.".to_string(),
                            time: now,
                            event: None,
                        };
                    }

                // Warn that there is no file
                } else {
                    return Notification::Warning {
                        message: "No Active Error Log.".to_string(),
                        time: now,
                        event: None,
                    };
                }

                // Return the error either way
                Notification::Error {
                    message: error,
                    time: now,
                    event,
                }
            }

            // Simply display warnings and updates
            EventUpdate::Warning(warning, event) => Notification::Warning {
                message: warning,
                time: now,
                event,
            },
            EventUpdate::Update(update) => Notification::Update {
                message: update,
                time: now,
            },

            // Broadcast events and display them
            EventUpdate::Broadcast(id, data) => {
                // Broadcast the event and data, if specified
                self.internal_send.send_broadcast(id.get_id(), data).await;

                // Send a current update with the item pair
                Notification::Current {
                    message: format!("{}", id),
                    time: now,
                }
            }

            // Notify of current events and display them
            EventUpdate::Current(id) => {
                // Send a current update with the item pair
                Notification::Current {
                    message: format!("{}", id),
                    time: now,
                }
            }

            // Update the state of a status
            EventUpdate::Status(status_id, new_state) => {
                // Send the change to the interface
                self.interface_send
                    .send(UpdateStatus {
                        status_id: status_id.clone(),
                        new_state: new_state.clone(),
                    })
                    .unwrap_or(());

                // Return the notification
                Notification::Update {
                    message: format!(
                        "Changing {} To {}.",
                        status_id.description, new_state.description
                    ),
                    time: now,
                }
            }

            // Save data to the system
            EventUpdate::Save(data) => {
                // Try to write the data to the game log
                if let Some(ref mut file) = self.game_log {
                    // Try to write to the file
                    if let Err(_) = file.write_all(
                        format!(
                            "{} — {}\n",
                            now.format("%F %H:%M"),
                            &data
                        )
                        .as_bytes(),
                    
                    // Post a message on error
                    ) {
                        return Notification::Error {
                            message: "Unable To Write To Game Log.".to_string(),
                            time: now,
                            event: None,
                        };
                    }

                // Warn that there is no file
                } else {
                    return Notification::Warning {
                        message: "No Active Game Log.".to_string(),
                        time: now,
                        event: None,
                    };
                }

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
        // Import libraries for testing
        use crate::definitions::{InternalSend, EventUpdate, ItemPair, Hidden};

        // Create the communication lines
        let (internal_send, _internal_recv) = InternalSend::new();
        let (interface_send, _interface_recv) = std_mpsc::channel();

        // Create a new logger instance
        let mut logger = Logger::new(None, None, internal_send, interface_send).unwrap();

        // Pass a series of updates to the logger and verify the output
        let mut result = logger.update(EventUpdate::Error("Test Error".to_string(), None)).await;
        assert_eq!(result[0].message(), "No Active Error Log.".to_string()); // because no error log was specified
        result = logger.update(EventUpdate::Warning("Test Warning".to_string(), None)).await;
        assert_eq!(result[0].message(), "Test Warning".to_string());
        assert_eq!(result[1].message(), "No Active Error Log.".to_string());
        result = logger.update(EventUpdate::Broadcast(ItemPair::new_unchecked(3, "Test Broadcast", Hidden), None)).await;
        assert_eq!(result[0].message(), "Test Broadcast (3)".to_string());
        assert_eq!(result[1].message(), "Test Warning".to_string());
        assert_eq!(result[2].message(), "No Active Error Log.".to_string());
        result = logger.update(EventUpdate::Current(ItemPair::new_unchecked(4, "Test Event", Hidden))).await;
        assert_eq!(result[0].message(), "Test Event (4)".to_string());
        assert_eq!(result[1].message(), "Test Broadcast (3)".to_string());
        assert_eq!(result[2].message(), "Test Warning".to_string());
        assert_eq!(result[3].message(), "No Active Error Log.".to_string());
        result = logger.update(EventUpdate::Update("Test Update".to_string())).await;
        assert_eq!(result[0].message(), "Test Update".to_string());
        assert_eq!(result[1].message(), "Test Event (4)".to_string());
        assert_eq!(result[2].message(), "Test Broadcast (3)".to_string());
        assert_eq!(result[3].message(), "Test Warning".to_string());
        assert_eq!(result[4].message(), "No Active Error Log.".to_string());
    }
}
