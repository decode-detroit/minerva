// Copyright (c) 2017 Decode Detroit
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

//! A module to define the macros used in editing items.


// Import the serde_yaml library
extern crate serde_yaml;

// Import GTK and GDK libraries
extern crate gdk;
extern crate gio;
extern crate glib;
extern crate gtk;


/// A macro that allows the user to set a widget as a drag source,
/// or a drag destination
///
#[macro_export]
macro_rules! drag {
    // Set a widget as a drag source
    (source $widget:expr) => ({
        $widget.drag_source_set(
            gdk::ModifierType::MODIFIER_MASK,
            &vec![
                gtk::TargetEntry::new("STRING", gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY,
        );
    });

    // Set a widget as a drag destination
    (dest $widget:expr) => ({
        $widget.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );
    })
}
