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

// Import other definitions
use super::{EventConnection, ReadResult};

// Import tokio elements
#[cfg(feature = "media-out")]
use tokio::runtime::Handle;
#[cfg(feature = "media-out")]
use tokio::process::Command;
#[cfg(feature = "media-out")]
use tokio::time::{sleep, Duration};

// Import reqwest elements
#[cfg(feature = "media-out")]
use reqwest::Client as AsyncClient;
#[cfg(feature = "media-out")]
use reqwest::blocking::Client;

// Import the failure elements
use failure::Error;

/// A structure to hold and manage the Apollo media player thread
///
#[cfg(feature = "media-out")]
struct ApolloThread;

// Implement the ApolloThread Functions
#[cfg(feature = "media-out")]
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
        let mut child = match Command::new("apollo").arg("-a").arg(&address).kill_on_drop(true).spawn() {
            // If the child process was created, return it
            Ok(child) => child,

            // Otherwise, warn of the error and return
            _ => {
                // Try looking in the local directory
                match Command::new("./apollo").arg("-a").arg(&address).kill_on_drop(true).spawn() {
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
                sleep(Duration::from_secs(3)).await;

                // Create a client for passing channel definitions
                let tmp_client = AsyncClient::new();
                    
                // Define the windows
                for (window_number, window_definition) in window_map.drain() {
                    // Recompose the window definition
                    let window = window_definition.add_number(window_number);

                    // Post the window to Apollo
                    let response = tmp_client.post(&format!("http://{}/defineWindow", &address)).json(&window).send().await;
                }

                // Define all the media channels
                for (channel_number, media_channel) in channel_map.drain() {
                    // Recompose the media channel
                    let channel = media_channel.add_number(channel_number);

                    // Post the channel to Apollo
                    let response = tmp_client.post(&format!("http://{}/defineChannel", &address)).json(&channel).send().await;
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
                child = match Command::new("apollo").arg("-a").arg(&address).kill_on_drop(true).spawn() {
                    // If the child process was created, return it
                    Ok(child) => child,

                    // Otherwise, warn of the error and end the thread
                    _ => {
                        // Try looking in the local directory
                        match Command::new("./apollo").arg("-a").arg(&address).kill_on_drop(true).spawn() {
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
pub struct MediaOut {
    #[cfg(feature = "media-out")]
    all_stop_media: Vec<MediaCue>,  // a vector of media cues for all stop
    media_map: MediaMap,            // the map of event ids to media cues
    #[cfg(feature = "media-out")]
    client: Option<Client>,         // the reqwest client for pass media changes
    #[cfg(feature = "media-out")]
    address: String,                // the address for requests to Apollo
}

// Implement key functionality for the Media Out structure
impl MediaOut {
    /// A function to create a new instance of the MediaOut, active version
    ///
    #[cfg(feature = "media-out")]
    pub async fn new(
        internal_send: InternalSend,
        all_stop_media: Vec<MediaCue>,
        media_map: MediaMap,
        channel_map: ChannelMap,
        window_map: WindowMap,
        apollo_params: ApolloParams,
    ) -> Result<MediaOut, Error> {
        // Copy the specified address or use the default
        let address = apollo_params.address.clone().unwrap_or(String::from("127.0.0.1:27655"));
        
        // Spin out thread to monitor and restart apollo, if requested
        if apollo_params.spawn {
            ApolloThread::spawn(internal_send, address.clone(), window_map, channel_map).await;
        }

        // Return the complete module
        Ok(MediaOut {
            all_stop_media,
            media_map,
            client: None,
            address,
        })
    }

    /// A function to create a new instance of the MediaOut, inactive version
    ///
    #[cfg(not(feature = "media-out"))]
    pub async fn new(
        _internal_send: InternalSend,
        _all_stop_media: Vec<MediaCue>,
        media_map: MediaMap,
        _channel_map: ChannelMap,
        _window_map: WindowMap,
        _apollo: ApolloParams,
    ) -> Result<MediaOut, Error> {
        // Return a partial module
        Ok(MediaOut { media_map })
    }

    // A helper function to add a new media cue
    #[cfg(feature = "media-out")]
    pub fn add_cue(&self, cue: MediaCue) -> Result<(), Error> {
        // Recompose the media cue into a helper
        let helper = cue.into_helper();
        
        // Pass the media cue to Apollo
        let response = self.client.as_ref().unwrap().post(&format!("http://{}/cueMedia", &self.address)).json(&helper).send()?;

        // Indicate success
        Ok(())
    }
}

// Implement the event connection trait for Media Out
impl EventConnection for MediaOut {
    /// A method to receive a new event, empty for this connection type
    ///
    fn read_events(&mut self) -> Vec<ReadResult> {
        Vec::new() // return an empty vector
    }

    /// A method to send a new event to the media connection, active version
    ///
    #[cfg(feature = "media-out")]
    fn write_event(&mut self, id: ItemId, _data1: u32, _data2: u32) -> Result<(), Error> {
        // Create the request client if it doen't exist
        if self.client.is_none() {
            self.client = Some(Client::new());
        }
        
        // Check to see if the event is all stop
        if id == ItemId::all_stop() {
            // Stop all the currently playing media
            let response = self.client.as_ref().unwrap().post(&format!("http://{}/allStop", &self.address)).send()?;

            // Run all of the all stop media, ignoring errors
            for media_cue in self.all_stop_media.iter() {
                // Add the media cues
                self.add_cue(media_cue.clone()).unwrap_or(());
            }

        // Check to see if the event is in the media map
        } else {
            // Pass the new media cue
            if let Some(media_cue) = self.media_map.get(&id) {
                self.add_cue(media_cue.clone())?;
            }
        }

        // If the event wasn't found or was processed correctly, indicate success
        Ok(())
    }

    /// A method to send a new event to the media connection, inactive version
    ///
    #[cfg(not(feature = "media-out"))]
    fn write_event(&mut self, id: ItemId, _data1: u32, _data2: u32) -> Result<(), Error> {
        // Show an error if compiled without the media module
        if let Some(_) = self.media_map.get(&id) {
            return Err(format_err!(
                "Program compiled without media support. See documentation."
            ));
        } else {
            return Ok(());
        }
    }

    /// A method to echo an event to the media connection
    fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        self.write_event(id, data1, data2)
    }
}
