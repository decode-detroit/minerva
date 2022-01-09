// Copyright (c) 2021 Decode Detroit
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

//! This module implements structures shared from the system connection
//! modules.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

// Import FNV HashMap
use fnv::FnvHashMap;

/// Define the instance identifier. Instances with the same identifier will trigger
/// events with one another; instances with different identifiers will not.
/// If no identifier is specified, this instance will accept all events and
/// produce events with the identifier 0.
///
#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Identifier {
    pub id: Option<u32>, // An optionally-specified identifier for this instance
}

// Implement display for identifier
impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.id {
            &Some(ref id) => write!(f, "{}", id),
            _ => write!(f, "default"),
        }
    }
}

/// An enum to specify the type of system connection.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConnectionType {
    /// A variant to connect with a ComedyComm serial port. This implementation
    /// assumes the serial connection uses the ComedyComm protocol.
    ComedySerial {
        path: PathBuf, // the location of the serial port
        baud: usize,   // the baud rate of the serial port
    },

    /// A variant to create a ZeroMQ connection. The connection type allows
    /// messages to be the sent and received. Received messages are echoed back
    /// to the send line so that all recipients will see the message.
    ZmqPrimary {
        send_path: PathBuf, // the location to bind the ZMQ sender
        recv_path: PathBuf, // the location to bind the ZMQ receiver
    },

    /// A variant to connect to an existing ZeroMQ connection over ZMQ.
    /// This connection presumes that a fully-functioning Minerva instance is
    /// is operating at the other end of the connection.
    ZmqSecondary {
        send_path: PathBuf, // the location to connect the ZMQ sender
        recv_path: PathBuf, // the location to connect the ZMQ receiver
    },

    /// A variant to connect with a DMX serial port. This connection type only allows
    /// messages to be the sent.
    DmxSerial {
        path: PathBuf,              // the location of the serial port
        all_stop_dmx: Vec<DmxFade>, // a vector of dmx fades for all stop
        dmx_map: DmxMap,            // the map of event ids to dmx fades
    },

    /// A variant to play media on the local screen. This connection type only allows
    /// messages to be sent
    Media {
        all_stop_media: Vec<MediaCue>,    // a vector of media cues for all stop
        media_map: MediaMap,              // the map of event ids to media cues
        channel_map: ChannelMap,          // the map of channel numbers to channel properties
        window_map: Option<WindowMap>,    // the map of window numbers to window properties
        apollo_params: ApolloParams,      // the parameters for Apollo media player, if specified
    },
}

/// A type to contain any number of connection types
///
pub type ConnectionSet = Vec<ConnectionType>;

/// A struct to define a single fade of a DMX channel
///
/// # Note
///
/// Assumes the channels are one-indexed (the DMX standard) rather than
/// zero-indexed.
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmxFade {
    pub channel: u32,               // the dmx channel to fade
    pub value: u8,                  // the final value at the end of the fade
    pub duration: Option<Duration>, // the duration of the fade (None if instantaneous)
}

/// A type to store a hashmap of event ids and DMX fades
///
pub type DmxMap = FnvHashMap<ItemId, DmxFade>;

/// A struct to define a single media track to play
///
/// # Note
///
/// The uri format must follow the URI syntax rules. This means local files must
/// by specified like "file:///absolute/path/to/file.mp4".
///
/// If a file is specified in the loop media field, the channel will loop this
/// media when this media completes. This takes priority over the channel loop
/// media field.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MediaCue {
    pub uri: String,                // the location of the video or audio file to play
    pub channel: u32, // the channel of the video or audio. New media sent to the same channel will replace the old media, starting instantly
    pub loop_media: Option<String>, // the location of media to loop after this media is complete
}

// A helper struct to define a single media cue.
// This version is serialized with camelCase to allow compatability with Apollo.
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaCueHelper {
    pub uri: String,
    pub channel: u32,
    pub loop_media: Option<String>,
}

// Implement conversion from media cue to media cue helper
impl MediaCue {
    pub fn into_helper(self) -> MediaCueHelper {
        // Recompose as a media cue helper
        MediaCueHelper {
            uri: self.uri,
            channel: self.channel,
            loop_media: self.loop_media,
        }
    }
}

/// A type to store a hashmap of event ids and Media Cues
///
pub type MediaMap = FnvHashMap<ItemId, MediaCue>;

/// A struct to hold the dimensions of a video Frame
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoFrame {
    pub window_number: u32, // the window number for the channel
    pub top: i32,           // the distance (in pixels) from the top of the display
    pub left: i32,          // the distance (in pixels) from the left of the display
    pub height: i32,        // the height of the video
    pub width: i32,         // the width of the video
}

