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
use crate::definitions::{
    DisplayComponent, ItemId, ItemPair, SystemUpdate, RequestType, Status, SyncSystemSend,
};

// Import standard library features
use std::rc::Rc;
use std::cell::RefCell;

// Import FNV HashSet
use fnv;
use self::fnv::FnvHashMap;

// Import GTK and GDK libraries
use gdk;
use gtk;
use self::gtk::prelude::*;
use self::gtk::GridExt;


/// A compound structure to store an item id and the widget in which
/// its description is being displayed
///
#[derive(Clone, Debug)]
pub struct ItemDisplay {
    item_id: ItemId,
    label: gtk::Button
}


/// A structure to contain the grid for editing an individual status.
///
#[derive(Clone, Debug)]
pub struct EditStatus {
    grid: gtk::Grid,                     // a grid to hold the events
    system_send: SyncSystemSend,             // a copy of the system send
    status_checkbox: gtk::CheckButton,   // the button that toggles whether the item is a scene
    status_selection: gtk::ComboBoxText, // the dropdown that toggles the status type
    edit_multistate: EditMultiState,     // the edit multi state
    edit_countedstate: EditCountedState  // the edit counted state
}

// Implement key features for edit status
impl EditStatus {
    // A function to create a new Edit Status
    //
    pub fn new(system_send: &SyncSystemSend, is_left: bool) -> EditStatus {
        // Create the top-level grid
        let grid = gtk::Grid::new();

        // Create the grid to hold the status type data
        let status_grid = gtk::Grid::new();

        // Construct the checkbox for the status
        let status_checkbox = gtk::CheckButton::with_label("Item Corresponds To A Status");

        // Connect the checkbox to the visibility of the grid
        status_checkbox.connect_toggled(clone!(status_grid => move | checkbox | {
            // Make the elements invisible if the box isn't checked
            if checkbox.get_active() {
                status_grid.show();
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
        let edit_multistate = EditMultiState::new(system_send, is_left);
        let edit_countedstate = EditCountedState::new(system_send, is_left);

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
        grid.show_all();
        
        // Default to unchecked
        status_checkbox.set_active(false);
        status_grid.hide();

        // Return the EditStatus struct
        EditStatus {
            grid,
            system_send: system_send.clone(),
            status_checkbox,
            status_selection,
            edit_multistate,
            edit_countedstate,
        }
    }

    // A method to load the status
    pub fn load_status(&mut self, status: Option<Status>) {
        // Check to see if it is a valid status
        if let Some(status) = status {
            // Show the status by setting the check box
            self.status_checkbox.set_active(true);

            // Check which status variant it is
            match status.clone() {
                Status::MultiState { current, allowed, no_change_silent } => {
                    // Change the dropdown
                    self.status_selection.set_active_id(Some("multistate"));

                    // Load the data into the Edit MultiState
                    self.edit_multistate.load_multistate(&self.system_send, &current, allowed)
                }
                Status::CountedState { current, trigger, anti_trigger, reset, default_count, .. } => {
                    // Change the dropdown
                    self.status_selection.set_active_id(Some("countedstate"));

                    // Load the data into the Edit Counted State
                    self.edit_countedstate.load_countedstate(&current, &trigger, &anti_trigger, &reset, default_count);
                }
            }

        // Otherwise, deselect edit status
        } else {
            self.status_checkbox.set_active(false);
        }
    }

    // A method to pass a requested item description to the multistate status
    pub fn update_multistate_description(&self, description: ItemPair, position: Option<usize>) {
        self.edit_multistate.update_description(description, position)
    }

    // A method to pass a requested item description to the counted state status
    pub fn update_countedstate_description(&self, description: ItemPair, state_type: String) {
        self.edit_countedstate.update_description(description, state_type)
    }

    // A method to pack the new status
    pub fn pack_status(&self) -> Option<Status> {
        // If the checkbox was not selected, return None
        if !self.status_checkbox.get_active() {
            return None;
        }

        // Pack the status depending on the status type
        match self.status_selection.get_active_id() {
            // If there is a selection, pack the corresponding status
            Some(status_type) => {
                // Match the selection and change the visible options
                match status_type.as_str() {
                    // The multistate variant
                    "multistate" => return Some(self.edit_multistate.pack_multistate()),

                    // The counted state variant
                    "countedstate" => return self.edit_countedstate.pack_countedstate(),

                    // Otherwise
                    _ => return None,
                }
            }

            // Otherwise
            None => return None
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
    grid: gtk::Grid,                       // the main grid for this element
    is_left: bool,                         // whether the element is on the left or right
    current_label: gtk::Button,            // a label to display the current state
    current_data: Rc<RefCell<ItemId>>,     // the data of the current state
    states_list: gtk::ListBox,             // a list box to display the allowed states
    states_data: Rc<RefCell<FnvHashMap<usize, ItemDisplay>>>,  // the database of the ids and display elements
    next_state: Rc<RefCell<usize>>,        // the next available state location
}

// Implement key features for Edit MultiState
impl EditMultiState {
    // A function to create a multistate status variant
    //
    fn new(system_send: &SyncSystemSend, is_left: bool) -> EditMultiState {
        // Create the top level grid
        let grid = gtk::Grid::new();

        // Create a label to hold the current state description
        let current_label = gtk::Button::with_label("Current State: None");

        // Create an ItemId to hold the current state data
        let current_data = Rc::new(RefCell::new(ItemId::all_stop()));

        // Make the current label a drag destination and source
        drag!(dest current_label);
        drag!(source current_label);

        // Set the callback function when data is received
        current_label.connect_drag_data_received(clone!(current_data => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the current data
                if let Ok(mut current) = current_data.try_borrow_mut() {
                    *current = item_pair.get_id();
                }

                // Update the description on the current label
                widget.set_label(&format!("Current State: {}", &item_pair.description));

                // Set the callback function when data is dragged
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));


        // Create the database of states
        let states_data = Rc::new(RefCell::new(FnvHashMap::default()));
        let next_state = Rc::new(RefCell::new(0));

        // Create the ListBox to hold the states
        let states_list = gtk::ListBox::new();
        states_list.set_selection_mode(gtk::SelectionMode::None);

        // Make the states list a drag destination
        drag!(dest states_list);

        // Set the callback function when data is received
        states_list.connect_drag_data_received(clone!(states_data, next_state, system_send => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Add the status to the user interface and database
                EditMultiState::add_state(
                    &system_send,
                    is_left,
                    &widget,
                    &states_data,
                    &next_state,
                    item_pair.get_id()
                );
            }
        }));

        // Create and format a scroll window to hold the list of states
        let states_scroll = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        states_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        states_scroll.set_hexpand(true);
        states_scroll.set_size_request(-1, 150);

        // Add the list box to the scrolling window
        states_scroll.add(&states_list);

        // Create the label for the list box
        let states_label = gtk::Label::new(Some("Allowed states"));

        // Attach the elements to the grid
        grid.attach(&current_label, 0, 0, 1, 1);
        grid.attach(&states_label, 0, 1, 1, 1);
        grid.attach(&states_scroll, 0, 2, 1, 1);
        grid.set_row_spacing(10); // Add some space
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.show_all();

        EditMultiState{
            grid,
            is_left,
            current_label,
            current_data,
            states_list,
            states_data,
            next_state,
        }
    }

    // A method to load in info for a multistate status
    pub fn load_multistate(&mut self, system_send: &SyncSystemSend, current: &ItemId, mut allowed: Vec<ItemId>) {
        // Clear the previous data
        self.clear();

        // Add the current id to the database
        if let Ok(mut current_data) = self.current_data.try_borrow_mut() {
            *current_data = current.clone();
        }

        // Request the description associated with the current id
        system_send.send(SystemUpdate::Request {
            reply_to: DisplayComponent::EditMultiStateStatus{ is_left: self.is_left, position: None },
            request: RequestType::Description { item_id: current.clone() },
        });


        // Add the states to the user interface and database
        for state_id in allowed.drain(..) {
            EditMultiState::add_state(
                system_send,
                self.is_left,
                &self.states_list,
                &self.states_data,
                &self.next_state,
                state_id
            )
        }
    }

    /// A method to pack a multistate status
    ///
    pub fn pack_multistate(&self) -> Status {
        // Unwrap the states database
        if let Ok(states_data) = self.states_data.try_borrow() {
            // Unwrap the current data
            if let Ok(current_data) = self.current_data.try_borrow() {
                // Create a vector to hold the events and a counter
                let mut allowed = Vec::new();

                // Copy all the elements into the vector
                for state in states_data.values() {
                    allowed.push(state.clone().item_id);
                }

                // Pack and return the data as a status
                return Status::MultiState {
                    current: current_data.clone(),
                    allowed,
                    no_change_silent: false, // FIXME
                };
            }
        }
        unreachable!();
    }

    /// A method to clear all the states
    ///
    pub fn clear(&self) {
        // Remove all the state list elements
        let to_remove_states = self.states_list.get_children();
        for item in to_remove_states {
            unsafe {
                item.destroy();
            }
        }

        // Empty the states database
        if let Ok(mut states_db) = self.states_data.try_borrow_mut() {
            states_db.clear();
        }
    }

    /// A helper function to add an event to the events list and database
    ///
    fn add_state(
        system_send: &SyncSystemSend,
        is_left: bool,
        states_list: &gtk::ListBox,
        states_data: &Rc<RefCell<FnvHashMap<usize, ItemDisplay>>>,
        next_state: &Rc<RefCell<usize>>,
        state_id: ItemId,
    ){
        // Try to get a mutable copy of the next_state
        let position = match next_state.try_borrow_mut() {
            Ok(mut position) => {
                let tmp = position.clone();
                *position = *position + 1;
                tmp
            }

            // If unable, exit immediately
            _ => return,
        };

        // Create a button to hold the event description
        let state_description = gtk::Button::new();

        // Make the label a drag source
        drag!(source state_description);

        // Add the state to the state database
        if let Ok(mut states_database) = states_data.try_borrow_mut() {
            states_database.insert(
                position,
                ItemDisplay {
                    item_id: state_id.clone(),
                    label: state_description.clone()
                }
            );
        }

        // Request the description associated with the id
        system_send.send(SystemUpdate::Request {
            reply_to: DisplayComponent::EditMultiStateStatus{ is_left, position: Some(position) },
            request: RequestType::Description { item_id: state_id.clone() },
        });

        // Create a delete button
        let state_delete = gtk::Button::with_label("Delete");

        // Create a grid to display the label and delete button, and add it to the event list
        let state_grid = gtk::Grid::new();
        state_grid.attach(&state_description, 0, 0, 1, 1);
        state_grid.attach(&state_delete, 1, 0, 1, 1);
        states_list.add(&state_grid);
        state_grid.show_all();

        // Add some space
        state_grid.set_margin_top(10);
        state_grid.set_margin_bottom(10);
        state_grid.set_column_spacing(10);
        state_grid.set_row_spacing(10);

        // Connect functionality to delete a state on the button click
        state_delete.connect_clicked(clone!(states_list, states_data, position => move |_| {
            // Remove the event element from the user interface
            if let Some(widget) = state_grid.get_parent() {
                states_list.remove(&widget);
            }

            // Remove the event from the database
            if let Ok(mut states_database) = states_data.try_borrow_mut() {
                states_database.remove(&position);
            }
        }));
    }

    /// A method to update the description of a state label
    ///
    pub fn update_description(&self, item_pair: ItemPair, position: Option<usize>) {
        // Check to see if a position was given
        if let Some(item_position) = position {
            // Unpack the database
            if let Ok(states_db) = self.states_data.try_borrow() {
                // Get the ItemDisplay associated with the position
                if let Some(item_info) = states_db.get(&item_position) {
                    // Set the label description
                    item_info.label.set_label(&item_pair.description);

                    // Set the callback function when data is dragged
                    item_info.label.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _|  {
                        // Serialize the data
                        if let Ok(data) = serde_yaml::to_string(&item_pair) {
                            selection_data.set_text(data.as_str());
                        }
                    }));
                }
            }
        } else {
            // Otherwise, update the current state description
            self.current_label.set_label(&format!("Current State: {}", &item_pair.description));

            // Set the callback function when data is dragged
            self.current_label.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _|  {
                // Serialize the data
                if let Ok(data) = serde_yaml::to_string(&item_pair) {
                    selection_data.set_text(data.as_str());
                }
            }));
        }
    }

