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

//! A module to load and play video files on this device

// Import the relevant structures into the correct namespace
use super::{EventConnection, ItemId, ReadResult};
use super::GeneralUpdate;

// Import GTK Library
#[cfg(feature = "video")]
extern crate glib;
#[cfg(feature = "video")]
extern crate gdk;
#[cfg(feature = "video")]
extern crate gtk;
#[cfg(feature = "video")]
use self::gtk::prelude::*;

// Import Gstreamer Library
#[cfg(feature = "video")]
extern crate gstreamer as gst;
#[cfg(feature = "video")]
extern crate gstreamer_video as gst_video;
#[cfg(feature = "video")]
use self::gst_video::prelude::*;

// Import FNV HashMap
extern crate fnv;
use self::fnv::FnvHashMap;

// Import the failure features
use failure::Error;

/// A struct to define a single video track to play
///
/// # Note
///
/// The uri format must follow the URI syntax rules. This means local files must
/// by specified like "file:///absolute/path/to/file.mp4".
///
/// If a video is specified in the loop video field, the channel will loop this
/// video when this video completes. This takes priority over the channel loop
/// video field.
///
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoCue {
    uri: String,            // the location of the video file to play
    channel: u32,           // the channel of the video. A new video sent to the same channel will replace the old video, starting instantly FIXME
    loop_video: Option<String>, // the location of a video to loop after this video is complete
}

/// A type to store a hashmap of event ids and Video Cues
///
pub type VideoMap = FnvHashMap<ItemId, VideoCue>;

/// A struct to define a single channel to display a video track
///
/// # Note
///
/// If a video is specified in the loop video field, the channel will loop this
/// video when the first video completes and anytime no other video has been
/// directed to play on the channel. If no loop video is specified, the channel
/// will hold on the last frame of the most recent video.
///
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoChannel {
    window_number: u32,         // the window number for the channel
    top: i32,                   // the distance (in pixels) from the top of the display
    left: i32,                  // the distance (in pixels) from the left of the display
    height: i32,                // the height of the video
    width: i32,                 // the width of the video
    loop_video: Option<String>, // the location of a video to loop in the background of this stream
}

/// A type to stote a hashmap of channel ids and allocations
///
pub type ChannelMap = FnvHashMap<u32, VideoChannel>;

/// A type to communicate a video stream to the front end of the program
#[cfg(feature = "video")]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VideoStream {
    pub channel: u32,                       // the channel where the video should be played
    pub window_number: u32,                 // the window where the video should be played
    pub allocation: gtk::Rectangle,         // the location of the video in the screen
    pub video_overlay: gst_video::VideoOverlay, // the video overlay which should be connected to the video id
}

/// A structure to hold and manipulate the connection to the video backend
///
pub struct VideoOut {
    #[cfg(feature = "video")]
    general_update: GeneralUpdate,          // the general send line to pass video streams to the user interface
    #[cfg(feature = "video")]
    channels: FnvHashMap<u32, gst::Element>,// the map of channels to active playbins    
    #[cfg(feature = "video")]
    all_stop_video: Vec<VideoCue>,          // a vector of video cues for all stop
    video_map: VideoMap,                    // the map of event ids to video cues
    #[cfg(feature = "video")]
    channel_map: ChannelMap,                // the map of channel numbers to allocations
}

// Implement key functionality for the Video Out structure
impl VideoOut {
    /// A function to create a new instance of the VideoOut, active version
    ///
    #[cfg(feature = "video")]
    pub fn new(
        general_update: &GeneralUpdate,
        all_stop_video: Vec<VideoCue>,
        video_map: VideoMap,
        channel_map: ChannelMap
    ) -> Result<VideoOut, Error> {
        // Try to initialize GStreamer
        gst::init()?;
    
        // Return the complete module
        Ok(VideoOut {
            general_update: general_update.clone(),
            channels: FnvHashMap::default(),
            all_stop_video,
            video_map,
            channel_map,
        })
    }

