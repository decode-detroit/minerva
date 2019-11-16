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
use super::event_handler::event::EventUpdate;
use super::{GeneralUpdate, InterfaceUpdate, ItemPair, UpdateStatus};

// Import standard library modules
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc;

// Import the failure features
use failure::Error as FailureError;

// Import the eternal time library
extern crate time;

/// An enum to contain system notifications in different types.
///
/// This notification type mirrors the event update type, but is only allowed
/// to contain strings for display to the user and the system time of the
/// notification (no other types, as in event update). This type also omits
/// several of the variants described in the event update as they are not
/// needed to be displayed to the user.
///
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Notification {
    /// An error type of notification
    Error {
        message: String,
        time: time::Tm,
        event: Option<ItemPair>,
    },

    /// A warning type of notification
    Warning {
        message: String,
        time: time::Tm,
        event: Option<ItemPair>,
    },

    /// A current event type of notification
    Current { message: String, time: time::Tm },

    /// Any other type of system update
    Update { message: String, time: time::Tm },
}

// Reexport the notification type variants
pub use self::Notification::{Current, Error, Update, Warning};

// Implement key features for the Notification type
impl Notification {
    /// A function to return a copy of the message inside the notification,
    /// regardless of variant.
    ///
    pub fn message(&self) -> String {
        match self {
            // For every variant type, return a copy of the message
            &Error { ref message, .. } => message.clone(),
            &Warning { ref message, .. } => message.clone(),
            &Current { ref message, .. } => message.clone(),
            &Update { ref message, .. } => message.clone(),
        }
    }

    /// A function to return a copy of the time inside the notification,
    /// regardless of variant.
    ///
    pub fn time(&self) -> time::Tm {
        match self {
            // For every variant type, return a copy of the message
            &Error { ref time, .. } => time.clone(),
            &Warning { ref time, .. } => time.clone(),
            &Current { ref time, .. } => time.clone(),
            &Update { ref time, .. } => time.clone(),
        }
    }
}

// Implement the display formatting for notifications.
impl fmt::Display for Notification {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // For every variant type, combine the message and notification time
            &Error {
                ref message,
                ref time,
                ..
            } => write!(
                f,
                "{}: {}",
                time.strftime("%a %T").unwrap_or_else(|_| time.asctime()),
                message
            ),
            &Warning {
                ref message,
                ref time,
                ..
            } => write!(
                f,
                "{}: {}",
                time.strftime("%a %T").unwrap_or_else(|_| time.asctime()),
                message
            ),
            &Current {
                ref message,
                ref time,
            } => write!(
                f,
                "{}: {}",
                time.strftime("%a %T").unwrap_or_else(|_| time.asctime()),
                message
            ),
            &Update {
                ref message,
                ref time,
            } => write!(
                f,
                "{}: {}",
                time.strftime("%a %T").unwrap_or_else(|_| time.asctime()),
                message
            ),
        }
    }
}

