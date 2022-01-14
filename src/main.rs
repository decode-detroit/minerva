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
mod web_interface;

// Import crate definitions
use crate::definitions::*;

// Import other structures into this module
use self::item_index::ItemIndex;
use self::style_sheet::StyleSheet;
use self::system_interface::SystemInterface;
use self::web_interface::WebInterface;

// Import failure features
#[macro_use]
extern crate failure;

// Import tracing features
use tracing_subscriber;

// Define program constants
const USER_STYLE_SHEET: &str = "/tmp/userStyles.css";

/// The Minerva structure to contain the program launching and overall
/// communication code.
///
struct Minerva;

// Implement the Minerva functionality
impl Minerva {
    /// A function to build the main program and the user interface
    ///
    async fn run() {
        // Create the item index to process item description requests
        let (mut item_index, index_access) = ItemIndex::new();

        // Run the item index in a new thread (needed here to allow the system interface to load)
        tokio::spawn(async move {
            item_index.run().await;
        });

        // Create the style sheet to process style requests
        let (mut style_sheet, style_access) = StyleSheet::new();

        // Run the style sheet in a new thread (needed here to allow the system interface to load)
        tokio::spawn(async move {
            style_sheet.run().await;
        });

        // Create the interface send
        let (interface_send, web_interface_recv) = InterfaceSend::new();

        // Launch the system interface to monitor and handle events
        let (system_interface, web_send) = SystemInterface::new(index_access.clone(), style_access.clone(), interface_send.clone()).await.expect("Unable To Create System Interface.");

        // Create a new web interface
        let mut web_interface = WebInterface::new(index_access.clone(), style_access.clone(), web_send);

        // Run the web interface in a new thread
        tokio::spawn(async move {
            web_interface.run(web_interface_recv).await;
        });

        // Block on the system interface
        system_interface.run().await;
    }
}

/// The main function of the program, simplified to as high a level as possible.
///
#[tokio::main]
async fn main() {
    // Initialize tracing FIXME Consider using this for easier debugging
    tracing_subscriber::fmt::init();

    // Create the program and run until directed otherwise
    Minerva::run().await;
}
