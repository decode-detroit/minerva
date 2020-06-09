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
    DescriptiveScene, ItemId, ItemPair, Scene,
};

// Import standard library features
use std::mem;
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


/// A structure to keep track of keybindings
///
#[derive(Clone, Debug)]
struct Keybinding {
    key_value: Option<u32>,     // the key value that triggers an event
    event_id: Option<ItemId>,   // the event id bound to this key
}

// Implement key features of the Keybinding
impl Keybinding {
    /// A method to update the key value
    ///
    fn update_key(&mut self, new_value: u32) {
        self.key_value = Some(new_value);
    }

    /// A method to update the event id
    fn update_event(&mut self, new_id: ItemId) {
        self.event_id = Some(new_id);
    }

    /// A method to pack the key value pair
    fn pack_binding(&self) -> Option<(u32, ItemId)> {
        // If both are specified, return the binding
        if let Some(value) = self.key_value {
            if let Some(id) = self.event_id {
                return Some((value, id));
            }
        }

        // Otherwise, return None
        None
    }
}

/// A structure to contain the grid for editing an individual scene.
///
#[derive(Clone, Debug)]
pub struct EditScene {
    grid: gtk::Grid,                   // a grid to hold the events
    window: gtk::ApplicationWindow,    // a copy of the application window
    scene_checkbox: gtk::CheckButton,  // the button that toggles whether the item is a scene
    events_list: gtk::ListBox,         // a list box to hold the events in the scene
    events_data: Rc<RefCell<FnvHashMap<usize, ItemId>>>, // a database of events in the scene
    next_event: Rc<RefCell<usize>>,    // the next available event location
    keys_list: gtk::ListBox,           // a list box to hold the key bindings for the scene
    keys_data: Rc<RefCell<FnvHashMap<usize, Keybinding>>>, // a database for the key bindings
    next_keybinding: Rc<RefCell<usize>>, // the next available keybinding location
    key_press_handler: Rc<RefCell<Option<glib::signal::SignalHandlerId>>>, // the active handler for setting shortcuts
}

