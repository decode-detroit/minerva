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
    ItemId, ItemPair, StatusDetail, SystemSend,
};

// Import standard library features
use std::rc::Rc;
use std::cell::RefCell;

// Import FNV HashSet
extern crate fnv;
use self::fnv::{FnvHashSet, FnvHashMap};

// Import GTK and GDK libraries
extern crate gdk;
extern crate gio;
extern crate gtk;
use self::gtk::prelude::*;
use self::gtk::GridExt;


/// A structure to contain the grid for editing an individual status.
///
#[derive(Clone, Debug)]
pub struct EditStatus {
    grid: gtk::Grid,                     // a grid to hold the events
    status_checkbox: gtk::CheckButton,   // the button that toggles whether the item is a scene
    status_selection: gtk::ComboBoxText, // the dropdown that toggles the status type
    edit_multistate: EditMultiState,     // the edit multi state detail
    edit_countedstate: EditCountedState  // the edit counted state detail
}

// Implement key features for edit status
impl EditStatus {
    // A function to create a new Edit Detail
    //
    pub fn new(system_send: &SystemSend) -> EditStatus {
        // Create the top-level grid
        let grid = gtk::Grid::new();

        // Create the grid to hold the status type data
        let status_grid = gtk::Grid::new();

        // Construct the checkbox for the status detail
        let status_checkbox = gtk::CheckButton::new_with_label("Item Corresponds To A Status");
        status_checkbox.set_active(false);

        // Connect the checkbox to the visibility of the grid
        status_checkbox.connect_toggled(clone!(status_grid => move | checkbox | {
            // Make the elements invisible if the box isn't checked
            if checkbox.get_active() {
                status_grid.show_all();
            } else {
                status_grid.hide();
            }
        }));

        // Create a dropdown for the action selection
        let status_selection = gtk::ComboBoxText::new();

        // Add the two available status types to the dropdown
        status_selection.append(Some("multistate"), "Multi State Status");
        status_selection.append(Some("countedstate"), "Counted State Status");

        // Create the different edit windows for the status types
        let edit_multistate = EditMultiState::new();
        let edit_countedstate = EditCountedState::new();

        // Create the status stack
        let status_stack = gtk::Stack::new();

        // Add the edit types to the status stack
        status_stack.add_named(edit_multistate.get_top_element(), "multistate");
        status_stack.add_named(edit_countedstate.get_top_element(), "countedstate");

        // Connect the function to trigger status selection changes
        status_selection.connect_changed(clone!(status_stack => move |dropdown| {
            // Identify the selected status type
            if let Some(status_str) = dropdown.get_active_id() {
                // Change the status stack to the requested variation
                status_stack.set_visible_child_full(&status_str, gtk::StackTransitionType::None);
            }
        }));

        // Add the dropdown and the action stack
        status_grid.attach(&status_selection, 0, 0, 1, 1);
        status_grid.attach(&status_stack, 0, 1, 1, 1);
        grid.attach(&status_checkbox, 0, 0, 1, 1);
        grid.attach(&status_grid, 0, 1, 1, 1);
        grid.set_column_spacing(10); // add some space
        grid.set_row_spacing(10);

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);

        grid.show_all();

        // Return the EditStatus struct
        EditStatus {
            grid,
            status_checkbox,
            status_selection,
            edit_multistate,
            edit_countedstate,
        }
    }

    // A method to load the status detail
    pub fn load_status(&mut self, status_detail: Option<StatusDetail>) {
        // Check to see if it is a valid status
        if let Some(status) = status_detail {
            // Show the status detail by setting the check box
            self.status_checkbox.set_active(true);

            // Check which status variant it is
            match status.clone() {
                StatusDetail::MultiState { allowed, .. } => {
                    // Change the dropdown
                    self.status_selection.set_active_id(Some("multistate"));

                    // Load the data into the Edit MultiState detail
                    self.edit_multistate.load_multistate(allowed)
                }
                StatusDetail::CountedState { trigger, anti_trigger, default_count, .. } => {
                    // Change the dropdown
                    self.status_selection.set_active_id(Some("countedstate"));

                    // Load the data into the Edit Counted State detail
                    self.edit_countedstate.load_countedstate(&trigger, &anti_trigger, default_count);
                }
            }
        }
    }

    // A method to return the top element
    //
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }
}

