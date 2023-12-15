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

//! A module to communicate using a DMX serial connection
//!
//! # Note
//! This module is currently limited to Enttec DMX USB Pro-compatible hardware.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::path::Path;
use std::time::{Duration, Instant};

// Import the tokio and tokio serial features
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_serial as serial;

// Import FNV HashMap
use fnv::FnvHashMap;

// Import anyhow features
use anyhow::Result;

// Define the communication constants
const COMMAND_START: u8 = 0x7E as u8; // the start of the command
const MESSAGE_LABEL: u8 = 0x06 as u8; // the message type label
const DATA_LSB: u8 = 0x01 as u8; // the data least significant bit
const DATA_MSB: u8 = 0x02 as u8; // the data most significant bit
const DMX_START_CODE: u8 = 0x00 as u8; // the DMX start code
const COMMAND_END: u8 = 0xE7 as u8; // the end of the command

// Define fade constants
const RESOLUTION: u64 = 50; // the time resolution of each fade, in ms

/// A structure to hold and manipulate the connection over serial
///
pub struct DmxInterface {
    load_fade: mpsc::Sender<DmxFade>, // a line to load the dmx fade into the queue
}

// Implement key functionality for the DMX structure
impl DmxInterface {
    /// A function to create a new instance of the DmxOut
    ///
    pub fn new(path: &Path) -> Result<Self> {
        // Create and configure a builder to connect to the underlying serial port
        let builder = serial::new(path.to_str().unwrap_or(""), 9600)
            .data_bits(serial::DataBits::Eight)
            .parity(serial::Parity::None)
            .stop_bits(serial::StopBits::One)
            .flow_control(serial::FlowControl::None)
            .timeout(Duration::from_millis(100));

        // Try to open the serial port
        let stream = serial::SerialStream::open(&builder)?;

        // Create a new DMX queue
        let (load_fade, receive_fade) = mpsc::channel(128);
        let mut dmx_queue = DmxQueue::new(stream, receive_fade);

        // Start the dmx queue thread
        tokio::spawn(async move {
            dmx_queue.run_loop().await;
        });

        // Return the new DmxOut instance
        Ok(Self { load_fade })
    }

    /// A method to play a new Dmx fade
    ///
    pub async fn play_fade(&self, fade: DmxFade) -> Result<()> {
        // Verify the range of the selected channel
        if (fade.channel > DMX_MAX) | (fade.channel < 1) {
            return Err(anyhow!("Selected DMX channel is out of range."));
        }

        // Send the fade to the background thread
        if let Err(_) = self.load_fade.send(fade).await {
            return Err(anyhow!("Background DMX thread has crashed."));
        }

        // If the fade was processed correctly, indicate success
        Ok(())
    }

    /// A method to load existing values for all the Dmx channels
    ///
    pub async fn restore_universe(&self, universe: DmxUniverse) {
        // For each channel, send a fade with no duration
        for channel in 1..DMX_MAX {
            self.load_fade
                .send(DmxFade {
                    channel,
                    value: universe.get(channel),
                    duration: None,
                })
                .await
                .unwrap_or(()); // fail silently
        }
    }
}

/// A convenience enum to indicate whether the dmx fade is still ongoing or is
/// complete.
enum FadeStatus {
    /// a variant indicating the fade is still in progress
    Ongoing(u8),

    /// a variant indicating that the fade is complete
    Complete(u8),
}

/// A struct to allow easier manipulation of queued DMX changes.
#[derive(Copy, Clone, PartialEq, Debug)]
struct DmxChange {
    start_time: Instant, // the original start time of the fade
    difference: f64,     // the difference between the start value and end value
    end_value: u8,       // the final value at the end of the fade
    duration: Duration,  // the duration of the fade (None if instantaneous)
}

// Implement the DMX Change features
impl DmxChange {
    /// A function to return a new DmxChange by composing the elements of the
    /// fade
    ///
    fn new(start_value: u8, end_value: u8, duration: Duration) -> DmxChange {
        // Compose and return the new dmx change
        DmxChange {
            start_time: Instant::now(),
            difference: start_value as f64 - end_value as f64,
            end_value,
            duration,
        }
    }

    /// A method to calculate the current value of the fade at the current time.
    /// Returns Ongoing if the fade is still in progess and Complete if the fade
    /// is complete.
    ///
    fn current_fade(&self) -> FadeStatus {
        // Calculate the ratio of elapsed time to total fade time
        let fade_factor = 1.0
            - (self.start_time.elapsed().as_millis() as f64)
                / (self.duration.as_millis() as f64 + 0.1); // cheap fix to avoid dividing by zero

        // If the fade factor is still greater than zero
        if fade_factor > 0.0 {
            // Return the correct fade amount with an ongoing fade
            return FadeStatus::Ongoing(
                ((self.end_value as f64) + (self.difference * fade_factor)) as u8,
            );

        // If the fade factor is zero (the fade is complete)
        } else {
            // Return the final value and a complete fade
            return FadeStatus::Complete(self.end_value);
        }
    }
}

/// A struct to hold a queue of future dmx changes. This struct launches a
/// separate daemon to preserve ordering of the changes and minimize the spread
/// of unnecessary threads. This version preserves the proper order of the dmx
/// changes.
///
pub struct DmxQueue {
    stream: serial::SerialStream,            // the serial port connection
    universe: DmxUniverse,                   // the current universe of all the channels
    queue_receive: mpsc::Receiver<DmxFade>, // the queue receiving line that sends additional fade items to the daemon
    dmx_changes: FnvHashMap<u32, DmxChange>, // the dmx queue holding the coming changes, sorted by channel
    is_write_waiting: bool, // a flag to indicate that a write is still waiting to be sent
}

