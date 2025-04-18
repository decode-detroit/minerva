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

//! This module implements structures shared from the system connection
//! modules.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::fmt;
use std::path::PathBuf;

// Import FNV HashSet
use fnv::FnvHashSet;

pub const UNIVERSAL_IDENTIFIER: u32 = 0;

/// The game instance identifier. Instances with the same identifier will trigger
/// events with one another; instances with different identifiers will not.
///
/// If no identifier is specified, this instance will accept all events and
/// produce events with the identifier 0.
///
/// If an identifier <x> is specified, this instance will only accept events with
/// the identifier <x> or the universal identifier (0), and it will produce events
/// with the identifier <x>.
///
/// Note: Specifying an identifier with the universersal identifier (0) is the
/// same as specifying None.
///
#[derive(PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Identifier {
    pub id: Option<u32>, // An optionally-specified identifier for this instance
}

// Implement display for identifier
impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.id {
            &Some(ref id) => write!(f, "{}", id),
            _ => write!(f, "~"),
        }
    }
}

/// An enum to specify the type of system connection.
///
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConnectionType {
    /// A variant to connect with a Mercury serial port. This implementation
    /// assumes the serial connection uses the Mercury event protocol.
    Mercury {
        path: PathBuf,                              // the location of the serial port
        alternate_paths: Vec<PathBuf>, // other locations where the serial port may appear
        baud: u32,                     // the baud rate of the serial port
        use_checksum: bool, // a flag indicating the system should use and verify 32bit checksums
        allowed_events: Option<FnvHashSet<ItemId>>, // if specified, the only events that can be sent to this connection
    },

    /// A variant to create a ZeroMQ connection. The connection type allows
    /// messages to be the sent and received. Received messages are echoed back
    /// to the send line so that all recipients will see the message.
    ZmqPrimary {
        send_path: PathBuf, // the location to bind the ZMQ sender
        recv_path: PathBuf, // the location to bind the ZMQ receiver
    },

    /// A variant to connect to an existing ZeroMQ connection over ZMQ.
    /// This connection presumes that a fully-functioning Minerva instance is
    /// is operating at the other end of the connection.
    ZmqSecondary {
        send_path: PathBuf, // the location to connect the ZMQ sender
        recv_path: PathBuf, // the location to connect the ZMQ receiver
    },
}

/// A type to contain any number of connection types
///
pub type ConnectionSet = Vec<ConnectionType>;
