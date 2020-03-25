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

//! A module to create, hold, and handle special windows for the user interface.
//! These additional dialog windows are typically launched from the system menu.

// Import the relevant structures into the correct namespace
use super::super::super::system_interface::{
    AllStop, BroadcastEvent, EditMode, FullStatus, Hidden, ItemId, ItemPair,
    KeyMap, ProcessEvent, SceneChange, StatusChange, StatusDescription, SystemSend,
};
use super::super::utils::{clean_text, decorate_label};
use super::NORMAL_FONT;

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;
use std::mem;

// Import FNV HashMap
extern crate fnv;
use self::fnv::FnvHashMap;

// Import GTK and GDK libraries
extern crate gdk;
extern crate gio;
extern crate gtk;
extern crate glib;
use self::gio::{ActionExt, SimpleAction};
use self::gtk::prelude::*;
use self::gtk::GridExt;

// Define and import constants
const STATE_LIMIT: usize = 20; // maximum character width of states
const DESCRIPTION_LIMIT: usize = 40; // shortcut event descriptions character limit
use super::super::WINDOW_TITLE; // the window title

/// A structure to contain the dialog for confirming the edit selection.
///
pub struct EditDialog {
    edit_mode: Rc<RefCell<bool>>, // a flag to indicate edit mode for the system
    window: gtk::ApplicationWindow, // a reference to the primary window
}

// Implement key features for the edit dialog
impl EditDialog {
    /// A function to create a new edit dialog structure.
    ///
    pub fn new(edit_mode: Rc<RefCell<bool>>, window: &gtk::ApplicationWindow) -> EditDialog {
        EditDialog {
            edit_mode,
            window: window.clone(),
        }
    }

    /// A method to launch the new edit dialog
    ///
    pub fn launch(&self, system_send: &SystemSend, checkbox: &SimpleAction) {
        // Create the new dialog
        let dialog = gtk::Dialog::new_with_buttons(
            Some("Switch To Edit Mode?"),
            Some(&self.window),
            gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
            &[
                ("Cancel", gtk::ResponseType::Cancel),
                ("Confirm", gtk::ResponseType::Ok),
            ],
        );
        dialog.set_position(gtk::WindowPosition::Center);

        // Access the content area and add the dropdown
        let content = dialog.get_content_area();
        let grid = gtk::Grid::new();
        content.add(&grid);

        // Add the dropdown and label
        grid.attach(&gtk::Label::new(Some("  Switching to edit mode will end the current games and is not possible to undo.  ")), 0, 0, 1, 1);

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);

        // Connect the close event for when the dialog is complete
        let edit_mode = self.edit_mode.clone();
        let window = self.window.clone();
        dialog.connect_response(clone!(system_send, checkbox => move |modal, id| {

            // Notify the system of the event change
            if id == gtk::ResponseType::Ok {

                // Change the internal flag to edit mode
                if let Ok(mut flag) = edit_mode.try_borrow_mut() {
                    *flag = true;
                }

                // Change the status of the checkbox
                checkbox.change_state(&(true).to_variant());

                // Send the edit mode notification to the underlying system
                system_send.send(AllStop);
                system_send.send(EditMode(true));

                // Change the title of the window
                window.set_title(format!("{} - Edit Mode", WINDOW_TITLE).as_str());
            }

            // Close the window either way
            modal.destroy();
        }));

        // Show the dialog and return
        dialog.show_all();
    }
}

/// A structure to contain the dialog for modifying an individual status.
///
pub struct StatusDialog {
    full_status: Rc<RefCell<FullStatus>>, // a hashmap of status id pairs and status descriptions, stored inside Rc/RefCell
    window: gtk::ApplicationWindow,       // a copy of the primary window
}

// Implement key features for the status dialog
impl StatusDialog {
    /// A function to create a new status dialog structure with the ability to
    /// modify an individual status.
    ///
    pub fn new(
        full_status: Rc<RefCell<FullStatus>>,
        window: &gtk::ApplicationWindow,
    ) -> StatusDialog {
        StatusDialog {
            full_status,
            window: window.clone(),
        }
    }

