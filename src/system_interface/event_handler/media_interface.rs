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
use tokio::runtime::Handle;
use tokio::time::{sleep, Duration};

// Import reqwest elements
use reqwest::{Client};

// Import the failure elements
use failure::Error;

/// A structure to hold and manage the Apollo media player thread
///
struct ApolloThread;

// Implement the ApolloThread Functions
impl ApolloThread {
    /// Spawn the monitoring thread
    async fn spawn(
        internal_send: InternalSend,
        address: String,
        mut window_map: WindowMap,
        mut channel_map: ChannelMap,
    ) {
        // Notify that the background process is starting
        log!(update internal_send => "Starting Apollo Media Player ...");

        // Create the child process
        let mut child = match Command::new("apollo")
            .arg("-a")
            .arg(&address)
            .kill_on_drop(true)
            .spawn()
        {
            // If the child process was created, return it
            Ok(child) => child,

            // Otherwise, warn of the error and return
            _ => {
                // Try looking in the local directory
                match Command::new("./apollo")
                    .arg("-a")
                    .arg(&address)
                    .kill_on_drop(true)
                    .spawn()
                {
                    // If the child process was created, return it
                    Ok(child) => child,

                    // Otherwise, warn of the error and return
                    _ => {
                        log!(err internal_send => "Unable To Start Apollo Media Player.");
                        return;
                    }
                }
            }
        };

        // Spawn a background thread to monitor the process
        Handle::current().spawn(async move {
            // Run indefinitely or until the process fails
            loop {
                // Wait several seconds for the server to start
                sleep(Duration::from_secs(2)).await;

                // Create a client for passing channel definitions
                let tmp_client = Client::new();

                // Define the windows
                for (window_number, window_definition) in window_map.drain() {
                    // Recompose the window definition
                    let window = window_definition.add_number(window_number);

                    // Post the window to Apollo
                    let _ = tmp_client
                        .post(&format!("http://{}/defineWindow", &address))
                        .json(&window)
                        .send()
                        .await;
                }

                // Define all the media channels
                for (channel_number, media_channel) in channel_map.drain() {
                    // Recompose the media channel
                    let channel = media_channel.add_number(channel_number);

                    // Post the channel to Apollo
                    let _ = tmp_client
                        .post(&format!("http://{}/defineChannel", &address))
                        .json(&channel)
                        .send()
                        .await;
                }

                // Wait for the process to finish or the sender to be poisoned
                tokio::select! {
                    // The process has finished
                    result = child.wait() => {
                        match result {
                            // Notify that the process has terminated
                            Ok(_) => log!(err internal_send => "Apollo Media Player Stopped."),

                            // If the process failed to run
                            _ => {
                                log!(err internal_send => "Unable To Run Apollo Media Player.");
                                break;
                            }
                        }
                    }

                    // Check if the internal send line has been dropped
                    _ = internal_send.closed() => break,
                }

                // Wait several seconds to restart the server
                sleep(Duration::from_secs(3)).await;

                // Notify that the background process is restarting
                log!(update internal_send => "Restarting Apollo Media Player ...");

                // Start the process again
                child = match Command::new("apollo")
                    .arg("-a")
                    .arg(&address)
                    .kill_on_drop(true)
                    .spawn()
                {
                    // If the child process was created, return it
                    Ok(child) => child,

                    // Otherwise, warn of the error and end the thread
                    _ => {
                        // Try looking in the local directory
                        match Command::new("./apollo")
                            .arg("-a")
                            .arg(&address)
                            .kill_on_drop(true)
                            .spawn()
                        {
                            // If the child process was created, return it
                            Ok(child) => child,

                            // Otherwise, warn of the error and return
                            _ => {
                                log!(err internal_send => "Unable To Start Apollo Media Player.");
                                break;
                            }
                        }
                    }
                };
            }
        });
    }
}

/// A structure to hold and manipulate the connection to the media backend
///
pub struct MediaInterface {
    channel_list: Vec<u32>, // a list of valid channels for this instance
    client: Option<Client>, // the reqwest client for pass media changes
    address: String,        // the address for requests to Apollo
}

// Implement key functionality for the Media Interface structure
impl MediaInterface {
    /// A function to create a new instance of the MediaInterface
    ///
    pub async fn new(
        internal_send: InternalSend,
        channel_map: ChannelMap,
        window_map: WindowMap,
        apollo_params: ApolloParams,
    ) -> Result<Self, Error> {
        // Copy the specified address or use the default
        let address = apollo_params
            .address
            .clone()
            .unwrap_or(String::from("127.0.0.1:27655"));

        // Collect the list of valid channels
        let channel_list = channel_map.keys().map(|key| key.clone()).collect();

        // Spin out thread to monitor and restart apollo, if requested
        if apollo_params.spawn {
            ApolloThread::spawn(internal_send, address.clone(), window_map, channel_map).await;
        }

        // Return the complete module
        Ok(Self {
            channel_list,
            client: None,
            address,
        })
    }

    // A helper method to send a new media cue
    pub async fn play_cue(&mut self, cue: MediaCue) -> Result<(), Error> {
        // Check that the channel is valid
        if !self.channel_list.contains(&cue.channel) {
            // If not, note the error
            return Err(format_err!("Channel for Media Cue not found."));
        }

        // Create the request client if it doesn't exist
        if self.client.is_none() {
            self.client = Some(Client::new());
        }

        // Recompose the media cue into a helper
        let helper: MediaCueHelper = cue.into();

        // Pass the media cue to Apollo
        self.client
            .as_ref()
            .unwrap()
            .post(&format!("http://{}/cueMedia", &self.address))
            .json(&helper)
            .send().await?;

        // Indicate success
        Ok(())
    }

    // A helper method to adjust the location of a video frame by one pixel in any direction
    pub async fn adjust_media(&mut self, adjustment: MediaAdjustment) -> Result<(), Error> {
        // Check that the channel is valid
        if !self.channel_list.contains(&adjustment.channel) {
            // If not, note the error
            return Err(format_err!("Channel for Media Alignment not found."));
        }

        // Create the request client if it doesn't exist
        if self.client.is_none() {
            self.client = Some(Client::new());
        }

        // Recompose the media cue into a helper
        let helper: MediaAdjustmentHelper = adjustment.into();

        // Pass the media cue to Apollo
        self.client
            .as_ref()
            .unwrap()
            .post(&format!("http://{}/alignChannel", &self.address))
            .json(&helper)
            .send().await?;

        // Indicate success
        Ok(())
    }

    // A helper method to reload a media playlist
    pub async fn restore_playlist(&mut self, mut playlist: MediaPlaylist) {
        // Look through the playlist
        for (channel, playback) in playlist.drain() {
            // Check that the channel is valid
            if !self.channel_list.contains(&channel) {
                // If not, skip this entry
                continue;
            }

            // Create the request client if it doesn't exist
            if self.client.is_none() {
                self.client = Some(Client::new());
            }

            // Extract the media cue as a helper and send it
            let cue: MediaCueHelper = playback.media_cue.into();
            let _ = self.client
                .as_ref()
                .unwrap()
                .post(&format!("http://{}/cueMedia", &self.address))
                .json(&cue)
                .send().await; // Ignore the result

            // Extract the duration change and send it
            let seek = SeekMediaHelper { channel, position: playback.time_since.as_millis() as u64};
            let _ = self.client
                .as_ref()
                .unwrap()
                .post(&format!("http://{}/seek", &self.address))
                .json(&seek)
                .send().await; // Ignore the result
        }
    }
}
