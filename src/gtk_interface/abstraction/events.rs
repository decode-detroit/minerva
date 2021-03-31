// Copyright (c) 2019 Decode Detroit
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

//! A module to create the control abstraction that generates default
//! content and allows easier interaction and manipulation with the interface.
//! system interface. This module links indirectly to the system interface and
//! sends any updates to the application window through gtk widgets.

// Import crate definitions
use crate::definitions::*;

// Import other definitions
use super::super::utils::{clean_text, decorate_label};
use super::{LARGE_FONT, NORMAL_FONT};

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::u32::MAX as U32_MAX;

// Import FNV HashMap
use fnv;
use self::fnv::FnvHashMap;

// Import GTK and GDK libraries
use gtk;
use self::gtk::prelude::*;

// Define module constants
const BUTTON_LIMIT: usize = 16; // maximum character width of buttons
const LABEL_LIMIT: usize = 30; // maximum character width of labels
const CONTROL_LIMIT: usize = 20; // maximum character width of control panel labels

/// A structure to hold all the event groups in the default interface.
///
/// This structure allows easier modification of the gtk event group interfaces
/// to simplify interaction between the interface and the underlying program.
///
#[derive(Clone, Debug)]
pub struct EventAbstraction {
    grid: gtk::Grid,                    // the top level grid containing both windows
    left_grid: gtk::Grid,               // the grid to hold the event elements(left side)
    right_grid: gtk::Grid,              // the grid to hold the event elements (right side)
    groups: Vec<EventGroupAbstraction>, // a vector of the current event groups
    side_panel: gtk::Grid,              // the container to hold the side panel events
    side_group: Option<EventGroupAbstraction>, // side panel group (for ungrouped events)
    spotlight_control: FnvHashMap<ItemId, Rc<RefCell<u32>>>, // holders for current spotlight counters
    spotlight_state: FnvHashMap<ItemId, Rc<RefCell<u32>>>,
    spotlight_button: FnvHashMap<ItemId, Rc<RefCell<u32>>>,
    is_font_large: bool,    // a flag to indicate the font size of the items
    is_high_contrast: bool, // a flag to indicate if the display is high contrast
}

// Implement key features for the Event Abstraction
impl EventAbstraction {
    /// A function to create a new Event Abstration instance.
    ///
    pub fn new() -> EventAbstraction {
        // Create the left and right grids
        let left_grid = gtk::Grid::new();
        left_grid.set_column_homogeneous(false); // set the row and column heterogeneous
        left_grid.set_row_homogeneous(false);
        left_grid.set_row_spacing(20); // add some space between the rows
        let right_grid = gtk::Grid::new();
        right_grid.set_column_homogeneous(false); // set the row and column heterogeneous
        right_grid.set_row_homogeneous(false);
        right_grid.set_row_spacing(20); // add some space between the rows

        // Create the scrolled windows and add the grids
        let left_window = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        left_window.add(&left_grid);
        left_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the window
        left_window.set_hexpand(true);
        left_window.set_vexpand(true);
        left_window.set_halign(gtk::Align::Fill);
        left_window.set_valign(gtk::Align::Fill);

        // Create the right scrolled window and add the grid
        let right_window = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        right_window.add(&right_grid);
        right_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the window
        right_window.set_hexpand(true);
        right_window.set_vexpand(true);
        right_window.set_halign(gtk::Align::Fill);
        right_window.set_valign(gtk::Align::Fill);

        // Create the top grid for holding the two windows and separator
        let grid = gtk::Grid::new();
        grid.set_column_homogeneous(false); // set the row and column heterogeneous
        grid.set_row_homogeneous(false);

        // Create the separator
        let separator = gtk::Separator::new(gtk::Orientation::Vertical);
        separator.set_valign(gtk::Align::Fill);
        separator.set_vexpand(true);

        // Add both windows to the top level grid
        grid.attach(&left_window, 0, 0, 1, 1);
        grid.attach(&separator, 1, 0, 1, 1);
        grid.attach(&right_window, 2, 0, 1, 1);

        // Create the side panel grid and set the features
        let side_panel = gtk::Grid::new();
        side_panel.set_column_homogeneous(false); // set the row and column heterogeneous
        side_panel.set_row_homogeneous(false);
        side_panel.set_row_spacing(5); // add some space between the rows
        side_panel.set_vexpand(true); // adjust the expansion parameters of the grid
        side_panel.set_hexpand(false);

        // Return the new Event Abstraction
        EventAbstraction {
            grid,
            left_grid,
            right_grid,
            groups: Vec::new(), // an empty list of event groups
            side_panel,
            side_group: None, // an empty side panel event group
            spotlight_control: FnvHashMap::default(),
            spotlight_state: FnvHashMap::default(),
            spotlight_button: FnvHashMap::default(),
            is_font_large: false,
            is_high_contrast: false,
        }
    }

