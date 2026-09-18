// Copyright (c) 2020 Decode Detroit
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

// Import tokio elements
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

// Import reqwest elements
use reqwest::Client;

// Import tracing features
use tracing::{error, info};

/// A helper enum to pass update types to the background thread
///
enum ApolloUpdate {
    MediaCue(MediaCue),
    MediaAdjustment(MediaAdjustment),
}

/// A structure to hold and manage the Apollo media player thread
///
struct ApolloThread;

// Implement the ApolloThread Functions
impl ApolloThread {
    /// Spawn a copy of Apollo and the monitoring thread
    async fn spawn(
        mut receiver: mpsc::Receiver<ApolloUpdate>,
        address: String,
        backup_location: Option<String>,
        mut window_map: WindowMap,
        mut channel_map: ChannelMap,
    ) {
        // Notify that the background process is starting
        info!("Starting Apollo media player ...");

        // Compose the arguments
        let mut arguments = vec!["-a".into(), address.clone()];

        // Add the backup location if specified
        if let Some(location) = backup_location {
            arguments.push("-b".into());
            arguments.push(location);
        }

        // Create the child process
        let mut child = match Command::new("apollo").args(&arguments).spawn() {
            // If the child process was created, return it
            Ok(child) => child,

            // Otherwise, try again in the local directory
            _ => {
                // Try looking in the local directory
                match Command::new("./apollo").args(&arguments).spawn() {
                    // If the child process was created, return it
                    Ok(child) => child,

                    // Otherwise, warn of the error and return
                    _ => {
                        error!("Unable to start Apollo media player.");
                        return;
                    }
                }
            }
        };

        // Create a client for passing Apollo updates
        let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
            // On error close the monitoring thread
            Err(_) => {
                error!("Unable to create Apollo communication client.");
                return;
            }

            // Otherwise, continue
            Ok(client) => client,
        };

        // Wait a second for the server to start
        sleep(Duration::from_secs(1)).await;

        // Define the windows
        for (window_number, window_definition) in window_map.drain() {
            // Recompose the window definition
            let window = window_definition.add_number(window_number);

            // Post the window to Apollo
            if let Err(err) = client
                .post(format!("http://{}/defineWindow", address))
                .json(&window)
                .send()
                .await
            {
                error!("Unable to define Apollo window: {}", err);
            }
        }

        // Define all the media channels
        for (channel_number, media_channel) in channel_map.drain() {
            // Recompose the media channel
            let channel = media_channel.add_number(channel_number);

            // Post the channel to Apollo
            if let Err(err) = client
                .post(format!("http://{}/defineChannel", address))
                .json(&channel)
                .send()
                .await
            {
                error!("Unable to define Apollo channel: {}", err);
            }
        }

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
                            Ok(_) => error!("Apollo media player stopped."),

                            // If the process failed to run
                            _ => {
                                error!("Unable to run Apollo media player.");
                                break;
                            }
                        }
                    }

                    // A message was received (or the line dropped)
                    possible_update = receiver.recv() => {
                        // If an update was received
                        if let Some(update) = possible_update {
                            match update {
                                ApolloUpdate::MediaCue(cue) => {
                                    // Recompose the media cue into a helper
                                    let helper: MediaCueHelper = cue.into();

                                    // Pass the media cue to Apollo
                                    if let Err(err) = client.post(format!("http://{}/cueMedia", address)).json(&helper).send().await {
                                        error!("Error with Cue Media: {}", err);
                                    }
                                }

                                ApolloUpdate::MediaAdjustment(adjustment) => {
                                    // Recompose the media cue into a helper
                                    let helper: MediaAdjustmentHelper = adjustment.into();

                                    // Pass the media cue to Apollo
                                    if let Err(err) = client.post(format!("http://{}/alignChannel", address)).json(&helper).send().await {
                                        error!("Error with Adjust Media: {}", err);
                                    }
                                }
                            }

                            // Start listening again for more messages
                            continue;

                        // Otherwise, the sending line has been dropped
                        } else {
                            // Notify of the closure
                            info!("Closing Apollo media player ...");

                            // Tell Apollo to close
                            let _ = client.post(format!("http://{}/close", address)).send().await;

                            // Exit the loop and close the background thread
                            break;
                        }
                    }
                }

                // Wait several seconds to restart the server
                sleep(Duration::from_secs(2)).await;

                // Notify that the background process is restarting
                info!("Restarting Apollo media player ...");

                // Start the process again
                child = match Command::new("apollo").args(&arguments).spawn() {
                    // If the child process was created, return it
                    Ok(child) => child,

                    // Otherwise, warn of the error and end the thread
                    _ => {
                        // Try looking in the local directory
                        match Command::new("./apollo").args(&arguments).spawn() {
                            // If the child process was created, return it
                            Ok(child) => child,

                            // Otherwise, warn of the error and return
                            _ => {
                                error!("Unable to start Apollo media player.");
                                break;
                            }
                        }
                    }
                };

                // Wait a second for the server to start
                sleep(Duration::from_secs(1)).await;

                // Define the windows
                for (window_number, window_definition) in window_map.drain() {
                    // Recompose the window definition
                    let window = window_definition.add_number(window_number);

                    // Post the window to Apollo
                    if let Err(err) = client
                        .post(format!("http://{}/defineWindow", address))
                        .json(&window)
                        .send()
                        .await
                    {
                        error!("Unable to define Apollo window: {}", err);
                    }
                }

                // Define all the media channels
                for (channel_number, media_channel) in channel_map.drain() {
                    // Recompose the media channel
                    let channel = media_channel.add_number(channel_number);

                    // Post the channel to Apollo
                    if let Err(err) = client
                        .post(format!("http://{}/defineChannel", address))
                        .json(&channel)
                        .send()
                        .await
                    {
                        error!("Unable to define Apollo channel: {}", err);
                    }
                }
            }
        });
    }

    /// Spawn the monitoring thread only
    async fn no_spawn(mut receiver: mpsc::Receiver<ApolloUpdate>, address: String) {
        // Notify that the background process is starting
        info!("Connecting to Apollo media player ...");

        // Create a client for passing Apollo updates
        let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
            // On error close the monitoring thread
            Err(_) => {
                error!("Unable to create Apollo communication client.");
                return;
            }

            // Otherwise, continue
            Ok(client) => client,
        };

        // Spawn a background thread to communicate
        tokio::spawn(async move {
            // Run indefinitely or until the line is closed
            loop {
                // If an update was received
                if let Some(update) = receiver.recv().await {
                    match update {
                        ApolloUpdate::MediaCue(cue) => {
                            // Recompose the media cue into a helper
                            let helper: MediaCueHelper = cue.into();

                            // Pass the media cue to Apollo
                            if let Err(err) = client
                                .post(format!("http://{}/cueMedia", address))
                                .json(&helper)
                                .send()
                                .await
                            {
                                error!("Error with Cue Media: {}", err);
                            }
                        }

                        ApolloUpdate::MediaAdjustment(adjustment) => {
                            // Recompose the media cue into a helper
                            let helper: MediaAdjustmentHelper = adjustment.into();

                            // Pass the media cue to Apollo
                            if let Err(err) = client
                                .post(format!("http://{}/alignChannel", address))
                                .json(&helper)
                                .send()
                                .await
                            {
                                error!("Error with Adjust Media: {}", err);
                            }
                        }
                    }

                // Otherwise, the sending line has been dropped
                } else {
                    // Notify of the closure
                    info!("Disconnecting from Apollo media player ...");

                    // Exit the loop and close the background thread
                    break;
                }
            }
        });
    }
}

