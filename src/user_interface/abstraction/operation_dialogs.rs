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
    BroadcastEvent, DisplayComponent, EventDelay, FullStatus, Hidden, ItemId,
    ItemPair, KeyMap, ProcessEvent, QueueEvent, ReplyType, Request, RequestType,
    SceneChange, StatusChange, StatusDescription, SystemSend,
};
#[cfg(feature = "media-out")]
use super::super::super::system_interface::VideoStream;
use super::super::utils::{clean_text, decorate_label};
use super::NORMAL_FONT;

// Import standard library features
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;
use std::time::Duration;
#[cfg(feature = "media-out")]
use std::ffi::c_void;

// Import FNV HashMap
use fnv;
use self::fnv::FnvHashMap;

// Import GTK and GDK libraries
use gdk;
use gtk;
use self::gtk::prelude::*;
#[cfg(feature = "media-out")]
use self::gdk::WindowExt;

// Import Gstreamer Library
#[cfg(feature = "media-out")]
use gstreamer_video as gst_video;
#[cfg(feature = "media-out")]
use self::gst_video::prelude::*;

// Define and import constants
const STATE_LIMIT: usize = 20; // maximum character width of states
const DESCRIPTION_LIMIT: usize = 40; // shortcut event descriptions character limit
const MINUTES_LIMIT: f64 = 10080.0; // maximum input time for a delayed event (one week)

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
                item.destroy();
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

                        // Get a copy of the full status
                        if let Ok(full_status) = protected_status.try_borrow() {

                            // Find the corresponding status
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
    key_map: KeyMap,                                          // the map of key codes to event ids
    system_send: SystemSend,                                  // a copy of system send
    window: gtk::ApplicationWindow,                           // a copy of the primary window
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
            &[("Close", gtk::ResponseType::Ok)],
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
        separator.set_hexpand(true);
        separator.set_halign(gtk::Align::Fill);
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
            grid.attach(
                &gtk::Label::new(Some(&format!("  {}  ", key))),
                0,
                count,
                1,
                1,
            );

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
        // Save the key map to be displayed or enabled/disabled
        self.key_map = key_map;

        // Enable the new shortcuts
        self.enable_shortcuts(true);
    }

    /// A method to enable or disable the keyboard shortcuts
    ///
    pub fn enable_shortcuts(&mut self, are_enabled: bool) {
        // Clear the old key press handler
        let mut tmp = None;
        mem::swap(&mut tmp, &mut self.key_press_handler);
        if let Some(handler) = tmp {
            self.window.disconnect(handler);
        }

        // If enabled, create the new handler
        if are_enabled {
            // Create a new handler (prevents any errant key presses if empty)
            let key_clone = self.key_map.clone();
            let send_clone = self.system_send.clone();
            self.key_press_handler = Some(
                // Attach the handler
                self.window.connect_key_press_event(move |_, key_press| {
                    // Check to see if it matches one of our events
                    if let Some(id) = key_clone.get(&key_press.get_keyval()) {
                        send_clone.send(ProcessEvent {
                            event: id.get_id(),
                            check_scene: true,
                            broadcast: true,
                        });
                    }

                    // Prevent any other keypress handlers from running
                    gtk::Inhibit(true)
                }),
            );
        }
    }
}

/// A structure to contain the dialog for triggering a custom event.
///
pub struct TriggerDialog {
    window: gtk::ApplicationWindow,        // a copy of the primary window
    description_label: Option<gtk::Label>, // the label which displays an item description
}

// Implement key features for the trigger dialog
impl TriggerDialog {
    /// A function to create a new trigger dialog structure.
    ///
    pub fn new(window: &gtk::ApplicationWindow) -> TriggerDialog {
        TriggerDialog {
            window: window.clone(),
            description_label: None,
        }
    }

    /// A method to launch the new trigger dialog
    ///
    pub fn launch(&mut self, system_send: &SystemSend, event: Option<ItemPair>) {
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

        // Add some space between the rows and columns
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);

        // Add the dropdown and label
        let label = gtk::Label::new(Some(
            " WARNING: Triggering a custom event may cause undesired behaviour. ",
        ));
        grid.attach(&label, 0, 0, 3, 1);

