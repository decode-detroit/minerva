// Copyright (c) 2017 Decode Detroit
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

//! A module to create the user interface to interact with the underlying
//! system interface. This module links directly to the system interface and
//! sends any updates to the application window. FIXME Update Definition

// Define public submodules
#[macro_use]
pub mod utils;

// Define private submodules
mod video_window;

// Import crate definitions
use crate::definitions::*;

// Import other definitions
use self::video_window::VideoWindow;

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

// Import GTK and GDK libraries
use glib;
use gtk;
use gtk::prelude::*;

// Define user interface constants
const REFRESH_RATE: u64 = 100; // the display refresh rate in milliseconds

/// A structure to contain the user interface and handle all updates to the
/// to the interface.
///
#[derive(Clone)]
pub struct GtkInterface {
    video_window: Rc<RefCell<VideoWindow>>, // the video window, wrapped in a refcell and rc for multi-referencing
}

// Implement key GtkInterface functionality
impl GtkInterface {
    /// A function to create a new, blank instance of the user interface. The
    /// window provided to the function should be the top-level window for the
    /// program.
    ///
    pub fn new(
        interface_receive: mpsc::Receiver<InterfaceUpdate>,
    ) -> Self {
        // Create the video window
        let video_window = VideoWindow::new();

        // Wrap the video window in an rc and refcell
        let video_window = Rc::new(RefCell::new(video_window));

        // Create the GtkInterface
        let gtk_interface = GtkInterface { video_window };

        // Launch the interface monitoring interrupt, currently set to ten times a second FIXME make this async
        let update_interface = clone!(gtk_interface => move || {
            gtk_interface.check_updates(&interface_receive);
            Continue(true) // continue looking for updates indefinitely
        });
        glib::timeout_add_local(Duration::from_millis(REFRESH_RATE), update_interface); // triggers once every 100ms

        // Return the new GtkInterface
        gtk_interface
    }

    /// A method to listen for modifications to the user interface.
    ///
    /// This method listens on the provided interface_update line for any changes
    /// to the interface. The method then processes any/all of these updates
    /// in the order that they were received.
    ///
    pub fn check_updates(&self, interface_update: &mpsc::Receiver<InterfaceUpdate>) {
        // Look for any updates and act upon them
        loop {
            // Check to see if there are any more updatess
            let update = match interface_update.try_recv() {
                Ok(update) => update,
                _ => return, // exit when there are no updates left
            };

            // Unpack the updates of every type
            match update {

                // Launch the video window or load the new stream
                InterfaceUpdate::Video { video_stream } => {
                    // Attempt to get a mutable copy of the video_window
                    let mut video_window = match self.video_window.try_borrow_mut() {
                        Ok(window) => window,

                        // If unable, exit immediately
                        Err(_) => return,
                    };

                    // Switch based on if a video stream was provided
                    if let Some(stream) = video_stream {
                        video_window.add_new_video(stream);

                    // Otherwise, destroy the video window
                    } else {
                        video_window.clear_all();
                    }
                }

                // Ignore all other types of updates
                _ => (),
            }
        }
    }
}
