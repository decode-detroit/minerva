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

// Import anyhow features
#[macro_use]
extern crate anyhow;

// Import tracing features
use tracing::Level;
use tracing_appender;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::prelude::*;

// Import clap features
use clap::Parser;

// Import sysinfo modules
use sysinfo::{Process, ProcessRefreshKind, RefreshKind, System, SystemExt};

/// Struct to hold the optional arguments for Minerva
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Arguments {
    /// Relative path to a configuration file
    #[arg(short, long, default_value = DEFAULT_FILE)]
    config: String,

    /// Flag to allow for multiple instances
    #[arg(short = 'm', long, default_value = "false")]
    allow_multiple: bool,

    /// Run address for the web interface
    #[arg(long, default_value = DEFAULT_RUN_ADDRESS)]
    run_addr: String,

    /// Edit address for the web interface
    #[arg(long, default_value = DEFAULT_EDIT_ADDRESS)]
    edit_addr: String,

    /// Limited access address for the web interface
    #[arg(long, default_value = DEFAULT_LIMITED_ADDRESS)]
    limited_addr: String,
}

/// The Minerva structure to contain the program launching and overall
/// communication code.
///
struct Minerva;

// Implement the Minerva functionality
impl Minerva {
    /// A function to setup the logging configuration
    ///
    fn setup_logging() -> tracing_appender::non_blocking::WorkerGuard {
        // Create the stdout layer
        let stdout_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_filter(DEFAULT_LOGLEVEL);

        // Create a user interface layer FIXME collect warning and error messages to display to the user
        //let buffer = Mutex::new(Writer::new(Arc::new(Vec::new()))); //Arc::new()
        //let buf_writer = buffer.clone().make_writer();
        //let user_layer = tracing_subscriber::fmt::layer().with_writer(buffer).with_ansi(false).with_target(false).with_filter(DEFAULT_LOGLEVEL);

        // Create the log file
        let file_appender = tracing_appender::rolling::daily(LOG_FOLDER, GAME_LOG);
        let (non_blocking, file_guard) = tracing_appender::non_blocking(file_appender);

        // Create the log file filter
        let file_filter = {
            filter_fn(|metadata| metadata.target() == GAME_LOG || metadata.level() == &Level::ERROR)
        };

        // Create the log file layer
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(false)
            .with_filter(file_filter);

        // Initialize tracing
        tracing_subscriber::registry()
            .with(stdout_layer)
            .with(file_layer)
            .init();

        // Return and file guard
        file_guard
    }

    /// A function to build the main program and the user interface
    ///
    async fn run(arguments: Arguments) {
        // Initialize logging (guard is held until the end of run())
        let _guard = Minerva::setup_logging();

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
            arguments.config,
        )
        .await;

        // Create a new web interface
        let mut web_interface =
            WebInterface::new(index_access.clone(), style_access.clone(), web_send);

        // Run the web interface in a new thread
        tokio::spawn(async move {
            web_interface
                .run(
                    web_interface_recv,
                    arguments.limited_addr,
                    arguments.run_addr,
                    arguments.edit_addr,
                )
                .await;
        });

        // Block on the system interface
        system_interface.run().await;
    }
}

/// The main function of the program, simplified to as high a level as possible.
///
#[tokio::main]
async fn main() {
    // Get the commandline arguments
    let arguments = Arguments::parse();

    // If not allowing multiple instances
    if !arguments.allow_multiple {
        // Get system information
        let refresh_kind = RefreshKind::new().with_processes(ProcessRefreshKind::everything());
        let sys_info = System::new_with_specifics(refresh_kind);

        // Check to ensure Minerva is not already running
        if sys_info
            .processes_by_exact_name("minerva")
            .collect::<Vec<&Process>>()
            .len()
            > 1
        {
            println!("Minerva is already running. Exiting ...");
            return;
        }
    }

    // Create the program and run until directed otherwise
    Minerva::run(arguments).await;
}
