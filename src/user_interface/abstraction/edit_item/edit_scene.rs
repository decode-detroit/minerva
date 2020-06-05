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
    DescriptiveScene, ItemId, ItemPair, Scene, SystemSend,
};
use super::super::super::utils::{clean_text, decorate_label};

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;
use std::mem;

// Import FNV HashSet
extern crate fnv;
use self::fnv::{FnvHashSet, FnvHashMap};

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
    events_data: Rc<RefCell<FnvHashSet<ItemId>>>, // a database for the events in the scene
    keys_list: gtk::ListBox,                 // a list box to hold the key bindings for the scene
    keys_data: Rc<RefCell<FnvHashMap<u32, ItemId>>>, // a database for the events with key bindings
    detail_checkbox: gtk::CheckButton,       // the button that toggles visibility of the detail
    key_press_handler: Rc<RefCell<Option<glib::signal::SignalHandlerId>>>, // the active handler for setting shortcuts
    window: gtk::ApplicationWindow,         // a copy of the application window
}

// Implement key features of the EditScene
impl EditScene {
    /// A function to create a new instance of the EditScene
    ///
    pub fn new(
        window: &gtk::ApplicationWindow,
        system_send: &SystemSend,
    ) -> EditScene {

        // Create the database to hold the event data
        let events_data = Rc::new(RefCell::new(FnvHashSet::default()));

        // Create the scrolling window to hold the list box of events
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

        // Create a label for the list of events
        let events_label = gtk::Label::new(Some("Events in the scene"));

        // Create the key map that will hold the key binding data
        let keys_data = Rc::new(RefCell::new(FnvHashMap::default()));

        // Create a scrolling window to hold the list box of key bindings
        let keys_scroll = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        keys_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        keys_scroll.set_hexpand(true);
        keys_scroll.set_vexpand(true);
        keys_scroll.set_halign(gtk::Align::Fill);
        keys_scroll.set_valign(gtk::Align::Fill);

        // Create the list box to hold the key binding data and add it to the scrolling window
        let keys_list = gtk::ListBox::new();
        keys_list.set_selection_mode(gtk::SelectionMode::None);
        keys_scroll.add(&keys_list);

        // Create a button to make a new key binding
        let add_key = gtk::Button::new_with_label("Add key binding");

        //Create the wrapped key press handler
        let key_press_handler = Rc::new(RefCell::new(None));

        // When the button is clicked, add a new keybinding button
        add_key.connect_clicked(clone!(keys_list, keys_data, window, key_press_handler => move |_| {
            EditScene::add_keybinding(
                &keys_list,
                keys_data.clone(),
                None, // No default keybinding
                None, // No default event
                key_press_handler.clone(),
                &window
            );
        }));

        // Create a label for the key bindings window
        let keys_label = gtk::Label::new(Some("Keyboard shortcuts"));

        // Make the list box a drag source for events
        events_list.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        events_list.connect_drag_data_received(clone!(
            events_data,
            keys_list,
            keys_data,
            window,
            key_press_handler =>
        move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Add the event to the appropriate lists (without a keybinding)
                EditScene::add_event(
                    &widget,
                    events_data.clone(),
                    &keys_list,
                    keys_data.clone(),
                    item_pair.clone(),
                    None,
                    key_press_handler.clone(),
                    &window
                );
            }
        }));

        // Construct the checkbox for the scene detail
        let detail_checkbox = gtk::CheckButton::new_with_label("Item Corresponds To A Scene");
        detail_checkbox.set_active(false);

        // Connect the checkbox to the visibility of the other elements
        detail_checkbox.connect_toggled(clone!(
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

        // Add some space on the top and bottom
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);

        // Add some space within
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        // Attach the elements to the grid
        grid.attach(&detail_checkbox, 0, 0, 2, 1);
        grid.attach(&events_label, 0, 1, 1, 1);
        grid.attach(&events_scroll, 0, 2, 1, 1);
        grid.attach(&keys_label, 1, 1, 1, 1);
        grid.attach(&keys_scroll, 1, 2, 1, 1);
        grid.attach(&add_key, 1, 3, 1, 1);

        // Make the checkbox visible but hide the other elements by default
        grid.show();
        detail_checkbox.show();

        EditScene{
            grid,
            system_send: system_send.clone(),
            events_list,
            events_data,
            keys_list,
            keys_data,
            detail_checkbox,
            key_press_handler,
            window: window.clone(),
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
                    // Check to see if the the keymap exists
                    match scene.key_map.clone() {
                        // If so, add the event with the binding
                        Some(keymap) => EditScene::add_event(
                            &self.events_list,
                            self.events_data.clone(),
                            &self.keys_list,
                            self.keys_data.clone(),
                            item_pair.clone(),
                            keymap.get(&item_pair),
                            self.key_press_handler.clone(),
                            &self.window,
                        ),
                        // Otherwise, add the event without a key binding

                        None => EditScene::add_event(
                            &self.events_list,
                            self.events_data.clone(),
                            &self.keys_list,
                            self.keys_data.clone(),
                            item_pair.clone(),
                            None,
                            self.key_press_handler.clone(),
                            &self.window,
                        )
                    }
                }
                // Show all the new items in the grid
                self.grid.show_all();
            }

            None => {
                // Hide the scene detail by unsetting the check box
                self.detail_checkbox.set_active(false);
            }
        }
    }

    /// A helper function to add an event and optional keybinding to the window
    ///
    fn add_event(
        event_list: &gtk::ListBox,
        event_data: Rc<RefCell<FnvHashSet<ItemId>>>,
        keybinding_list: &gtk::ListBox,
        key_data: Rc<RefCell<FnvHashMap<u32, ItemId>>>,
        event: ItemPair,
        keybinding: Option<&u32>,
        key_press_handler: Rc<RefCell<Option<glib::signal::SignalHandlerId>>>,
        window: &gtk::ApplicationWindow,
    ){
        // Create a label with the event description and add it to the user interface event list
        let event_description = gtk::Label::new(Some(&event.description()));
        event_list.add(&event_description);

        // Add the event to the event database
        if let Ok(mut events_database) = event_data.try_borrow_mut() {
            events_database.insert(event.get_id());
        }

        // If a keybinding exists, add it to the keybinding list
        if let Some(key) = keybinding {
            EditScene::add_keybinding(
                keybinding_list,
                key_data,
                Some(event),
                Some(key),
                key_press_handler,
                window
            );
        }
    }

    /// A helper function to add a keybinding button to the keybinding list
    ///
    fn add_keybinding(
        keybinding_list: &gtk::ListBox,
        key_data: Rc<RefCell<FnvHashMap<u32, ItemId>>>,
        event: Option<ItemPair>,
        keybinding: Option<&u32>,
        key_press_handler: Rc<RefCell<Option<glib::signal::SignalHandlerId>>>,
        window: &gtk::ApplicationWindow,
    ){
        // Create a label to hold the event description
        let event_label = gtk::Label::new(Some("Event: None"));
        // If an event is given
        if let Some(event) = event.clone() {
            // Display the event description in the user interface
            event_label.set_label(&format!("Event: {}", &event.description()));
        }

        // Create a label for the keybinding button
        let key_label = gtk::Label::new(Some("Keyboard shortcut:"));

        // Create a button to hold the key binding
        let key_button = gtk::Button::new_with_label("None");
        // If a key binding is given
        if let Some(key) = keybinding {
            // Get the name of the key
            let key_name = match gdk::keyval_name(*key) {
                Some(gstring) => String::from(gstring),
                None => String::from("Invalid Key Code"),
            };
            // Display it as a label on the button
            key_button.set_label(&key_name);
        }

        // Unwrap the key database
        if let Ok(mut key_database) = key_data.try_borrow_mut() {
            // Add the event/key pair to the keys database
            match event.clone() {
                Some(event) => {
                    match keybinding {
                        // If the key and event exist, add them
                        Some(key) => key_database.insert(key.clone(), event.get_id()),
                        // Otherwise add the default key
                        None => key_database.insert(0, event.get_id()), // FIXME default key?
                    };
                },
                None => {
                    match keybinding {
                        // If the key exists, add it with the default event
                        Some(key) => key_database.insert(key.clone(), ItemId::all_stop()),
                        // Otherwise add the default key and event
                        None => key_database.insert(0, ItemId::all_stop()),
                    };
                }
            }
        }

        // Connect the key press event handler to the key button
        key_button.connect_clicked(clone!(event, window, key_data => move |button| {
            // Display a message on the button for the user to press a key
            button.set_label("Press a key");

            // Connect the handler
            EditScene::register_input(
                &button,
                key_data.clone(),
                &window,
                key_press_handler.clone()
            );
        }));

        // Create the list box row container
        let row = gtk::ListBoxRow::new();

        // Attach a drag receiver to the listbox row
        row.connect_drag_data_received(clone!(
            key_data,
            event_label,
            key_button
        => move |_, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };
                // Update the event label with the item description
                event_label.set_label(&format!("Event: {}", &item_pair.description()));

                // Add the event id to the keys database
                if let Ok(mut keys_db) = key_data.try_borrow_mut() {
                    // Get the key binding on the button
                    if let Some(button_name) = key_button.get_label() {
                        // Convert the button name to the associated key value
                        let button_val = match button_name.as_str() {
                            // If "None", assign the default value
                            "None" => 0,
                            // Otherwise, get the value
                            _ => gdk::keyval_from_name(button_name.as_str())
                        };

                        // Insert the new item id into the database
                        keys_db.insert(button_val.clone(), item_pair.get_id());
                    }
                }
            }
        }));

        // Wrap all the info in a grid and add it to the list box row
        let keybinding_info = gtk::Grid::new();
        keybinding_info.attach(&event_label, 0, 0, 2, 1);
        keybinding_info.attach(&key_label, 0, 1, 1, 1);
        keybinding_info.attach(&key_button, 1, 1, 1, 1);
        row.add(&keybinding_info);

        // Add the row to the list box
        keybinding_list.add(&row);

        // Show the new row
        keybinding_list.show_all();
    }

    /// A method to register a keyboard input and display it on a button
    fn register_input(
        button: &gtk::Button,
        key_data: Rc<RefCell<FnvHashMap<u32, ItemId>>>,
        window: &gtk::ApplicationWindow,
        handler: Rc<RefCell<Option<glib::signal::SignalHandlerId>>>
    ){
        // Unwrap the key press handler
        if let Ok(mut key_press_handler) = handler.try_borrow_mut() {
            // Clear the old key press handler
            let mut tmp = None;
            mem::swap(&mut tmp, &mut key_press_handler);
            if let Some(key_press_handler) = tmp {
                window.disconnect(key_press_handler);
            }

            *key_press_handler = Some(
                // Attach the handler
                window.connect_key_press_event(clone!(button => move |_, key_press| {
                    // Get the name of the key pressed
                    let key = match gdk::keyval_name(key_press.get_keyval()) {
                        Some(gstring) => String::from(gstring),
                        None => String::from("Invalid Key Code"),
                    };

                    // Get the old key name to update the key_database
                    let old_name = match button.get_label() {
                        Some(name) => name,
                        _ => unreachable!()
                    };

                    // Get the old key value
                    let old_value = match old_name.as_str() {
                        // If "None", assign the default value
                        "None" => 0,
                        // Otherwise, get the value from the name
                        _ => gdk::keyval_from_name(old_name.as_str())
                    };

                    // Display the key name on the button
                    button.set_label(&key);

                    // Unwrap the key binding database
                    if let Ok(key_database) = key_data.try_borrow_mut() {
                        // Get the event associated with the old button
                        if let Some(event) = key_database.get(&old_value) {
                            // Create a mutable copy to insert and remove
                            let mut key_database_mut = key_database.clone();

                            // Delete the old key binding information
                            key_database_mut.remove(&old_value);

                            // Add the new key binding information
                            key_database_mut.insert(key_press.get_keyval(), event.clone());
                        }
                    }

                    // Prevent any other keypress handlers from running
                    gtk::Inhibit(true)
                })),
            );
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
        if !self.detail_checkbox.get_active() {
            return None;
        }

        // Unwrap the events database
        match self.events_data.try_borrow() {
            Ok(events) => {
                // Unwrap the keys database
                match self.keys_data.try_borrow() {
                    Ok(key_map) =>  {
                        // Pack the data into a scene
                        let scene = Scene {
                            events: events.clone(),
                            key_map: Some(key_map.clone())
                        };

                        // Return the complete scene
                        Some(scene)
                    },

                    //Unreachable
                    _ => unreachable!()
                }
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