    /// A function to create a new instance of the videoOut, inactive version
    ///
    #[cfg(not(feature = "video"))]
    pub fn new(
        _general_update: &GeneralUpdate,
        _all_stop_video: Vec<VideoCue>,
        video_map: VideoMap,
        _channel_map: ChannelMap
    ) -> Result<VideoOut, Error> {
        // Return a partial module
        Ok(VideoOut {
            video_map,
        })
    }
    
    // A helper function to correctly add a new video cue
    #[cfg(feature = "video")]
    fn add_cue(
        general_update: &GeneralUpdate,
        channels: &mut FnvHashMap<u32, gst::Element>,
        channel_map: &ChannelMap,
        video_cue: VideoCue
    ) -> Result<(), Error> {
        // Check to see if there is an existing channel
        if let Some(channel) = channels.get(&video_cue.channel) {
            // Stop the previous video
            channel.set_state(gst::State::Null)?;
            
            // Add the uri to this channel
            channel.set_property("uri", &video_cue.uri)?;
            
            // If a loop was specified, replace the loop video
            if let Some(loop_video) = video_cue.loop_video {
                VideoOut::add_loop_video(&channel, loop_video)?;
            
            // Otherwise, try to use the channel loop video instead
            } else {
                // Try to get the channel information
                if let Some(video_channel) = channel_map.get(&video_cue.channel) {
                    // If a loop was specified, replace the loop video
                    if let Some(loop_video) = video_channel.loop_video.clone() {
                        VideoOut::add_loop_video(&channel, loop_video)?;
                    }
                }
            }
            
            // Make sure it is playing
            channel.set_state(gst::State::Playing)?;
        
        // Otherwise, create a new channel
        } else {
            // Try to get the channel information
            let (window_number, allocation, possible_loop) = match channel_map.get(&video_cue.channel) {
                Some(video_channel) => {
                    (video_channel.window_number,
                    gtk::Rectangle {
                        x: video_channel.top,
                        y: video_channel.left,
                        width: video_channel.width,
                        height: video_channel.height,
                    },
                    video_channel.loop_video.clone())
                }
                
                // If the channel information isn't available, throw an error
                _ => return Err(format_err!("Video channel {} not specified.", video_cue.channel)),
            };
            
            // Create a new playbin
            let playbin = gst::ElementFactory::make("playbin", None)?;
            
            // Set the uri for the playbin
            playbin.set_property("uri", &video_cue.uri)?;
            
            // Try to create the video overlay
            let video_overlay = match playbin.clone().dynamic_cast::<gst_video::VideoOverlay>() {
                Ok(overlay) => overlay,
                _ => return Err(format_err!("Unable to create video stream.")),
            };
            
            // Send the new video stream to the user interface
            general_update.send_new_video(
                VideoStream {
                    window_number,
                    channel: video_cue.channel,
                    allocation,
                    video_overlay,
                }
            );
            
            // If a loop was specified, create the loop video
            if let Some(loop_video) = video_cue.loop_video {
                VideoOut::add_loop_video(&playbin, loop_video)?;
            
            // Otherwise, try to use the channel loop video instead
            } else {
                // If a loop was specified, create the loop video
                if let Some(loop_video) = possible_loop {
                    VideoOut::add_loop_video(&playbin, loop_video)?;
                }
            }
                        
            // Start playing the video
            playbin.set_state(gst::State::Playing)?;
            
            // Add the playbin to the channels
            channels.insert(video_cue.channel, playbin);
        }
        
        // Indicate success
        Ok(())
    }
    
