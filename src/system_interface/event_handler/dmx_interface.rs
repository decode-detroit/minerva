// Copyright (c) 2024 Decode Detroit
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

//! A module to load and play video and audio files on this device

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::path::PathBuf;

// Import tokio elements
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

// Import reqwest elements
use reqwest::Client;

// Import tracing features
use tracing::{error, info};

/// A structure to hold and manage the Vulcan DMX controller thread
///
struct VulcanThread;

// Implement the VulcanThread Functions
impl VulcanThread {
    /// Spawn a copy of vulcan and the monitoring thread
    async fn spawn(
        mut receiver: mpsc::Receiver<DmxFade>,
        path: PathBuf,
        address: String,
        backup_location: Option<String>,
    ) {
        // Notify that the background process is starting
        info!("Starting Vulcan DMX controller ...");

        // Compose the arguments
        let mut arguments = vec![
            "-p".into(),
            path.to_str().unwrap_or("").into(),
            "-a".into(),
            address.clone(),
        ];

        // Add the backup location if specified
        if let Some(location) = backup_location {
            arguments.push("-b".into());
            arguments.push(location);
        }

        // Create the child process
        let mut child = match Command::new("vulcan").args(&arguments).spawn() {
            // If the child process was created, return it
            Ok(child) => child,

            // Otherwise, try again in the local directory
            _ => {
                // Try looking in the local directory
                match Command::new("./vulcan").args(&arguments).spawn() {
                    // If the child process was created, return it
                    Ok(child) => child,

                    // Otherwise, warn of the error and return
                    _ => {
                        error!("Unable to start Vulcan DMX controller.");
                        return;
                    }
                }
            }
        };

        // Create a client for passing dmx information
        let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
            // On error close the monitoring thread
            Err(_) => {
                error!("Unable to create Vulcan communication client.");
                return;
            }

            // Otherwise, continue
            Ok(client) => client,
        };

        // Wait a second for the server to start
        sleep(Duration::from_secs(1)).await;

        // Spawn a background thread to monitor the process
        tokio::spawn(async move {
            // Run indefinitely or until the process fails
            loop {
                // Wait for a message, the process to finish, or the sender to be poisoned
                tokio::select! {
                    // The process has finished
                    result = child.wait() => {
                        match result {
                            // Notify that the process has terminated
                            Ok(_) => error!("Vulcan DMX controller stopped."),

                            // If the process failed to run
                            _ => {
                                error!("Unable to run Vulcan DMX controller.");
                                break;
                            }
                        }
                    }

                    // A message was received (or the line dropped)
                    possible_fade = receiver.recv() => {
                        // If a fade was received
                        if let Some(fade) = possible_fade {
                            // Recompose the dmx fade into a helper
                            let helper: DmxFadeHelper = fade.into();

                            // Pass the dmx fade on to Vulcan
                            if let Err(err) = client.post(format!("http://{}/playFade", address)).json(&helper).send().await {
                                error!("Error with DMX Fade: {}", err);
                            };

                            // Start listening again for more messages
                            continue;

                        // Otherwise, the sending line has been dropped
                        } else {
                            // Notify of the closure
                            info!("Closing Vulcan DMX controller ...");

                            // Tell Vulcan to close
                            let _ = client.post(format!("http://{}/close", address)).send().await;

                            // Exit the loop and close the background thread
                            break;
                        }
                    }
                }

                // Wait several seconds to restart the server
                sleep(Duration::from_secs(2)).await;

                // Notify that the background process is restarting
                info!("Restarting Vulcan DMX contoller ...");

                // Start the process again
                child = match Command::new("vulcan").args(&arguments).spawn() {
                    // If the child process was created, return it
                    Ok(child) => child,

                    // Otherwise, try again in the local directory
                    _ => {
                        // Try looking in the local directory
                        match Command::new("./vulcan").args(&arguments).spawn() {
                            // If the child process was created, return it
                            Ok(child) => child,

                            // Otherwise, warn of the error and return
                            _ => {
                                error!("Unable to start Vulcan DMX controller.");
                                break;
                            }
                        }
                    }
                };

                // Wait a second for the server to start
                sleep(Duration::from_secs(1)).await;
            }
        });
    }

    /// Spawn the monitoring thread only
    async fn no_spawn(mut receiver: mpsc::Receiver<DmxFade>, address: String) {
        // Notify that the background process is starting
        info!("Connection to Vulcan DMX controller ...");

        // Create a client for passing dmx information
        let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
            // On error close the monitoring thread
            Err(_) => {
                error!("Unable to create Vulcan communication client.");
                return;
            }

            // Otherwise, continue
            Ok(client) => client,
        };

        // Spawn a background thread to communicate
        tokio::spawn(async move {
            // Run indefinitely or until the line is closed
            loop {
                // If a fade was received
                if let Some(fade) = receiver.recv().await {
                    // Recompose the dmx fade into a helper
                    let helper: DmxFadeHelper = fade.into();

                    // Pass the dmx fade on to Vulcan
                    if let Err(err) = client
                        .post(format!("http://{}/playFade", address))
                        .json(&helper)
                        .send()
                        .await
                    {
                        error!("Error with DMX Fade: {}", err);
                    };

                    // Start listening again for more messages
                    continue;

                // Otherwise, the sending line has been dropped
                } else {
                    // Notify of the closure
                    info!("Disconnecting from Vulcan DMX controller ...");

                    // Exit the loop and close the background thread
                    break;
                }
            }
        });
    }
}

/// A structure to hold and manipulate the connection to the dmx backend
///
pub struct DmxInterface {
    sender: mpsc::Sender<DmxFade>, // a line to pass fades to the background thread. The line is poisoned when this structure is dropped
}

// Implement key functionality for the DMX Interface structure
impl DmxInterface {
    /// A function to create a new instance of the MediaInterface
    ///
    pub async fn new(vulcan_params: VulcanParams, backup_location: Option<String>) -> Self {
        // Copy the specified address or use the default
        let address = vulcan_params
            .address
            .clone()
            .unwrap_or(String::from("127.0.0.1:8852"));

        // Create a channel to notify the background thread to close
        let (sender, receiver) = mpsc::channel(512);

        // Spin out thread to monitor and restart vulcan, if requested
        if vulcan_params.spawn {
            VulcanThread::spawn(
                receiver,
                vulcan_params.path.unwrap_or_default(),
                address,
                backup_location,
            )
            .await;

        // Otherwise, just spin the background thread for communication
        } else {
            VulcanThread::no_spawn(receiver, address).await;
        }

        // Return the complete module
        Self { sender }
    }

    /// A method to send a new dmx fade to the dmx controller
    ///
    /// This method passes the request to the background thread for processing.
    /// If the request fails, the error will be passed through the tracing library.
    ///
    pub async fn play_fade(&mut self, fade: DmxFade) {
        // Verify the range of the selected channel
        if (fade.channel > DMX_MAX) | (fade.channel < 1) {
            error!("Error with DMX playback: Selected DMX channel is out of range.");
            return;
        }

        // Send the dmx fade to the background thread
        self.sender.send(fade).await.unwrap_or(());
    }
}
