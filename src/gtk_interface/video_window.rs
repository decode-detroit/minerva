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

//! A module to create, hold, and handle special windows for the user interface.
//! These additional dialog windows are typically launched from the system menu.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

// Import GTK and GDK libraries
use gdk::Cursor;
use gtk::prelude::*;

// Import Gstreamer Library
use self::gst_video::prelude::*;
use gstreamer_video as gst_video;

// Import FNV HashMap
use fnv::FnvHashMap;

/// A structure to contain the window for displaying video streams.
///
pub struct VideoWindow {
    overlay_map: FnvHashMap<u32, gtk::Overlay>, // the overlay widget
    channel_map: Rc<RefCell<FnvHashMap<std::string::String, gtk::Rectangle>>>, // the mapping of channel numbers to allocations
}

// Implement key features for the video window
impl VideoWindow {
    /// A function to create a new prompt string dialog structure.
    ///
    pub fn new() -> VideoWindow {
        // Create the overlay map
        let overlay_map = FnvHashMap::default();

        // Create the channel map
        let channel_map: Rc<RefCell<FnvHashMap<std::string::String, gtk::Rectangle>>> =
            Rc::new(RefCell::new(FnvHashMap::default()));

        // Return the completed Video Window
        VideoWindow {
            overlay_map,
            channel_map,
        }
    }

    /// A method to clear all video windows
    ///
    pub fn clear_all(&mut self) {
        // Destroy any open windows
        for (_, overlay) in self.overlay_map.drain() {
            if let Some(window) = overlay.parent() {
                unsafe {
                    window.destroy();
                }
            }
        }

        // Empty the channel map
        if let Ok(mut map) = self.channel_map.try_borrow_mut() {
            map.clear();
        }
    }

    /// A method to add a new video to the video window
    ///
    pub fn add_new_video(&mut self, video_stream: VideoStream) {
        // Create a new video area
        let video_area = gtk::DrawingArea::new();

        // Try to add the video area to the channel map
        match self.channel_map.try_borrow_mut() {
            // Insert the new channel
            Ok(mut map) => {
                map.insert(video_stream.channel.to_string(), video_stream.allocation);
            }

            // Fail silently
            _ => return,
        }
        video_area.set_widget_name(&video_stream.channel.to_string());

        // Extract the window number and dimensions (for use below)
        let window_number = video_stream.window_number;
        let dimensions = video_stream.dimensions;

        // Connect the realize signal for the video area
        video_area.connect_realize(move |video_area| {
            // Extract a reference for the video overlay
            let video_overlay = &video_stream.video_overlay;

            // Try to get a copy of the GDk window
            let gdk_window = match video_area.window() {
                Some(window) => window,
                None => {
                    println!("Unable to get current window for video overlay.");
                    return;
                }
            };

            // Set the window cursor to blank
            let display = gdk_window.display();
            let cursor = Cursor::for_display(&display, gdk::CursorType::BlankCursor);
            gdk_window.set_cursor(Some(&cursor));

            // Check to make sure the window is native
            if !gdk_window.ensure_native() {
                println!("Widget is not located inside a native window.");
                return;
            }

            // Extract the display type of the window
            let display_type = gdk_window.display().type_().name();

            // Switch based on the platform
            #[cfg(target_os = "linux")]
            {
                // Check if we're using X11
                if display_type == "GdkX11Display" {
                    // Connect to the get_xid function
                    extern "C" {
                        pub fn gdk_x11_window_get_xid(
                            window: *mut glib::object::GObject,
                        ) -> *mut c_void;
                    }

                    // Connect the video overlay to the correct window handle
                    #[allow(clippy::cast_ptr_alignment)]
                    unsafe {
                        let xid = gdk_x11_window_get_xid(gdk_window.as_ptr() as *mut _);
                        video_overlay.set_window_handle(xid as usize);
                    }
                } else {
                    println!("Unsupported display type: {}", display_type);
                }
            }

            // If on Mac OS
            #[cfg(target_os = "macos")]
            {
                // Check if we're using Quartz
                if display_type_name == "GdkQuartzDisplay" {
                    extern "C" {
                        pub fn gdk_quartz_window_get_nsview(
                            window: *mut glib::object::GObject,
                        ) -> *mut c_void;
                    }

                    #[allow(clippy::cast_ptr_alignment)]
                    unsafe {
                        let window = gdk_quartz_window_get_nsview(gdk_window.as_ptr() as *mut _);
                        video_overlay.set_window_handle(window as usize);
                    }
                } else {
                    println!("Unsupported display type {}", display_type);
                }
            }
        });

        // Check to see if there is already a matching window
        if let Some(overlay) = self.overlay_map.get(&window_number) {
            // Add the video area to the overlay
            overlay.add_overlay(&video_area);

            // Show the video area
            video_area.show();

        // Otherwise, create a new window
        } else {
            // Create the new window and pass dimensions if specified
            let (window, overlay) = self.new_window(dimensions);

            // Add the video area to the overlay
            overlay.add_overlay(&video_area);

            // Save the overlay in the overlay map
            self.overlay_map.insert(window_number, overlay);

            // Show the window
            window.show_all();
        }
    }

    // A helper function to create a new video window and return the window and overlay
    //
    fn new_window(&self, dimensions: Option<(i32, i32)>) -> (gtk::Window, gtk::Overlay) {
        // Create the new window
        let window = gtk::Window::new(gtk::WindowType::Toplevel);

        // Set window parameters
        window.set_decorated(false);
        window.fullscreen();

        // Create black background
        let background = gtk::DrawingArea::new();
        background.connect_draw(|_, cr| {
            // Draw the background black
            cr.set_source_rgb(0.0, 0.0, 0.0);
            cr.paint().unwrap_or(());
            Inhibit(true)
        });

        // Set the minimum window dimensions, if specified
        if let Some((height, width)) = dimensions {
            background.set_size_request(height, width);
        }

        // Create the overlay and add the background
        let overlay = gtk::Overlay::new();
        overlay.add(&background);

        // Connect the get_child_position signal
        let channel_map = self.channel_map.clone();
        overlay.connect_get_child_position(move |_, widget| {
            // Try to get the channel map
            if let Ok(map) = channel_map.try_borrow() {
                // Look up the name in the channel map
                if let Some(allocation) = map.get(&widget.widget_name().to_string()) {
                    // Return the completed allocation
                    return Some(allocation.clone());
                }
            }

            // Return None on failure
            None
        });

        // Add the overlay to the window
        window.add(&overlay);

        // Return the overlay
        (window, overlay)
    }
}