/// A structure to hold and manipulate the connection to the media backend
///
pub struct MediaInterface {
    channel_list: Vec<u32>, // a list of valid channels for this instance
    sender: mpsc::Sender<ApolloUpdate>, // a line to pass updates to the background thread. The line is poisoned when this structure is dropped
}

// Implement key functionality for the Media Interface structure
impl MediaInterface {
    /// A function to create a new instance of the MediaInterface
    ///
    pub async fn new(
        channel_map: ChannelMap,
        window_map: WindowMap,
        apollo_params: ApolloParams,
        backup_location: Option<String>,
    ) -> Self {
        // Copy the specified address or use the default
        let address = apollo_params
            .address
            .clone()
            .unwrap_or(String::from("127.0.0.1:27655"));

        // Collect the list of valid channels
        let channel_list = channel_map.keys().copied().collect();

        // Create a channel to notify the background thread to close
        let (sender, receiver) = mpsc::channel(512); // don't need space for any messages

        // Spin out thread to monitor and restart apollo, if requested
        if apollo_params.spawn {
            ApolloThread::spawn(receiver, address, backup_location, window_map, channel_map).await;

        // Otherwise, just spin the background thread for communication
        } else {
            ApolloThread::no_spawn(receiver, address).await;
        }

        // Return the complete module
        Self {
            channel_list,
            sender,
        }
    }

    /// A method to send a new media cue to the media controller
    ///
    /// This method passes the request to the background thread for processing.
    /// If the request fails, the error will be passed through the tracing library.
    ///
    pub async fn play_cue(&mut self, cue: MediaCue) {
        // If there is a channel list
        if !self.channel_list.is_empty() {
            // Check that the channel is valid
            if !self.channel_list.contains(&cue.channel) {
                // If not, note the error and return
                error!("Channel for Media Cue not found.");
                return;
            }

        // Return if there is no channel list
        } else {
            error!("No media channels have been specified.");
            return;
        }

        // Send the media cue to the background thread
        self.sender
            .send(ApolloUpdate::MediaCue(cue))
            .await
            .unwrap_or(());
    }

    /// A method to adjust the location of a video frame by one pixel in any direction
    ///
    /// This method passes the request to the background thread for processing.
    /// If the request fails, the error will be passed through the tracing library.
    ///
    pub async fn adjust_media(&mut self, adjustment: MediaAdjustment) {
        // If there is a channel list
        if !self.channel_list.is_empty() {
            // Check that the channel is valid
            if !self.channel_list.contains(&adjustment.channel) {
                // If not, note the error and return
                error!("Channel for Media Cue not found.");
                return;
            }

        // Return if there is no channel list
        } else {
            error!("No media channels have been specified.");
            return;
        }

        // Send the media adjustment to the background thread
        self.sender
            .send(ApolloUpdate::MediaAdjustment(adjustment))
            .await
            .unwrap_or(());
    }
}
