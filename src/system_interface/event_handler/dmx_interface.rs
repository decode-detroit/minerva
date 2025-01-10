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
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

// Import reqwest elements
use reqwest::Client;

// Import tracing features
use tracing::{error, info};

// Import anyhow features
use anyhow::Result;

/// A structure to hold and manage tthe Vulcan DMX controller thread
///
struct VulcanThread;

// Implement the VulcanThread Functions
impl VulcanThread {
    /// Spawn the monitoring thread
    async fn spawn(
        mut close_receiver: mpsc::Receiver<()>,
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

        // Spawn a background thread to monitor the process
        Handle::current().spawn(async move {
            // Run indefinitely or until the process fails
            loop {
                // Wait a second for the server to start
                sleep(Duration::from_secs(1)).await;

                // Create a client for passing dmx information
                let tmp_client = Client::new();

                // Wait for the process to finish or the sender to be poisoned
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

                    // Check if the close notification line has been dropped
                    _ = close_receiver.recv() => {
                        // Notify of the closure
                        info!("Closing Vulcan DMX controller ...");

                        // Tell Vulcan to close
                        let _ = tmp_client
                                        .post(&format!("http://{}/close", &address))
                                        .send()
                                        .await;

                        // Exit the loop and close the background thread
                        break;
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
            }
        });
    }
}

/// A structure to hold and manipulate the connection to the dmx backend
///
pub struct DmxInterface {
    client: Option<Client>, // the reqwest client for passing media changes
    address: String,        // the address for requests to Apollo
    _close_sender: mpsc::Sender<()>, // a line to notify the background thread to close
                            // the line is never used, but is poisoned when dropped
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
        let (_close_sender, close_receiver) = mpsc::channel(1); // don't need space for any messages

        // Spin out thread to monitor and restart vulcan, if requested
        if vulcan_params.spawn {
            VulcanThread::spawn(
                close_receiver,
                vulcan_params.path.unwrap_or(PathBuf::new()),
                address.clone(),
                backup_location,
            )
            .await;
        }

        // Return the complete module
        Self {
            client: None,
            address,
            _close_sender,
        }
    }

    // A helper method to send a new dmx fade
    pub async fn play_fade(&mut self, fade: DmxFade) -> Result<()> {
        // Verify the range of the selected channel
        if (fade.channel > DMX_MAX) | (fade.channel < 1) {
            return Err(anyhow!("Selected DMX channel is out of range."));
        }

        // Create the request client if it doesn't exist
        if self.client.is_none() {
            self.client = Some(Client::new());
        }

        // Recompose the dmx fade into a helper
        let helper: DmxFadeHelper = fade.into();

        // Pass the dmx fade on to Vulcan
        self.client
            .as_ref()
            .unwrap()
            .post(&format!("http://{}/playFade", &self.address))
            .json(&helper)
            .send()
            .await?;

        // Indicate success
        Ok(())
    }

    // A helper method to reload a DMX universe
    #[allow(dead_code)] // Allow dead code, reserved for future use
    pub async fn restore_universe(&mut self, universe: DmxUniverse) -> Result<()> {
        // Create the request client if it doesn't exist
        if self.client.is_none() {
            self.client = Some(Client::new());
        }

        // Recompose the dmx fade into a helper
        let helper: DmxUniverseHelper = universe.into();

        // Pass the dmx fade on to Vulcan
        self.client
            .as_ref()
            .unwrap()
            .post(&format!("http://{}/loadUniverse", &self.address))
            .json(&helper)
            .send()
            .await?;

        // Indicate success
        Ok(())
    }
}