        // Add a separator
        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        separator.set_hexpand(true);
        separator.set_halign(gtk::Align::Fill);
        grid.attach(&separator, 0, 1, 3, 1);

        // Create the headers
        let label = gtk::Label::new(Some(" Event ID "));
        grid.attach(&label, 0, 2, 1, 1);
        let label = gtk::Label::new(Some(" Event Description "));
        grid.attach(&label, 1, 2, 2, 1);

        // Create the event selection
        let event_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Description label for the current event
        let event_description = gtk::Label::new(Some(""));
        event_description.set_hexpand(true);
        event_description.set_halign(gtk::Align::Fill);
        self.description_label = Some(event_description.clone());

        // Connect the update description function to the spin button
        event_spin.connect_property_value_notify(clone!(system_send => move |spin| {
            // Request a new description from the system
            system_send.send(Request {
                reply_to: DisplayComponent::TriggerDialog,
                request: RequestType::Description {
                    item_id: ItemId::new_unchecked(spin.get_value() as u32),
                }
            });
        }));

        // If an id was specified, use it
        if let Some(event_pair) = event {
            event_spin.set_value(event_pair.id() as f64);
            event_description.set_text(&event_pair.description());
        }

        // Add them to the grid
        grid.attach(&event_spin, 0, 3, 1, 1);
        grid.attach(&event_description, 1, 3, 2, 1);

        // Add a separator
        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        separator.set_hexpand(true);
        separator.set_halign(gtk::Align::Fill);
        grid.attach(&separator, 0, 4, 3, 1);

        // Add a separator for the delay (lower down)
        let delay_separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        separator.set_hexpand(true);
        separator.set_halign(gtk::Align::Fill);
        grid.attach(&delay_separator, 0, 6, 3, 1);

        // Create the delay headers and spin buttons
        let delay_label = gtk::Label::new(Some(" Delay "));
        grid.attach(&delay_label, 0, 7, 1, 2);
        let minutes_label = gtk::Label::new(Some(" Minutes "));
        grid.attach(&minutes_label, 1, 7, 1, 1);
        let seconds_label = gtk::Label::new(Some(" Seconds "));
        grid.attach(&seconds_label, 2, 7, 1, 1);
        let minutes_spin = gtk::SpinButton::new_with_range(0.0, MINUTES_LIMIT, 1.0);
        grid.attach(&minutes_spin, 1, 8, 1, 1);
        let seconds_spin = gtk::SpinButton::new_with_range(0.0, 59.0, 1.0);
        grid.attach(&seconds_spin, 2, 8, 1, 1);

        // Create the checkboxes
        let now_checkbox = gtk::CheckButton::new_with_label("Trigger Now");
        now_checkbox.set_active(true);
        let broadcast_checkbox = gtk::CheckButton::new_with_label("Skip All Checks");
        broadcast_checkbox.set_active(false);
        let scene_checkbox = gtk::CheckButton::new_with_label("Check Scene");
        scene_checkbox.set_active(true);
        grid.attach(&now_checkbox, 0, 5, 1, 1);
        grid.attach(&broadcast_checkbox, 1, 5, 1, 1);
        grid.attach(&scene_checkbox, 2, 5, 1, 1);

        // Make changes to the interface when trigger now is changed
        now_checkbox.connect_toggled(clone!(broadcast_checkbox, scene_checkbox, delay_separator, delay_label, minutes_label, seconds_label, minutes_spin, seconds_spin => move | checkbox | {
            // Make the other two checkboxes visible
            if checkbox.get_active() {
                broadcast_checkbox.show();

                // Check to make sure broadcast isn't selected
                if !broadcast_checkbox.get_active() {
                    scene_checkbox.show();
                }

                // Hide the delay inputs
                delay_separator.hide();
                delay_label.hide();
                minutes_label.hide();
                seconds_label.hide();
                minutes_spin.hide();
                seconds_spin.hide();

            // Make the delay options visible
            } else {
                delay_separator.show();
                delay_label.show();
                minutes_label.show();
                seconds_label.show();
                minutes_spin.show();
                seconds_spin.show();

                // Hide the other checkboxes
                scene_checkbox.hide();
                broadcast_checkbox.hide();
            }
        }));