/// A structure to handle all logging and update processing for the program.
///
pub struct Logger {
    game_log: Option<File>,                        // game log file for the program
    error_log: Option<File>,                       // error log file for the program
    old_notifications: Vec<Notification>, // internal list of notifications less than 1 minute old
    general_update: GeneralUpdate,        // broadcast channel for current events
    interface_send: mpsc::Sender<InterfaceUpdate>, // an update line for passing updates to the user interface
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
        general_update: GeneralUpdate,
        interface_send: mpsc::Sender<InterfaceUpdate>,
    ) -> Result<Logger, FailureError> {
        // Attempt to open the game log file
        let game_log = match log_path {
            // If a file was specified, try to load it
            Some(mut filepath) => {
                // Use the current time for each instance
                let time = time::now();
                filepath.push(
                    format!(
                        "game_log_{:04}-{:02}-{:02}_{:02}-{:02}.txt",
                        time.tm_year + 1900,
                        time.tm_mon + 1,
                        time.tm_mday,
                        time.tm_hour,
                        time.tm_min
                    )
                    .as_str(),
                );

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
            // If a file was specified, try to load it
            Some(filepath) => match File::create(filepath.to_str().unwrap_or("")) {
                Ok(file) => Some(file),
                Err(_) => return Err(format_err!("Unable to create error log file.")),
            },

            // If a file was not specified, run without a log file
            None => None,
        };

        // Return the new logger
        Ok(Logger {
            game_log,
            error_log,
            old_notifications: Vec::new(),
            general_update,
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
    pub fn update(&mut self, update: EventUpdate) -> Vec<Notification> {
        // Unpack the new update into a notification
        let mut notifications = vec![self.unpack_update(update)];

        // Iterate through the old notifications
        for old_note in self.old_notifications.drain(..) {
            // If the notification is younger than one minute, add it back
            if (time::now() - old_note.time()) < time::Duration::minutes(1) {
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
    fn unpack_update(&mut self, update: EventUpdate) -> Notification {
        // Unpack the event update based on its subtype
        match update {
            // Log and display errors
            EventUpdate::Error(error, event) => {
                // Note the current time
                let now = time::now();

                // Try to write it to the file
                if let Some(ref mut file) = self.error_log {
                    file.write(
                        format!(
                            "{:04}-{:02}-{:02} {:02}:{:02} — ERROR: {}\n",
                            now.tm_year + 1900,
                            now.tm_mon + 1,
                            now.tm_mday,
                            now.tm_hour,
                            now.tm_min,
                            &error
                        )
                        .as_bytes(),
                    )
                    .unwrap_or(0);
                }

                // Return the error either way
                Error {
                    message: error,
                    time: now,
                    event,
                }
            }

            // Simply display warnings and updates
            EventUpdate::Warning(warning, event) => Warning {
                message: warning,
                time: time::now(),
                event,
            },
            EventUpdate::Update(update) => Update {
                message: update,
                time: time::now(),
            },

            // Broadcast events and display them
            EventUpdate::Broadcast(id) => {
                // Broadcast the event
                self.general_update.send_broadcast(id.get_id());

                // Send a current update with the item pair
                Current {
                    message: format!("{}", id),
                    time: time::now(),
                }
            }

            // Broadcast events with data and display them
            EventUpdate::BroadcastData(id, data) => {
                // Broadcast the event
                self.general_update.send_broadcast_data(id.get_id(), data);

                // Send a current update with the item pair
                Current {
                    message: format!("{}", id),
                    time: time::now(),
                }
            }

            // Notify of current events and display them
            EventUpdate::Current(id) => {
                // Send a current update with the item pair
                Current {
                    message: format!("{}", id),
                    time: time::now(),
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
                Update {
                    message: format!(
                        "Changing {} To {}.",
                        status_id.description, new_state.description
                    ),
                    time: time::now(),
                }
            }

            // Save data to the system
            EventUpdate::Save(data) => {
                // Get the current time
                let now = time::now();

                // Try to write the data to the game log
                if let Some(ref mut file) = self.game_log {
                    file.write(
                        format!(
                            "{:04}-{:02}-{:02} {:02}:{:02} — {:?}\n",
                            now.tm_year + 1900,
                            now.tm_mon + 1,
                            now.tm_mday,
                            now.tm_hour,
                            now.tm_min,
                            &data
                        )
                        .as_bytes(),
                    )
                    .unwrap_or(0);
                }

                // Print the full data to the notification area
                #[cfg(test)]
                return Current {
                    message: format!("Got Data: {:?}", data),
                    time: now,
                };

                // Otherwise return a message about saving the data
                #[cfg(not(test))]
                return Current {
                    message: "Saved Data To Game Log.".to_string(),
                    time: now,
                };
            }
        }
    }
}

// Tests of the logging module
#[cfg(test)]
mod tests {
    use super::*;

    // Test the logging module
    /*#[test]
    fn test_logging() {
        // Import libraries for testing
        use super::super::super::GeneralUpdate;
        use super::super::super::GeneralUpdateType;

        // Create the output lines
        let (gen_tx, gen_rx) = GeneralUpdate::new();
        let (int_tx, int_rx) = mpsc::channel();

        // Create an empty logger instance
        Logger::new(None, None,

        // Generate a few messages
        update!(err tx => "Test Error {}", 1);
        update!(warn tx => "Test Warning {}", 2);
        update!(broadcast tx => ItemPair::new(3, "Test Event 3", Hidden).unwrap());
        update!(now tx => ItemPair::new(4, "Test Event 4", Hidden).unwrap());
        update!(update tx => "Test Update {}", "5");

        // Create the test vector
        let test = vec![
            GeneralUpdateType::Update(Error("Test Error 1".to_string())),
            GeneralUpdateType::Update(Warning("Test Warning 2".to_string())),
            GeneralUpdateType::Update(Broadcast(ItemPair::new(3, "Test Event 3", Hidden).unwrap())),
            GeneralUpdateType::Update(Current(ItemPair::new(4, "Test Event 4", Hidden).unwrap())),
            GeneralUpdateType::Update(Update("Test Update 5".to_string())),
        ];

        // Print and check the messages received (wait at most half a second)
        test_vec!(=rx, test);
    }*/
}