// Implement the Dmx Queue methods
impl DmxQueue {
    /// A function to create a new dmx queue.
    ///
    /// This function returns a new dmx queue which will send segments of a fade
    /// (at time resolution RESOLUTION) to the specified serial port.
    ///
    pub fn new(stream: serial::SerialStream, queue_receive: mpsc::Receiver<DmxFade>) -> DmxQueue {
        // Return the newly constructed dmx queue
        DmxQueue {
            stream,
            universe: DmxUniverse::new(),
            queue_receive,
            dmx_changes: FnvHashMap::default(),
            is_write_waiting: false,
        }
    }

    /// An internal function to run the queue in an infinite loop. This function
    /// should be launched as a new background thread for the queue.
    ///
    async fn run_loop(&mut self) {
        // Run the background process indefinitely
        loop {
            // Check to see if there are changes in the queue or a write waiting
            if !self.dmx_changes.is_empty() || self.is_write_waiting {
                // Update the current status for every fade
                let mut new_changes = FnvHashMap::default();
                for (channel, change) in self.dmx_changes.drain() {
                    // Check to see if the fade is complete
                    match change.current_fade() {
                        // If ongoing, re-save the change
                        FadeStatus::Ongoing(value) => {
                            self.universe.set(channel, value);
                            new_changes.insert(channel, change);
                        }

                        // If complete, drop the change
                        FadeStatus::Complete(value) => {
                            self.universe.set(channel, value);
                        }
                    }
                }

                // Replace the old changes with the new changes
                self.dmx_changes = new_changes;

                // Write the changed values
                self.write_frame().await;

                // Look for a new fade message
                tokio::select! {
                    // If a message was recieved, process the fade
                    Some(new_fade) = self.queue_receive.recv() => self.process_fade(new_fade).await,

                    // Only wait for the resolution
                    _ = sleep(Duration::from_millis(RESOLUTION)) => (), // move on
                }

            // Otherwise just wait for new message indefinitely
            } else {
                // Process a message when received
                if let Some(new_fade) = self.queue_receive.recv().await {
                    self.process_fade(new_fade).await;
                }
            }
        }
    }

    /// A helper function to process new dmx fade messages
    ///
    async fn process_fade(&mut self, dmx_fade: DmxFade) {
        // Check whether there is a fade duration specified
        match dmx_fade.duration {
            // If a fade duration was specified
            Some(duration) => {
                // Repack the fade as a dmx change
                let change = DmxChange::new(
                    self.universe.get(dmx_fade.channel),
                    dmx_fade.value,
                    duration,
                );

                // Save the new fade, replace the existing fade if necessary
                self.dmx_changes.insert(dmx_fade.channel, change);
            }

            // Otherwise
            None => {
                // Remove a fade on that channel, if it exists
                self.dmx_changes.remove(&dmx_fade.channel);

                // Make the change immediately
                self.universe.set(dmx_fade.channel, dmx_fade.value);
                self.write_frame().await;
            }
        }
    }

    /// A helper function to write the existing frame to the serial port
    ///
    async fn write_frame(&mut self) {
        // Add the message header
        let mut bytes = Vec::new();
        bytes.push(COMMAND_START);
        bytes.push(MESSAGE_LABEL);
        bytes.push(DATA_LSB);
        bytes.push(DATA_MSB);
        bytes.push(DMX_START_CODE);

        // Add the current universe to the message
        bytes.append(&mut self.universe.as_bytes());

        // Add the message ending
        bytes.push(COMMAND_END);

        // Check that the serial port is ready
        tokio::select! {
            // If the serial stream is available
            Ok(_) = self.stream.writable() => {
                // Try to send the universe to the DMX contoller
                if let Ok(sent_bytes) = self.stream.try_write(bytes.as_slice()) {
                    // If the bytes match
                    if sent_bytes == bytes.len() {
                        // Mark the write as complete
                        self.is_write_waiting = false;

                    // Otherwise, mark the write as incomplete
                    } else {
                        self.is_write_waiting = true;
                    }

                // Otherwise, mark the write as incomplete
                } else {
                    self.is_write_waiting = true;
                }
            }

            // Only wait for the resolution
            _ = sleep(Duration::from_millis(RESOLUTION)) => {
                // Mark the write as still waiting
                self.is_write_waiting = true;
            }
        }
    }
}

// Tests of the DMX Interface module
#[cfg(test)]
mod tests {
    use super::*;

    // Test the fading of a single dmx channel
    #[tokio::test]
    async fn test_light() {
        // Import standard library features
        use std::thread;
        use std::time::Duration;

        // Create a DMX Interface on USB0
        let interface = DmxInterface::new(&Path::new("/dev/ttyUSB0"))
            .expect("Unable to connect to DMX on USB0.");

        // Play a fade up on channel 1
        interface
            .play_fade(DmxFade {
                channel: 1,
                value: 255,
                duration: Some(Duration::from_secs(3)),
            })
            .await
            .unwrap();
        thread::sleep(Duration::from_secs(5));

        // Play a fade down on channel 1
        interface
            .play_fade(DmxFade {
                channel: 1,
                value: 0,
                duration: Some(Duration::from_secs(3)),
            })
            .await
            .unwrap();
        thread::sleep(Duration::from_secs(5));
    }
}
