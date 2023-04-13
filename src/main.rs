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

// Import standard library features
use std::env;
use std::fs::DirBuilder;
use std::path::PathBuf;
use std::sync::Arc;
use std::fs::{File, OpenOptions};

// Import anyhow features
#[macro_use]
extern crate anyhow;

// Import tracing features
use tracing::error;
use tracing_subscriber::prelude::*;
use tracing_subscriber::filter::LevelFilter;

// Import the chrono library
use chrono::Local;

// Import sysinfo modules
use sysinfo::{System, SystemExt, Process, RefreshKind, ProcessRefreshKind};

// Define program constants
const USER_STYLE_SHEET: &str = "/tmp/userStyles.css";
const DEFAULT_LOGLEVEL: LevelFilter = LevelFilter::INFO; // FIXME for testing until proper logging is reimplemented
const FILE_LOGLEVEL: LevelFilter = LevelFilter::INFO;
const LOG_FOLDER: &str = "log/"; // the default log folder
const GAME_LOG: &str = "game_log"; // the default logging filename

/// The Minerva structure to contain the program launching and overall
/// communication code.
///
struct Minerva;

// Implement the Minerva functionality
impl Minerva {
    /// A function to setup the logging configuration
    /// 
    fn setup_logging() {
        // Try to load loggging folder and set the filepath
        let filepath = match env::current_dir() {
            // If the path loads
            Ok(mut path) => {
                // Create the log folder path
                path.push(LOG_FOLDER); // append the log folder

                // Make sure the log folder exists
                let builder = DirBuilder::new();
                builder.create(path.clone()).unwrap_or(()); // ignore if it already exits

                // Set the file for today
                path.push(format!("{}_{}.txt", GAME_LOG, Local::now().format("%Y-%m-%d")).as_str());

                // Return the completed path
                path
            }

            // Default to relative folder path
            _ => {
                let mut path = PathBuf::from(LOG_FOLDER);
                path.push(format!("{}{}.txt", GAME_LOG, Local::now().format("%Y-%m-%d")).as_str());

                // Return the relative path
                path
            }
        };

        // Open the game log
        let possible_file = match OpenOptions::new().append(true).open(filepath.to_str().unwrap_or("")) {
            Ok(file) => Some(file),

            // If the file does not exist
            Err(_) => {
                // Try to create the filepath instead
                match File::create(filepath.to_str().unwrap_or("")) {
                    Ok(file) => Some(file),
                    
                    // If we're unable to open the file, just log to terminal
                    Err(_) => {
                        error!("Unable to create game log file.");
                        None
                    }
                }
            }
        };

        // Create the stdout layer
        let stdout_layer = tracing_subscriber::fmt::layer().with_target(false).with_filter(DEFAULT_LOGLEVEL);

        // Create the log file layer, if available
        if let Some(file) = possible_file {
            let file_layer = tracing_subscriber::fmt::layer().with_writer(Arc::new(file)).with_ansi(false).with_target(false).with_filter(FILE_LOGLEVEL);
            
            // Initialize tracing
            tracing_subscriber::registry().with(stdout_layer).with(file_layer).init();
        
        // Otherwise, just log to the terminal
        } else {
            // Initialize tracing
            tracing_subscriber::registry().with(stdout_layer).init();
        }
    }

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
        let (system_interface, web_send) = SystemInterface::new(
            index_access.clone(),
            style_access.clone(),
            interface_send.clone(),
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

    // Initialize logging
    Minerva::setup_logging();

    // Create the program and run until directed otherwise
    Minerva::run().await;
}
