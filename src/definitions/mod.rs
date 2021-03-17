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

// Define submodules
#[macro_use]
mod test;
mod item;
#[macro_use]
mod event;
mod status;
mod scene;
mod connections;
mod communication;

// Reexport all the definitions from the submodules
pub use self::test::*;
pub use self::item::*;
pub use self::event::*;
pub use self::status::*;
pub use self::scene::*;
pub use self::connections::*;
pub use self::communication::*;