// A helper struct to define the dimensions of a video window.
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFrameHelper {
    pub window_number: u32,
    pub top: i32,
    pub left: i32,
    pub height: i32,
    pub width: i32,
}

// Implement conversion features for VideoFrame
impl VideoFrame {
    pub fn into_helper(self) -> VideoFrameHelper {
        // Return the completed video window helper
        return VideoFrameHelper {
            window_number: self.window_number,
            top: self.top,
            left: self.left,
            height: self.height,
            width: self.width,
        }
    }
}

/// Am enum to specify the type of audio output device
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AudioDevice {
    /// An ALSA audio sink with a device name
    Alsa { device_name: String },

    /// A Pulse Audio sink with a device name
    Pulse { device_name: String },

    /// A Jack Audio sink with no parameters
    Jack,
}

// A helper struct to specify the type of audio output device
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioDeviceHelper {
    /// An ALSA audio sink with a device name
    Alsa { device_name: String },

    /// A Pulse Audio sink with a device name
    Pulse { device_name: String },

    /// A Jack Audio sink with no parameters
    Jack,
}

// Implement conversion features for VideoWindow
impl AudioDevice {
    pub fn into_helper(self) -> AudioDeviceHelper {
        // Return the completed audio device helper
        match self {
            AudioDevice::Alsa { device_name } => AudioDeviceHelper::Alsa { device_name },
            AudioDevice::Pulse { device_name } => AudioDeviceHelper::Pulse { device_name },
            AudioDevice::Jack => AudioDeviceHelper::Jack,
        }
    }
}

/// A struct to define a single channel to display a media track
///
/// # Note
///
/// If media is specified in the loop media field, the channel will loop this
/// media when the first media completes and anytime no other media has been
/// directed to play on the channel. If no loop media is specified, the channel
/// will hold on the last frame of the most recent media.
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaChannel {
    pub video_frame: Option<VideoFrame>, // the video frame. Defaults to a new window generated by gstreamer
    pub window_dimensions: Option<(i32, i32)>, // the minimum size of the window (defaults to fullscreen, but can be made larger to stretch across multiple screens)
    pub audio_device: Option<AudioDevice>, // the audio device. Defaults to the system default
    pub loop_media: Option<String>, // the media (video or audio) to loop when no other media is playing
}

// A helper struct to define a single channel to display a media track.
// This version includes the channel number in the struck to allow easier
// passing to apollo.
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaChannelHelper {
    pub channel: u32,
    pub video_frame: Option<VideoFrameHelper>,
    pub window_dimensions: Option<(i32, i32)>,
    pub audio_device: Option<AudioDeviceHelper>,
    pub loop_media: Option<String>,
}

// Implement conversion features for Media Channel
impl MediaChannel {
    // Add the channel number to an existing media channel
    pub fn add_channel(self, channel: u32) -> MediaChannelHelper {
        // Convert the video window, if specified
        let video_frame = match self.video_frame {
            Some(frame) => Some(frame.into_helper()),
            None => None,
        };

        // Convert the audio device, if specified
        let audio_device = match self.audio_device {
            Some(device) => Some(device.into_helper()),
            None => None,
        };

        // Return the completed media channel helper
        return MediaChannelHelper {
            channel,
            video_frame,
            window_dimensions: self.window_dimensions,
            audio_device,
            loop_media: self.loop_media,
        }
    }
}

/// A type to store a hashmap of channel ids and allocations
///
pub type ChannelMap = FnvHashMap<u32, MediaChannel>;

/// A struct to define an application window to hold one or more media channels
///
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowDefinition {
    pub window_number: u32,             // the channel where the video should be played
    pub fullscreen: bool,               // a flag to indicate whether the window should be fullscreen
    pub dimensions: Option<(i32, i32)>, // the minimum dimensions of the window
}

// A helper struct to define an application window to hold one or more media channels
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowDefinitionHelper {
    pub window_number: u32,
    pub fullscreen: bool,
    pub dimensions: Option<(i32, i32)>,
}

// Implement conversion features for WindowDefinition
impl WindowDefinition {
    pub fn into_helper(self) -> WindowDefinitionHelper {
        // Return the completed window definition helper
        WindowDefinitionHelper {
            window_number: self.window_number,
            fullscreen: self.fullscreen,
            dimensions: self.dimensions,
        }
    }
}

/// A type to store a hashmap of window ids and parameters
///
pub type WindowMap = FnvHashMap<u32, WindowDefinition>;

/// A struct to hold parameters for Apollo media player
/// 
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApolloParams {
    pub spawn: bool,                // a flag if Minerva should spawn and manage the Apollo process
    pub address: Option<String>,    // the address where Apollo will listen for instructions, defaults to Apollo on localhost
}
 