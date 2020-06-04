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

//! A module to create, hold, and handle special dialogs for the edit view of
//! the user interface. These additional dialog windows are typically launched
//! from the edit screen.

// Import the relevant structures into the correct namespace
use super::super::super::super::system_interface::{
    DescriptiveScene, ItemPair, SystemSend,
};
use super::super::super::utils::{clean_text, decorate_label};

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;

// Import FNV HashMap
extern crate fnv;
use self::fnv::FnvHashMap;

// Import GTK and GDK libraries
extern crate gdk;
extern crate gio;
extern crate gtk;
use self::gtk::prelude::*;
use self::gtk::GridExt;


/// A structure to contain the grid for editing an individual scene.
///
#[derive(Clone, Debug)]
pub struct EditScene {
    grid: gtk::Grid,                         // a grid to hold the events
    system_send: SystemSend,                 // a reference to the system send line
    events_list: gtk::ListBox,               // a list box to hold the events in the scene
    detail_checkbox: gtk::CheckButton,       // the button that toggles visibility of the detail
    events: Vec<ItemPair>,                   // a vector of the events in the scene
}

// Implement key features of the EditScene
impl EditScene {
    /// A function to create a new instance of the EditScene
    ///
    pub fn new(
        system_send: &SystemSend,
    ) -> EditScene {


        // Create the scrolling window that contains the list box
        let events_scroll = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        events_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        events_scroll.set_hexpand(true);
        events_scroll.set_vexpand(true);
        events_scroll.set_halign(gtk::Align::Fill);
        events_scroll.set_valign(gtk::Align::Fill);

        // Create the list box to hold the event data and add it to the scrolling window
        let events_list = gtk::ListBox::new();
        events_scroll.add(&events_list);

        // Make the list box a drag source for events
        events_list.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        events_list.connect_drag_data_received(|widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Create a button with the item description
                let event_button = gtk::Button::new_with_label(&item_pair.description());

                // Add the button to the list box and show it
                widget.add(&event_button);
                event_button.show();
            }
        });


        // Construct the checkbox for the scene detail
        let detail_checkbox = gtk::CheckButton::new_with_label("Item Corresponds To A Scene");
        detail_checkbox.set_active(false);

        // Connect the checkbox to the visibility of the other elements
        detail_checkbox.connect_toggled(clone!(events_scroll => move | checkbox | {
            // Make the elements invisible if the box isn't checked
            if checkbox.get_active() {
                events_scroll.show();
            } else {
                events_scroll.hide();
            }
        }));

        // Create an empty vector to hold the events in the Scene
        let events = Vec::new();

        // Create a grid to hold the events and keyboard shortcuts
        let grid = gtk::Grid::new();

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        // Add some space within
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        // Attach the elements to the grid
        grid.attach(&detail_checkbox, 0, 0, 2, 1);
        grid.attach(&events_scroll, 0, 1, 1, 1);

        // Make the checkbox visible but hide the other elements by default
        grid.show();
        detail_checkbox.show();

        EditScene{
            grid,
            system_send: system_send.clone(),
            events_list,
            detail_checkbox,
            events,
        }
    }

    /// A method to update the list of all events in the scene
    ///
    pub fn update_info(&self, scene: Option<DescriptiveScene>) {
        // Clear the current events in the ListBox
        self.clear();

        // Check if the scene is valid
        match scene {
            Some(scene) =>  {
                // Show the scene detail by setting the check box
                self.detail_checkbox.set_active(true);

                // Iterate through the item pairs in the events vector
                for item_pair in scene.events {
                    // Create a label with the event description
                    let event_description = gtk::Label::new(Some(&item_pair.description()));

                    // Create a spinbutton with the keyboard shortcut and a label for it
                    let keymap_label = gtk::Label::new(Some("Keyboard shortcut:"));
                    // FIXME range on keymap spin?
                    let keymap_spin = gtk::SpinButton::new_with_range(1.0, 1000.0, 1.0);

                    // Check to see if the the keymap exists
                    if let Some(keymap) = scene.key_map.clone() {
                        // Get the key with the associated item id
                        if let Some(key) = keymap.get(&item_pair) {
                            // Set the value of the SpinButton to be the keyboard shortcut
                            // ASK: can the original keymaps in scenes be 2-to-1 (two keys to one item)?
                            keymap_spin.set_value(*key as f64);
                        }
                    }

                    // Create a grid to wrap all the event info
                    let event_grid = gtk::Grid::new();
                    event_grid.attach(&event_description, 0, 0, 1, 1);
                    event_grid.attach(&keymap_label, 1, 0, 1, 1);
                    event_grid.attach(&keymap_spin, 2, 0, 1, 1);

                    // Add the event information to the event list
                    self.events_list.add(&event_grid);

                }
                // Show all the buttons in the grid
                self.grid.show_all();
            }

            None => {
                // Hide the scene detail by unsetting the check box
                self.detail_checkbox.set_active(false);
            }
        }
    }

    /// A method to clear all the listed events in the ListBox
    ///
    pub fn clear(&self) {
        // Remove all the user interface elements
        let to_remove = self.events_list.get_children();
        for item in to_remove {
            item.destroy();
        }
    }

    /// A method to return the top element
    ///
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }
}