    // A helper function to add a singal watch to handle looping videos
    #[cfg(feature = "video")]
    fn add_loop_video(playbin: &gst::Element, loop_video: String) -> Result<(), Error> {
        
        // Try to create the playbin bus
        let bus = match playbin.get_bus() {
            Some(bus) => bus,
            None => return Err(format_err!("Unable to initialize the loop video.")),
        };
        
        // Try to remove the old watch, if it exists
        bus.remove_watch().unwrap_or(());
        
        // Create a week referene to the playbin
        let channel_weak = playbin.downgrade();
        
        // Connect the signal handler for the end of stream notification
        bus.add_watch(move |_, msg| {
            // Try to get a strong reference to the channel
            let channel = match channel_weak.upgrade() {
                Some(channel) => channel,
                None => return glib::Continue(true), // Fail silently
            };

            // Match the message type
            match msg.view() {
                // If the end of stream message is received
                gst::MessageView::Eos(..) => {
                    // Stop the previous video
                    channel.set_state(gst::State::Null).unwrap_or(gst::StateChangeSuccess::Success);
                    
                    // Add the loop uri to this channel
                    channel.set_property("uri", &loop_video).unwrap_or(());
                                
                    // Try to start playing the video
                    channel.set_state(gst::State::Playing).unwrap_or(gst::StateChangeSuccess::Success);
                }
                
                // Ignore other messages
                _ => (),
            }
            
            // Continue with other signal handlers
            glib::Continue(true)
        })?;
        
        // Indicate success
        Ok(())
    }
}

// Implement the event connection trait for Video Out
impl EventConnection for VideoOut {
    /// A method to receive a new event, empty for this connection type
    ///
    fn read_events(&mut self) -> Vec<ReadResult> {
        Vec::new() // return an empty vector
    }

    /// A method to send a new event to the video connection, active version
    ///
    #[cfg(feature = "video")]
    fn write_event(&mut self, id: ItemId, _data1: u32, _data2: u32) -> Result<(), Error> {   
        // Check to see if the event is all stop
        if id == ItemId::all_stop() {
            // Stop all the currently playing videos
            for (_, channel) in self.channels.iter() {
                channel.set_state(gst::State::Null).unwrap_or(gst::StateChangeSuccess::Success);
            }
            
            // Run all of the all stop video, ignoring errors
            for video_cue in self.all_stop_video.iter() {
                // Add the audio cue
                VideoOut::add_cue(&self.general_update, &mut self.channels, &self.channel_map, video_cue.clone()).unwrap_or(());
            }

        // Check to see if the event is in the video map
        } else {
            if let Some(video_cue) = self.video_map.get(&id) {
                VideoOut::add_cue(&self.general_update, &mut self.channels, &self.channel_map, video_cue.clone())?;
            }
        }

        // If the event wasn't found or was processed correctly, indicate success
        Ok(())
    }
    
    /// A method to send a new event to the video connection, inactive version
    ///
    #[cfg(not(feature = "video"))]
    fn write_event(&mut self, id: ItemId, _data1: u32, _data2: u32) -> Result<(), Error> {
        // Check to see if the event is in the map
        if let Some(_) = self.video_map.get(&id) {
            return Err(format_err!("Program compiled without video support. See documentation."));
        } else {
            return Ok(());
        }
    }

    /// A method to echo an event to the video connection
    fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        self.write_event(id, data1, data2)
    }
}

// Implement the drop trait for VideoOut
#[cfg(feature = "video")]
impl Drop for VideoOut {
    /// This method sets any active playbins to NULL
    ///
    fn drop(&mut self) {
        // For every playbin in the active channels
        for (_, playbin) in self.channels.drain() {
            // Set the playbin state to Null
            playbin.set_state(gst::State::Null).unwrap_or(gst::StateChangeSuccess::Success);
            
            // Try to remove the bus signal watch
            if let Some(bus) = playbin.get_bus() {
                bus.remove_watch().unwrap_or(());
            }   
        }
    }
}   

// Tests of the VideoOut module
#[cfg(test)]
mod tests {
    use super::*;

    // Import the library items for the testing function
    use std::thread;
    use std::time::{Duration, Instant};

    // Test the function by
    fn main() {
        unimplemented!();
    }
}
