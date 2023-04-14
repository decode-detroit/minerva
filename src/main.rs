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
#![recursion_limit = "256"]

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

// Import standard libary features
use std::io::BufWriter;
use std::sync::{Arc, Mutex};

// Import anyhow features
#[macro_use]
extern crate anyhow;

// Import tracing features
use tracing::{info, error, Level};
use tracing_subscriber::prelude::*;
use tracing_subscriber::filter::filter_fn;
use tracing_appender;

// Import sysinfo modules
use sysinfo::{System, SystemExt, Process, RefreshKind, ProcessRefreshKind};

// Define constants
pub const USER_STYLE_SHEET: &str = "/tmp/userStyles.css";
pub const DEFAULT_LOGLEVEL: Level = Level::ERROR;
pub const DEBUG_LOGLEVEL: Level = Level::INFO;
pub const LOG_FOLDER: &str = "log/"; // the default log folder

/// The Minerva structure to contain the program launching and overall
/// communication code.
///
struct Minerva;

// Implement the Minerva functionality
impl Minerva {
    /// A function to setup the logging configuration
    /// 
    fn setup_logging() -> (Arc<Mutex<bool>>, tracing_appender::non_blocking::WorkerGuard) {
        // Create the debug mode flag
        let is_debug = Arc::new(Mutex::new(false));
        let clone_debug = is_debug.clone();

        // Create the filter function for debug mode
        let debug_filter = filter_fn(move |metadata| {
            if let Ok(is_debug) = clone_debug.try_lock() {
                if *is_debug {
                    return metadata.level() == &DEBUG_LOGLEVEL;
                } else {
                    return metadata.level() == &DEFAULT_LOGLEVEL;
                }
            } else {
                return metadata.level() == &DEFAULT_LOGLEVEL;
            }
        });

        // Create the stdout layer
        let stdout_layer = tracing_subscriber::fmt::layer().with_target(false).with_filter(debug_filter);

        // Create a user interface layer
        //let buffer = Arc::new(Mutex::new(BufWriter::new(Vec::new())));
        //let buf_writer = buffer.clone().make_writer();
        //let user_layer = tracing_subscriber::fmt::layer::<Registry>().with_writer(buf_writer).with_ansi(false).with_target(false).with_filter(DEFAULT_LOGLEVEL);

        // Create the log file
        let file_appender = tracing_appender::rolling::daily(LOG_FOLDER, GAME_LOG);
        let (non_blocking, file_guard) = tracing_appender::non_blocking(file_appender);

        // Create the log file filter
        let file_filter = {
            filter_fn(|metadata| {
                metadata.target() == GAME_LOG || metadata.level() == &Level::ERROR
            }
        )};

        // Create the log file layer
        //let file_layer = tracing_subscriber::fmt::layer().with_writer(non_blocking).with_ansi(false).with_target(false).with_filter(file_filter);
        
        // Initialize tracing
        tracing_subscriber::registry().with(stdout_layer).init(); //.with(file_layer).init();

        // Return the debug flag and file guard
        (is_debug, file_guard)
    }

    /// A function to build the main program and the user interface
    ///
    async fn run() {
        // Initialize logging (guard is held until the end of run())
        let (log_handle, _guard) = Minerva::setup_logging();

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
        let (system_interface, web_send) = SystemInterface::new(
            index_access.clone(),
            style_access.clone(),
            interface_send.clone(),
            log_handle,
        )
        .await;

        // Create a new web interface
        let mut web_interface =
            WebInterface::new(index_access.clone(), style_access.clone(), web_send);

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
    // Get system information
    let refresh_kind = RefreshKind::new().with_processes(ProcessRefreshKind::everything());
    let sys_info = System::new_with_specifics(refresh_kind);
    
    // Check to ensure Minerva is not already running FIXME allow multiple instances with commandline argument
    if sys_info.processes_by_exact_name("minerva").collect::<Vec::<&Process>>().len() > 1 {
        println!("Minerva is already running. Exiting ...");
        return;
    }

    // If successful, drop the uneeded information
    drop(refresh_kind);
    drop(sys_info);

    // Create the program and run until directed otherwise
    Minerva::run().await;
}
