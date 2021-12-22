// Copyright (c) 2019 Decode Detroit
// Author: Patton Doyle
// Based on examples from gtk-rs (MIT License)
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

//! The main module of the minerva program which pulls from the other modules.

// Allow deeper recursion testing for web server
#![recursion_limit="256"]

// Import YAML processing libraries
#[macro_use]
extern crate serde;

// Define program modules
#[macro_use]
mod definitions;
mod item_index;
mod style_sheet;
mod system_interface;
#[cfg(feature = "media-out")]
#[macro_use]
mod gtk_interface;
mod web_interface;

// Import crate definitions
use crate::definitions::*;

// Import other structures into this module
#[cfg(feature = "media-out")]
use self::gtk_interface::GtkInterface;
use self::item_index::ItemIndex;
use self::style_sheet::StyleSheet;
use self::system_interface::SystemInterface;
use self::web_interface::WebInterface;

// Import standard library features
use std::thread;

// Import failure features
#[macro_use]
extern crate failure;

// Import tracing features
use tracing_subscriber;

// Import GTK and GIO libraries
use gio::prelude::*;
use gtk::prelude::*;

// Import tokio features
use tokio::runtime::Runtime;

// Define program constants
const LOGO_SQUARE: &str = "logo_square.png";
const WINDOW_TITLE: &str = "Minerva";
const USER_STYLE_SHEET: &str = "/tmp/userStyles.css";

/// The Minerva structure to contain the program launching and overall
/// communication code.
///
pub struct Minerva {}

// Implement the Minerva functionality
impl Minerva {
    /// A function to build the main program and the user interface
    ///
    pub fn build_program(application: &gtk::Application) {
        // Create the tokio runtime
        let runtime = Runtime::new().expect("Unable To Create Tokio Runtime.");

        // Create the item index to process item description requests
        let (mut item_index, index_access) = ItemIndex::new();

        // Run the item index in a new thread (needed here to allow the system interface to load)
        runtime.spawn(async move {
            item_index.run().await;
        });

        // Create the style sheet to process style requests
        let (mut style_sheet, style_access) = StyleSheet::new();

        // Run the style sheet in a new thread (needed here to allow the system interface to load)
        runtime.spawn(async move {
            style_sheet.run().await;
        });

        // Create the interface send
        #[cfg(feature = "media-out")]
        let (interface_send, gtk_interface_recv, web_interface_recv) = InterfaceSend::new();
        #[cfg(not(feature = "media-out"))]
        let (interface_send, web_interface_recv) = InterfaceSend::new();

        // Launch the system interface to monitor and handle events
        let (system_interface, web_send) = runtime
            .block_on(async {
                SystemInterface::new(index_access.clone(), style_access.clone(), interface_send.clone()).await
            })
            .expect("Unable To Create System Interface.");

        // Create a new web interface
        let mut web_interface = WebInterface::new(index_access.clone(), style_access.clone(), web_send);

        // Spin the runtime into a native thread
        thread::spawn(move || {
            // Run the system interface in a new thread
            runtime.spawn(async move {
                system_interface.run().await;
            });

            // Block on the web interface
            runtime.block_on(async move {
                web_interface.run(web_interface_recv).await;
            });
        });

        // Create the application window, but do not show it
        let window = gtk::ApplicationWindow::new(application);

        // Create the gtk interface structure to handle video and media playback
        #[cfg(feature = "media-out")]
        GtkInterface::new(
            gtk_interface_recv,
        );

        // Open the user interface
        gtk::show_uri(None, "http://localhost:64636", 0).unwrap_or(());

        // Set the default parameters for the window FIXME
        /*window.set_title(WINDOW_TITLE);
        window.set_icon_from_file(LOGO_SQUARE).unwrap_or(()); // give up if unsuccessful

        // Disable the delete button for the window
        window.set_deletable(false);

        // Show the window
        window.show();*/
    }
}

/// The main function of the program, simplified to as high a level as possible
/// to allow GTK+ to work its startup magic.
///
fn main() {
    // Initialize tracing FIXME Consider using this for easier debugging
    tracing_subscriber::fmt::init();
    
    // Create the gtk application window. Failure results in immediate panic!
    let application = gtk::Application::new(None, gio::ApplicationFlags::empty());

    // Create the program and launch the background thread
    application.connect_startup(move |gtk_app| {
        Minerva::build_program(gtk_app);
    });

    // Connect the activate-specific function (as compared with open-specific function)
    application.connect_activate(|_| {});

    // Run the application until all the windows are closed
    application.run();
}