    /// A method to return a reference to the top element of the interface,
    /// currently the top level grid.
    ///
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    /// A method to return the side panel container for special events.
    ///
    pub fn get_side_panel(&self) -> &gtk::Grid {
        &self.side_panel
    }

    /// A method to select the font size of the event array.
    ///
    pub fn select_font(&mut self, is_large: bool) {
        self.is_font_large = is_large;
    }

    /// A method to select the color contrast of the event array.
    ///
    pub fn select_contrast(&mut self, is_hc: bool) {
        self.is_high_contrast = is_hc;
    }

    /// A method to clear the old event groups and event grids to create a fresh
    /// event abstraction.
    ///
    pub fn clear(&mut self) {
        // Remove all the the children from the left and right grids
        let to_remove = self.left_grid.get_children();
        for item in to_remove {
            item.hide(); // necessary for proper functioning of the spotlight feature
            unsafe {
                item.destroy();
            }
        }
        let to_remove = self.right_grid.get_children();
        for item in to_remove {
            item.hide(); // necessary for proper functioning of the spotlight feature
            unsafe {
                item.destroy();
            }
        }

        // Remove all the children in the side panel grid
        let to_remove = self.side_panel.get_children();
        for item in to_remove {
            item.hide(); // necessary for proper functioning of the spotlight feature
            unsafe {
                item.destroy();
            }
        }

        // Clear the group list and the side group to clear dangling references
        self.groups = Vec::new();
        self.side_group = None;
    }