// Create the multistate variant
#[derive(Clone, Debug)]
struct EditMultiState {
    grid: gtk::Grid,            // the main grid for this element
    states: Vec<ItemId>,        // vector of state ids
    states_list: gtk::ListBox,  // a list box to display the allowed states
}

// Implement key features for Edit MultiState
impl EditMultiState {
    // A function to create a multistate status variant
    //
    fn new() -> EditMultiState {
        // Create the top level grid
        let grid = gtk::Grid::new();

        // Create the vector of states
        let states = Vec::new();

        // Create the ListBox to hold the states
        let states_list = gtk::ListBox::new();

        // Create an format a scroll window to hold the list of states
        let states_scroll = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        states_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        states_scroll.add(&states_list);

        // Attach the scroll window to the grid
        grid.attach(&states_scroll, 0, 0, 1, 1);
        grid.show_all();

        EditMultiState{
            grid,
            states,
            states_list,
        }
    }

    // A method to load in info for a multistate status
    pub fn load_multistate(&mut self, mut allowed: Vec<ItemId>) {
        // Set the allowed states for the status
        self.states = allowed.clone();

        // Add the states to the user interface
        for state_id in allowed.drain(..) {
            // Create a new label with the state id
            let state_label = gtk::Label::new(Some(&state_id.id().to_string()));

            // Attach the label to the list box
            self.states_list.add(&state_label);
        }
    }

    // A method to return the top element
    //
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }
}

// FIXME Create the counted state variant
#[derive(Clone, Debug)]
struct EditCountedState {
    grid: gtk::Grid,                     // the main grid for this element
    count_spin: gtk::SpinButton,         // the spin to hold the default count
    trigger_spin: gtk::SpinButton,     // the spin to hold the trigger item id
    antitrigger_spin: gtk::SpinButton, // the spin to hold the antitrigger item id
}

// Implement key features for Edit MultiState
impl EditCountedState {
    // A function to create a multistate status variant
    //
    fn new() -> EditCountedState {
        // Create the top level grid
        let grid = gtk::Grid::new();

        // Create the label and spin button for the default count
        let count_label = gtk::Label::new(Some("Default Count:"));
        let count_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Create the label and spin button for the trigger item id
        let trigger_label = gtk::Label::new(Some("Trigger State:"));
        let trigger_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Create the label and spin button for the anti-trigger item id
        let antitrigger_label = gtk::Label::new(Some("Antitrigger State:"));
        let antitrigger_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Connect the trigger spin button as a drag source
        trigger_spin.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        trigger_spin.connect_drag_data_received(|widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the current spin button value
                widget.set_value(item_pair.id() as f64);
            }
        });

        // Connect the antitrigger spin button as a drag source
        antitrigger_spin.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        antitrigger_spin.connect_drag_data_received(|widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the current spin button value
                widget.set_value(item_pair.id() as f64);
            }
        });

        // Add the elements to the grid
        grid.attach(&count_label, 0, 0, 1, 1);
        grid.attach(&count_spin, 1, 0, 1, 1);
        grid.attach(&trigger_label, 0, 1, 1, 1);
        grid.attach(&trigger_spin, 1, 1, 1, 1);
        grid.attach(&antitrigger_label, 0, 2, 1, 1);
        grid.attach(&antitrigger_spin, 1, 2, 1, 1);
        grid.set_margin_top(10); // Add some space
        grid.set_margin_bottom(10);
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        // Show all elements
        grid.show_all();

        EditCountedState{
            grid,
            count_spin,
            trigger_spin,
            antitrigger_spin,
        }
    }

    // A method to load in info for a countedstate status
    pub fn load_countedstate(&self, trigger: &ItemId, antitrigger: &ItemId, default_count: u32) {
        // Load the default count
        self.count_spin.set_value(default_count as f64);

        // Load the trigger id
        self.trigger_spin.set_value(trigger.id() as f64);

        // Load the antitrigger id
        self.antitrigger_spin.set_value(antitrigger.id() as f64);

    }

    // A method to return the top element
    //
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }
}