    /// A method to launch the new status dialog with the current state of
    /// all of the statuses in the current configuration.
    ///
    pub fn launch(&self, system_send: &SystemSend, status: Option<ItemPair>) {
        // Create the new dialog
        let dialog = gtk::Dialog::new_with_buttons(
            Some("Modify Status"),
            Some(&self.window),
            gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
            &[
                ("Cancel", gtk::ResponseType::Cancel.into()),
                ("Confirm", gtk::ResponseType::Ok.into()),
            ],
        );
        dialog.set_position(gtk::WindowPosition::Center);

        // Create the status selection dropdown
        let status_selection = gtk::ComboBoxText::new();

        // Try to get a readable copy of the full status
        let full_status = match self.full_status.try_borrow() {
            Ok(full_status) => full_status,
            Err(_) => return, // fail silently
        };

        // Add each of the available status to the dropdown
        for status_pair in full_status.keys() {
            let id_str: &str = &status_pair.id().to_string();
            status_selection.append(Some(id_str), &status_pair.description());
        }

        // Create the state selection flowbox
        let state_box = gtk::FlowBox::new();
        state_box.set_orientation(gtk::Orientation::Horizontal);
        state_box.set_selection_mode(gtk::SelectionMode::Single);
        state_box.set_hexpand(true);
        state_box.set_halign(gtk::Align::Fill);
        state_box.set_size_request(400, 10);

        // Create the state hashmap
        let state_map = Rc::new(RefCell::new(FnvHashMap::default()));

        // Connect the function to trigger when the status selection changes
        let protected_status = self.full_status.clone();
        status_selection.connect_changed(clone!(state_box, state_map => move |dropdown| {

            // Remove all the existing items in the state box and vector
            let to_remove = state_box.get_children();
            for item in to_remove {
                state_box.remove(&item);
            }
            let mut map = match state_map.try_borrow_mut() {
                Ok(map) => map,
                Err(_) => return,
            };
            map.clear();

            // Identify and forward the selected event
            if let Some(id_str) = dropdown.get_active_id() {

                // Try to parse the requested id
                if let Ok(id_number) = id_str.parse::<u32>() {

                    // Try to compose the id into an item
                    if let Some(id) = ItemPair::new(id_number, "", Hidden) {

                        // Look up the corresponding status detail
                        if let Ok(full_status) = protected_status.try_borrow() {

                            // Find the corresponding detail
                            if let Some(&StatusDescription { ref current, ref allowed }) = full_status.get(&id) {
                                // Extract the allowed ids and add them to the states
                                for (num, state_pair) in allowed.iter().enumerate() {

                                    // Create a new flow box child and add the label
                                    let child = gtk::FlowBoxChild::new();
                                    let text = clean_text(&state_pair.description, STATE_LIMIT, false, false, true);
                                    let label = gtk::Label::new(None);
                                    decorate_label(&label, &text, state_pair.display, &full_status, NORMAL_FONT, false, None);
                                    let button = gtk::Button::new();
                                    button.connect_clicked(clone!(state_box, child => move |_| {
                                        state_box.select_child(&child);
                                    }));
                                    button.add(&label);
                                    child.add(&button);

                                    // Add the child to the state box
                                    state_box.insert(&child, num as i32);

                                    // Add the location and id number to the map
                                    map.insert(state_pair.get_id(), num as i32);
                                }

                                // Show all the state box items
                                state_box.show_all();

                                // Set the current state
                                if let Some(num) = map.get(&current.get_id()) {
                                    if let Some(child) = state_box.get_child_at_index(num.clone()) {
                                        state_box.select_child(&child);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }));

        // Select the relevant status, if specified
        if let Some(status_id) = status {
            status_selection.set_active_id(Some(status_id.id().to_string().as_str()));
        }

        // Access the content area and add the dropdown
        let content = dialog.get_content_area();
        let grid = gtk::Grid::new();
        content.add(&grid);

        // Add the dropdowns and labels
        grid.attach(&gtk::Label::new(Some("  Status  ")), 0, 0, 1, 1);
        grid.attach(&status_selection, 1, 0, 1, 1);
        grid.attach(&gtk::Label::new(Some("  State  ")), 2, 0, 1, 1);
        grid.attach(&state_box, 3, 0, 1, 1);

        // Add some space between the rows and columns
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);

        // Connect the close event for when the dialog is complete
        dialog.connect_response(clone!(system_send =>
        move |modal, id| {

            // Try to get a mutable copy of the event
            let map = match state_map.try_borrow() {
                Ok(map) => map,
                Err(_) => return,
            };

            // Notify the system of the event change
            if id == gtk::ResponseType::Ok {

                // Identify and forward the selected status
                if let Some(id_status) = status_selection.get_active_id() {

                    // Try to parse the requested id
                    if let Ok(status_number) = id_status.parse::<u32>() {

                        // Try to compose the id as an item
                        if let Some(status_id) = ItemId::new(status_number) {

                            // Identify and forward the selected state
                            let mut state = ItemId::new_unchecked(0);
                            for child in state_box.get_selected_children() {

                                // Match the child to the id number
                                let index = child.get_index();
                                for (id, num) in map.iter() {
                                    if *num == index {
                                        state = id.clone();
                                    }
                                }
                            }

                            // Send the new state update to the system
                            system_send.send(StatusChange { status_id, state });
                        }
                    }
                }
            }

            // Close the window either way
            modal.destroy();
        }));

        // Show the dialog and return
        dialog.show_all();
    }
}

/// A structure to contain the dialog for jumping between individual scenes.
///
pub struct JumpDialog {
    scenes: Vec<ItemPair>,          // a vector of the available scenes
    window: gtk::ApplicationWindow, // a copy of the primary window
}

// Implement key features for the jump dialog
impl JumpDialog {
    /// A function to create a new jump dialog structure with the ability to
    /// change between individual scenes.
    ///
    pub fn new(window: &gtk::ApplicationWindow) -> JumpDialog {
        JumpDialog {
            scenes: Vec::new(),
            window: window.clone(),
        }
    }

    /// A method to launch the new jump dialog with the current list of available
    /// scenes in the configuration.
    ///
    pub fn launch(&self, system_send: &SystemSend, scene: Option<ItemPair>) {
        // Create the new dialog
        let dialog = gtk::Dialog::new_with_buttons(
            Some("Jump To ..."),
            Some(&self.window),
            gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
            &[
                ("Cancel", gtk::ResponseType::Cancel),
                ("Confirm", gtk::ResponseType::Ok),
            ],
        );
        dialog.set_position(gtk::WindowPosition::Center);

        // Create the scene selection dropdown
        let scene_selection = gtk::ComboBoxText::new();

        // Add each of the available scene to the dropdown
        for scene_pair in self.scenes.iter() {
            let id_str: &str = &scene_pair.id().to_string();
            scene_selection.append(Some(id_str), &scene_pair.description());
        }

        // Change to the selected scene, if selected
        if let Some(scene_pair) = scene {
            scene_selection.set_active_id(Some(&scene_pair.id().to_string()));
        }

        // Access the content area and add the dropdown
        let content = dialog.get_content_area();
        let grid = gtk::Grid::new();
        content.add(&grid);

        // Add the dropdown and label
        grid.attach(&gtk::Label::new(Some("  Jump To:  ")), 0, 0, 1, 1);
        grid.attach(&scene_selection, 1, 0, 1, 1);

        // Add some space between the rows and columns
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);

        // Connect the close event for when the dialog is complete
        dialog.connect_response(clone!(scene_selection, system_send => move |modal, id| {

            // Notify the system of the event change
            if id == gtk::ResponseType::Ok {

                // Identify and forward the selected scene
                if let Some(id_scene) = scene_selection.get_active_id() {

                    // Try to parse the requested id
                    if let Ok(scene_number) = id_scene.parse::<u32>() {

                        // Try to compose the id as an item
                        if let Some(scene) = ItemId::new(scene_number) {

                            // Send the new state update to the system
                            system_send.send(SceneChange { scene });
                        }
                    }
                }
            }

            // Close the window either way
            modal.destroy();
        }));

        // Show the dialog and return
        dialog.show_all();
    }

    /// A method to update the available scenes in the jump dialog.
    ///
    pub fn update_scenes(&mut self, new_scenes: Vec<ItemPair>) {
        self.scenes = new_scenes;
    }
}

/// A structure to contain the dialog for the keyboard shortcuts.
///
pub struct ShortcutsDialog {
    key_press_handler: Option<glib::signal::SignalHandlerId>, // the active handler
    key_map: KeyMap, // the map of key codes to event ids
    system_send: SystemSend, // a copy of system send
    window: gtk::ApplicationWindow, // a copy of the primary window
}

// Implement key features for the shortcuts dialog
impl ShortcutsDialog {
    /// A function to create a new shortcuts dialog structure with the ability 
    /// to bind and display keyboard shortcuts
    ///
    pub fn new(system_send: &SystemSend, window: &gtk::ApplicationWindow) -> ShortcutsDialog {
        ShortcutsDialog {
            key_press_handler: None,
            key_map: KeyMap::default(),
            system_send: system_send.clone(),
            window: window.clone(),
        }
    }

    /// A method to launch the new jump dialog with the current list of available
    /// scenes in the configuration.
    ///
    pub fn launch(&self) {
        // Create the new dialog
        let dialog = gtk::Dialog::new_with_buttons(
            Some("Keyboard Shortcuts"),
            Some(&self.window),
            gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
            &[
                ("Close", gtk::ResponseType::Ok),
            ],
        );
        dialog.set_position(gtk::WindowPosition::Center);

        // Access the content area and add the primary grid
        let content = dialog.get_content_area();
        let grid = gtk::Grid::new();
        content.add(&grid);

        // Add the grid columns
        let tmp = gtk::Label::new(None);
        tmp.set_markup("<span size='13000'>Shortcut Key</span>");
        grid.attach(&tmp, 0, 0, 1, 1);
        let tmp = gtk::Label::new(None);
        tmp.set_markup("<span size='13000'>Event Description</span>");
        grid.attach(&tmp, 1, 0, 1, 1);
        
        // Add a separator
        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        separator.set_halign(gtk::Align::Fill);
        separator.set_hexpand(true);
        grid.attach(&separator, 0, 1, 2, 1);

        // Populate the grid with any shortcuts
        let mut count = 2;
        for (key, id) in self.key_map.iter() {
            // Add the event description
            let description = clean_text(&id.description, DESCRIPTION_LIMIT, false, false, true);
            grid.attach(&gtk::Label::new(Some(&description)), 1, count, 1, 1);
            
            // Add the shortcut description
            let key = match gdk::keyval_name(key.clone()) {
                Some(gstring) => String::from(gstring),
                None => String::from("Invalid Key Code"),
            };
            grid.attach(&gtk::Label::new(Some(&format!("  {}  ", key))), 0, count, 1, 1);
            
            // Increment the count
            count = count + 1;
        }
        
        // Add the none label if there are no shortcuts
        if count == 1 {
            grid.attach(&gtk::Label::new(Some("No Active Shortcuts")), 0, 1, 2, 1);
        }

        // Add some space between the rows and columns
        grid.set_column_spacing(20);
        grid.set_row_spacing(10);

        // Add some space on all the sides
        grid.set_margin_top(20);
        grid.set_margin_bottom(20);
        grid.set_margin_start(20);
        grid.set_margin_end(20);

        // Connect the close event for when the dialog is complete
        dialog.connect_response(|modal, _| {
            // Close the window
            modal.destroy();
        });

        // Show the dialog and return
        dialog.show_all();
    }

    /// A method to update the keyboard shortcuts.
    ///
    pub fn update_shortcuts(&mut self, key_map: KeyMap) {
        // Clear the old key press handler
        let mut tmp = None;
        mem::swap(&mut tmp, &mut self.key_press_handler);
        if let Some(handler) = tmp {
            self.window.disconnect(handler);
        }
        
        // Create a new handler (prevents any errant key presses if empty)
        let key_clone = key_map.clone();
        let send_clone = self.system_send.clone();
        self.key_press_handler = Some(
            // Attach the handler
            self.window.connect_key_press_event(move |_, key_press| {
                // Check to see if it matches one of our events
                if let Some(id) = key_clone.get(&key_press.get_keyval()) {
                    send_clone.send(ProcessEvent { event: id.get_id(), check_scene: true, broadcast: true});
                }
                
                // Prevent any other handlers from running
                gtk::Inhibit(true)
            })
        );
        
        // Save the key map to be displayed
        self.key_map = key_map;
   }
}

/// A structure to contain the dialog for triggering a custom event.
///
pub struct TriggerDialog {
    window: gtk::ApplicationWindow, // a copy of the primary window
}

// Implement key features for the trigger dialog
impl TriggerDialog {
    /// A function to create a new trigger dialog structure.
    ///
    pub fn new(window: &gtk::ApplicationWindow) -> TriggerDialog {
        TriggerDialog {
            window: window.clone(),
        }
    }

    /// A method to launch the new trigger dialog
    ///
    pub fn launch(&self, system_send: &SystemSend, event: Option<ItemPair>) {
        // Create the new dialog
        let dialog = gtk::Dialog::new_with_buttons(
            Some("Manually Trigger Event"),
            Some(&self.window),
            gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
            &[
                ("Cancel", gtk::ResponseType::Cancel),
                ("Confirm", gtk::ResponseType::Ok),
            ],
        );
        dialog.set_position(gtk::WindowPosition::Center);

        // Access the content area and add the dropdown
        let content = dialog.get_content_area();
        let grid = gtk::Grid::new();
        content.add(&grid);

        // Add the dropdown and label
        let label = gtk::Label::new(Some(
            " Warning: Triggering A Custom Event May Cause Undesired Behaviour. ",
        ));
        grid.attach(&label, 0, 0, 4, 1);

        // Description label for the current event
        let event_description = gtk::Label::new(Some(""));

        // Create the event selection
        let event_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // If an id was specified, use it
        if let Some(event_pair) = event {
            event_spin.set_value(event_pair.id() as f64);
            event_description.set_text(&event_pair.description());
        }

        // Create the checkboxes
        let scene_checkbox = gtk::CheckButton::new_with_label("Check Scene");
        scene_checkbox.set_active(true);
        let broadcast_checkbox = gtk::CheckButton::new_with_label("Skip All Checks");
        broadcast_checkbox.set_active(false);
        grid.attach(&event_spin, 0, 1, 1, 1);
        grid.attach(&event_description, 1, 1, 1, 1);
        grid.attach(&scene_checkbox, 2, 1, 1, 1);
        grid.attach(&broadcast_checkbox, 3, 1, 1, 1);

        // Add some space between the rows and columns
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        
        // Make sure the scene checkbox is false when broadcast is selected
        broadcast_checkbox.connect_toggled(clone!(scene_checkbox => move | checkbox | {
            if checkbox.get_active() {
                scene_checkbox.set_active(false);
            }
        }));

        // Connect the close event for when the dialog is complete
        dialog.connect_response(clone!(system_send, event_spin, scene_checkbox, broadcast_checkbox => move |modal, id| {

            // Notify the system of the event change
            if id == gtk::ResponseType::Ok {

                // If broadcast is selected, just send a broadcast event
                if broadcast_checkbox.get_active() {
                    system_send.send(BroadcastEvent { event: ItemPair::new_unchecked(event_spin.get_value() as u32, "", Hidden), data: None});
                
                // Otherwise, send the selected event to the system
                } else { system_send.send(ProcessEvent { event: ItemId::new_unchecked(event_spin.get_value() as u32), check_scene: scene_checkbox.get_active(), broadcast: true});
                }
            }

            // Close the window either way
            modal.destroy();
        }));

        // Show the dialog and return
        dialog.show_all();
    }
}

/// A structure to contain the dialog for soliciting a string from the user.
///
pub struct PromptStringDialog {
    window: gtk::ApplicationWindow, // a copy of the primary window
}

// Implement key features for the prompt string dialog
impl PromptStringDialog {
    /// A function to create a new prompt string dialog structure.
    ///
    pub fn new(window: &gtk::ApplicationWindow) -> PromptStringDialog {
        PromptStringDialog {
            window: window.clone(),
        }
    }

    /// A method to launch the new prompt string dialog
    ///
    pub fn launch(&self, system_send: &SystemSend, event: ItemPair) {
        // Create the new dialog
        let dialog = gtk::Dialog::new_with_buttons(
            Some(&clean_text(&event.description, STATE_LIMIT, false, false, true)),
            Some(&self.window),
            gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
            &[
                ("Cancel", gtk::ResponseType::Cancel),
                ("Confirm", gtk::ResponseType::Ok),
            ],
        );
        dialog.set_position(gtk::WindowPosition::Center);

        // Access the content area and add the dropdown
        let content = dialog.get_content_area();
        let grid = gtk::Grid::new();
        content.add(&grid);

        // Add the dropdown and label
        let label = gtk::Label::new(Some(
            " Enter Text ",
        ));
        grid.attach(&label, 0, 0, 1, 1);

        // Create the text entry area of the dialog
        let buffer = gtk::TextBuffer::new(Some(&gtk::TextTagTable::new())); // because gtk struggles with typing
        let view = gtk::TextView::new_with_buffer(&buffer);
        view.set_size_request(200, 100);
        grid.attach(&view, 0, 1, 1, 1);

        // Add some space between the rows and columns
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);

        // Connect the close event for when the dialog is complete
        dialog.connect_response(clone!(system_send, buffer => move |modal, id| {

            // Notify the system of the event change
            if id == gtk::ResponseType::Ok {

                // Extract the completed text
                let start = buffer.get_start_iter();
                let end = buffer.get_end_iter();
                if let Some(gtext) = buffer.get_text(&start, &end, false) {
                    
                    // Convert the text into bytes
                    let mut bytes = gtext.to_string().into_bytes();
                         
                    // Save the length of the new vector
                    let length = bytes.len() as u32;
                    let mut data = vec![length];
                    
                    // Convert the bytes into a u32 Vec
                    let (mut first, mut second, mut third, mut fourth) = (0, 0, 0, 0);
                    for (num, byte) in bytes.drain(..).enumerate() {
                        
                        // Repack the data efficiently
                        match num % 4 {
                            0 => first = byte as u32,
                            1 => second = byte as u32,
                            2 => third = byte as u32,
                            _ => {
                                fourth = byte as u32;
                                data.push((first << 24) | (second << 16) | (third << 8) | fourth);
                            }
                        }
                    }
                    
                    // Save the last bit of data if the total doesn't add to 4
                    if (length % 4) != 0 {
                       data.push((first << 24) | (second << 16) | (third << 8) | fourth);
                    }
                    
                    // Send each bit of data to the system
                    for num in data.drain(..) {
                        system_send.send(BroadcastEvent { event: event.clone(), data: Some(num)});
                    }
                }
            }

            // Close the window either way
            modal.destroy();
        }));

        // Show the dialog and return
        dialog.show_all();
    }
}

/*/// A structure to contain the dialog for modifying an individual event. This
/// dialog is currently rather inconvenient to use as it is not made for use
/// during typical operations.
///
/// FIXME NEED to ADD the SEND DATA VARIANT
///
#[derive(Clone, Debug)]
pub struct EditEventDialog {
    window: gtk::ApplicationWindow, // a copy of the primary window
}

// Implement key features for the edit event dialog
impl EditEventDialog {
    /// A function to create a new edit event dialog structure with the ability
    /// to modify an individual event detail.
    ///
    pub fn new(window: &gtk::ApplicationWindow) -> EditEventDialog {
        EditEventDialog {
            window: window.clone(),
        }
    }

    /// A method to launch the edit event dialog. If there is an event detail
    /// provided, the dialog will edit that event. Otherwise, the event will
    /// create a new event.
    ///
    pub fn launch(
        &self,
        system_send: &SystemSend,
        old_pair: Option<ItemPair>,
        old_detail: Option<EventDetail>,
    ) {
        // Create the new dialog
        let dialog = gtk::Dialog::new_with_buttons(
            Some("Add/Edit Event Detail"),
            Some(&self.window),
            gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
            &[
                ("Cancel", gtk::ResponseType::Cancel),
                ("Confirm", gtk::ResponseType::Ok),
            ],
        );
        dialog.set_position(gtk::WindowPosition::Center);

        // Create the stack of event detail options
        let detail_stack = gtk::Stack::new();

        // Add the selection label to the stack
        let select_type_label = gtk::Label::new(Some("Select Event Type"));
        detail_stack.add_named(&select_type_label, "empty");
        detail_stack.set_visible_child_full("empty", gtk::StackTransitionType::SlideDown);

        // Create and add the new scene variant to the detail stack
        let edit_new_scene = EditNewScene::new();
        detail_stack.add_named(edit_new_scene.get_top_element(), "scene");

        // Create and add the modify status variant to the detail stack
        let edit_modify_status = EditModifyStatus::new();
        detail_stack.add_named(edit_modify_status.get_top_element(), "status");

        // Create and add the trigger event variant to the detail stack
        let edit_trigger_events = EditQueueEvent::new();
        detail_stack.add_named(edit_trigger_events.get_top_element(), "events");

        // Create and add the cancel event variant to the detail stack
        let edit_cancel_events = EditCancelEvent::new();
        detail_stack.add_named(edit_cancel_events.get_top_element(), "cancel");

        // Create and add the save data variant to the detail stack
        let edit_save_data = EditSaveData::new();
        detail_stack.add_named(edit_save_data.get_top_element(), "data");

        // Create and add the grouped event variant to the detail stack
        let edit_grouped_event = EditGroupedEvent::new();
        detail_stack.add_named(edit_grouped_event.get_top_element(), "grouped");

        // Create the edit overview
        let edit_overview = EditOverview::new(&detail_stack);

        // Add the overview and the stack to the dialog grid
        let dialog_grid = gtk::Grid::new();
        dialog_grid.attach(edit_overview.get_top_element(), 0, 0, 1, 1);
        dialog_grid.attach(&detail_stack, 0, 1, 1, 1);

        // Add some space between the rows and columns
        dialog_grid.set_column_spacing(10);
        dialog_grid.set_row_spacing(10);

        // Add some space on all the sides
        dialog_grid.set_margin_top(10);
        dialog_grid.set_margin_bottom(10);
        dialog_grid.set_margin_start(10);
        dialog_grid.set_margin_end(10);

        // Add the primary grid to the dialog
        let content = dialog.get_content_area();
        content.add(&dialog_grid);

        // If there is an old event pair, load it into the edit overview
        if let Some(pair) = old_pair {
            edit_overview.load_pair(pair);
        }

        // If there is an old detail, load it into the window
        if let Some(detail) = old_detail {
            // Load the first action into the window
            for action in detail {
                match action {
                    // Load the new scene variant
                    EventAction::NewScene { new_scene } => {
                        edit_new_scene.load_detail(new_scene);
                        edit_overview.choose_detail("scene")
                    }

                    // Load the modify status variant
                    EventAction::ModifyStatus {
                        status_id,
                        new_state,
                    } => {
                        edit_modify_status.load_detail(status_id, new_state);
                        edit_overview.choose_detail("status");
                    }

                    // Load the trigger event variant
                    EventAction::QueueEvent { event } => {
                        edit_trigger_events.load_detail(vec!(event));
                        edit_overview.choose_detail("events");
                    }

                    // Load the cancel events variant
                    EventAction::CancelEvent { event } => {
                        edit_cancel_events.load_detail(vec!(event));
                        edit_overview.choose_detail("cancel");
                    }

                    // Load the save data variant
                    EventAction::SaveData { data } => {
                        // FIXME edit_save_data.load_detail(data);
                        edit_overview.choose_detail("data");
                    }

                    // Ignore the send data variant FIXME
                    EventAction::SendData { data } => (),

                    // Load the grouped event variant
                    EventAction::GroupedEvent {
                        status_id,
                        event_map,
                    } => {
                        edit_grouped_event.load_detail(status_id, event_map);
                        edit_overview.choose_detail("grouped");
                    }
                }
                // FIXME
                break
            }
        }

        // Connect the close event for when the dialog is complete
        dialog.connect_response(clone!(system_send, edit_overview, edit_new_scene, edit_modify_status, edit_trigger_events, edit_cancel_events, edit_save_data, edit_grouped_event => move |modal, id| {

            // Check to see if the event edit was confirmed
            if id == gtk::ResponseType::Ok {

                // Process the information for the event overview
                let (event_pair, detail_type) = edit_overview.pack_pair();

                // Process the information for the event detail
                let event_detail = match detail_type.as_str() {

                    // Pack the new scene variant
                    "scene" => edit_new_scene.pack_detail(),

                    // Pack the modify status variant
                    "status" => edit_modify_status.pack_detail(),

                    // Pack the trigger event variant
                    "events" => edit_trigger_events.pack_detail(),

                    // Pack the cancel events variant
                    "cancel" => edit_cancel_events.pack_detail(),

                    // Pack the save data variant
                    "data" => edit_save_data.pack_detail(),

                    // Pack the grouped event variant
                    "grouped" => edit_grouped_event.pack_detail(),

                    // Should be impossible
                    _ => unreachable!(),
                };

                // Send the edited event back to the system
                system_send.send(EditDetail { event_pair, event_detail });
            }

            // Close the window either way
            modal.destroy();
        }));

        // Show the dialog and return
        dialog.show_all();
    }
}

// Create the event overview variant
//
#[derive(Clone, Debug)]
struct EditOverview {
    grid: gtk::Grid,                                     // the main grid for this element
    id_spin: gtk::SpinButton,                            // the spin selection for the event id
    description: gtk::Entry,                             // the description of the event
    display_type: gtk::ComboBoxText,                     // the display type selection for the event
    displaycontrol_position_checkbox: gtk::CheckButton,  // the position checkbox
    displaycontrol_position: gtk::SpinButton,            // the spin selection for position
    displaycontrol_color_checkbox: gtk::CheckButton,     // the color checkbox
    displaycontrol_color: gtk::ColorButton,              // the color selection button
    displaycontrol_highlight_checkbox: gtk::CheckButton, // the highlight checkbox
    displaycontrol_highlight: gtk::ColorButton,          // the highlight selection button
    displaycontrol_highstate_checkbox: gtk::CheckButton, // the highlight state checkbox
    displaycontrol_highstate_status: gtk::SpinButton,    // the highlight state status spin
    displaycontrol_highstate_state: gtk::SpinButton,     // the highlight state state spin
    displaywith_spin: gtk::SpinButton,                   // the spin selection for the group id
    displaywith_position_checkbox: gtk::CheckButton,     // the position checkbox
    displaywith_position: gtk::SpinButton,               // the spin selection for the position
    displaywith_color_checkbox: gtk::CheckButton,        // the color checkbox
    displaywith_color: gtk::ColorButton,                 // the color selection button
    displaywith_highlight_checkbox: gtk::CheckButton,    // the highlight checkbox
    displaywith_highlight: gtk::ColorButton,             // the highlight selection button
    displaywith_highstate_checkbox: gtk::CheckButton,    // the highlight state checkbox
    displaywith_highstate_status: gtk::SpinButton,       // the highlight state status spin
    displaywith_highstate_state: gtk::SpinButton,        // the highlight state state spin
    displaydebug_checkbox: gtk::CheckButton,             // the checkbox for group id
    displaydebug_spin: gtk::SpinButton,                  // the spin selection for the group id
    displaydebug_position_checkbox: gtk::CheckButton,    // the position checkbox
    displaydebug_position: gtk::SpinButton,              // the spin selection for position
    displaydebug_color_checkbox: gtk::CheckButton,       // the color checkbox
    displaydebug_color: gtk::ColorButton,                // the color selection button
    displaydebug_highlight_checkbox: gtk::CheckButton,   // the highlight checkbox
    displaydebug_highlight: gtk::ColorButton,            // the highlight selection button
    displaydebug_highstate_checkbox: gtk::CheckButton,   // the highlight state checkbox
    displaydebug_highstate_status: gtk::SpinButton,      // the highlight state status spin
    displaydebug_highstate_state: gtk::SpinButton,       // the highlight state state spin
    labelcontrol_position_checkbox: gtk::CheckButton,    // the position checkbox
    labelcontrol_position: gtk::SpinButton,              // the spin selection for position
    labelcontrol_color_checkbox: gtk::CheckButton,       // the color checkbox
    labelcontrol_color: gtk::ColorButton,                // the color selection button
    labelcontrol_highlight_checkbox: gtk::CheckButton,   // the highlight checkbox
    labelcontrol_highlight: gtk::ColorButton,            // the highlight selection button
    labelcontrol_highstate_checkbox: gtk::CheckButton,   // the highlight state checkbox
    labelcontrol_highstate_status: gtk::SpinButton,      // the highlight state status spin
    labelcontrol_highstate_state: gtk::SpinButton,       // the highlight state state spin
    labelhidden_position_checkbox: gtk::CheckButton,     // the position checkbox
    labelhidden_position: gtk::SpinButton,               // the spin selection for position
    labelhidden_color_checkbox: gtk::CheckButton,        // the color check
    labelhidden_color: gtk::ColorButton,                 // the color selection button
    labelhidden_highlight_checkbox: gtk::CheckButton,    // the highlight checkbox
    labelhidden_highlight: gtk::ColorButton,             // the highlight selection button
    detail_selection: gtk::ComboBoxText, // the detail variant selection for the event
}

impl EditOverview {
    // A function to create an edit overview
    fn new(detail_stack: &gtk::Stack) -> EditOverview {
        // Create the event number and description
        let id_label = gtk::Label::new(Some("  Event Id  "));
        let id_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        id_spin.set_size_request(100, 30);
        id_spin.set_hexpand(false);
        let description = gtk::Entry::new();
        description.set_placeholder_text(Some("Enter a description here"));

        // Add the display type dropdown
        let display_type_label = gtk::Label::new(Some("  Display Type  "));
        let display_type = gtk::ComboBoxText::new();
        display_type.append(Some("displaycontrol"), "Display Control");
        display_type.append(Some("displaywith"), "Display With");
        display_type.append(Some("displaydebug"), "Display Debug");
        display_type.append(Some("labelcontrol"), "Label Control");
        display_type.append(Some("labelhidden"), "Label Hidden");
        display_type.append(Some("hidden"), "Hidden");

        // Add the displaycontrol type position, color, and highlight items
        let displaycontrol_position_checkbox = gtk::CheckButton::new_with_label("Display Position");
        let displaycontrol_position = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let displaycontrol_color_checkbox = gtk::CheckButton::new_with_label("Custom Text Color");
        let displaycontrol_color = gtk::ColorButton::new();
        displaycontrol_color.set_title("Text Color");
        let displaycontrol_highlight_checkbox =
            gtk::CheckButton::new_with_label("Custom Text Highlight");
        let displaycontrol_highlight = gtk::ColorButton::new();
        displaycontrol_highlight.set_title("Text Highlight Color");
        let displaycontrol_highstate_checkbox =
            gtk::CheckButton::new_with_label("Status-Based Highlighting");
        let displaycontrol_highstate_status =
            gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let displaycontrol_highstate_state = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Compose the displaycontrol grid
        let displaycontrol_grid = gtk::Grid::new();
        displaycontrol_grid.attach(&displaycontrol_position_checkbox, 0, 0, 1, 1);
        displaycontrol_grid.attach(&displaycontrol_position, 1, 0, 1, 1);
        displaycontrol_grid.attach(&displaycontrol_color_checkbox, 0, 1, 1, 1);
        displaycontrol_grid.attach(&displaycontrol_color, 1, 1, 1, 1);
        displaycontrol_grid.attach(&displaycontrol_highlight_checkbox, 0, 2, 1, 1);
        displaycontrol_grid.attach(&displaycontrol_highlight, 1, 2, 1, 1);
        displaycontrol_grid.attach(&displaycontrol_highstate_checkbox, 0, 3, 1, 1);
        displaycontrol_grid.attach(&displaycontrol_highstate_status, 1, 3, 1, 1);
        displaycontrol_grid.attach(&displaycontrol_highstate_state, 2, 3, 1, 1);
        displaycontrol_grid.set_column_spacing(10); // Add some space
        displaycontrol_grid.set_row_spacing(10);
        displaycontrol_grid.show_all();

        // Add the displaywith type spin items
        let displaywith_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Add the displaywith type position and color items
        let displaywith_position_checkbox = gtk::CheckButton::new_with_label("Display Position");
        let displaywith_position = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let displaywith_color_checkbox = gtk::CheckButton::new_with_label("Custom Text Color");
        let displaywith_color = gtk::ColorButton::new();
        displaywith_color.set_title("Text Color");
        let displaywith_highlight_checkbox =
            gtk::CheckButton::new_with_label("Custom Text Highlight");
        let displaywith_highlight = gtk::ColorButton::new();
        displaywith_highlight.set_title("Text Highlight Color");
        let displaywith_highstate_checkbox =
            gtk::CheckButton::new_with_label("Status-Based Highlighting");
        let displaywith_highstate_status = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let displaywith_highstate_state = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Compose the displaywith grid
        let displaywith_grid = gtk::Grid::new();
        displaywith_grid.attach(&displaywith_spin, 0, 0, 1, 1);
        displaywith_grid.attach(&displaywith_position_checkbox, 0, 1, 1, 1);
        displaywith_grid.attach(&displaywith_position, 1, 1, 1, 1);
        displaywith_grid.attach(&displaywith_color_checkbox, 0, 2, 1, 1);
        displaywith_grid.attach(&displaywith_color, 1, 2, 1, 1);
        displaywith_grid.attach(&displaywith_highlight_checkbox, 0, 3, 1, 1);
        displaywith_grid.attach(&displaywith_highlight, 1, 3, 1, 1);
        displaywith_grid.attach(&displaywith_highstate_checkbox, 0, 4, 1, 1);
        displaywith_grid.attach(&displaywith_highstate_status, 1, 4, 1, 1);
        displaywith_grid.attach(&displaywith_highstate_state, 2, 4, 1, 1);
        displaywith_grid.set_column_spacing(10); // Add some space
        displaywith_grid.set_row_spacing(10);
        displaywith_grid.show_all();

        // Add the displaydebug type spin items
        let displaydebug_checkbox = gtk::CheckButton::new_with_label("Display With Group");
        let displaydebug_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Add the displaydebug type position and color items
        let displaydebug_position_checkbox = gtk::CheckButton::new_with_label("Display Position");
        let displaydebug_position = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let displaydebug_color_checkbox = gtk::CheckButton::new_with_label("Custom Text Color");
        let displaydebug_color = gtk::ColorButton::new();
        displaydebug_color.set_title("Text Color");
        let displaydebug_highlight_checkbox =
            gtk::CheckButton::new_with_label("Custom Text Highlight");
        let displaydebug_highlight = gtk::ColorButton::new();
        displaydebug_highlight.set_title("Text Highlight Color");
        let displaydebug_highstate_checkbox =
            gtk::CheckButton::new_with_label("Status-Based Highlighting");
        let displaydebug_highstate_status = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let displaydebug_highstate_state = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Compose the displaydebug grid
        let displaydebug_grid = gtk::Grid::new();
        displaydebug_grid.attach(&displaydebug_checkbox, 0, 0, 1, 1);
        displaydebug_grid.attach(&displaydebug_spin, 1, 0, 1, 1);
        displaydebug_grid.attach(&displaydebug_position_checkbox, 0, 1, 1, 1);
        displaydebug_grid.attach(&displaydebug_position, 1, 1, 1, 1);
        displaydebug_grid.attach(&displaydebug_color_checkbox, 0, 2, 1, 1);
        displaydebug_grid.attach(&displaydebug_color, 1, 2, 1, 1);
        displaydebug_grid.attach(&displaydebug_highlight_checkbox, 0, 3, 1, 1);
        displaydebug_grid.attach(&displaydebug_highlight, 1, 3, 1, 1);
        displaydebug_grid.attach(&displaydebug_highstate_checkbox, 0, 4, 1, 1);
        displaydebug_grid.attach(&displaydebug_highstate_status, 1, 4, 1, 1);
        displaydebug_grid.attach(&displaydebug_highstate_state, 2, 4, 1, 1);
        displaydebug_grid.set_column_spacing(10); // Add some space
        displaydebug_grid.set_row_spacing(10);
        displaydebug_grid.show_all();

        // Add the labelcontrol type position, color, and highlight items
        let labelcontrol_position_checkbox = gtk::CheckButton::new_with_label("Display Position");
        let labelcontrol_position = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let labelcontrol_color_checkbox = gtk::CheckButton::new_with_label("Custom Text Color");
        let labelcontrol_color = gtk::ColorButton::new();
        labelcontrol_color.set_title("Text Color");
        let labelcontrol_highlight_checkbox =
            gtk::CheckButton::new_with_label("Custom Text Highlight");
        let labelcontrol_highlight = gtk::ColorButton::new();
        labelcontrol_highlight.set_title("Text Highlight Color");
        let labelcontrol_highstate_checkbox =
            gtk::CheckButton::new_with_label("Status-Based Highlighting");
        let labelcontrol_highstate_status = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let labelcontrol_highstate_state = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Compose the labelcontrol grid
        let labelcontrol_grid = gtk::Grid::new();
        labelcontrol_grid.attach(&labelcontrol_position_checkbox, 0, 0, 1, 1);
        labelcontrol_grid.attach(&labelcontrol_position, 1, 0, 1, 1);
        labelcontrol_grid.attach(&labelcontrol_color_checkbox, 0, 1, 1, 1);
        labelcontrol_grid.attach(&labelcontrol_color, 1, 1, 1, 1);
        labelcontrol_grid.attach(&labelcontrol_highlight_checkbox, 0, 2, 1, 1);
        labelcontrol_grid.attach(&labelcontrol_highlight, 1, 2, 1, 1);
        labelcontrol_grid.attach(&labelcontrol_highstate_checkbox, 0, 3, 1, 1);
        labelcontrol_grid.attach(&labelcontrol_highstate_status, 1, 3, 1, 1);
        labelcontrol_grid.attach(&labelcontrol_highstate_state, 2, 3, 1, 1);
        labelcontrol_grid.set_column_spacing(10); // Add some space
        labelcontrol_grid.set_row_spacing(10);
        labelcontrol_grid.show_all();

        // Add the labelhidden position, color, and highlight items
        let labelhidden_position_checkbox = gtk::CheckButton::new_with_label("Display Position");
        let labelhidden_position = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let labelhidden_color_checkbox = gtk::CheckButton::new_with_label("Custom Text Color");
        let labelhidden_color = gtk::ColorButton::new();
        labelhidden_color.set_title("Text Color");
        labelhidden_color.set_size_request(140, 30);
        let labelhidden_highlight_checkbox =
            gtk::CheckButton::new_with_label("Custom Text Highlight");
        let labelhidden_highlight = gtk::ColorButton::new();
        labelhidden_highlight.set_title("Text Highlight Color");
        labelhidden_highlight.set_size_request(140, 30);

        // Compose the labelhidden grid
        let labelhidden_grid = gtk::Grid::new();
        labelhidden_grid.attach(&labelhidden_position_checkbox, 0, 0, 1, 1);
        labelhidden_grid.attach(&labelhidden_position, 1, 0, 1, 1);
        labelhidden_grid.attach(&labelhidden_color_checkbox, 0, 1, 1, 1);
        labelhidden_grid.attach(&labelhidden_color, 1, 1, 1, 1);
        labelhidden_grid.attach(&labelhidden_highlight_checkbox, 0, 2, 1, 1);
        labelhidden_grid.attach(&labelhidden_highlight, 1, 2, 1, 1);

        // Fill the display type stack
        let display_stack = gtk::Stack::new();
        display_stack.add_named(&displaycontrol_grid, "displaycontrol");
        display_stack.add_named(&displaywith_grid, "displaywith");
        display_stack.add_named(&displaydebug_grid, "displaydebug");
        display_stack.add_named(&labelcontrol_grid, "labelcontrol");
        display_stack.add_named(&labelhidden_grid, "labelhidden");
        let blank_label = gtk::Label::new(None);
        display_stack.add_named(&blank_label, "hidden");
        blank_label.show();

        // Connect the function to trigger display type changes
        display_type.connect_changed(clone!(display_stack => move |dropdown| {

            // Identify the selected detail type
            if let Some(detail_str) = dropdown.get_active_id() {

                // Change the dialog stack to the requested variation
                display_stack.set_visible_child_full(&detail_str, gtk::StackTransitionType::SlideDown);
            }
        }));

        // Create the event detail selection dropdown
        let detail_selection = gtk::ComboBoxText::new();

        // Add each of the available detail types to the dropdown
        detail_selection.append(Some("scene"), "New Scene");
        detail_selection.append(Some("status"), "Modify Status");
        detail_selection.append(Some("events"), "Trigger Events");
        detail_selection.append(Some("cancel"), "Cancel Events");
        detail_selection.append(Some("data"), "Save Data");
        detail_selection.append(Some("grouped"), "Grouped Event");

        // Connect the function to trigger detail selection changes
        detail_selection.connect_changed(clone!(detail_stack => move |dropdown| {

            // Identify the selected detail type
            if let Some(detail_str) = dropdown.get_active_id() {

                // Change the dialog stack to the requested variation
                detail_stack.set_visible_child_full(&detail_str, gtk::StackTransitionType::SlideDown);
            }
        }));

        // Create the edit overview grid and populate it
        let overview_grid = gtk::Grid::new();
        overview_grid.attach(&id_label, 0, 0, 2, 1);
        overview_grid.attach(&id_spin, 2, 0, 1, 1);
        overview_grid.attach(&description, 0, 1, 3, 1);
        overview_grid.attach(&display_type_label, 0, 2, 1, 1);
        overview_grid.attach(&display_type, 1, 2, 1, 1);
        overview_grid.attach(&display_stack, 2, 2, 1, 1);
        overview_grid.attach(&detail_selection, 0, 3, 3, 1);
        overview_grid.set_column_spacing(10); // Add some space
        overview_grid.set_row_spacing(10);

        // Create and return the edit overview
        overview_grid.show_all();
        EditOverview {
            grid: overview_grid,
            id_spin,
            description,
            display_type,
            displaycontrol_position_checkbox,
            displaycontrol_position,
            displaycontrol_color_checkbox,
            displaycontrol_color,
            displaycontrol_highlight_checkbox,
            displaycontrol_highlight,
            displaycontrol_highstate_checkbox,
            displaycontrol_highstate_status,
            displaycontrol_highstate_state,
            displaywith_spin,
            displaywith_position_checkbox,
            displaywith_position,
            displaywith_color_checkbox,
            displaywith_color,
            displaywith_highlight_checkbox,
            displaywith_highlight,
            displaywith_highstate_checkbox,
            displaywith_highstate_status,
            displaywith_highstate_state,
            displaydebug_checkbox,
            displaydebug_spin,
            displaydebug_position_checkbox,
            displaydebug_position,
            displaydebug_color_checkbox,
            displaydebug_color,
            displaydebug_highlight_checkbox,
            displaydebug_highlight,
            displaydebug_highstate_checkbox,
            displaydebug_highstate_status,
            displaydebug_highstate_state,
            labelcontrol_position_checkbox,
            labelcontrol_position,
            labelcontrol_color_checkbox,
            labelcontrol_color,
            labelcontrol_highlight_checkbox,
            labelcontrol_highlight,
            labelcontrol_highstate_checkbox,
            labelcontrol_highstate_status,
            labelcontrol_highstate_state,
            labelhidden_position_checkbox,
            labelhidden_position,
            labelhidden_color_checkbox,
            labelhidden_color,
            labelhidden_highlight_checkbox,
            labelhidden_highlight,
            detail_selection,
        }
    }

    // A function to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A function to load an event pair into the edit overview
    //
    fn load_pair(&self, pair: ItemPair) {
        // Update the id of the event
        self.id_spin.set_value(pair.id() as f64);

        // Update the event description
        self.description.set_text(pair.description().as_str());

        // Update the display type for the event
        match pair.display {
            // the displaycontrol variant
            DisplayControl {
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            } => {
                // Switch to the displaycontrol type
                self.display_type.set_active_id(Some("displaycontrol"));

                // If there is a position, set it
                match position {
                    None => self.displaycontrol_position_checkbox.set_active(false),
                    Some(number) => {
                        self.displaycontrol_position_checkbox.set_active(true);
                        self.displaycontrol_position.set_value(number as f64);
                    }
                }

                // If there is a color, set it
                match color {
                    None => self.displaycontrol_color_checkbox.set_active(false),
                    Some((new_red, new_green, new_blue)) => {
                        self.displaycontrol_color_checkbox.set_active(true);
                        let new_color = gdk::RGBA {
                            red: new_red as f64 / 255.0,
                            green: new_green as f64 / 255.0,
                            blue: new_blue as f64 / 255.0,
                            alpha: 1.0,
                        };
                        self.displaycontrol_color.set_rgba(&new_color);
                    }
                }

                // If there is a highlight, set it
                match highlight {
                    None => self.displaycontrol_highlight_checkbox.set_active(false),
                    Some((new_red, new_green, new_blue)) => {
                        self.displaycontrol_highlight_checkbox.set_active(true);
                        let new_color = gdk::RGBA {
                            red: new_red as f64 / 255.0,
                            green: new_green as f64 / 255.0,
                            blue: new_blue as f64 / 255.0,
                            alpha: 1.0,
                        };
                        self.displaycontrol_highlight.set_rgba(&new_color);
                    }
                }

                // If there is a highlight state, set it
                match highlight_state {
                    None => self.displaycontrol_highstate_checkbox.set_active(false),
                    Some((new_status, new_state)) => {
                        self.displaycontrol_highstate_checkbox.set_active(true);
                        self.displaycontrol_highstate_status
                            .set_value(new_status.id() as f64);
                        self.displaycontrol_highstate_state
                            .set_value(new_state.id() as f64);
                    }
                }
            }

            // the displaywith variant
            DisplayWith {
                group_id,
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            } => {
                // Switch to the displaywith type and set the group id
                self.display_type.set_active_id(Some("displaywith"));
                self.displaywith_spin.set_value(group_id.id() as f64);

                // If there is a position, set it
                match position {
                    None => self.displaywith_position_checkbox.set_active(false),
                    Some(number) => {
                        self.displaywith_position_checkbox.set_active(true);
                        self.displaywith_position.set_value(number as f64);
                    }
                }

                // If there is a color, set it
                match color {
                    None => self.displaywith_color_checkbox.set_active(false),
                    Some((new_red, new_green, new_blue)) => {
                        self.displaywith_color_checkbox.set_active(true);
                        let new_color = gdk::RGBA {
                            red: new_red as f64 / 255.0,
                            green: new_green as f64 / 255.0,
                            blue: new_blue as f64 / 255.0,
                            alpha: 1.0,
                        };
                        self.displaywith_color.set_rgba(&new_color);
                    }
                }

                // If there is a highlight, set it
                match highlight {
                    None => self.displaywith_highlight_checkbox.set_active(false),
                    Some((new_red, new_green, new_blue)) => {
                        self.displaywith_highlight_checkbox.set_active(true);
                        let new_color = gdk::RGBA {
                            red: new_red as f64 / 255.0,
                            green: new_green as f64 / 255.0,
                            blue: new_blue as f64 / 255.0,
                            alpha: 1.0,
                        };
                        self.displaywith_highlight.set_rgba(&new_color);
                    }
                }

                // If there is a highlight state, set it
                match highlight_state {
                    None => self.displaywith_highstate_checkbox.set_active(false),
                    Some((new_status, new_state)) => {
                        self.displaywith_highstate_checkbox.set_active(true);
                        self.displaywith_highstate_status
                            .set_value(new_status.id() as f64);
                        self.displaywith_highstate_state
                            .set_value(new_state.id() as f64);
                    }
                }
            }

            // the displaydebug variant
            DisplayDebug {
                group_id,
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            } => {
                // Switch to the displaydebug type
                self.display_type.set_active_id(Some("displaydebug"));

                // If theere is a group id, set it
                match group_id {
                    None => self.displaydebug_checkbox.set_active(false),
                    Some(id) => {
                        self.displaydebug_checkbox.set_active(true);
                        self.displaydebug_spin.set_value(id.id() as f64);
                    }
                }

                // If there is a position, set it
                match position {
                    None => self.displaydebug_position_checkbox.set_active(false),
                    Some(number) => {
                        self.displaydebug_position_checkbox.set_active(true);
                        self.displaydebug_position.set_value(number as f64);
                    }
                }

                // If there is a color, set it
                match color {
                    None => self.displaydebug_color_checkbox.set_active(false),
                    Some((new_red, new_green, new_blue)) => {
                        self.displaydebug_color_checkbox.set_active(true);
                        let new_color = gdk::RGBA {
                            red: new_red as f64 / 255.0,
                            green: new_green as f64 / 255.0,
                            blue: new_blue as f64 / 255.0,
                            alpha: 1.0,
                        };
                        self.displaydebug_color.set_rgba(&new_color);
                    }
                }

                // If there is a highlight, set it
                match highlight {
                    None => self.displaydebug_highlight_checkbox.set_active(false),
                    Some((new_red, new_green, new_blue)) => {
                        self.displaydebug_highlight_checkbox.set_active(true);
                        let new_color = gdk::RGBA {
                            red: new_red as f64 / 255.0,
                            green: new_green as f64 / 255.0,
                            blue: new_blue as f64 / 255.0,
                            alpha: 1.0,
                        };
                        self.displaydebug_highlight.set_rgba(&new_color);
                    }
                }

                // If there is a highlight state, set it
                match highlight_state {
                    None => self.displaydebug_highstate_checkbox.set_active(false),
                    Some((new_status, new_state)) => {
                        self.displaydebug_highstate_checkbox.set_active(true);
                        self.displaydebug_highstate_status
                            .set_value(new_status.id() as f64);
                        self.displaydebug_highstate_state
                            .set_value(new_state.id() as f64);
                    }
                }
            }

            // the labelcontrol variant
            LabelControl {
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            } => {
                // Switch to the labelcontrol type
                self.display_type.set_active_id(Some("labelcontrol"));

                // If there is a position, set it
                match position {
                    None => self.labelcontrol_position_checkbox.set_active(false),
                    Some(number) => {
                        self.labelcontrol_position_checkbox.set_active(true);
                        self.labelcontrol_position.set_value(number as f64);
                    }
                }

                // If there is a color, set it
                match color {
                    None => self.labelcontrol_color_checkbox.set_active(false),
                    Some((new_red, new_green, new_blue)) => {
                        self.labelcontrol_color_checkbox.set_active(true);
                        let new_color = gdk::RGBA {
                            red: new_red as f64 / 255.0,
                            green: new_green as f64 / 255.0,
                            blue: new_blue as f64 / 255.0,
                            alpha: 1.0,
                        };
                        self.labelcontrol_color.set_rgba(&new_color);
                    }
                }

                // If there is a highlight, set it
                match highlight {
                    None => self.labelcontrol_highlight_checkbox.set_active(false),
                    Some((new_red, new_green, new_blue)) => {
                        self.labelcontrol_highlight_checkbox.set_active(true);
                        let new_color = gdk::RGBA {
                            red: new_red as f64 / 255.0,
                            green: new_green as f64 / 255.0,
                            blue: new_blue as f64 / 255.0,
                            alpha: 1.0,
                        };
                        self.labelcontrol_highlight.set_rgba(&new_color);
                    }
                }

                // If there is a highlight state, set it
                match highlight_state {
                    None => self.labelcontrol_highstate_checkbox.set_active(false),
                    Some((new_status, new_state)) => {
                        self.labelcontrol_highstate_checkbox.set_active(true);
                        self.labelcontrol_highstate_status
                            .set_value(new_status.id() as f64);
                        self.labelcontrol_highstate_state
                            .set_value(new_state.id() as f64);
                    }
                }
            }

            // the label hidden variant
            LabelHidden {
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            } => {
                //Switch to the labelhidden type
                self.display_type.set_active_id(Some("labelhidden"));

                // If there is a position, set it
                match position {
                    None => self.labelhidden_position_checkbox.set_active(false),
                    Some(number) => {
                        self.labelhidden_position_checkbox.set_active(true);
                        self.labelhidden_position.set_value(number as f64);
                    }
                }

                // If there is a color, set it
                match color {
                    None => self.labelhidden_color_checkbox.set_active(false),
                    Some((new_red, new_green, new_blue)) => {
                        self.labelhidden_color_checkbox.set_active(true);
                        let new_color = gdk::RGBA {
                            red: new_red as f64 / 255.0,
                            green: new_green as f64 / 255.0,
                            blue: new_blue as f64 / 255.0,
                            alpha: 1.0,
                        };
                        self.labelhidden_color.set_rgba(&new_color);
                    }
                }

                // If there is a highlight, set it
                match highlight {
                    None => self.labelhidden_highlight_checkbox.set_active(false),
                    Some((new_red, new_green, new_blue)) => {
                        self.labelhidden_highlight_checkbox.set_active(true);
                        let new_color = gdk::RGBA {
                            red: new_red as f64 / 255.0,
                            green: new_green as f64 / 255.0,
                            blue: new_blue as f64 / 255.0,
                            alpha: 1.0,
                        };
                        self.labelhidden_highlight.set_rgba(&new_color);
                    }
                }
            }

            // the hidden variant
            Hidden => {
                self.display_type.set_active_id(Some("hidden"));
            }
        }
    }

    // A function to switch to the requested detail for the event
    //
    fn choose_detail(&self, detail: &str) {
        // Pass the chosen detail to the detail selection
        self.detail_selection.set_active_id(Some(detail));
    }

    // A function to pack the event pair into an item pair
    //
    fn pack_pair(&self) -> (ItemPair, String) {
        // Create the new item id (allows unchecked creation, unlike item pair)
        let event_id = ItemId::new_unchecked(self.id_spin.get_value() as u32);

        // Create the new item description
        let tmp_desc = self.description.get_text().unwrap_or(String::new().into());
        let tmp_disp_id = self
            .display_type
            .get_active_id()
            .unwrap_or(String::from("hidden").into());
        let tmp_disp = match tmp_disp_id.as_str() {
            // For the displaycontrol type
            "displaycontrol" => {
                // Extract the position, if selected
                let mut position = None;
                if self.displaycontrol_position_checkbox.get_active() {
                    position = Some(self.displaycontrol_position.get_value() as u32);
                }

                // Extract the color, if selected
                let mut color = None;
                if self.displaycontrol_color_checkbox.get_active() {
                    let gdk::RGBA {
                        red, green, blue, ..
                    } = self.displaycontrol_color.get_rgba();
                    color = Some((
                        (red * 255.0) as u8,
                        (green * 255.0) as u8,
                        (blue * 255.0) as u8,
                    ));
                }

                // Extract the highlight, if selected
                let mut highlight = None;
                if self.displaycontrol_highlight_checkbox.get_active() {
                    let gdk::RGBA {
                        red, green, blue, ..
                    } = self.displaycontrol_highlight.get_rgba();
                    highlight = Some((
                        (red * 255.0) as u8,
                        (green * 255.0) as u8,
                        (blue * 255.0) as u8,
                    ));
                }

                // Extract the highlight state, if selected
                let mut highlight_state = None;
                if self.displaycontrol_highstate_checkbox.get_active() {
                    highlight_state = Some((
                        ItemId::new_unchecked(
                            self.displaycontrol_highstate_status.get_value() as u32
                        ),
                        ItemId::new_unchecked(
                            self.displaycontrol_highstate_state.get_value() as u32
                        ),
                    ));
                }

                // Return the completed display type
                DisplayControl {
                    position,
                    color,
                    highlight,
                    highlight_state,
                    spotlight: None, // FIXME
                }
            }

            // For the displaywith type
            "displaywith" => {
                // Extract the position, if selected
                let mut position = None;
                if self.displaywith_position_checkbox.get_active() {
                    position = Some(self.displaywith_position.get_value() as u32);
                }

                // Extract the color, if selected
                let mut color = None;
                if self.displaywith_color_checkbox.get_active() {
                    let gdk::RGBA {
                        red, green, blue, ..
                    } = self.displaywith_color.get_rgba();
                    color = Some((
                        (red * 255.0) as u8,
                        (green * 255.0) as u8,
                        (blue * 255.0) as u8,
                    ));
                }

                // Extract the highlight, if selected
                let mut highlight = None;
                if self.displaywith_highlight_checkbox.get_active() {
                    let gdk::RGBA {
                        red, green, blue, ..
                    } = self.displaywith_highlight.get_rgba();
                    highlight = Some((
                        (red * 255.0) as u8,
                        (green * 255.0) as u8,
                        (blue * 255.0) as u8,
                    ));
                }

                // Extract the highlight state, if selected
                let mut highlight_state = None;
                if self.displaywith_highstate_checkbox.get_active() {
                    highlight_state = Some((
                        ItemId::new_unchecked(self.displaywith_highstate_status.get_value() as u32),
                        ItemId::new_unchecked(self.displaywith_highstate_state.get_value() as u32),
                    ));
                }

                // Return the completed display type
                DisplayWith {
                    group_id: ItemId::new_unchecked(self.displaywith_spin.get_value() as u32),
                    position,
                    color,
                    highlight,
                    highlight_state,
                    spotlight: None, // FIXME
                }
            }

            // For the displaydebug type
            "displaydebug" => {
                // Extract the group id, if selected
                let mut group_id = None;
                if self.displaydebug_position_checkbox.get_active() {
                    group_id = Some(ItemId::new_unchecked(
                        self.displaydebug_spin.get_value() as u32
                    ));
                }

                // Extract the position, if selected
                let mut position = None;
                if self.displaydebug_position_checkbox.get_active() {
                    position = Some(self.displaydebug_position.get_value() as u32);
                }

                // Extract the color, if selected
                let mut color = None;
                if self.displaydebug_color_checkbox.get_active() {
                    let gdk::RGBA {
                        red, green, blue, ..
                    } = self.displaydebug_color.get_rgba();
                    color = Some((
                        (red * 255.0) as u8,
                        (green * 255.0) as u8,
                        (blue * 255.0) as u8,
                    ));
                }

                // Extract the highlight, if selected
                let mut highlight = None;
                if self.displaydebug_highlight_checkbox.get_active() {
                    let gdk::RGBA {
                        red, green, blue, ..
                    } = self.displaydebug_highlight.get_rgba();
                    highlight = Some((
                        (red * 255.0) as u8,
                        (green * 255.0) as u8,
                        (blue * 255.0) as u8,
                    ));
                }

                // Extract the highlight state, if selected
                let mut highlight_state = None;
                if self.displaydebug_highstate_checkbox.get_active() {
                    highlight_state = Some((
                        ItemId::new_unchecked(self.displaydebug_highstate_status.get_value() as u32),
                        ItemId::new_unchecked(self.displaydebug_highstate_state.get_value() as u32),
                    ));
                }

                // Return the completed display type
                DisplayDebug {
                    group_id,
                    position,
                    color,
                    highlight,
                    highlight_state,
                    spotlight: None, // FIXME
                }
            }

            // For the labelcontrol type
            "labelcontrol" => {
                // Extract the position, if selected
                let mut position = None;
                if self.labelcontrol_position_checkbox.get_active() {
                    position = Some(self.labelcontrol_position.get_value() as u32);
                }

                // Extract the color, if selected
                let mut color = None;
                if self.labelcontrol_color_checkbox.get_active() {
                    let gdk::RGBA {
                        red, green, blue, ..
                    } = self.labelcontrol_color.get_rgba();
                    color = Some((
                        (red * 255.0) as u8,
                        (green * 255.0) as u8,
                        (blue * 255.0) as u8,
                    ));
                }

                // Extract the highlight, if selected
                let mut highlight = None;
                if self.labelcontrol_highlight_checkbox.get_active() {
                    let gdk::RGBA {
                        red, green, blue, ..
                    } = self.labelcontrol_highlight.get_rgba();
                    highlight = Some((
                        (red * 255.0) as u8,
                        (green * 255.0) as u8,
                        (blue * 255.0) as u8,
                    ));
                }

                // Extract the highlight state, if selected
                let mut highlight_state = None;
                if self.labelcontrol_highstate_checkbox.get_active() {
                    highlight_state = Some((
                        ItemId::new_unchecked(self.labelcontrol_highstate_status.get_value() as u32),
                        ItemId::new_unchecked(self.labelcontrol_highstate_state.get_value() as u32),
                    ));
                }

                // Return the completed display type
                LabelControl {
                    position,
                    color,
                    highlight,
                    highlight_state,
                    spotlight: None, // FIXME
                }
            }

            // For the labelhidden type
            "labelhidden" => {
                // Extract the position, if selected
                let mut position = None;
                if self.labelhidden_position_checkbox.get_active() {
                    position = Some(self.labelhidden_position.get_value() as u32);
                }

                // Extract the color, if selected
                let mut color = None;
                if self.labelhidden_color_checkbox.get_active() {
                    let gdk::RGBA {
                        red, green, blue, ..
                    } = self.labelhidden_color.get_rgba();
                    color = Some((
                        (red * 255.0) as u8,
                        (green * 255.0) as u8,
                        (blue * 255.0) as u8,
                    ));
                }

                // Extract the highlight, if selected
                let mut highlight = None;
                if self.labelhidden_highlight_checkbox.get_active() {
                    let gdk::RGBA {
                        red, green, blue, ..
                    } = self.labelhidden_highlight.get_rgba();
                    highlight = Some((
                        (red * 255.0) as u8,
                        (green * 255.0) as u8,
                        (blue * 255.0) as u8,
                    ));
                }

                // Return the completed display type
                LabelHidden {
                    position,
                    color,
                    highlight,
                    highlight_state: None, // FIXME need additional parameters for this
                    spotlight: None, // FIXME
                }
            }

            // For the hidden type
            "hidden" => Hidden,
            _ => unreachable!(),
        };
        let event_description = ItemDescription::new(&tmp_desc, tmp_disp);

        // Create the pair from the item
        (
            ItemPair::from_item(event_id, event_description),
            self.detail_selection
                .get_active_id()
                .unwrap_or(String::from("scene").into())
                .into(),
            // TODO: Conversion to and then from GString won't be necessary
            // once GStrings are implemented properly.
        )
    }
}

// Create the new scene variant
//
#[derive(Clone, Debug)]
struct EditNewScene {
    grid: gtk::Grid,                 // the main grid for this element
    new_scene_spin: gtk::SpinButton, // the spin button for the new scene id
}

impl EditNewScene {
    // A function to create a new scene variant
    //
    fn new() -> EditNewScene {
        // Create the grid for the new scene variant
        let new_scene_grid = gtk::Grid::new();

        // Add a label and spin to the new scene grid
        let new_scene_label = gtk::Label::new(Some("Scene Id"));
        new_scene_label.set_size_request(80, 30);
        new_scene_label.set_hexpand(false);
        new_scene_label.set_vexpand(false);
        let new_scene_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        new_scene_spin.set_size_request(100, 30);
        new_scene_spin.set_hexpand(false);
        new_scene_grid.attach(&new_scene_label, 0, 0, 1, 1);
        new_scene_grid.attach(&new_scene_spin, 1, 0, 1, 1);
        new_scene_grid.set_column_spacing(10); // Add some space
        new_scene_grid.set_row_spacing(10);

        // Create and return the EditNewscene variant
        new_scene_grid.show_all();
        EditNewScene {
            grid: new_scene_grid,
            new_scene_spin,
        }
    }

    // A function to return the top element of the new scene variant
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A function to load an event detail into the new scene variant
    //
    fn load_detail(&self, new_scene: ItemId) {
        self.new_scene_spin.set_value(new_scene.id() as f64);
    }

    // A function to pack and return the event detail
    //
    fn pack_detail(&self) -> EventDetail {
        // Pack the new scene id into a detail FIXME
        vec!(EventAction::NewScene {
            new_scene: ItemId::new_unchecked(self.new_scene_spin.get_value() as u32),
        })
    }
}

// Create the modify status variant
//
#[derive(Clone, Debug)]
struct EditModifyStatus {
    grid: gtk::Grid,              // the main grid for this element
    status_spin: gtk::SpinButton, // the status spin button
    state_spin: gtk::SpinButton,  // the state spin button
}

impl EditModifyStatus {
    // A function to ceate a modify status variant
    //
    fn new() -> EditModifyStatus {
        // Create the grid for the modify status variant
        let modify_status_grid = gtk::Grid::new();

        // Add a labels and spins to the modify status grid
        let status_id_label = gtk::Label::new(Some("Status Id"));
        status_id_label.set_size_request(80, 30);
        status_id_label.set_hexpand(false);
        status_id_label.set_vexpand(false);
        let status_id_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        status_id_spin.set_size_request(100, 30);
        status_id_spin.set_hexpand(false);
        let state_id_label = gtk::Label::new(Some("State Id"));
        state_id_label.set_size_request(80, 30);
        state_id_label.set_hexpand(false);
        state_id_label.set_vexpand(false);
        let state_id_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        state_id_spin.set_size_request(100, 30);
        state_id_spin.set_hexpand(false);

        // Place everything into the grid
        modify_status_grid.attach(&status_id_label, 0, 0, 1, 1);
        modify_status_grid.attach(&status_id_spin, 1, 0, 1, 1);
        modify_status_grid.attach(&state_id_label, 0, 1, 1, 1);
        modify_status_grid.attach(&state_id_spin, 1, 1, 1, 1);
        modify_status_grid.set_column_spacing(10); // Add some space
        modify_status_grid.set_row_spacing(10);

        // Create and return the EditModifyStatus variant
        modify_status_grid.show_all();
        EditModifyStatus {
            grid: modify_status_grid,
            status_spin: status_id_spin,
            state_spin: state_id_spin,
        }
    }

    // A function to return the top element of the modify status variant
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A function to load an event detail into the modify status variant
    //
    fn load_detail(&self, status_id: ItemId, new_state: ItemId) {
        self.status_spin.set_value(status_id.id() as f64);
        self.state_spin.set_value(new_state.id() as f64);
    }

    // A function to pack and return the event detail
    //
    fn pack_detail(&self) -> EventDetail {
        // Pack the new scene id into a detail FIXME
        vec!(EventAction::ModifyStatus {
            status_id: ItemId::new_unchecked(self.status_spin.get_value() as u32),
            new_state: ItemId::new_unchecked(self.state_spin.get_value() as u32),
        })
    }
}

// Create the queue event variant
//
#[derive(Clone, Debug)]
struct EditQueueEvent {
    grid: gtk::Grid,                  // the main grid for this element
    trigger_event_list: gtk::ListBox, // the list for events in this variant
}

impl EditQueueEvent {
    // A function to ceate a trigger events variant
    //
    fn new() -> EditQueueEvent {
        // Create the list for the trigger events variant
        let trigger_event_list = gtk::ListBox::new();
        trigger_event_list.set_selection_mode(gtk::SelectionMode::None);

        // Create a button to add events to the list
        let add_button = gtk::Button::new_from_icon_name(
            Some("list-add-symbolic"),
            gtk::IconSize::Button.into(),
        );
        add_button.connect_clicked(clone!(trigger_event_list => move |_| {

            // Add an event to the list
            EditQueueEvent::add_event(&trigger_event_list, None);
        }));

        // Create the scrollable window for the list
        let event_window = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        event_window.add(&trigger_event_list);
        event_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        event_window.set_hexpand(false);
        event_window.set_vexpand(true);
        event_window.set_valign(gtk::Align::Fill);

        // Add the button below the data list
        let trigger_event_grid = gtk::Grid::new();
        trigger_event_grid.attach(&event_window, 0, 0, 1, 1);
        trigger_event_grid.attach(&add_button, 0, 1, 1, 1);
        trigger_event_grid.set_column_spacing(10); // Add some space
        trigger_event_grid.set_row_spacing(10);

        // Create and return the trigger events variant
        trigger_event_grid.show_all();
        EditQueueEvent {
            grid: trigger_event_grid,
            trigger_event_list,
        }
    }

    // A function to return the top element of the trigger events variant
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A function to load an event detail into the trigger events variant
    //
    fn load_detail(&self, events: Vec<EventDelay>) {
        // Add each event to the list
        for event in events {
            EditQueueEvent::add_event(&self.trigger_event_list, Some(event));
        }
    }

    // A helper function to add an event to the data list
    //
    fn add_event(trigger_event_list: &gtk::ListBox, event: Option<EventDelay>) {
        // Create an empty spin box for the list
        let event_grid = gtk::Grid::new();
        let event_label = gtk::Label::new(Some("Event"));
        event_label.set_size_request(80, 30);
        event_label.set_hexpand(false);
        event_label.set_vexpand(false);
        let event_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        event_spin.set_size_request(100, 30);
        event_spin.set_hexpand(false);

        // Add spin adjustments for the event delay minutes and seconds
        let minutes_label = gtk::Label::new(Some("  Delay: Minutes  "));
        minutes_label.set_size_request(120, 30);
        minutes_label.set_hexpand(false);
        minutes_label.set_vexpand(false);
        let minutes = gtk::SpinButton::new_with_range(0.0, MINUTES_LIMIT, 1.0);
        minutes.set_size_request(100, 30);
        minutes.set_hexpand(false);
        let seconds_label = gtk::Label::new(Some("  Seconds  "));
        seconds_label.set_size_request(80, 30);
        seconds_label.set_hexpand(false);
        seconds_label.set_vexpand(false);
        let seconds = gtk::SpinButton::new_with_range(0.0, 60.0, 1.0);
        seconds.set_size_request(100, 30);
        seconds.set_hexpand(false);

        // Add a button to delete the item from the list
        let delete_button = gtk::Button::new_from_icon_name(
            Some("edit-delete-symbolic"),
            gtk::IconSize::Button.into(),
        );
        delete_button.connect_clicked(clone!(trigger_event_list, event_grid => move |_| {
            if let Some(widget) = event_grid.get_parent() {
                trigger_event_list.remove(&widget);
            }
        }));

        // Add all the components to the event grid
        event_grid.attach(&event_label, 0, 0, 1, 1);
        event_grid.attach(&event_spin, 1, 0, 1, 1);
        event_grid.attach(&minutes_label, 2, 0, 1, 1);
        event_grid.attach(&minutes, 3, 0, 1, 1);
        event_grid.attach(&seconds_label, 4, 0, 1, 1);
        event_grid.attach(&seconds, 5, 0, 1, 1);
        event_grid.attach(&delete_button, 6, 0, 1, 1);
        event_grid.set_column_spacing(10); // Add some space
        event_grid.set_row_spacing(10);

        // Set the value of the event delay if it was provided
        if let Some(event_delay) = event {
            event_spin.set_value(event_delay.id().id() as f64);

            // Calculate the minutes and seconds of the duration
            if let Some(delay) = event_delay.delay() {
                // May be and empty delay
                let time = delay.as_secs();
                let remainder = time % 60;
                minutes.set_value(((time - remainder) / 60) as f64);
                seconds.set_value(remainder as f64);
            }
        }

        // Add the new grid to the list
        event_grid.show_all();
        trigger_event_list.add(&event_grid);
    }

    // A function to pack and return the event detail
    //
    fn pack_detail(&self) -> EventDetail {
        // Create the event vector
        let mut events = Vec::new();

        // Fill the vector with the events in the list
        let mut i: i32 = 0;
        loop {
            // Iterate through the events in the list
            match self.trigger_event_list.get_row_at_index(i) {
                // Extract each row and include the event
                Some(row) => {
                    if let Some(tmp_grid) = row.get_child() {
                        // Recast the widget as a grid
                        if let Ok(event_grid) = tmp_grid.downcast::<gtk::Grid>() {
                            // Extract the event number
                            let evnt = match event_grid.get_child_at(1, 0) {
                                Some(spin_tmp) => {
                                    if let Ok(event_spin) = spin_tmp.downcast::<gtk::SpinButton>() {
                                        event_spin.get_value() as u32
                                    } else {
                                        unreachable!()
                                    }
                                }
                                None => unreachable!(),
                            };

                            // Extract the minute count
                            let mins = match event_grid.get_child_at(4, 0) {
                                Some(spin_tmp) => {
                                    if let Ok(minute_spin) = spin_tmp.downcast::<gtk::SpinButton>()
                                    {
                                        minute_spin.get_value() as u32
                                    } else {
                                        unreachable!()
                                    }
                                }
                                None => unreachable!(),
                            };

                            // Extract the second number
                            let secs = match event_grid.get_child_at(6, 0) {
                                Some(spin_tmp) => {
                                    if let Ok(second_spin) = spin_tmp.downcast::<gtk::SpinButton>()
                                    {
                                        second_spin.get_value() as u32
                                    } else {
                                        unreachable!()
                                    }
                                }
                                None => unreachable!(),
                            };

                            // Compose the new delay
                            let mut dly = None;
                            if (mins != 0) | (secs != 0) {
                                dly = Some(Duration::from_secs((secs + (mins * 60)) as u64));
                            }

                            // Create and add the event delay
                            let event_delay = EventDelay::new(dly, ItemId::new_unchecked(evnt));
                            events.push(event_delay);
                        }
                    }

                    // Move to the next row
                    i = i + 1;
                }

                // Break when there are no more rows
                None => break,
            }
        }

        // Pack the new scene id into a detail
        let mut tmp = Vec::new();
        for event in events {
            tmp.push(EventAction::QueueEvent { event });
        }
        tmp
    }
}

// Create the cancel events variant
//
#[derive(Clone, Debug)]
struct EditCancelEvent {
    grid: gtk::Grid,                 // the main grid for this element
    cancel_event_list: gtk::ListBox, // the list for events in this variant
}

impl EditCancelEvent {
    // A function to ceate a cancel events variant
    //
    fn new() -> EditCancelEvent {
        // Create the list for the cancel events variant
        let cancel_event_list = gtk::ListBox::new();
        cancel_event_list.set_selection_mode(gtk::SelectionMode::None);

        // Create a button to add events to the list
        let add_button = gtk::Button::new_from_icon_name(
            Some("list-add-symbolic"),
            gtk::IconSize::Button.into(),
        );
        add_button.connect_clicked(clone!(cancel_event_list => move |_| {

            // Add an event to the list
            EditCancelEvent::add_event(&cancel_event_list, None);
        }));

        // Create the scrollable window for the list
        let event_window = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        event_window.add(&cancel_event_list);
        event_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        event_window.set_hexpand(false);
        event_window.set_vexpand(true);
        event_window.set_valign(gtk::Align::Fill);

        // Add the button below the data list
        let cancel_event_grid = gtk::Grid::new();
        cancel_event_grid.attach(&event_window, 0, 0, 1, 1);
        cancel_event_grid.attach(&add_button, 0, 1, 1, 1);
        cancel_event_grid.set_column_spacing(10); // Add some space
        cancel_event_grid.set_row_spacing(10);

        // Create and return the cancel events variant
        cancel_event_grid.show_all();
        EditCancelEvent {
            grid: cancel_event_grid,
            cancel_event_list,
        }
    }

    // A function to return the top element of the cancel events variant
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A function to load an event detail into the cancel events variant
    //
    fn load_detail(&self, events: Vec<ItemId>) {
        // Add each event to the list
        for event in events {
            EditCancelEvent::add_event(&self.cancel_event_list, Some(event));
        }
    }

    // A helper function to add an event to the data list
    //
    fn add_event(cancel_event_list: &gtk::ListBox, event_id: Option<ItemId>) {
        // Create an empty spin box for the list
        let event_grid = gtk::Grid::new();
        let event_label = gtk::Label::new(Some("Event"));
        event_label.set_size_request(80, 30);
        event_label.set_hexpand(false);
        event_label.set_vexpand(false);
        let event_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        event_spin.set_size_request(100, 30);
        event_spin.set_hexpand(false);

        // Add a button to delete the item from the list
        let delete_button = gtk::Button::new_from_icon_name(
            Some("edit-delete-symbolic"),
            gtk::IconSize::Button.into(),
        );
        delete_button.connect_clicked(clone!(cancel_event_list, event_grid => move |_| {
            if let Some(widget) = event_grid.get_parent() {
                cancel_event_list.remove(&widget);
            }
        }));

        // Add all the components to the event grid
        event_grid.attach(&event_label, 0, 0, 1, 1);
        event_grid.attach(&event_spin, 1, 0, 1, 1);
        event_grid.attach(&delete_button, 2, 0, 1, 1);
        event_grid.set_column_spacing(10); // Add some space
        event_grid.set_row_spacing(10);

        // Set the value of the event id if it was provided
        if let Some(id) = event_id {
            event_spin.set_value(id.id() as f64);
        }

        // Add the new grid to the list
        event_grid.show_all();
        cancel_event_list.add(&event_grid);
    }

    // A function to pack and return the event detail
    //
    fn pack_detail(&self) -> EventDetail {
        // Create the event vector
        let mut events = Vec::new();

        // Fill the vector with the events in the list
        let mut i: i32 = 0;
        loop {
            // Iterate through the events in the list
            match self.cancel_event_list.get_row_at_index(i) {
                // Extract each row and include the event
                Some(row) => {
                    if let Some(tmp_grid) = row.get_child() {
                        // Recast the widget as a grid
                        if let Ok(event_grid) = tmp_grid.downcast::<gtk::Grid>() {
                            // Extract the event number
                            let evnt = match event_grid.get_child_at(1, 0) {
                                Some(spin_tmp) => {
                                    if let Ok(event_spin) = spin_tmp.downcast::<gtk::SpinButton>() {
                                        event_spin.get_value() as u32
                                    } else {
                                        unreachable!()
                                    }
                                }
                                None => unreachable!(),
                            };

                            // Create and add the event delay
                            let event_id = ItemId::new_unchecked(evnt);
                            events.push(event_id);
                        }
                    }

                    // Move to the next row
                    i = i + 1;
                }

                // Break when there are no more rows
                None => break,
            }
        }

        // Pack the new scene id into a detail
        let mut tmp = Vec::new();
        for event in events {
            tmp.push(EventAction::CancelEvent { event });
        }
        tmp
    }
}

// Create the save data variant
//
#[derive(Clone, Debug)]
struct EditSaveData {
    grid: gtk::Grid,              // the main grid for this element
    save_data_list: gtk::ListBox, // the list of data items for this variant
}

impl EditSaveData {
    // A function to ceate a save data variant
    //
    fn new() -> EditSaveData {
        // Create the list for the save data variant
        let save_data_list = gtk::ListBox::new();
        save_data_list.set_selection_mode(gtk::SelectionMode::None);

        // Create a button to add data to the list
        let add_button = gtk::Button::new_from_icon_name(
            Some("list-add-symbolic"),
            gtk::IconSize::Button.into(),
        );
        add_button.connect_clicked(clone!(save_data_list => move |_| {

            // Add a data item to the list
            EditSaveData::add_data(&save_data_list, None);
        }));

        // Create the scrollable window for the list
        let data_window = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        data_window.add(&save_data_list);
        data_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        data_window.set_hexpand(false);
        data_window.set_vexpand(true);
        data_window.set_valign(gtk::Align::Fill);

        // Add the button below the data list
        let save_data_grid = gtk::Grid::new();
        save_data_grid.attach(&data_window, 0, 0, 1, 1);
        save_data_grid.attach(&add_button, 0, 1, 1, 1);
        save_data_grid.set_column_spacing(10); // Add some space
        save_data_grid.set_row_spacing(10);

        // Create and return the save data variant
        save_data_grid.show_all();
        EditSaveData {
            grid: save_data_grid,
            save_data_list,
        }
    }

    // A function to return the top element of the save data variant
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A function to load an event detail into the save data variant
    //
    fn load_detail(&self, data: Vec<u32>) {
        // Add each item to the list
        for number in data {
            EditSaveData::add_data(&self.save_data_list, Some(number));
        }
    }

    // A helper function to add an item to the data list
    //
    fn add_data(save_data_list: &gtk::ListBox, data: Option<u32>) {
        // Create an empty spin box for the list
        let item_grid = gtk::Grid::new();
        let item_label = gtk::Label::new(Some("Save Data:"));
        item_label.set_size_request(80, 30);
        item_label.set_hexpand(false);
        item_label.set_vexpand(false);
        let item_spin = gtk::SpinButton::new_with_range(0.0, 4294967295.0, 1.0);
        item_spin.set_size_request(100, 30);
        item_spin.set_hexpand(false);

        // Add a button to delete the item from the list
        let delete_button = gtk::Button::new_from_icon_name(
            Some("edit-delete-symbolic"),
            gtk::IconSize::Button.into(),
        );
        delete_button.connect_clicked(clone!(save_data_list, item_grid => move |_| {
            if let Some(widget) = item_grid.get_parent() {
                save_data_list.remove(&widget);
            }
        }));
        item_grid.attach(&item_label, 0, 0, 1, 1);
        item_grid.attach(&item_spin, 1, 0, 1, 1);
        item_grid.attach(&delete_button, 2, 0, 1, 1);

        // Set the value of the data if it was provided
        if let Some(number) = data {
            item_spin.set_value(number as f64);
        }

        // Add the new grid to the list
        item_grid.show_all();
        save_data_list.add(&item_grid);
    }

    // A function to pack and return the event detail
    //
    fn pack_detail(&self) -> EventDetail {
        // Create the event vector
        let mut data = Vec::new();

        // Fill the vector with the data in the list
        let mut i: i32 = 0;
        loop {
            // Iterate through the events in the list
            match self.save_data_list.get_row_at_index(i) {
                // Extract each row and include the event
                Some(row) => {
                    if let Some(tmp_grid) = row.get_child() {
                        // Recast the widget as a grid
                        if let Ok(item_grid) = tmp_grid.downcast::<gtk::Grid>() {
                            // Extract the event number
                            let num = match item_grid.get_child_at(1, 0) {
                                Some(spin_tmp) => {
                                    if let Ok(num_spin) = spin_tmp.downcast::<gtk::SpinButton>() {
                                        num_spin.get_value() as u32
                                    } else {
                                        unreachable!()
                                    }
                                }
                                None => unreachable!(),
                            };

                            // Add the save data number
                            data.push(num);
                        }
                    }

                    // Move to the next row
                    i = i + 1;
                }

                // Break when there are no more rows
                None => break,
            }
        }

        // Pack the new scene id into a detail FIXME
        vec!(EventAction::SaveData { data: DataType::UserString })
    }
}

// Create the grouped event variant
//
#[derive(Clone, Debug)]
struct EditGroupedEvent {
    grid: gtk::Grid,                  // the main grid for this element
    grouped_event_list: gtk::ListBox, // the list for events in this variant
    status_spin: gtk::SpinButton,     // the status id for this variant
}

impl EditGroupedEvent {
    // A function to ceate a grouped event variant
    //
    fn new() -> EditGroupedEvent {
        // Create the list for the trigger events variant
        let grouped_event_list = gtk::ListBox::new();
        grouped_event_list.set_selection_mode(gtk::SelectionMode::None);

        // Create the status spin
        let status_grid = gtk::Grid::new();
        let status_label = gtk::Label::new(Some("Status"));
        status_label.set_size_request(80, 30);
        status_label.set_hexpand(false);
        status_label.set_vexpand(false);
        let status_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        status_spin.set_size_request(100, 30);
        status_spin.set_hexpand(false);
        status_grid.attach(&status_label, 0, 0, 1, 1);
        status_grid.attach(&status_spin, 1, 0, 1, 1);

        // Create a button to add events to the list
        let add_button = gtk::Button::new_from_icon_name(
            Some("list-add-symbolic"),
            gtk::IconSize::Button.into(),
        );
        add_button.connect_clicked(clone!(grouped_event_list => move |_| {

            // Add a new blank event to the list
            EditGroupedEvent::add_event(&grouped_event_list, None, None);
        }));

        // Create the scrollable window for the list
        let group_window = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        group_window.add(&grouped_event_list);
        group_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        group_window.set_hexpand(false);
        group_window.set_vexpand(true);
        group_window.set_valign(gtk::Align::Fill);

        // Add the status above and button below the event list
        let grouped_event_grid = gtk::Grid::new();
        grouped_event_grid.attach(&status_grid, 0, 0, 1, 1);
        grouped_event_grid.attach(&group_window, 0, 1, 1, 1);
        grouped_event_grid.attach(&add_button, 0, 2, 1, 1);
        grouped_event_grid.set_column_spacing(10); // Add some space
        grouped_event_grid.set_row_spacing(10);

        // Create and return the grouped event variant
        grouped_event_grid.show_all();
        EditGroupedEvent {
            grid: grouped_event_grid,
            grouped_event_list,
            status_spin,
        }
    }

    // A function to return the top element of the save data variant
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A function to load an event detail into the grouped event variant
    //
    fn load_detail(&self, status_id: ItemId, event_map: FnvHashMap<ItemId, ItemId>) {
        // Change the status id
        self.status_spin.set_value(status_id.id() as f64);

        // Add each event in the map to the list
        for (state_id, event_id) in event_map.iter() {
            EditGroupedEvent::add_event(
                &self.grouped_event_list,
                Some(state_id.id()),
                Some(event_id.id()),
            );
        }
    }

    // A helper function to add a grouped event to the list
    fn add_event(grouped_event_list: &gtk::ListBox, state_id: Option<u32>, event_id: Option<u32>) {
        // Create a state spin box for the list
        let group_grid = gtk::Grid::new();
        let state_label = gtk::Label::new(Some("State"));
        state_label.set_size_request(80, 30);
        state_label.set_hexpand(false);
        state_label.set_vexpand(false);
        let state_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        state_spin.set_size_request(100, 30);
        state_spin.set_hexpand(false);

        // Create a event spin box for the list
        let event_label = gtk::Label::new(Some("Event"));
        event_label.set_size_request(80, 30);
        event_label.set_hexpand(false);
        event_label.set_vexpand(false);
        let event_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        event_spin.set_size_request(100, 30);
        event_spin.set_hexpand(false);

        // Add a button to delete the item from the list
        let delete_button = gtk::Button::new_from_icon_name(
            Some("edit-delete-symbolic"),
            gtk::IconSize::Button.into(),
        );
        delete_button.connect_clicked(clone!(grouped_event_list, group_grid => move |_| {
            if let Some(widget) = group_grid.get_parent() {
                grouped_event_list.remove(&widget);
            }
        }));

        // Add all the items to the group grid
        group_grid.attach(&state_label, 0, 0, 1, 1);
        group_grid.attach(&state_spin, 1, 0, 1, 1);
        group_grid.attach(&event_label, 2, 0, 1, 1);
        group_grid.attach(&event_spin, 3, 0, 1, 1);
        group_grid.attach(&delete_button, 4, 0, 1, 1);

        // Set the value of the grouped event if it was provided
        if let Some(state) = state_id {
            state_spin.set_value(state as f64);
        }
        if let Some(event) = event_id {
            event_spin.set_value(event as f64);
        }

        // Add the new grid to the list
        group_grid.show_all();
        grouped_event_list.add(&group_grid);
    }

    // A function to pack and return the event detail
    //
    fn pack_detail(&self) -> EventDetail {
        // Create the event vector
        let mut event_map = FnvHashMap::default();

        // Extract the status id
        let status_id = ItemId::new_unchecked(self.status_spin.get_value() as u32);

        // Fill the maps with the grouped events in the list
        let mut i: i32 = 0;
        loop {
            // Iterate through the events in the list
            match self.grouped_event_list.get_row_at_index(i) {
                // Extract each row and include the event
                Some(row) => {
                    if let Some(tmp_grid) = row.get_child() {
                        // Recast the widget as a grid
                        if let Ok(grouped_grid) = tmp_grid.downcast::<gtk::Grid>() {
                            // Extract the state number
                            let state = match grouped_grid.get_child_at(1, 0) {
                                Some(spin_tmp) => {
                                    if let Ok(state_spin) = spin_tmp.downcast::<gtk::SpinButton>() {
                                        state_spin.get_value() as u32
                                    } else {
                                        unreachable!()
                                    }
                                }
                                None => unreachable!(),
                            };

                            // Extract the event number
                            let event = match grouped_grid.get_child_at(4, 0) {
                                Some(spin_tmp) => {
                                    if let Ok(event_spin) = spin_tmp.downcast::<gtk::SpinButton>() {
                                        event_spin.get_value() as u32
                                    } else {
                                        unreachable!()
                                    }
                                }
                                None => unreachable!(),
                            };

                            // Add the state and event pair to the map
                            let state_id = ItemId::new_unchecked(state);
                            let event_id = ItemId::new_unchecked(event);
                            event_map.insert(state_id, event_id);
                        }
                    }

                    // Move to the next row
                    i = i + 1;
                }

                // Break when there are no more rows
                None => break,
            }
        }

        // Pack the new scene id into a detail FIXME
        vec!(EventAction::GroupedEvent {
            status_id,
            event_map,
        })
    }
}*/
