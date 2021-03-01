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

//! This module implements structures shared from the system connections
//! modules.

// Import standard library features
use std::fmt;

// Import Gstreamer Library
#[cfg(feature = "media-out")]
use gstreamer_video as gst_video;

/// Define the instance identifier. Instances with the same identifier will trigger
/// events with one another; instances with different identifiers will not.
/// If no identifier is specified, this instance will accept all events and
/// produce events with the identifier 0.
///
#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Identifier {
  pub id: Option<u32>,  // An optionally-specified identifier for this instance
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

/// A type to communicate a video stream to the front end of the program
#[cfg(feature = "media-out")]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VideoStream {
    pub channel: u32,                       // the channel where the video should be played
    pub window_number: u32,                 // the window where the video should be played
    pub allocation: gtk::Rectangle,         // the location of the video in the screen
    pub video_overlay: gst_video::VideoOverlay, // the video overlay which should be connected to the video id
}
