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

// Import YAML processing libraries
#[macro_use]
extern crate serde;

// Define program modules
mod system_interface;
#[macro_use]
mod user_interface;
mod web_interface;

// Import the relevant structures into the correct namespace
use self::system_interface::{SystemInterface, SyncSystemSend};
use self::user_interface::UserInterface;
use self::web_interface::{WebInterface, WebReply};

// Import standard library features
use std::thread;
use std::env::args;
use std::sync::mpsc as std_mpsc;

// Import failure features
#[macro_use]
extern crate failure;

// Import GTK and GIO libraries
use gio;
use gtk;
use self::gtk::prelude::*;
use self::gio::prelude::*;
use self::gtk::SettingsExt;

// Import tokio features
use tokio::runtime::Runtime;

// Define program constants
const LOGO_SQUARE: &str = "logo_square.png";
const LOGO_WIDE: &str = "logo_wide.png";
const GTK_THEME: &str = "Materia-dark";
const FONT: &str = "Inter";
const WINDOW_TITLE: &str = "Minerva";

/// The Minerva structure to contain the program launching and overall
/// communication code.
///
pub struct Minerva {}

// Implement the Minerva functionality
impl Minerva {
    /// A function to build the main program and the user interface
    ///
    pub fn build_program(application: &gtk::Application) {
        // Load the gtk theme for this application
        if let Some(settings) = gtk::Settings::get_default() {
            settings.set_property_gtk_theme_name(Some(GTK_THEME));
            settings.set_property_gtk_font_name(Some(FONT));
        }
        
        // Create the tokio runtime
        let runtime = Runtime::new().expect("Unable To Create Tokio Runtime.");
        
        // Launch the system interface to monitor and handle events
        let (interface_send, interface_receive) = std_mpsc::channel();
        let (system_interface, system_send) = runtime.block_on(async {
            SystemInterface::new(interface_send.clone()).await
        }).expect("Unable To Create System Interface.");

        // Create a new web interface
        let mut web_interface = WebInterface::new(&system_send);

        // Create the new sync system send to work with the user interface
        let sync_send = SyncSystemSend::from_async(runtime.handle(), &system_send);

        // Spin the runtime into a native thread
        thread::spawn(move || {
            // Open the system interface in a new thread
            runtime.spawn(async move {
                system_interface.run().await;
            });

            // Block on the web interface
            runtime.block_on(async move {
                web_interface.run().await;
            });
        });

        // Create the application window
        let window = gtk::ApplicationWindow::new(application);

        // Create the user interface structure to handle user interface updates
        UserInterface::new(
            application,
            &window,
            sync_send,
            interface_send,
            interface_receive,
        );

        // Set the default parameters for the window
        window.set_title(WINDOW_TITLE);
        window.set_border_width(3);
        window.set_position(gtk::WindowPosition::Center);
        window.set_default_size(1500, 800);
        window.set_icon_from_file(LOGO_SQUARE).unwrap_or(()); // give up if unsuccessful

        // Disable the delete button for the window
        window.set_deletable(false);

        // Show the window
        window.show();
    }
}

/// The main function of the program, simplified to as high a level as possible
/// to allow GTK+ to work its startup magic.
///
fn main() {
    // Create the gtk application window. Failure results in immediate panic!
    let application = gtk::Application::new(None, gio::ApplicationFlags::empty())
        .expect("Initialization Failed For Unknown Reasons.");

    // Create the program and launch the background thread
    application.connect_startup(move |gtk_app| {
        Minerva::build_program(gtk_app);
    });

    // Connect the activate-specific function (as compared with open-specific function)
    application.connect_activate(|_| {});

    // Run the application until all the windows are closed
    application.run(&args().collect::<Vec<_>>());
}