    /// A method to clear the old event grid and load the provided window into
    /// the event grid, updating the Event Abstraction in the process.
    ///
    /// This method will send the triggered event id as a System Update on the
    /// system send line.
    ///
    pub fn update_window(
        &mut self,
        current_scene: ItemPair,
        mut statuses: Vec<ItemPair>,
        mut window: EventWindow,
        full_status: &FullStatus,
        gtk_send: &GtkSend,
        interface_send: &mpsc::Sender<InterfaceUpdate>,
    ) {
        // Empty the old event grid
        self.clear();

        // Copy the spotlight counts and drain them
        let spotlight_control: FnvHashMap<ItemId, Rc<RefCell<u32>>> =
            self.spotlight_control.drain().collect();
        let spotlight_state: FnvHashMap<ItemId, Rc<RefCell<u32>>> =
            self.spotlight_state.drain().collect();
        let spotlight_button: FnvHashMap<ItemId, Rc<RefCell<u32>>> =
            self.spotlight_button.drain().collect();

        // Set the font size
        let font_size = match self.is_font_large {
            false => NORMAL_FONT,
            true => LARGE_FONT,
        };

        // Copy the available groups into new group abstractions
        let mut groups_raw = Vec::new();
        for group in window.drain(..) {
            // Try to load the group into a group abstraction
            if let Some(grp_abstraction) = EventGroupAbstraction::new(
                group,
                gtk_send,
                interface_send,
                full_status,
                font_size,
                self.is_high_contrast,
                &spotlight_state,
                &mut self.spotlight_state,
                &spotlight_button,
                &mut self.spotlight_button,
            ) {
                // If it has an id, add it to the group list
                if let Some(_) = grp_abstraction.group_id {
                    // Identify the group position
                    match grp_abstraction.position.clone() {
                        // If the position is defined, set it
                        Some(number) => groups_raw.push((number, grp_abstraction)),

                        // Otherwise, set the maximum default position
                        None => groups_raw.push((U32_MAX, grp_abstraction)),
                    }

                // Otherwise place it in the default group
                } else {
                    self.side_group = Some(grp_abstraction);
                }
            }
        }

        // Reorder the groups to follow position
        groups_raw.sort_by_key(|pair| {
            let &(ref position, _) = pair;
            position.clone()
        });

        // Strip the raw groups to remove position
        for (_, group) in groups_raw.drain(..) {
            self.groups.push(group);
        }

        // Add the current scene detail
        let current_title = gtk::Label::new(None);
        current_title.set_markup(&format!("<span size='{}'>Current Scene:</span>", font_size));
        current_title.set_halign(gtk::Align::End);
        current_title.set_margin_end(5);
        current_title.show();
        self.side_panel.attach(&current_title, 0, 1, 1, 1);
        let current_label = gtk::Label::new(None);
        let current_markup =
            clean_text(&current_scene.description, CONTROL_LIMIT, true, false, true);
        decorate_label(
            &current_label,
            &current_markup,
            current_scene.display,
            full_status,
            font_size,
            self.is_high_contrast,
            None, // No spotlight
        );
        current_label.set_halign(gtk::Align::Start);
        current_label.set_margin_start(5);
        current_label.show();
        self.side_panel.attach(&current_label, 1, 1, 1, 1);

        // Add title and control separator
        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        separator.set_halign(gtk::Align::Fill);
        separator.set_hexpand(true);
        separator.show();
        self.side_panel.attach(&separator, 0, 2, 2, 1);

        // Sort the statuses
        let mut paired = Vec::new();
        for status in statuses.drain(..) {
            // Extract the position of the status display
            if let LabelControl { position, .. } = status.display.clone() {
                // Unpack the position
                match position {
                    // Add the position if it exists
                    Some(num) => paired.push((num, status)),

                    // Otherwise, use the max available
                    None => paired.push((U32_MAX, status)),
                }
            }
        }

        // Reorder the statuses to follow position
        paired.sort_by_key(|pair| {
            let &(ref position, _) = pair;
            position.clone()
        });

        // Strip the raw buttons to remove position
        let mut sorted = Vec::new();
        for (_, status) in paired.drain(..) {
            sorted.push(status);
        }

        // Add the status/state pairs to the side panel
        let mut status_count: i32 = 0;
        for status in sorted.drain(..) {
            // Put the name of the status
            let status_title = gtk::Label::new(None);
            let status_markup = clean_text(&status.description, CONTROL_LIMIT, true, false, true);
            decorate_label(
                &status_title,
                &status_markup,
                status.display,
                full_status,
                font_size,
                self.is_high_contrast,
                None, // No spotlight
            );
            status_title.set_halign(gtk::Align::End);
            status_title.set_margin_end(5);
            status_title.show();
            self.side_panel
                .attach(&status_title, 0, 3 + status_count, 1, 1);

            // Find the corresponding current state
            let state = gtk::Label::new(Some("Not Available"));
            if let Some(&StatusDescription { ref current, .. }) = full_status.get(
                &ItemPair::from_item(status.get_id(), ItemDescription::new("", Hidden)),
            ) {
                // See if there is an existing spotlight expiration for this state
                let expiration = match spotlight_control.get(&current.get_id()) {
                    Some(exp) => exp.clone(),
                    None => Rc::new(RefCell::new(U32_MAX)),
                };

                // Decorate the state label
                let state_markup =
                    clean_text(&current.description, CONTROL_LIMIT, true, false, true);
                decorate_label(
                    &state,
                    &state_markup,
                    current.display,
                    full_status,
                    font_size,
                    self.is_high_contrast,
                    Some(expiration.clone()), // spotlight
                );

                // If the expiration was used, save it
                if Rc::strong_count(&expiration) > 1 {
                    self.spotlight_control.insert(current.get_id(), expiration);
                }
            }

            // Format the state label
            state.set_halign(gtk::Align::Start);
            state.set_margin_start(5);
            state.show();
            self.side_panel.attach(&state, 1, 3 + status_count, 1, 1);

            // Add a separator
            status_count = status_count + 1;
            let new_sep = gtk::Separator::new(gtk::Orientation::Horizontal);
            new_sep.set_halign(gtk::Align::Fill);
            new_sep.set_hexpand(true);
            new_sep.show();
            self.side_panel.attach(&new_sep, 0, 3 + status_count, 2, 1);

            // Increment and continue
            status_count = status_count + 1;
        }

        // Try to attach the side panel group to the side panel grid
        if let Some(ref group) = self.side_group {
            // Create the found side panel group
            let grid = group.as_grid();
            grid.set_halign(gtk::Align::Fill);
            self.side_panel.attach(&grid, 0, 3 + status_count, 2, 1);
        }

        // Attach all the regular event groups to the event grid in two columns
        let half: usize = (self.groups.len() / 2) + (self.groups.len() % 2); // round up
        for (num, group) in self.groups.iter().enumerate() {
            // Switch left and right sides, half on each
            if num < half {
                // Attach the found group to the left grid
                self.left_grid.attach(&group.as_grid(), 0, num as i32, 1, 1);
            } else {
                // Attach the found group to the right grid
                self.right_grid
                    .attach(&group.as_grid(), 0, (num - half) as i32, 1, 1);
            }
        }
    }
}

