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

//! This module implements structures to define the background process.

// Import standard library features
use std::path::PathBuf;

/// A struct to define the elements of a background process
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackgroundProcess {
    pub process: PathBuf,       // the location (relative or absolute) of the process to run
    pub arguments: Vec<String>, // any arguments to pass to the process
    pub keepalive: bool, // a flag to indicate if the process should be restarted if it stops/fails
}
