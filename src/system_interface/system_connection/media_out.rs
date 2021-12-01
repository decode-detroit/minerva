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
#[cfg(feature = "media-out")]
use crate::definitions::VideoStream;

// Import standard library features
#[cfg(feature = "media-out")]
use std::sync::{Arc, Mutex};

// Import GTK Library
#[cfg(feature = "media-out")]
use glib;
#[cfg(feature = "media-out")]
use gtk;
#[cfg(feature = "media-out")]
use gtk::prelude::*;

// Import Gstreamer Library
#[cfg(feature = "media-out")]
use gst_video::prelude::*;
#[cfg(feature = "media-out")]
use gstreamer as gst;
#[cfg(feature = "media-out")]
use gstreamer_video as gst_video;

// Import FNV HashMap
#[cfg(feature = "media-out")]
use fnv::FnvHashMap;

// Import the failure features
use failure::Error;

/// A helper type to store the playbin and loop media uri
#[cfg(feature = "media-out")]
struct InternalChannel {
    playbin: gst::Element,                  // the playbin for this channel
    loop_media: Arc<Mutex<Option<String>>>, // the loop media handle for this channel
}

/// A structure to hold and manipulate the connection to the media backend
///
pub struct MediaOut {
    #[cfg(feature = "media-out")]
    internal_send: InternalSend, // the general send line to pass video streams to the user interface
    #[cfg(feature = "media-out")]
    channels: FnvHashMap<u32, InternalChannel>, // the map of channels numbers to internal channels
    #[cfg(feature = "media-out")]
    all_stop_media: Vec<MediaCue>, // a vector of media cues for all stop
    media_map: MediaMap, // the map of event ids to media cues
    #[cfg(feature = "media-out")]
    channel_map: ChannelMap, // the map of channel numbers to channel information
}

// Implement key functionality for the Media Out structure
impl MediaOut {
    /// A function to create a new instance of the MediaOut, active version
    ///
    #[cfg(feature = "media-out")]
    pub fn new(
        internal_send: &InternalSend,
        all_stop_media: Vec<MediaCue>,
        media_map: MediaMap,
        channel_map: ChannelMap,
    ) -> Result<MediaOut, Error> {
        // Try to initialize GStreamer
        gst::init()?;

        // Return the complete module
        Ok(MediaOut {
            internal_send: internal_send.clone(),
            channels: FnvHashMap::default(),
            all_stop_media,
            media_map,
            channel_map,
        })
    }

    /// A function to create a new instance of the MediaOut, inactive version
    ///
    #[cfg(not(feature = "media-out"))]
    pub fn new(
        _internal_send: &InternalSend,
        _all_stop_media: Vec<MediaCue>,
        media_map: MediaMap,
        _channel_map: ChannelMap,
    ) -> Result<MediaOut, Error> {
        // Return a partial module
        Ok(MediaOut { media_map })
    }

    // A helper function to correctly add a new media cue
    #[cfg(feature = "media-out")]
    fn add_cue(
        internal_send: &InternalSend,
        channels: &mut FnvHashMap<u32, InternalChannel>,
        channel_map: &ChannelMap,
        media_cue: MediaCue,
    ) -> Result<(), Error> {
        // Check to see if there is an existing channel
        if let Some(channel) = channels.get(&media_cue.channel) {
            // Add the uri to this channel
            channel.playbin.set_property("uri", &media_cue.uri)?;

            // Stop the previous media
            channel.playbin.set_state(gst::State::Null)?;

            // Make sure the new media is playing
            channel.playbin.set_state(gst::State::Playing)?;

            // Try to get a lock on the loop media
            if let Ok(mut media) = channel.loop_media.lock() {
                // Try to get a copy of the channel's loop media
                let channel_loop = match channel_map.get(&media_cue.channel) {
                    Some(media_channel) => media_channel.loop_media.clone(),
                    None => None,
                };

                // Replace the media with the local loop or channel loop
                *media = media_cue.loop_media.or(channel_loop);

            // Otherwise, throw an error
            } else {
                return Err(format_err!("Unable to Change Loop Media."));
            }

        // Otherwise, create a new channel
        } else {
            // Try to get the channel information
            let (possible_window, possible_device, possible_loop) =
                match channel_map.get(&media_cue.channel) {
                    Some(media_channel) => (
                        media_channel.video_window.clone(),
                        media_channel.audio_device.clone(),
                        media_channel.loop_media.clone(),
                    ),

                    // If the channel information isn't available, throw an error
                    _ => {
                        return Err(format_err!(
                            "Media channel {} not specified.",
                            media_cue.channel
                        ))
                    }
                };

            // Create a new playbin
            let playbin = gst::ElementFactory::make("playbin", None)?;

            // Match based on the audio device specified
            match possible_device {
                // An ALSA device
                Some(AudioDevice::Alsa { device_name }) => {
                    // Create and set the audio sink
                    let audio_sink = gst::ElementFactory::make("alsasink", None)?;
                    audio_sink.set_property("device", &device_name)?;
                    playbin.set_property("audio-sink", &audio_sink)?;
                }

                // A Pulse Audio device
                Some(AudioDevice::Pulse { device_name }) => {
                    // Create and set the audio sink
                    let audio_sink = gst::ElementFactory::make("pulsesink", None)?;
                    audio_sink.set_property("device", &device_name)?;
                    playbin.set_property("audio-sink", &audio_sink)?;
                }

                // Ignore all others
                _ => (),
            }

            // Set the uri for the playbin
            playbin.set_property("uri", &media_cue.uri)?;

            // If a video window was specified
            if let Some(video_window) = possible_window {
                // Compose the allocation
                let allocation = gtk::Rectangle {
                    x: video_window.left,
                    y: video_window.top,
                    width: video_window.width,
                    height: video_window.height,
                };

                // Try to create the video overlay
                let video_overlay = match playbin.clone().dynamic_cast::<gst_video::VideoOverlay>()
                {
                    Ok(overlay) => overlay,
                    _ => return Err(format_err!("Unable to create video stream.")),
                };

                // Send the new video stream to the user interface
                let video_stream = VideoStream {
                    window_number: video_window.window_number,
                    channel: media_cue.channel,
                    allocation,
                    video_overlay,
                };
                internal_send.send_new_video(video_stream);
            } // Otherwise, any window creation (if needed) is left to gstreamer

            // Create the loop media mutex and resolve the current loop
            let loop_media = Arc::new(Mutex::new(media_cue.loop_media.or(possible_loop)));

            // Create the loop media callback
            MediaOut::create_loop_callback(&playbin, loop_media.clone())?;

            // Start playing the media
            playbin.set_state(gst::State::Playing)?;

            // Add the playbin to the channels
            channels.insert(
                media_cue.channel,
                InternalChannel {
                    playbin,
                    loop_media,
                },
            );
        }

        // Indicate success
        Ok(())
    }