    /// A method to return the top element
    ///
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }
}

// Create the counted state variant
#[derive(Clone, Debug)]
struct EditCountedState {
    grid: gtk::Grid,                    // the main grid for this element
    system_send: SyncSystemSend,            // a copy of the system send line
    is_left: bool,                      // whether this element is on the left or right
    status_data: Rc<RefCell<FnvHashMap<String, ItemId>>>,   // a database for the data associated with the status
    current_label: gtk::Button,          // the button to display the current state
    trigger_label: gtk::Button,          // the button to display the trigger state
    antitrigger_label: gtk::Button,      // the button to display the antitrigger state
    reset_label: gtk::Button,            // the button to display the reset state
    count_spin: gtk::SpinButton,        // the spin to hold the default count
}

// Implement key features for Edit MultiState
impl EditCountedState {
    // A function to create a multistate status variant
    //
    fn new(system_send: &SyncSystemSend, is_left: bool) -> EditCountedState {
        // Create the top level grid
        let grid = gtk::Grid::new();

        // Create the database to hold the status data
        let status_data = Rc::new(RefCell::new(FnvHashMap::default()));

        // Create the label for the current item id
        let current_label = gtk::Button::with_label("Current State: None");

        // Create the label for the trigger item id
        let trigger_label = gtk::Button::with_label("Trigger State: None");

        // Create the label for the anti-trigger item id
        let antitrigger_label = gtk::Button::with_label("Anti-Trigger State: None");

        // Create the label for the reset item id
        let reset_label = gtk::Button::with_label("Reset State: None");

        // Create the label and spin button for the default count
        let count_label = gtk::Label::new(Some("Default Count:"));
        let count_spin = gtk::SpinButton::with_range(1.0, 536870911.0, 1.0);

        // Make the current button a drag source and destination
        drag!(source current_label);
        drag!(dest current_label);

        // Set the callback function when data is received
        current_label.connect_drag_data_received(clone!(status_data => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the user interface
                widget.set_label(&item_pair.description);

                // Update the status database
                if let Ok(mut database) = status_data.try_borrow_mut() {
                    database.insert("current".to_string(), item_pair.get_id());
                }

                // Set the callback function when data is dragged
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));

        // Make the trigger label a drag source and destination
        drag!(source trigger_label);
        drag!(dest trigger_label);

        // Set the callback function when data is received
        trigger_label.connect_drag_data_received(clone!(status_data => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the user interface
                widget.set_label(&item_pair.description);

                // Update the status database
                if let Ok(mut database) = status_data.try_borrow_mut() {
                    database.insert("trigger".to_string(), item_pair.get_id());
                }

                // Set the callback function when data is dragged
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));

        // Make the antitrigger label a drag source and destination
        drag!(source antitrigger_label);
        drag!(dest antitrigger_label);

        // Set the callback function when data is received
        antitrigger_label.connect_drag_data_received(clone!(status_data => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the user interface
                widget.set_label(&item_pair.description);

                // Update the status database
                if let Ok(mut database) = status_data.try_borrow_mut() {
                    database.insert("antitrigger".to_string(), item_pair.get_id());
                }

                // Set the callback function when data is dragged
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));

        // Make the reset label a drag source and destination
        drag!(source reset_label);
        drag!(dest reset_label);

        // Set the callback function when data is received
        reset_label.connect_drag_data_received(clone!(status_data => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the user interface
                widget.set_label(&item_pair.description);

                // Update the status database
                if let Ok(mut database) = status_data.try_borrow_mut() {
                    database.insert("reset".to_string(), item_pair.get_id());
                }

                // Set the callback function when data is dragged
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));

        // Add the elements to the grid
        grid.attach(&current_label, 0, 0, 1, 1);
        grid.attach(&trigger_label, 0, 1, 1, 1);
        grid.attach(&antitrigger_label, 0, 2, 1, 1);
        grid.attach(&reset_label, 0, 3, 1, 1);
        grid.attach(&count_label, 0, 4, 1, 1);
        grid.attach(&count_spin, 1, 4, 1, 1);
        grid.set_margin_top(10); // Add some space
        grid.set_margin_bottom(10);
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        // Show all elements
        grid.show_all();

        EditCountedState{
            grid,
            system_send: system_send.clone(),
            is_left,
            status_data,
            current_label,
            trigger_label,
            antitrigger_label,
            reset_label,
            count_spin,
        }
    }