        // Make sure the scene checkbox is hidden when broadcast is selected
        broadcast_checkbox.connect_toggled(clone!(scene_checkbox => move | checkbox | {
            // Make sure the checkbox is hidden
            if checkbox.get_active() {
                scene_checkbox.hide();

            // Otherwise show it
            } else {
                scene_checkbox.show();
            }
        }));

        // Connect the close event for when the dialog is complete
        dialog.connect_response(clone!(system_send, event_spin, now_checkbox, broadcast_checkbox, scene_checkbox, minutes_spin, seconds_spin => move |modal, id| {
            // Notify the system of the event change
            if id == gtk::ResponseType::Ok {
                // If trigger now is not selected, send a delayed event
                if !now_checkbox.get_active() {
                    // Extract the delay times
                    let minutes = minutes_spin.get_value() as u32;
                    let seconds = seconds_spin.get_value() as u32;

                    // Compose the new delay
                    let mut delay = None;
                    if (minutes != 0) | (seconds != 0) {
                        delay = Some(Duration::from_secs((seconds + (minutes * 60)) as u64));
                    }

                    // Send the new event
                    system_send.send(QueueEvent { event_delay: EventDelay::new(delay, ItemId::new_unchecked(event_spin.get_value() as u32))});

                // If broadcast is selected, send a broadcast event
                } else if broadcast_checkbox.get_active() {
                    system_send.send(BroadcastEvent { event: ItemPair::new_unchecked(event_spin.get_value() as u32, "", Hidden), data: None});

                // Otherwise, send the event to be processed by the system
                } else { system_send.send(ProcessEvent { event: ItemId::new_unchecked(event_spin.get_value() as u32), check_scene: scene_checkbox.get_active(), broadcast: true});
                }
            }

            // Close the window either way
            modal.destroy();
        }));

        // Show the dialog and components
        dialog.show_all();