    // A helper function to create a signal watch to handle looping media
    #[cfg(feature = "media-out")]
    fn create_loop_callback(
        playbin: &gst::Element,
        loop_media: Arc<Mutex<Option<String>>>,
    ) -> Result<(), Error> {
        // Try to access the playbin bus
        let bus = match playbin.bus() {
            Some(bus) => bus,
            None => return Err(format_err!("Unable to set loop media: Invalid bus.")),
        };

        // Create a week reference to the playbin
        let channel_weak = playbin.downgrade();

        // Connect the signal handler for the end of stream notification
        if let Err(_) = bus.add_watch(move |_, msg| {
            // If the end of stream message is received
            if let gst::MessageView::Eos(..) = msg.view() {
                // Wait for access to the current loop media
                if let Ok(possible_media) = loop_media.lock() {
                    // If the media was specified
                    if let Some(media) = possible_media.clone() {
                        // Try to get a strong reference to the channel
                        let channel = match channel_weak.upgrade() {
                            Some(channel) => channel,
                            None => return glib::Continue(true), // Fail silently, but try again
                        };
                        
                        // If media was specified, add the loop uri to this channel
                        channel.set_property("uri", &media).unwrap_or(());

                        // Try to stop any playing media
                        channel
                            .set_state(gst::State::Null)
                            .unwrap_or(gst::StateChangeSuccess::Success);

                        // Try to start playing the media
                        channel
                            .set_state(gst::State::Playing)
                            .unwrap_or(gst::StateChangeSuccess::Success);
                    }
                }
            }

            // Continue with other signal handlers
            glib::Continue(true)

            // Warn the user of failure
        }) {
            return Err(format_err!("Unable to set loop media: Duplicate watch."));
        }

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
        // Check to see if the event is all stop
        if id == ItemId::all_stop() {
            // Stop all the currently playing media FIXME May crash on set_state, so currently disabled
            /*for (_, channel) in self.channels.iter() {
                channel
                    .playbin
                    .set_state(gst::State::Null)
                    .unwrap_or(gst::StateChangeSuccess::Success);
            }*/

            // Run all of the all stop media, ignoring errors
            for media_cue in self.all_stop_media.iter() {
                // Add the audio cue
                MediaOut::add_cue(
                    &self.internal_send,
                    &mut self.channels,
                    &self.channel_map,
                    media_cue.clone(),
                )
                .unwrap_or(());
            }

        // Check to see if the event is in the media map
        } else {
            if let Some(media_cue) = self.media_map.get(&id) {
                MediaOut::add_cue(
                    &self.internal_send,
                    &mut self.channels,
                    &self.channel_map,
                    media_cue.clone(),
                )?;
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

// Implement the drop trait for MediaOut
#[cfg(feature = "media-out")]
impl Drop for MediaOut {
    /// This method sets any active playbins to NULL
    ///
    fn drop(&mut self) {
        // Destroy the video window
        self.internal_send.send_clear_videos();

        // For every playbin in the active channels
        for (_, channel) in self.channels.drain() {
            // Try to remove the bus signal watch
            if let Some(bus) = channel.playbin.bus() {
                bus.remove_watch().unwrap_or(());
            }
        }
    }
}