/// An internal structure to hold an individual event group in the default interface.
///
/// This structure allows easier modification of the gtk event group interfaces
/// to simplify interaction between the interface and the underlying program.
///
#[derive(Clone, Debug)]
struct EventGroupAbstraction {
    group_id: Option<ItemId>, // the id for the group for rapid identification
    position: Option<u32>,    // the position for the group for ordering on the screen
    header: gtk::Label,       // the label attached to the header for the group
    state_selection: Option<gtk::Button>, // the dropdown for switching between states (if it exists)
    buttons: Vec<gtk::Button>,            // the event buttons for the event group
}

// Implement key features for the Event Group Abstraction
impl EventGroupAbstraction {
    /// A function to create a new abstraction from the provided EventGroup.
    ///
    fn new(
        event_group: EventGroup,
        gtk_send: &GtkSend,
        interface_send: &mpsc::Sender<InterfaceUpdate>,
        full_status: &FullStatus,
        font_size: u32,
        is_high_contrast: bool,
        old_spotlight_state: &FnvHashMap<ItemId, Rc<RefCell<u32>>>,
        spotlight_state: &mut FnvHashMap<ItemId, Rc<RefCell<u32>>>,
        old_spotlight_button: &FnvHashMap<ItemId, Rc<RefCell<u32>>>,
        spotlight_button: &mut FnvHashMap<ItemId, Rc<RefCell<u32>>>,
    ) -> Option<EventGroupAbstraction> {
        // Create the empty header
        let header = gtk::Label::new(None);
        header.set_margin_start(10);
        header.set_margin_end(10);
        header.set_hexpand(false);
        header.set_halign(gtk::Align::End);

        // If there is an id, create the header and id and add it to the group
        let group_id;
        let position;
        let mut state_selection = None;
        match event_group.group_id.clone() {
            // Create the group_id and header for the particular group
            Some(id_pair) => {
                group_id = Some(id_pair.get_id());
                let header_markup = &format!(
                    "<span size='14000'>{}</span>",
                    clean_text(&id_pair.description, LABEL_LIMIT, false, false, true)
                );
                position = decorate_label(
                    &header,
                    &header_markup,
                    id_pair.display,
                    full_status,
                    font_size,
                    is_high_contrast,
                    None, // No spotlight
                );

                // Create the status selection button (if the status exists)
                if let Some(&StatusDescription { ref current, .. }) = full_status.get(&id_pair) {
                    // See if there is an existing spotlight expiration for this state
                    let expiration = match old_spotlight_state.get(&current.get_id()) {
                        Some(exp) => exp.clone(),
                        None => Rc::new(RefCell::new(U32_MAX)),
                    };

                    // Create the new state selection label
                    let state_label = gtk::Label::new(None);
                    let state_markup =
                        clean_text(&current.description, LABEL_LIMIT, true, false, true);
                    decorate_label(
                        &state_label,
                        &state_markup,
                        current.display,
                        full_status,
                        font_size,
                        is_high_contrast,
                        Some(expiration.clone()), // spotlight
                    );

                    // If the expiration was used, save it
                    if Rc::strong_count(&expiration) > 1 {
                        spotlight_state.insert(current.get_id(), expiration);
                    }

                    // Create the button and add the label
                    state_label.show();
                    let selection = gtk::Button::new();
                    selection.add(&state_label);
                    selection.set_hexpand(false);
                    selection.set_halign(gtk::Align::Start);

                    // Connect the status dialog when clicked
                    selection.connect_clicked(clone!(interface_send => move |_| {
                        // Send the status dialog to the user interface
                        interface_send.send(LaunchWindow {
                            window_type: WindowType::Status(Some(id_pair.clone()))
                        }).unwrap_or(());
                    }));

                    // Set the new state selection
                    state_selection = Some(selection);
                }
            }

            // Create the default id and header
            None => {
                group_id = None;
                position = None;
            }
        }

        // Create a new button for each of the group events
        let mut buttons_raw = Vec::new();
        for event in event_group.group_events {
            // Create a new button
            let button_label = gtk::Label::new(None);
            let button_markup = clean_text(&event.description, BUTTON_LIMIT, true, false, true);

            // See if there is an existing spotlight expiration for this state
            let expiration = match old_spotlight_button.get(&event.get_id()) {
                Some(exp) => exp.clone(),
                None => Rc::new(RefCell::new(U32_MAX)),
            };

            // Set the markup based on the requested color and extract the position
            let button_position = decorate_label(
                &button_label,
                &button_markup,
                event.display,
                full_status,
                font_size,
                is_high_contrast,
                Some(expiration.clone()),
            );

            // Set the features of the new label and place it on the button
            button_label.show();
            button_label.set_halign(gtk::Align::Center);
            let button = gtk::Button::new();
            button.add(&button_label);

            // If the expiration was used, save it and use it in button clicked
            if Rc::strong_count(&expiration) > 1 {
                spotlight_button.insert(event.get_id(), expiration.clone());

                // Create the new button action and connect it
                button.connect_clicked(clone!(gtk_send => move |_| {
                    // Send the event trigger to the underlying system
                    gtk_send.send(UserRequest::ProcessEvent { event: event.get_id(), check_scene: true, broadcast: true});

                    // Stop the button from flashing, if it is
                    if let Ok(mut count) = expiration.try_borrow_mut() {
                        *count = 1;
                    }
                }));

            // Otherwise, just set the button click normally
            } else {
                // Create the new button action and connect it
                button.connect_clicked(clone!(gtk_send => move |_| {
                    // Send the event trigger to the underlying system
                    gtk_send.send(UserRequest::ProcessEvent { event: event.get_id(), check_scene: true, broadcast: true});
                }));
            }

            // Add the position and button to the list
            match button_position {
                // Use the position, if provided
                Some(number) => buttons_raw.push((number, button)),

                // Otherwise, default to the maximum possible
                None => buttons_raw.push((U32_MAX, button)),
            }
        }

        // Reorder the buttons to follow position
        buttons_raw.sort_by_key(|pair| {
            let &(ref position, _) = pair;
            position.clone()
        });

        // Strip the raw buttons to remove position
        let mut buttons = Vec::new();
        for (_, button) in buttons_raw.drain(..) {
            buttons.push(button);
        }

        // If there are some buttons in the abstraction
        if buttons.len() > 0 {
            // Return the new group abstraction
            return Some(EventGroupAbstraction {
                group_id,
                position,
                header,
                state_selection,
                buttons,
            });
        }

        // Otherwise, return nothing
        None
    }

