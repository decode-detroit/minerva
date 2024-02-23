// Copyright (c) 2019-2024 Decode Detroit
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

//! A module to communicate using the Mercury serial protocol
//!
//! # Note
//!
//! This module is a dummy module for Windows installations due to a bug
//! in Tokio Serial.


// Import crate definitions
use crate::definitions::*;

// Import other definitions
use super::{EventConnection, EventWithData};

// Import standard library modules and traits
use std::path::PathBuf;

// Import FNV HashSet
use fnv::FnvHashSet;

// Import anyhow features
use anyhow::Result;

/// A structure to hold and manipulate the connection over serial, dummy version
///
pub struct Mercury {}

// Implement key functionality for the Mercury structure
impl Mercury {
    /// A function to create a new instance of the Mercury
    ///
    pub fn new(
        _path: &PathBuf,
        _baud: u32,
        _use_checksum: bool,
        _allowed_events: Option<FnvHashSet<ItemId>>,
        _write_timeout: u64,
    ) -> Result<Self> {
        Ok(Self {})
    }
}

// Implement the event connection trait for Mercury
impl EventConnection for Mercury {
    /// A method to receive a new event from the serial connection
    ///
    async fn read_event(&mut self) -> Option<EventWithData> {
        // Return no events
        None
    }

    /// A method to send a new event to the serial connection
    ///
    async fn write_event(&mut self, _id: ItemId, _data1: u32, _data2: u32) -> Result<()> {
        // Return an error
        Err(anyhow!("Mercury connections are not yet supported on Windows."))
    }

    /// A method to echo an event to the serial connection
    ///
    async fn echo_event(&mut self, _id: ItemId, _data1: u32, _data2: u32) -> Result<()> {
        // Do nothing
        Ok(())
    }

    /// A method to process any pending writes to the serial connection
    ///
    async fn process_pending(&mut self) -> bool {
        // Do nothing
        false
    }
}