    /// A method to load in info for a countedstate status
    ///
    pub fn load_countedstate(&self, current: &ItemId, trigger: &ItemId, antitrigger: &ItemId, reset: &ItemId, default_count: u32) {
        // Load the data into the database
        if let Ok(mut database) = self.status_data.try_borrow_mut() {
            database.insert("current".to_string(), current.clone());
            database.insert("trigger".to_string(), trigger.clone());
            database.insert("antitrigger".to_string(), antitrigger.clone());
            database.insert("reset".to_string(), reset.clone());
        }

        // Request the current id description
        self.system_send.send(SystemUpdate::Request {
            reply_to: DisplayComponent::EditCountedStateStatus {
                is_left: self.is_left,
                state_type: String::from("current")
            },
            request: RequestType::Description { item_id: current.clone() },
        });

        // Request the trigger id description
        self.system_send.send(SystemUpdate::Request {
            reply_to: DisplayComponent::EditCountedStateStatus {
                is_left: self.is_left,
                state_type: String::from("trigger")
            },
            request: RequestType::Description { item_id: trigger.clone() },
        });

        // Request the antitrigger id description
        self.system_send.send(SystemUpdate::Request {
            reply_to: DisplayComponent::EditCountedStateStatus {
                is_left: self.is_left,
                state_type: String::from("antitrigger")
            },
            request: RequestType::Description { item_id: antitrigger.clone() },
        });

        // Request the reset id description
        self.system_send.send(SystemUpdate::Request {
            reply_to: DisplayComponent::EditCountedStateStatus {
                is_left: self.is_left,
                state_type: String::from("reset")
            },
            request: RequestType::Description { item_id: reset.clone() },
        });

        // Load the default count
        self.count_spin.set_value(default_count as f64);

    }

