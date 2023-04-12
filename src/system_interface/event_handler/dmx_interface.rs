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
//!
//! This module is currently limited to Enttec DMX USB Pro-compatible hardware.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

// Import Tokio features
use tokio::task;

// Import the serial features
use serial;
use serial::prelude::*;

// Import FNV HashMap
use fnv::FnvHashMap;

// Import the failure features
use failure::Error;

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
    pub fn new(path: &PathBuf) -> Result<Self, Error> {
        // Connect to the underlying serial port
        let mut port = serial::open(path)?;

        // Try to configure the serial port
        let settings = serial::PortSettings {
            baud_rate: serial::BaudRate::from_speed(9600),
            char_size: serial::Bits8,
            parity: serial::ParityNone,
            stop_bits: serial::Stop1,
            flow_control: serial::FlowNone,
        };
        port.configure(&settings)?;

        // Adjust the timeout for the serial port
        port.set_timeout(Duration::from_millis(100))?;

        // Create a new DMX queue
        let (load_fade, receive_fade) = mpsc::channel();
        let mut dmx_queue = DmxQueue::new(port, receive_fade);

        // Start the dmx queue thread
        task::spawn_blocking(move || {
            dmx_queue.run_loop();
        });

        // Return the new DmxOut instance
        Ok(Self { load_fade })
    }

    /// A method to play a new Dmx fade
    ///
    pub fn play_fade(&self, fade: DmxFade) -> Result<(), Error> {
        // Verify the range of the selected channel
        if (fade.channel > DMX_MAX) | (fade.channel < 1) {
            return Err(format_err!("Selected DMX channel is out of range."));
        }

        // Send the fade to the background thread
        if let Err(_) = self.load_fade.send(fade.clone()) {
            return Err(format_err!("Background DMX fading control has crashed."));
        }

        // If the fade was processed correctly, indicate success
        Ok(())
    }

    /// A method to load existing values for all the Dmx channels
    ///
    pub fn restore_universe(&self, universe: DmxUniverse) {
        // For each channel, send a fade with no duration
        for channel in 1..DMX_MAX {
            self.load_fade.send(DmxFade { channel, value: universe.get(channel), duration: None }).unwrap_or(()); // fail silently
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
/// TODO: Rewrite this implementation to use a tokio thread
///
pub struct DmxQueue {
    port: serial::SystemPort,                // the serial port of the connection
    universe: DmxUniverse,                   // the current universe of all the channels
    queue_receive: mpsc::Receiver<DmxFade>,  // the queue receiving line that sends additional fade items to the daemon
    dmx_changes: FnvHashMap<u32, DmxChange>, // the dmx queue holding the coming changes, sorted by channel
}

// Implement the Dmx Queue methods
impl DmxQueue {
    /// A function to create a new dmx queue.
    ///
    /// This function returns a new dmx queue which will send segments of a fade
    /// (at time resolution RESOLUTION) to the specified port. This
    /// implementation of the queue launches a background thread to manage
    /// updates.
    ///
    pub fn new(port: serial::SystemPort, queue_receive: mpsc::Receiver<DmxFade>) -> DmxQueue {
        // Return the newly constructed dmx queue
        DmxQueue {
            port,
            universe: DmxUniverse::new(),
            queue_receive,
            dmx_changes: FnvHashMap::default(),
        }
    }

    /// An internal function to run the queue in an infinite loop. This function
    /// should be launched as a new background thread for the queue.
    ///
    fn run_loop(&mut self) {
        // Run the background process indefinitely
        loop {
            // Check to see if there are changes in the queue
            if !self.dmx_changes.is_empty() {
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

                // Write the changed state(s)
                self.write_frame();

                // Replace the old changes with the new changes
                self.dmx_changes = new_changes;

                // Wait a short period for a new fade message
                match self
                    .queue_receive
                    .recv_timeout(Duration::from_millis(RESOLUTION))
                {
                    // Process a message if received
                    Ok(new_fade) => self.process_fade(new_fade),

                    // Ignore timeout messages
                    Err(mpsc::RecvTimeoutError::Timeout) => (),

                    // Quit the thread on any other error
                    _ => break,
                }

            // Otherwise just wait for new message indefinitely
            } else {
                // Process a message if received
                match self.queue_receive.recv() {
                    // Add the new fade to the queue
                    Ok(new_fade) => self.process_fade(new_fade),

                    // Quit the thread on any error
                    _ => break,
                }
            }
        }
    }

    /// A helper function to process new dmx fade messages
    ///
    fn process_fade(&mut self, dmx_fade: DmxFade) {
        // Check whether there is a fade specified
        match dmx_fade.duration {
            // If a fade was specified
            Some(duration) => {
                // Repack the fade as a dmx change
                let change =
                    DmxChange::new(self.universe.get(dmx_fade.channel), dmx_fade.value, duration);

                // Save the new fade, replace the existing fade if necessary
                self.dmx_changes.insert(dmx_fade.channel, change);
            }

            // Otherwise
            None => {
                // Remove a fade on that channel, if it exists
                self.dmx_changes.remove(&dmx_fade.channel);

                // Make the change immediately
                self.universe.set(dmx_fade.channel, dmx_fade.value);
                self.write_frame();
            }
        }
    }

    /// A helper function to write the existing frame to the serial port
    ///
    fn write_frame(&mut self) {
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

        // Send the bytes to the board
        self.port.write(bytes.as_slice()).unwrap_or(0); // silently ignore errors
    }
}

// Tests of the DMXOut module
#[cfg(test)]
mod tests {
    use super::*;

    // Test the fading of a single dmx channel
    #[test]
    fn test_light() {
        // Import standard library features
        use std::thread;
        use std::time::Duration;

        // Create a DMX Interface on USB0
        let interface = DmxInterface::new(&PathBuf::from("/dev/ttyUSB0")).expect("Unable to connect to DMX on USB0.");

        // Play a fade up on channel 1
        interface.play_fade(DmxFade { channel: 1, value: 255, duration: Some(Duration::from_secs(3)) }).unwrap();
        thread::sleep(Duration::from_secs(5));

        // Play a fade down on channel 1
        interface.play_fade(DmxFade { channel: 1, value: 0, duration: Some(Duration::from_secs(3)) }).unwrap();
        thread::sleep(Duration::from_secs(5));
    }
}
