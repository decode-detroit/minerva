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

//! This module defines all structures and types used across modules.

// Define program constants
pub const DEFAULT_FILE: &str = "default.yaml"; // the default configuration filename
pub const DEFAULT_LOGLEVEL: &str = "Info";
pub const LOG_FOLDER: &str = "log/"; // the default log folder
pub const GAME_LOG: &str = "game_log"; // the default logging filename

// Define web interface constants
pub const DEFAULT_LIMITED_ADDRESS: &str = "127.0.0.1:64635";
pub const DEFAULT_RUN_ADDRESS: &str = "127.0.0.1:64636";
pub const DEFAULT_EDIT_ADDRESS: &str = "127.0.0.1:64637";

// Define testing submodule
#[cfg(test)]
mod test;

// Define submodules
mod background;
mod backup;
mod connections;
#[allow(dead_code)] // Allow dead code, reserved for future use
mod dmx;
mod event;
mod group;
#[allow(dead_code)] // Allow dead code, reserved for future use
mod index;
mod interface;
mod item;
mod media;
mod scene;
mod status;
#[allow(dead_code)] // Allow dead code, reserved for future use
mod style;
mod system;

// Reexport all the definitions from the submodules
pub use self::background::*;
pub use self::backup::*;
pub use self::connections::*;
pub use self::dmx::*;
pub use self::event::*;
pub use self::group::*;
pub use self::index::*;
pub use self::interface::*;
pub use self::item::*;
pub use self::media::*;
pub use self::scene::*;
pub use self::status::*;
pub use self::style::*;
pub use self::system::*;

// Reexport the testing module and definitions
#[cfg(test)]
pub use self::test::*;