    pub fn pack_countedstate(&self) -> Option<Status> {
        // Extract the default count from the spin button
        let default_count = self.count_spin.get_value() as u32;

        // Try to borrow a copy of the status database
        if let Ok(database) = self.status_data.try_borrow_mut() {
            // Pack up the data into a status
            return Some(Status::CountedState {
                current: *database.get("current")?,
                trigger: *database.get("trigger")?,
                anti_trigger: *database.get("antitrigger")?,
                reset: *database.get("reset")?,
                count: default_count.clone(), // the count is always set to default_count
                default_count: default_count.clone(),
                no_change_silent: false, // FIXME
            });
        }

        // Unreachable
        None

    }

    /// A method to update the description of a state label
    ///
    pub fn update_description(&self, item_pair: ItemPair, state_type: String) {
        // Determine which label to update
        match state_type.as_str() {
            // The current state variant
            "current" => {
                // Set the text on the label
                self.current_label.set_label(&format!("Current State: {}", &item_pair.description));

                // Set the callback function when data is dragged
                self.current_label.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _|  {
                    // Serialize the data
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            },

            "trigger" => {
                // Set the text on the label
                self.trigger_label.set_label(&format!("Trigger State: {}", &item_pair.description));

                // Set the callback function when data is dragged
                self.trigger_label.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _|  {
                    // Serialize the data
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            },

            "antitrigger" => {
                // Set the text on the label
                self.antitrigger_label.set_label(&format!("Anti-Trigger State: {}", &item_pair.description));

                // Set the callback function when data is dragged
                self.antitrigger_label.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _|  {
                    // Serialize the data
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            },

            "reset" => {
                // Set the text on the label
                self.reset_label.set_label(&format!("Reset State: {}", &item_pair.description));

                // Set the callback function when data is dragged
                self.reset_label.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _|  {
                    // Serialize the data
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            },

            // Unreachable
            _ => unreachable!()
        }
    }

    /// A method to return the top element
    ///
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }
}