// Implement key features of the EditScene
impl EditScene {
    /// A function to create a new instance of the EditScene
    ///
    pub fn new(
        window: &gtk::ApplicationWindow,
    ) -> EditScene {

        // Create the database to hold the event data
        let events_data = Rc::new(RefCell::new(FnvHashMap::default()));
        let next_event = Rc::new(RefCell::new(0));

        // Create the scrolling window to hold the list box of events
        let events_scroll = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        events_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        events_scroll.set_hexpand(true);
        events_scroll.set_size_request(-1, 150);

        // Create the list box to hold the events and add it to the scrolling window
        let events_list = gtk::ListBox::new();
        events_list.set_selection_mode(gtk::SelectionMode::None);
        events_scroll.add(&events_list);

        // Create a label for the list of events
        let events_label = gtk::Label::new(Some("Events in the scene"));

        // Make the list box a drag destination for events
        events_list.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        events_list.connect_drag_data_received(clone!(events_data, next_event =>
        move |events_list, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Add the event to the event list
                EditScene::add_event(&events_list, &events_data, &next_event, item_pair);
            }
        }));

        // Create the key map that will hold the key binding data
        let keys_data = Rc::new(RefCell::new(FnvHashMap::default()));
        let next_keybinding = Rc::new(RefCell::new(0));

        // Create a scrolling window to hold the list box of key bindings
        let keys_scroll = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        keys_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        keys_scroll.set_hexpand(true);
        keys_scroll.set_size_request(-1, 150);

        // Create the list box to hold the key bindings and add it to the scrolling window
        let keys_list = gtk::ListBox::new();
        keys_list.set_selection_mode(gtk::SelectionMode::None);
        keys_scroll.add(&keys_list);

        // Create a button to make a new key binding
        let add_key = gtk::Button::new_with_label("Add key binding");

        //Create the wrapped key press handler
        let key_press_handler = Rc::new(RefCell::new(None));

        // When the button is clicked, add a new keybinding button
        add_key.connect_clicked(clone!(window, keys_list, keys_data, next_keybinding, key_press_handler => move |_| {
            EditScene::add_keybinding(
                &window,
                &keys_list,
                &keys_data,
                &next_keybinding,
                &key_press_handler,
                None, // No default keybinding
                None, // No default event
            );
        }));

        // Create a label for the key bindings window
        let keys_label = gtk::Label::new(Some("Keyboard shortcuts"));

        // Construct the checkbox for the scene detail
        let scene_checkbox = gtk::CheckButton::new_with_label("Item Corresponds To A Scene");
        scene_checkbox.set_active(false);

        // Connect the checkbox to the visibility of the other elements
        scene_checkbox.connect_toggled(clone!(
            events_label,
            events_scroll,
            keys_label,
            keys_scroll,
            add_key
        => move | checkbox | {
            // Make the elements invisible if the box isn't checked
            if checkbox.get_active() {
                events_label.show();
                events_scroll.show();
                keys_label.show();
                keys_scroll.show();
                add_key.show();
            } else {
                events_label.hide();
                events_scroll.hide();
                keys_label.hide();
                keys_scroll.hide();
                add_key.hide();
            }
        }));

        // Create a grid to hold the events and keyboard shortcuts
        let grid = gtk::Grid::new();
        grid.set_margin_top(10);  // Add some space on the top and bottom
        grid.set_margin_bottom(10);
        grid.set_column_spacing(10); // Add some space within
        grid.set_row_spacing(10);

        // Attach the elements to the grid
        grid.attach(&scene_checkbox, 0, 0, 2, 1);
        grid.attach(&events_label, 0, 1, 1, 1);
        grid.attach(&events_scroll, 0, 2, 1, 2);
        grid.attach(&keys_label, 1, 1, 1, 1);
        grid.attach(&keys_scroll, 1, 2, 1, 1);
        grid.attach(&add_key, 1, 3, 1, 1);

        // Make the checkbox visible but hide the other elements by default
        grid.show_all();
        scene_checkbox.set_active(false);

        EditScene{
            grid,
            window: window.clone(),
            scene_checkbox,
            events_list,
            events_data,
            next_event,
            keys_list,
            keys_data,
            next_keybinding,
            key_press_handler,
        }
    }

    /// A method to return the top element
    ///
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    /// A method to update the list of all events in the scene
    ///
    pub fn update_info(&self, scene: Option<DescriptiveScene>) {
        // Clear the current events in the ListBox
        self.clear();

        // Check if the scene is valid
        match scene {
            Some(mut scene) =>  {
                // Show the scene detail by setting the check box
                self.scene_checkbox.set_active(true);

                // Iterate through the item pairs in the events vector
                for item_pair in scene.events.drain(..) {
                    // Add each event to the events list
                    EditScene::add_event(
                        &self.events_list,
                        &self.events_data,
                        &self.next_event,
                        item_pair,
                    );
                }

                // Add any keybindings to the list
                if let Some(mut key_map) = scene.key_map {
                    // Look at each binding
                    for (key, id) in key_map.drain() {
                        // Add each to the interface
                        EditScene::add_keybinding(
                            &self.window,
                            &self.keys_list,
                            &self.keys_data,
                            &self.next_keybinding,
                            &self.key_press_handler,
                            Some(id),
                            Some(key)
                        );
                    }
                }
            }

            None => {
                // Hide the scene detail by unsetting the check box
                self.scene_checkbox.set_active(false);
            }
        }
    }

    /// A method to clear all the listed events in the ListBoxes
    ///
    pub fn clear(&self) {
        // Remove all the event list elements
        let to_remove_events = self.events_list.get_children();
        for item in to_remove_events {
            item.destroy();
        }

        // Remove all key binding elements
        let to_remove_keys = self.keys_list.get_children();
        for item in to_remove_keys {
            item.destroy();
        }

        // Empty the events database
        if let Ok(mut events_db) = self.events_data.try_borrow_mut() {
            events_db.clear();
        }

        // Empty the key bindings database
        if let Ok(mut keys_db) = self.keys_data.try_borrow_mut() {
            keys_db.clear();
        }
    }

    // A method to pack and return the scene
    //
    pub fn pack_detail(&self) -> Option<Scene> {
        // If the checkbox was not selected, return None
        if !self.scene_checkbox.get_active() {
            return None;
        }

        // Unwrap the events database
        if let Ok(events_data) = self.events_data.try_borrow() {
            // Unwrap the keys database
            if let Ok(keys_data) = self.keys_data.try_borrow() {
                // Create a hash set to hold the events and a counter
                let mut events = FnvHashSet::default();

                // Copy all the elements into the hash set
                for event in events_data.values() {
                    events.insert(event.clone());
                }

                // Create a hash set to hold the events and a couple counters
                let mut key_map = FnvHashMap::default();

                // Copy all the valid bindings into the hash map
                for binding in keys_data.values() {
                    // Try to pack the binding
                    if let Some((key, id)) = binding.pack_binding() {
                        key_map.insert(key, id);
                    }
                }

                // Set the key map as none if there are no bindings
                let key_map = match key_map.len() {
                    0 => None,
                    _ => Some(key_map),
                };

                // Pack and return the data as a scene
                return Some(Scene {
                    events,
                    key_map,
                });
            }
        }

        // Unreachable
        None
    }

    /// A helper function to add an event to the events list and database
    ///
    fn add_event(
        event_list: &gtk::ListBox,
        event_data: &Rc<RefCell<FnvHashMap<usize, ItemId>>>,
        next_event: &Rc<RefCell<usize>>,
        event: ItemPair,
    ){
        // Try to get a mutable copy of the next_event
        let position = match next_event.try_borrow_mut() {
            Ok(mut position) => {
                let tmp = position.clone();
                *position = *position + 1;
                tmp
            }

            // If unable, exit immediately
            _ => return,
        };

        // Add the event to the event database
        if let Ok(mut events_database) = event_data.try_borrow_mut() {
            events_database.insert(position, event.get_id());
        }

        // Create a label with the event description
        let event_description = gtk::Label::new(Some(&event.description()));

        // Create a delete button
        let event_delete = gtk::Button::new_with_label("Delete");

        // Create a grid to display the label and button, and add it to the event list
        let event_grid = gtk::Grid::new();
        event_grid.attach(&event_description, 0, 0, 1, 1);
        event_grid.attach(&event_delete, 1, 0, 1, 1);
        event_list.add(&event_grid);
        event_grid.show_all();

        // Connect functionality to delete an event on the button click
        event_delete.connect_clicked(clone!(event_list, event_data, position => move |_| {
            // Remove the event element from the user interface
            if let Some(widget) = event_grid.get_parent() {
                event_list.remove(&widget);
            }

            // Remove the event from the database
            if let Ok(mut events_database) = event_data.try_borrow_mut() {
                events_database.remove(&position);
            }
        }));
    }

    /// A helper function to add a keybinding button to the keybinding list
    ///
    fn add_keybinding(
        window: &gtk::ApplicationWindow,
        keys_list: &gtk::ListBox,
        keys_data: &Rc<RefCell<FnvHashMap<usize, Keybinding>>>,
        next_keybinding: &Rc<RefCell<usize>>,
        key_press_handler: &Rc<RefCell<Option<glib::signal::SignalHandlerId>>>,
        event: Option<ItemPair>,
        key_value: Option<u32>,
    ){
        // Try to get a mutable copy of the next_keybinding
        let position = match next_keybinding.try_borrow_mut() {
            Ok(mut position) => {
                let tmp = position.clone();
                *position = *position + 1;
                tmp
            }

            // If unable, exit immediately
            _ => return,
        };

        // Create a label to hold the event description
        let event_label = gtk::Label::new(Some("Event: None"));

        // If an event is given
        if let Some(event) = event.clone() {
            // Display the event description in the user interface
            event_label.set_text(&format!("Event: {}", &event.description()));
        }

        // Create a label for the keybinding button
        let key_label = gtk::Label::new(Some("Keyboard shortcut:"));

        // Create a button to hold the key binding
        let key_button = gtk::Button::new_with_label("None");

        // If a key value is given
        if let Some(key) = key_value {
            // Get the name of the key
            let key_name = match gdk::keyval_name(key) {
                Some(gstring) => String::from(gstring),
                None => String::from("Invalid Key Code"),
            };

            // Display it as a label on the button
            key_button.set_label(&key_name);
        }

        // Connect the key press event handler to the key button
        key_button.connect_clicked(clone!(
            window,
            keys_data,
            key_press_handler
        => move |button| {
            // Display a message on the button for the user to press a key
            button.set_label("Press a key");

            // Connect the handler
            EditScene::register_binding(
                &button,
                &window,
                &keys_data,
                position,
                &key_press_handler
            );
        }));

        // Unwrap the key database
        if let Ok(mut data) = keys_data.try_borrow_mut() {
            // Compose the keybinding
            let keybinding = match event {
                Some(event) => Keybinding { key_value, event_id: Some(event.get_id()) },
                None => Keybinding { key_value, event_id: None },
            };

            // Add the key binding to the keys database
            data.insert(position, keybinding);
        }

        // Create the delete button
        let key_delete = gtk::Button::new_with_label("Delete");

        // Create the list box grid element
        let keybinding_info = gtk::Grid::new();

        // Remove the user interface element and database entry when clicked
        key_delete.connect_clicked(clone!(keys_list, keys_data, keybinding_info, position => move |_| {
            // Remove the event element from the user interface
            if let Some(widget) = keybinding_info.get_parent() {
                keys_list.remove(&widget);
            }

            // Remove the key binding from the database
            if let Ok(mut data) = keys_data.try_borrow_mut() {
                data.remove(&position);
            }
        }));

        // Wrap all the info in the grid and add it to the list box row
        keybinding_info.attach(&event_label, 0, 0, 2, 1);
        keybinding_info.attach(&key_label, 0, 1, 1, 1);
        keybinding_info.attach(&key_button, 1, 1, 1, 1);
        keybinding_info.attach(&key_delete, 2, 0, 1, 1);
        keys_list.add(&keybinding_info);

        // Set up the row to receive a dropped item pair
        keybinding_info.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Attach a drag receiver to the listbox row
        keybinding_info.connect_drag_data_received(clone!(keys_data, event_label => move
        |_, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Unwrap the key binding database
                if let Ok(mut data) = keys_data.try_borrow_mut() {

                    // Get the keybinding for the position
                    if let Some(binding) = data.get_mut(&position) {

                        // Update the key value
                        binding.update_event(item_pair.get_id());
                    }
                }

                // Update the event label with the item description
                event_label.set_label(&format!("Event: {}", &item_pair.description()));
            }
        }));

        // Show the new row
        keybinding_info.show_all();
    }

    /// A helper function to register a keyboard input and display it on a button
    ///
    fn register_binding(
        button: &gtk::Button,
        window: &gtk::ApplicationWindow,
        keys_data: &Rc<RefCell<FnvHashMap<usize, Keybinding>>>,
        position: usize,
        handler: &Rc<RefCell<Option<glib::signal::SignalHandlerId>>>
    ){
        // Unwrap the key press handler
        if let Ok(mut key_press_handler) = handler.try_borrow_mut() {
            // Clear the old key press handler
            let mut tmp = None;
            mem::swap(&mut tmp, &mut key_press_handler);
            if let Some(key_press_handler) = tmp {
                window.disconnect(key_press_handler);
            }

            // Attach the new handler
            *key_press_handler = Some(
                window.connect_key_press_event(clone!(button, keys_data, handler, window => move |_, key_press| {
                    // Get the name of the key pressed
                    let key = match gdk::keyval_name(key_press.get_keyval()) {
                        Some(gstring) => String::from(gstring),
                        None => String::from("Invalid Key Code"),
                    };

                    // Display the key name on the button
                    button.set_label(&key);

                    // Unwrap the key binding database
                    if let Ok(mut data) = keys_data.try_borrow_mut() {

                        // Get the keybinding for the position
                        if let Some(binding) = data.get_mut(&position) {

                            // Update the key value
                            binding.update_key(key_press.get_keyval());
                        }
                    }

                    // Disconnect the signal handler
                    if let Ok(mut key_press_handler) = handler.try_borrow_mut() {
                        // Clear the old key press handler
                        let mut tmp = None;
                        mem::swap(&mut tmp, &mut key_press_handler);
                        if let Some(key_press_handler) = tmp {
                            window.disconnect(key_press_handler);
                        }
                    }

                    // Prevent any other keypress handlers from running
                    gtk::Inhibit(true)
                })),
            );
        }
    }
}