    /// A method to compose the event group into a scrollable, horiztonal grid
    /// with a group title, optional status, and a flowbox of buttons.
    ///
    fn as_grid(&self) -> gtk::Grid {
        // Create the top level grid for this group
        let grid = gtk::Grid::new();

        // Define the formatting for this grid
        grid.set_column_homogeneous(false); // set row and column heterogeneous
        grid.set_row_homogeneous(false);
        grid.set_row_spacing(2); // add some space between the rows

        // Create the flowbox for the buttons
        let button_box = gtk::FlowBox::new();
        button_box.set_orientation(gtk::Orientation::Horizontal);
        button_box.set_selection_mode(gtk::SelectionMode::None);
        button_box.set_hexpand(true);
        button_box.set_halign(gtk::Align::Fill);
        button_box.set_column_spacing(0); // Remove any column spacing

        // Populate the button grid
        for button in self.buttons.iter() {
            // Add each button to the grid
            button_box.add(button);
        }

        // If there is a group id
        if let Some(_) = self.group_id {
            // Create a placeholder for expanding along the event buttons
            let dummy = gtk::Label::new(Some(""));
            dummy.set_halign(gtk::Align::Fill);
            dummy.set_hexpand(true);
            dummy.show();

            // Add the label, status, and window to the grid
            grid.attach(&self.header, 0, 0, 1, 1);

            // Add the status dropdown if it exists
            if let Some(ref selection) = self.state_selection {
                grid.attach(selection, 1, 0, 1, 1);
                grid.attach(&dummy, 2, 0, 1, 1);
                grid.attach(&button_box, 0, 1, 3, 1);

            // Otherwise just add the window
            } else {
                grid.attach(&dummy, 1, 0, 1, 1);
                grid.attach(&button_box, 0, 1, 2, 1);
            }

        // Otherwise, just attach the button box
        } else {
            grid.attach(&button_box, 0, 0, 1, 1);
        }

        // Show all the elements in the group
        self.show_all();
        button_box.show();
        grid.show();

        // Return the new grid
        grid
    }

    /// A method to show all the gtk widgets in the group abstraction.
    ///
    fn show_all(&self) {
        // Show the header and the label
        self.header.show();
        if let Some(ref selection) = self.state_selection {
            selection.show();
        }

        // Show all the event buttons
        for button in self.buttons.iter() {
            button.show();
        }
    }
}