        // Hide the delay inputs and return
        delay_separator.hide();
        delay_label.hide();
        minutes_label.hide();
        seconds_label.hide();
        minutes_spin.hide();
        seconds_spin.hide();
    }

    // A method to update the information displayed in the dialog
    pub fn update_info(&self, reply: ReplyType) {
        // Update the even description, ignore others
        if let ReplyType::Description { description } = reply {
            // Update the event description, if it exists
            if let Some(ref label) = self.description_label {
                label.set_text(&description.description);
            }
        }
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
            Some(&clean_text(
                &event.description,
                STATE_LIMIT,
                false,
                false,
                true,
            )),
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
        let label = gtk::Label::new(Some(" Enter Text "));
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

/// A structure to contain the window for displaying video streams.
///
#[cfg(feature = "media-out")]
pub struct VideoWindow {
    overlay_map: FnvHashMap<u32, gtk::Overlay>, // the overlay widget
    channel_map: Rc<RefCell<FnvHashMap<std::string::String, gtk::Rectangle>>>, // the mapping of channel numbers to allocations
}

// Implement key features for the video window
#[cfg(feature = "media-out")]
impl VideoWindow {
    /// A function to create a new prompt string dialog structure.
    ///
    pub fn new() -> VideoWindow {
        // Create the overlay map
        let overlay_map = FnvHashMap::default();
        
        // Create the channel map
        let channel_map: Rc<RefCell<FnvHashMap<std::string::String, gtk::Rectangle>>> =
            Rc::new(RefCell::new(FnvHashMap::default()));

        // Return the completed Video Window
        VideoWindow {
            overlay_map,
            channel_map,
        }
    }
    
    /// A method to clear all video windows
    ///
    pub fn clear_all(&mut self) {
        // Destroy any open windows
        for (_, overlay) in self.overlay_map.drain() {
            if let Some(window) = overlay.get_parent() {
                window.destroy();
            }
        }
        
        // Empty the channel map
        if let Ok(mut map) = self.channel_map.try_borrow_mut() {
            map.clear();
        }
    }

    /// A method to add a new video to the video window
    ///
    pub fn add_new_video(&mut self, video_stream: VideoStream) {
        // Create a new video area
        let video_area = gtk::DrawingArea::new();
        
        // Try to add the video area to the channel map
        match self.channel_map.try_borrow_mut() {
            // Insert the new channel
            Ok(mut map) => {
                map.insert(video_stream.channel.to_string(), video_stream.allocation);
            }
            
            // Fail silently
            _ => return,
        }
        video_area.set_widget_name(&video_stream.channel.to_string());
        
        // Extract the window number (for use below)
        let window_number = video_stream.window_number;
        
        // Connect the realize signal for the video area
        video_area.connect_realize(move |video_area| {
            // Extract a reference for the video overlay
            let video_overlay = &video_stream.video_overlay;
            
            // Try to get a copy of the GDk window
            let gdk_window = match video_area.get_window() {
                Some(window) => window,
                None => {
                    println!("Unable to get current window for video overlay.");
                    return;
                }
            };
            
            // Check to make sure the window is native
            if !gdk_window.ensure_native() {
                println!("Widget is not located inside a native window.");
                return;
            }

            // Extract the display type of the window
            let display_type = gdk_window.get_display().get_type().name();
            
            // Switch based on the platform
            #[cfg(target_os = "linux")]
            {
                // Check if we're using X11
                if display_type == "GdkX11Display" {
                    // Connect to the get_xid function
                    extern "C" {
                        pub fn gdk_x11_window_get_xid(
                            window: *mut glib::object::GObject,
                        ) -> *mut c_void;
                    }

                    // Connect the video overlay to the correct window handle
                    #[allow(clippy::cast_ptr_alignment)]
                    unsafe {
                        let xid = gdk_x11_window_get_xid(gdk_window.as_ptr() as *mut _);
                        video_overlay.set_window_handle(xid as usize);
                    }
                } else {
                    println!("Unsupported display type: {}", display_type);
                }
            }
            
            // If on Mac OS
            #[cfg(target_os = "macos")]
            {
                // Check if we're using Quartz
                if display_type_name == "GdkQuartzDisplay" {
                    extern "C" {
                        pub fn gdk_quartz_window_get_nsview(
                            window: *mut glib::object::GObject,
                        ) -> *mut c_void;
                    }

                    #[allow(clippy::cast_ptr_alignment)]
                    unsafe {
                        let window = gdk_quartz_window_get_nsview(gdk_window.as_ptr() as *mut _);
                        video_overlay.set_window_handle(window as usize);
                    }
                } else {
                    println!("Unsupported display type {}", display_type);
                }
            }
        });
        
        // Check to see if there is already a matching window
        if let Some(overlay) = self.overlay_map.get(&window_number) {
            // Add the video area to the overlay
            overlay.add_overlay(&video_area);
            
            // Show the video area
            video_area.show();
        
        // Otherwise, create a new window
        } else {
            // Create the new window
            let (window, overlay) = self.new_window();
            
            // Add the video area to the overlay
            overlay.add_overlay(&video_area);
            
            // Save the overlay in the overlay map
            self.overlay_map.insert(window_number, overlay);
            
            // Show the window
            window.show_all();
        }
    }
    
    // A helper function to create a new video window and return the window and overlay
    //
    fn new_window(&self) -> (gtk::Window, gtk::Overlay) {
        // Create the new window
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        
        // Set window parameters
        window.set_decorated(false);
        window.fullscreen();
        
        // Create black background
        let background = gtk::DrawingArea::new();
        background.connect_draw(|_, cr| {
            // Draw the background black
            cr.set_source_rgb(0.0, 0.0, 0.0);
            cr.paint();
            Inhibit(true)
        });
        
        // Create the overlay and add the background
        let overlay = gtk::Overlay::new();
        overlay.add(&background);
        
        // Connect the get_child_position signal
        let channel_map = self.channel_map.clone();
        overlay.connect_get_child_position(move |_, widget| {
            // Try to get the channel map
            if let Ok(map) = channel_map.try_borrow() {
                // Try to get the widget name
                if let Some(name) = widget.get_widget_name() {
                    // Look up the name in the channel map
                    if let Some(allocation) = map.get(name.as_str()) {
                        // Return the completed allocation
                        return Some(allocation.clone());
                    }
                }
            }
            
            // Return None on failure
            None
        });
        
        // Add the overlay to the window
        window.add(&overlay);
        
        // Return the overlay
        (window, overlay)
    }
}
