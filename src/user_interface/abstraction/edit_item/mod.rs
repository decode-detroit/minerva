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

//! A module to create the edit abstraction for modifying the details individual
//! items (including events, statuses, and scenes). This module links directly
//! to the system interface to request and modify data in the configuration.

// Define private submodules
mod edit_action;
mod edit_scene;

// Import the relevant structures into the correct namespace
use self::edit_action::EditAction;
use self::edit_scene::EditScene;
use super::super::super::system_interface::{
    DisplayComponent, DisplayControl, DisplayDebug, DisplayWith, Edit, EventAction,
    EventDetail, Hidden, InterfaceUpdate, ItemDescription, ItemId, ItemPair,
    LabelControl, LabelHidden, Modification, ReplyType, Request, RequestType,
    StatusDetail, SystemSend,
};

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

// Import FNV HashMap
extern crate fnv;
use self::fnv::FnvHashMap;

// Import the serde_yaml library
extern crate serde_yaml;

// Import GTK and GDK libraries
extern crate gdk;
extern crate gio;
extern crate glib;
extern crate gtk;
use self::gtk::prelude::*;

/// A structure to contain the the item editing funcitonality.
///
/// This structure automatically detects if an item corresponds to an event,
/// status, or scene (or a combination of those items) and allows the user to
/// modify all the details associated with that item.
#[derive(Clone, Debug)]
pub struct EditItemAbstraction {
    grid: gtk::Grid,                               // the grid to hold underlying elements
    system_send: SystemSend,                       // a copy of the system send line
    interface_send: mpsc::Sender<InterfaceUpdate>, // a copy of the interface send line
    current_id: Rc<RefCell<Option<ItemId>>>,       // the wrapped current item id
    item_list: ItemList,                           // the item list section of the window
    edit_overview: Rc<RefCell<EditOverview>>,      // the wrapped edit overview section of the window
    edit_detail: Rc<RefCell<EditDetail>>,          // the wrapped edit detail section of the window
    edit_scene: Rc<RefCell<EditScene>>,            // the wrapped edit scene section of the window
}

// Implement key features for the EditItemAbstration
impl EditItemAbstraction {
    /// A function to create a new instance of the Edit Item Abstraction. This
    /// function loads all the default widgets into the interface and returns
    /// a new copy to allow insertion into higher-level elements.
    ///
    pub fn new(
        system_send: &SystemSend,
        interface_send: &mpsc::Sender<InterfaceUpdate>,
    ) -> EditItemAbstraction {
        // Create the control grid for holding all the universal controls
        let grid = gtk::Grid::new();

        // Set the features of the grid
        grid.set_column_homogeneous(false); // set the row and column heterogeneous
        grid.set_row_homogeneous(false);
        grid.set_column_spacing(10); // add some internal space
        grid.set_row_spacing(10);

        // Format the whole grid
        grid.set_hexpand(false);
        grid.set_vexpand(true);


        // Create the item list title and attach it to the grid
        let grid_title = gtk::Label::new(Some("All Items"));
        grid.attach(&grid_title, 0, 0, 1, 1);

        // Add the top separator for the item list
        let items_separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        items_separator.set_halign(gtk::Align::Fill);
        items_separator.set_hexpand(true);
        grid.attach(&items_separator, 0, 1, 1, 1);

        // Create the item list that holds the buttons with all item data
        let item_list = ItemList::new(system_send);

        // Add the item list to the grid on the left
        grid.attach(item_list.get_top_element(), 0, 2, 1, 2);

        // Create the grid that holds all the edit item options
        let edit_grid = gtk::Grid::new();

        // Create the edit scene window and attach it to the edit grid
        let edit_scene = EditScene::new(system_send);
        edit_grid.attach(edit_scene.get_top_element(), 1, 3, 2, 1);

        // Create the edit title
        let edit_title = gtk::Label::new(Some("  Edit Selected Item  "));
        edit_title.set_size_request(-1, 30);
        grid.attach(&edit_title, 1, 0, 1, 1);

        // Connect the drag destination to edit_title
        edit_title.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Wrap the edit scene window
        let edit_scene = Rc::new(RefCell::new(edit_scene));

        // Set the callback function when data is received
        let current_id = Rc::new(RefCell::new(None));
        edit_title.connect_drag_data_received(clone!(system_send, current_id, edit_scene => move |_, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Try to update the current id
                if let Ok(mut current_id) = current_id.try_borrow_mut() {
                    *current_id = Some(item_pair.get_id());
                }

                // Borrow, unwrap, and pass the current id to the edit scene window
                let scene = match edit_scene.try_borrow_mut() {
                    Ok(mut scene) => scene.load_info(current_id.clone()),
                    _ => return,
                };

                // Refresh the current data
                EditItemAbstraction::refresh(item_pair.get_id(), &system_send)
            }
        }));

        // Add the top separator
        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        separator.set_halign(gtk::Align::Fill);
        separator.set_hexpand(true);
        grid.attach(&separator, 1, 1, 2, 1);

        // Create the scrollable window for the edit item fields
        let edit_window = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types

        // Set the scrollable window to scroll up/down
        edit_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Add the scrollable window to the grid
        grid.attach(&edit_window, 1, 2, 2, 1);

        // Add the edit grid as a child of the scrollable window
        edit_window.add(&edit_grid);

        // Format the scrolling window
        edit_window.set_hexpand(true);
        edit_window.set_vexpand(true);
        edit_window.set_halign(gtk::Align::Fill);
        edit_window.set_valign(gtk::Align::Fill);

        // Create the edit overview and add it to the edit grid
        let edit_overview = EditOverview::new();
        edit_grid.attach(edit_overview.get_top_element(), 1, 0, 2, 1);

        // Add the event separator
        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        separator.set_halign(gtk::Align::Fill);
        separator.set_hexpand(true);
        edit_grid.attach(&separator, 1, 1, 2, 1);

        // Create the edit detail and add it to the grid
        let edit_detail = EditDetail::new(system_send);
        edit_grid.attach(edit_detail.get_top_element(), 1, 2, 2, 1);


        // Create the save button
        let save = gtk::Button::new_with_label("  Save Changes  ");
        grid.attach(&save, 2, 0, 1, 1);

        // Connect the save button click callback
        let edit_overview = Rc::new(RefCell::new(edit_overview));
        let edit_detail = Rc::new(RefCell::new(edit_detail));
        save.connect_clicked(clone!(system_send, current_id, edit_overview, edit_detail => move |_| {
            // Try to borrow the the current id
            let current_id = match current_id.try_borrow() {
                Ok(id) => id,
                _ => return,
            };

            // Try to borrow the edit overview
            let overview = match edit_overview.try_borrow() {
                Ok(overview) => overview,
                _ => return,
            };

            // Try to borrow the edit detail
            let detail = match edit_detail.try_borrow() {
                Ok(detail) => detail,
                _ => return,
            };

            // Check to make sure there is a current id
            let item_id = match *current_id {
                Some(id) => id,
                _ => return, // FIXME warn the user
            };

            // Collect the information and save it
            let item_pair = ItemPair::from_item(item_id, overview.pack_description());
            let mut modifications = vec!(Modification::ModifyItem { item_pair });

            // If the detail was provided, update it
            if let Some(event_detail) = detail.pack_detail() {
                modifications.push(Modification::ModifyEvent { item_id, event_detail });
            }

            // Save the edit to the configuration
            system_send.send(Edit { modifications });

            // Refresh from the underlying data (will process after the save)
            EditItemAbstraction::refresh(item_id, &system_send);
        }));




        // Add some space on all the sides and show the components
        grid.set_column_spacing(100); // Add some space between the left and right
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);

        // Return the new Control Abstraction
        EditItemAbstraction {
            grid,
            system_send: system_send.clone(),
            interface_send: interface_send.clone(),
            current_id,
            item_list,
            edit_overview,
            edit_detail,
            edit_scene,
        }
    }

    /// A method to return a reference to the top element of the interface,
    /// currently grid.
    ///
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    /// A method to load a new item into the edit item window
    ///
    pub fn load_item(&mut self, id: Option<ItemId>) {
        // Change the current item id
        match self.current_id.try_borrow_mut() {
            Ok(mut current_id) => *current_id = id,
            _ => return,
        }

        // Refresh all the item components
        if let Some(item_id) = id {
            EditItemAbstraction::refresh(item_id, &self.system_send);
        }
    }

    // A function to refresh the components of the current item
    //
    fn refresh(item_id: ItemId, system_send: &SystemSend) {
        // Request new data for each component
        system_send.send(Request {
            reply_to: DisplayComponent::EditItemOverview,
            request: RequestType::Description { item_id },
        });
        system_send.send(Request {
            reply_to: DisplayComponent::EditItemOverview,
            request: RequestType::Detail { item_id },
        });
    }

    /// A method to process information updates received from the system
    ///
    pub fn update_info(&self, reply_to: DisplayComponent, reply: ReplyType) {
        // Unpack reply_to
        match reply_to {
            // Unpack the reply
            DisplayComponent::EditItemOverview => {
                match reply {
                    // The description variant
                    ReplyType::Description { description } => {
                        // Try to borrow the edit overview
                        if let Ok(overview) = self.edit_overview.try_borrow() {
                            overview.load_description(description);
                        }
                    }

                    // The detail variant
                    ReplyType::Detail { event_detail } => {
                        // Try to borrow the edit detail
                        if let Ok(detail) = self.edit_detail.try_borrow() {
                            detail.load_detail(event_detail);
                        }
                    }

                    _ => {
                        unreachable!();
                    }
                }
            }

            DisplayComponent::EditAction => {
                if let ReplyType::Status { status_detail } = reply {
                    // Try to borrow the edit detail
                    if let Ok(detail) = self.edit_detail.try_borrow() {
                        detail.update_info(status_detail);
                    }
                }
            }

            DisplayComponent::ItemList => {
                if let ReplyType::Items { items } = reply {
                    self.item_list.update_info(items);
                }
            }

            DisplayComponent::EditScene => {
                if let ReplyType::Scene { scene } = reply {
                    // Try to borrow the edit scene detail
                    if let Ok(scene_detail) = self.edit_scene.try_borrow() {
                        scene_detail.update_info(scene);
                    }
                }
            }

            _ => unreachable!(),
        }
    }
}

// Create a structure for listing all items
#[derive(Clone, Debug)]
struct ItemList {
    grid: gtk::Grid,              // the main grid for this element
    system_send: SystemSend,      // a copy of the system send line
}

// Implement key features of the ItemList
impl ItemList {
    /// A function to create a new ItemList
    ///
    fn new(system_send: &SystemSend,) -> ItemList {
        // Get all items in the configuration
        system_send.send(Request {
            reply_to: DisplayComponent::ItemList,
            request: RequestType::Items,
        });

        // Add the top-level grid
        let grid = gtk::Grid::new();

        ItemList {
            grid,
            system_send: system_send.clone(),
        }
    }

    /// A method to make a button for each item in the configuration file
    ///
    fn update_info(&self, items: Vec<ItemPair>) {
        // Create the scrolling window that contains the list box
        let items_scroll = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        items_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window and attach it to the grid
        items_scroll.set_hexpand(true);
        items_scroll.set_vexpand(true);
        items_scroll.set_halign(gtk::Align::Fill);
        items_scroll.set_valign(gtk::Align::Fill);
        self.grid.attach(&items_scroll, 0, 1, 1, 1);

        // Create the list box to hold the buttons with the item data and add it to the scrolling window
        let items_list_box = gtk::ListBox::new();
        items_scroll.add(&items_list_box);

        // Iterate through the item pairs in the items vector
        for item_pair in items {
            // Create the button to hold the data
            let item_button = gtk::Button::new_with_label(&item_pair.description());

            // Make the button a drag source
            item_button.drag_source_set(
                gdk::ModifierType::MODIFIER_MASK,
                &vec![
                    gtk::TargetEntry::new("STRING", gtk::TargetFlags::SAME_APP, 0),
                ],
                gdk::DragAction::COPY,
            );

            // Serialize the item pair data
            item_button.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                if let Ok(data) = serde_yaml::to_string(&item_pair) {
                    selection_data.set_text(data.as_str());
                }
            }));

            // Add the button to the list box
            items_list_box.add(&item_button);
        }
        // Show all the buttons in the grid
        self.grid.show_all();
    }

    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }
}


// Create a structure for editing the item description of the item
#[derive(Clone, Debug)]
struct EditOverview {
    grid: gtk::Grid,                      // the main grid for this element
    description: gtk::Entry,              // the description of the item
    display_type: gtk::ComboBoxText,      // the display type selection for the event
    group_checkbox: gtk::CheckButton,     // the checkbox for group id
    group: gtk::SpinButton,               // the spin selection for the group id
    position_checkbox: gtk::CheckButton,  // the position checkbox
    position: gtk::SpinButton,            // the spin selection for position
    color_checkbox: gtk::CheckButton,     // the color checkbox
    color: gtk::ColorButton,              // the color selection button
    highlight_checkbox: gtk::CheckButton, // the highlight checkbox
    highlight: gtk::ColorButton,          // the highlight selection button
    spotlight_checkbox: gtk::CheckButton, // the spotlight checkbox
    spotlight: gtk::SpinButton,           // the spin selection for spotlight number
    highstate_checkbox: gtk::CheckButton, // the highlight state checkbox
    highstate_status: gtk::SpinButton,    // the highlight state status spin
    highstate_state: gtk::SpinButton,     // the highlight state state spin
}

// Implement key features of the Edit Overview
impl EditOverview {
    /// A function to create a new edit overview
    ///
    fn new() -> EditOverview {
        // Create the entry for the item description
        let overview_label = gtk::Label::new(Some("Item Description:"));
        let description = gtk::Entry::new();
        description.set_placeholder_text(Some("Enter Item Description Here"));

        // Add the display type dropdown
        let display_type_label = gtk::Label::new(Some("Where to Display Item:"));
        let display_type = gtk::ComboBoxText::new();
        display_type.append(
            Some("displaycontrol"),
            "Event, Control: Appears as event in control area when available",
        );
        display_type.append(
            Some("displaywith"),
            "Event, General: Appears as event in general area when available",
        );
        display_type.append(
            Some("labelcontrol"),
            "Status, Control: Appears as status in control area when available",
        );
        display_type.append(
            Some("labelhidden"),
            "Status, General: Appears as status in general area when available",
        );
        display_type.append(
            Some("displaydebug"),
            "Debug Mode: Only appears as status or event in debug mode",
        );
        display_type.append(
            Some("hidden"),
            "Hide Item: Event not visible, status hidden when possible",
        );

        // Create the group spin options
        let group_checkbox = gtk::CheckButton::new_with_label("Show In Control Area");
        let group_label = gtk::Label::new(None);
        group_label.set_markup("Group Number:");
        let group = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Set up the spin button to receive a dropped event id
        group.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        group.connect_drag_data_received(|widget, _, _, _, selection_data, _, _| {
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

        // Connect the checkbox callback function to change label text
        group_checkbox.connect_toggled(clone!(group_label => move | checkbox | {
            // Strikethrough the text when checkbox is selected
            if checkbox.get_active() {
                group_label.set_markup("<s>Group Number:</s>");
            } else {
                group_label.set_markup("Group Number:");
            }
        }));

        // Create the position option
        let position_checkbox = gtk::CheckButton::new_with_label("Display Position");
        let position_label = gtk::Label::new(None);
        position_label.set_markup("<s>Position Number:</s>");
        let position = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        position_checkbox.connect_toggled(clone!(position_label => move | checkbox | {
            // Strikethrough the text when checkbox not selected
            if checkbox.get_active() {
                position_label.set_markup("Position Number:");
            } else {
                position_label.set_markup("<s>Position Number:</s>");
            }
        }));

        // Create the color option
        let color_checkbox = gtk::CheckButton::new_with_label("Custom Text Color");
        let color_label = gtk::Label::new(None);
        color_label.set_markup("<s>Select Color:</s>");
        let color = gtk::ColorButton::new();
        color.set_title("Text Color");
        color_checkbox.connect_toggled(clone!(color_label => move | checkbox | {
            // Strikethrough the text when checkbox not selected
            if checkbox.get_active() {
                color_label.set_markup("Select Color:");
            } else {
                color_label.set_markup("<s>Select Color:</s>");
            }
        }));

        // Create the highlight option
        let highlight_checkbox = gtk::CheckButton::new_with_label("Custom Text Highlight");
        let highlight_label = gtk::Label::new(None);
        highlight_label.set_markup("<s>Select Color:</s>");
        let highlight = gtk::ColorButton::new();
        highlight.set_title("Text Highlight Color");
        highlight_checkbox.connect_toggled(clone!(highlight_label => move | checkbox | {
            // Strikethrough the text when checkbox not selected
            if checkbox.get_active() {
                highlight_label.set_markup("Select Color:");
            } else {
                highlight_label.set_markup("<s>Select Color:</s>");
            }
        }));

        // Create the spotlight option
        let spotlight_checkbox = gtk::CheckButton::new_with_label("Spotlight Changes");
        let spotlight_label = gtk::Label::new(None);
        spotlight_label.set_markup("<s>Flash Cycles:</s>");
        let spotlight = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        spotlight_checkbox.connect_toggled(clone!(spotlight_label => move | checkbox | {
            // Strikethrough the text when checkbox not selected
            if checkbox.get_active() {
                spotlight_label.set_markup("Flash Cycles:");
            } else {
                spotlight_label.set_markup("<s>Flash Cycles:</s>");
            }
        }));

        // Create the highlight state options
        let highstate_checkbox = gtk::CheckButton::new_with_label("Status-Based Highlighting");
        let highstate_status = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let status_label = gtk::Label::new(None);
        status_label.set_markup("<s>Status Number:</s>");
        let highstate_state = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let state_label = gtk::Label::new(None);
        state_label.set_markup("<s>State Number:</s>");
        highstate_checkbox.connect_toggled(clone!(status_label, state_label => move | checkbox | {
            // Strikethrough the text when checkbox not selected
            if checkbox.get_active() {
                status_label.set_markup("Status Number:");
                state_label.set_markup("State Number:");
            } else {
                status_label.set_markup("<s>Status Number:</s>");
                state_label.set_markup("<s>State Number:</s>");
            }
        }));

        // Set up the hightlight status spin button to receive a dropped event id
        highstate_status.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        highstate_status.connect_drag_data_received(|widget, _, _, _, selection_data, _, _| {
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

        // Set up the hightlight state spin button to receive a dropped event id
        highstate_state.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        highstate_state.connect_drag_data_received(|widget, _, _, _, selection_data, _, _| {
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

        // Compose the display grid
        let display_grid = gtk::Grid::new();
        display_grid.attach(&group_checkbox, 0, 0, 1, 1);
        display_grid.attach(&group_label, 1, 0, 1, 1);
        display_grid.attach(&group, 2, 0, 1, 1);
        display_grid.attach(&position_checkbox, 0, 1, 1, 1);
        display_grid.attach(&position_label, 1, 1, 1, 1);
        display_grid.attach(&position, 2, 1, 1, 1);
        display_grid.attach(&color_checkbox, 0, 2, 1, 1);
        display_grid.attach(&color_label, 1, 2, 1, 1);
        display_grid.attach(&color, 2, 2, 1, 1);
        display_grid.attach(&highlight_checkbox, 0, 3, 1, 1);
        display_grid.attach(&highlight_label, 1, 3, 1, 1);
        display_grid.attach(&highlight, 2, 3, 1, 1);
        display_grid.attach(&spotlight_checkbox, 0, 4, 1, 1);
        display_grid.attach(&spotlight_label, 1, 4, 1, 1);
        display_grid.attach(&spotlight, 2, 4, 1, 1);
        display_grid.attach(&highstate_checkbox, 0, 5, 1, 1);
        display_grid.attach(&status_label, 1, 5, 1, 1);
        display_grid.attach(&highstate_status, 2, 5, 1, 1);
        display_grid.attach(&state_label, 1, 6, 1, 1);
        display_grid.attach(&highstate_state, 2, 6, 1, 1);
        display_grid.set_column_spacing(10); // Add some space
        display_grid.set_row_spacing(10);
        display_grid.set_halign(gtk::Align::End);

        // Connect the function to trigger display type changes
        display_type.connect_changed(clone!(
            group_checkbox,
            group_label,
            group,
            position_checkbox,
            position_label,
            position,
            color_checkbox,
            color_label,
            color,
            highlight_checkbox,
            highlight_label,
            highlight,
            spotlight_checkbox,
            spotlight_label,
            spotlight,
            highstate_checkbox,
            highstate_status,
            status_label,
            highstate_state,
            state_label
        => move |dropdown| {
            // Identify the selected detail type
            if let Some(display_type) = dropdown.get_active_id() {
                // Match the selection and change the visible options
                match display_type.as_str() {
                    // the DisplayControl variant
                    "displaycontrol" => {
                        group_checkbox.hide();
                        group_label.hide();
                        group.hide();
                        position_checkbox.show();
                        position_label.show();
                        position.show();
                        color_checkbox.show();
                        color_label.show();
                        color.show();
                        highlight_checkbox.show();
                        highlight_label.show();
                        highlight.show();
                        spotlight_checkbox.show();
                        spotlight_label.show();
                        spotlight.show();
                        highstate_checkbox.show();
                        highstate_status.show();
                        status_label.show();
                        highstate_state.show();
                        state_label.show();
                    }

                    // the DisplayWith variant
                    "displaywith" => {
                        group_checkbox.hide();
                        group_label.show();
                        group_label.set_markup("Group Number:");
                        group.show();
                        position_checkbox.show();
                        position_label.show();
                        position.show();
                        color_checkbox.show();
                        color_label.show();
                        color.show();
                        highlight_checkbox.show();
                        highlight_label.show();
                        highlight.show();
                        spotlight_checkbox.show();
                        spotlight_label.show();
                        spotlight.show();
                        highstate_checkbox.show();
                        highstate_status.show();
                        status_label.show();
                        highstate_state.show();
                        state_label.show();
                    }

                    // the DisplayDebug variant
                    "displaydebug" => {
                        group_checkbox.show();
                        group_label.show();
                        if group_checkbox.get_active() {
                            group_label.set_markup("<s>Group Number:</s>");
                        }
                        group.show();
                        position_checkbox.show();
                        position_label.show();
                        position.show();
                        color_checkbox.show();
                        color_label.show();
                        color.show();
                        highlight_checkbox.show();
                        highlight_label.show();
                        highlight.show();
                        spotlight_checkbox.show();
                        spotlight_label.show();
                        spotlight.show();
                        highstate_checkbox.show();
                        highstate_status.show();
                        status_label.show();
                        highstate_state.show();
                        state_label.show();
                    }

                    // the LabelControl variant
                    "labelcontrol" => {
                        group_checkbox.hide();
                        group_label.hide();
                        group.hide();
                        position_checkbox.show();
                        position_label.show();
                        position.show();
                        color_checkbox.show();
                        color_label.show();
                        color.show();
                        highlight_checkbox.show();
                        highlight_label.show();
                        highlight.show();
                        spotlight_checkbox.show();
                        spotlight_label.show();
                        spotlight.show();
                        highstate_checkbox.show();
                        highstate_status.show();
                        status_label.show();
                        highstate_state.show();
                        state_label.show();
                    }

                    // the LabelHidden variant
                    "labelhidden" => {
                        group_checkbox.hide();
                        group_label.hide();
                        group.hide();
                        position_checkbox.show();
                        position_label.show();
                        position.show();
                        color_checkbox.show();
                        color_label.show();
                        color.show();
                        highlight_checkbox.show();
                        highlight_label.show();
                        highlight.show();
                        spotlight_checkbox.show();
                        spotlight_label.show();
                        spotlight.show();
                        highstate_checkbox.show();
                        highstate_status.show();
                        status_label.show();
                        highstate_state.show();
                        state_label.show();
                    }

                    // the Hidden variant
                    _ => {
                        group_checkbox.hide();
                        group_label.hide();
                        group.hide();
                        position_checkbox.hide();
                        position_label.hide();
                        position.hide();
                        color_checkbox.hide();
                        color_label.hide();
                        color.hide();
                        highlight_checkbox.hide();
                        highlight_label.hide();
                        highlight.hide();
                        spotlight_checkbox.hide();
                        spotlight_label.hide();
                        spotlight.hide();
                        highstate_checkbox.hide();
                        highstate_status.hide();
                        status_label.hide();
                        highstate_state.hide();
                        state_label.hide();
                    }
                }
            }
        }));

        // Create the edit overview grid and populate it
        let grid = gtk::Grid::new();
        grid.attach(&overview_label, 0, 0, 1, 1);
        grid.attach(&description, 1, 0, 2, 1);
        grid.attach(&display_type_label, 0, 1, 1, 1);
        grid.attach(&display_type, 1, 1, 2, 1);
        grid.attach(&display_grid, 0, 2, 3, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the edit overview
        EditOverview {
            grid,
            description,
            display_type,
            group_checkbox,
            group,
            position_checkbox,
            position,
            color_checkbox,
            color,
            highlight_checkbox,
            highlight,
            spotlight_checkbox,
            spotlight,
            highstate_checkbox,
            highstate_status,
            highstate_state,
        }
    }

    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load an item description into the edit overview
    //
    fn load_description(&self, description: ItemDescription) {
        // Update the event description
        self.description.set_text(&description.description);

        // Create default placeholders for the display settings
        let mut new_group = None;
        let mut new_position = None;
        let mut new_color = None;
        let mut new_highlight = None;
        let mut new_highlight_state = None;
        let mut new_spotlight = None;

        // Update the display type for the event
        match description.display {
            // the DisplayControl variant
            DisplayControl {
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            } => {
                // Change the visible options
                self.display_type.set_active_id(Some("displaycontrol"));

                // Save the available elements
                new_position = position;
                new_color = color;
                new_highlight = highlight;
                new_highlight_state = highlight_state;
                new_spotlight = spotlight;
            }

            // the DisplayWith variant
            DisplayWith {
                group_id,
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            } => {
                // Change the visible options
                self.display_type.set_active_id(Some("displaywith"));

                // Save the available elements
                new_group = Some(group_id);
                new_position = position;
                new_color = color;
                new_highlight = highlight;
                new_highlight_state = highlight_state;
                new_spotlight = spotlight;
            }

            // The DisplayDebug variant
            DisplayDebug {
                group,
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            } => {
                // Change the visible options
                self.display_type.set_active_id(Some("displaydebug"));

                // Save the available elements
                new_group = group;
                new_position = position;
                new_color = color;
                new_highlight = highlight;
                new_highlight_state = highlight_state;
                new_spotlight = spotlight;
            }

            // the LabelControl variant
            LabelControl {
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            } => {
                // Change the visible options
                self.display_type.set_active_id(Some("labelcontrol"));

                // Save the available elements
                new_position = position;
                new_color = color;
                new_highlight = highlight;
                new_highlight_state = highlight_state;
                new_spotlight = spotlight;
            }

            // the LabelHidden variant
            LabelHidden {
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            } => {
                // Change the visible options
                self.display_type.set_active_id(Some("labelhidden"));

                // Save the available elements
                new_position = position;
                new_color = color;
                new_highlight = highlight;
                new_highlight_state = highlight_state;
                new_spotlight = spotlight;
            }

            // the Hidden variant
            Hidden => {
                self.display_type.set_active_id(Some("hidden"));
            }
        }

        // If there is a new group id, set it
        match new_group {
            None => self.group_checkbox.set_active(true),
            Some(id) => {
                self.group_checkbox.set_active(false);
                self.group.set_value(id.id() as f64);
            }
        }

        // If there is a new position, set it
        match new_position {
            None => self.position_checkbox.set_active(false),
            Some(number) => {
                self.position_checkbox.set_active(true);
                self.position.set_value(number as f64);
            }
        }

        // If there is a new color, set it
        match new_color {
            None => self.color_checkbox.set_active(false),
            Some((new_red, new_green, new_blue)) => {
                self.color_checkbox.set_active(true);
                let tmp_color = gdk::RGBA {
                    red: new_red as f64 / 255.0,
                    green: new_green as f64 / 255.0,
                    blue: new_blue as f64 / 255.0,
                    alpha: 1.0,
                };
                self.color.set_rgba(&tmp_color);
            }
        }

        // If there is a new highlight, set it
        match new_highlight {
            None => self.highlight_checkbox.set_active(false),
            Some((new_red, new_green, new_blue)) => {
                self.highlight_checkbox.set_active(true);
                let tmp_color = gdk::RGBA {
                    red: new_red as f64 / 255.0,
                    green: new_green as f64 / 255.0,
                    blue: new_blue as f64 / 255.0,
                    alpha: 1.0,
                };
                self.highlight.set_rgba(&tmp_color);
            }
        }

        // If there is a new highlight state, set it
        match new_highlight_state {
            None => self.highstate_checkbox.set_active(false),
            Some((new_status, new_state)) => {
                self.highstate_checkbox.set_active(true);
                self.highstate_status.set_value(new_status.id() as f64);
                self.highstate_state.set_value(new_state.id() as f64);
            }
        }

        // If there is a new spotlight, set it
        match new_spotlight {
            None => self.spotlight_checkbox.set_active(false),
            Some(number) => {
                self.spotlight_checkbox.set_active(true);
                self.spotlight.set_value(number as f64);
            }
        }
    }

    // A method to pack the item description
    //
    fn pack_description(&self) -> ItemDescription {
        // Create the new item description
        let tmp_description = self.description.get_text().unwrap_or(String::new().into());

        // Create default placeholders for the display settings
        let mut group = None;
        let group_id = ItemId::new_unchecked(self.group.get_value() as u32);
        let mut position = None;
        let mut color = None;
        let mut highlight = None;
        let mut highlight_state = None;
        let mut spotlight = None;

        // Extract the group id, if selected
        if !self.group_checkbox.get_active() {
            group = Some(group_id);
        }

        // Extract the position, if selected
        if self.position_checkbox.get_active() {
            position = Some(self.position.get_value() as u32);
        }

        // Extract the color, if selected
        if self.color_checkbox.get_active() {
            let gdk::RGBA {
                red, green, blue, ..
            } = self.color.get_rgba();
            color = Some((
                (red * 255.0) as u8,
                (green * 255.0) as u8,
                (blue * 255.0) as u8,
            ));
        }

        // Extract the highlight, if selected
        if self.highlight_checkbox.get_active() {
            let gdk::RGBA {
                red, green, blue, ..
            } = self.highlight.get_rgba();
            highlight = Some((
                (red * 255.0) as u8,
                (green * 255.0) as u8,
                (blue * 255.0) as u8,
            ));
        }

        // Extract the highlight state, if selected
        if self.highstate_checkbox.get_active() {
            highlight_state = Some((
                ItemId::new_unchecked(self.highstate_status.get_value() as u32),
                ItemId::new_unchecked(self.highstate_state.get_value() as u32),
            ));
        }

        // Extract the spotlight, if selected
        if self.spotlight_checkbox.get_active() {
            spotlight = Some(self.spotlight.get_value() as u32);
        }

        // Get the current display type
        let display_type = self
            .display_type
            .get_active_id()
            .unwrap_or(String::from("hidden").into());

        // Collect information based on the display type
        let tmp_display = match display_type.as_str() {
            // Compose the DisplayControl type
            "displaycontrol" => DisplayControl {
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            },

            // Compose the DisplayWith type
            "displaywith" => DisplayWith {
                group_id,
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            },

            // Compose the DisplayDebug type
            "displaydebug" => DisplayDebug {
                group,
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            },

            // Compose the LabelControl type
            "labelcontrol" => LabelControl {
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            },

            // Compose the LabelHidden type
            "labelhidden" => LabelHidden {
                position,
                color,
                highlight,
                highlight_state,
                spotlight,
            },

            // For the hidden type
            _ => Hidden,
        };

        // Return the complete Item Description
        ItemDescription::new(&tmp_description, tmp_display)
    }
}

// Create a structure for editing the event detail
#[derive(Clone, Debug)]
struct EditDetail {
    grid: gtk::Grid,                   // the main grid for this element
    edit_action: Rc<RefCell<EditAction>>, // a wrapped dialog to edit the current action
    detail_checkbox: gtk::CheckButton, // the checkbox to indicate an active event
    event_actions: Rc<RefCell<FnvHashMap<usize, EventAction>>>, // a wrapped hash map of actions (may be empty)
    next_position: Rc<RefCell<usize>>, // the next available position in the hash map
    action_list: gtk::ListBox,         // the visible list for event actions
}

// Implement key features for Edit Detail
impl EditDetail {
    // A function to create a new Edit Detail
    //
    fn new(system_send: &SystemSend) -> EditDetail {
        // Create the grid
        let grid = gtk::Grid::new();

        // Construct the checkbox for the event detail
        let detail_checkbox = gtk::CheckButton::new_with_label("Item Corresponds To An Event");
        detail_checkbox.set_active(true);

        // Create the empty event actions
        let event_actions = Rc::new(RefCell::new(FnvHashMap::default()));

        // Create the starting next position
        let next_position = Rc::new(RefCell::new(0));

        // Create the action list for the events
        let action_list = gtk::ListBox::new();
        action_list.set_selection_mode(gtk::SelectionMode::None);

        // Create a new edit action dialog
        let tmp_edit_action = EditAction::new(system_send, &event_actions);
        grid.attach(tmp_edit_action.get_top_element(), 1, 1, 1, 2);
        let edit_action = Rc::new(RefCell::new(tmp_edit_action));

        // Create a button to add actions to the list
        let add_button = gtk::Button::new_from_icon_name(
            Some("list-add-symbolic"),
            gtk::IconSize::Button.into(),
        );
        add_button.connect_clicked(
            clone!(edit_action, event_actions, next_position, action_list => move |_| {
                // Add an empty action to the list
                EditDetail::add_event(&edit_action, &event_actions, &next_position, &action_list, None);
            }),
        );

        // Create the scrollable window for the list
        let action_window = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        action_window.add(&action_list);
        action_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        action_window.set_hexpand(true);
        action_window.set_size_request(-1, 150);

        // Connect the checkbox to the visibility of the other elements
        detail_checkbox.connect_toggled(clone!(action_window, add_button => move | checkbox | {
            // Make the elements invisible if the box isn't checked
            if checkbox.get_active() {
                action_window.show();
                add_button.show();
            } else {
                action_window.hide();
                add_button.hide();
            }
        }));

        // Add the button below the data list
        grid.attach(&detail_checkbox, 0, 0, 1, 1);
        grid.attach(&action_window, 0, 1, 1, 1);
        grid.attach(&add_button, 0, 2, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the trigger events variant
        EditDetail {
            grid,
            edit_action,
            detail_checkbox,
            event_actions,
            next_position,
            action_list,
        }
    }

    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load an existing event detail, or no detail
    //
    fn load_detail(&self, event_detail: Option<EventDetail>) {
        // See if a detail was specified
        let mut detail = match event_detail {
            // If a detail was specified, switch the checkbox
            Some(detail) => {
                self.detail_checkbox.set_active(true);
                detail
            }

            // Otherwise, uncheck the checkbox and return
            None => {
                self.detail_checkbox.set_active(false);
                return;
            }
        };

        // Remove the existing event actions
        if let Ok(mut actions) = self.event_actions.try_borrow_mut() {
            actions.clear();
        }

        // Clear the existing list of actions
        for item in self.action_list.get_children() {
            item.destroy();
        }

        // For each event action, create a new action in the list
        for action in detail.drain(..) {
            EditDetail::add_event(
                &self.edit_action,
                &self.event_actions,
                &self.next_position,
                &self.action_list,
                Some(action),
            );
        }
    }

    // A method to pack the listed actions into an event detail
    //
    fn pack_detail(&self) -> Option<EventDetail> {

        // If the checkbox was not selected, return None
        if !self.detail_checkbox.get_active() {
            return None;
        }

        // Access the current event detail
        match self.event_actions.try_borrow_mut() {
            // Return the current event detail, composed as a vector
            Ok(detail) => {
                // Create a vector to hold the actions and a counter
                let mut actions = Vec::new();
                let mut count = 0;

                while actions.len() < detail.len() {
                    // Try to get each element, zero indexed
                    if let Some(action) = detail.get(&count) {
                        actions.push(action.clone());
                    }

                    // Increment the count
                    count = count + 1;
                }

                // Return the completed vector
                Some(actions)
            }

            // Unreachable
            _ => unreachable!(),
        }
    }

    // A method to load new information into the edit action dialog
    //
    fn update_info(&self, status_detail: Option<StatusDetail>) {
        // Try to get access the edit action dialog
        if let Ok(dialog) = self.edit_action.try_borrow() {
            dialog.update_info(status_detail);
        }
    }

    // A helper function to add an action to the action list
    //
    fn add_event(
        edit_action: &Rc<RefCell<EditAction>>,
        event_actions: &Rc<RefCell<FnvHashMap<usize, EventAction>>>,
        next_position: &Rc<RefCell<usize>>,
        action_list: &gtk::ListBox,
        action: Option<EventAction>,
    ) {
        // Try to get a mutable copy of the event actions
        let mut actions = match event_actions.try_borrow_mut() {
            Ok(actions) => actions,

            // If unable, exit immediately
            _ => return,
        };

        // Try to get a mutable copy of the next_position
        let position = match next_position.try_borrow_mut() {
            Ok(mut position) => {
                let tmp = position.clone();
                *position = *position + 1;
                tmp
            }

            // If unable, exit immediately
            _ => return,
        };

        // Create and populate the information-holding label
        let overview = gtk::Label::new(Some("Unspecified Action"));
        if let Some(action) = action {
            // Add a copy of the action to the detail
            actions.insert(position, action.clone());

            // Unpack the action
            match action {
                EventAction::NewScene { .. } => overview.set_text("New Scene"),
                EventAction::ModifyStatus { .. } => overview.set_text("Modify Status"),
                EventAction::QueueEvent { .. } => overview.set_text("Queue Event"),
                EventAction::CancelEvent { .. } => overview.set_text("Cancel Event"),
                EventAction::SaveData { .. } => overview.set_text("Save Data"),
                EventAction::SendData { .. } => overview.set_text("Send Data"),
                EventAction::GroupedEvent { .. } => overview.set_text("Grouped Event"),
            }

        // Default to a new scene action
        } else {
            actions.insert(
                position,
                EventAction::NewScene {
                    new_scene: ItemId::new_unchecked(0),
                },
            );
        }

        // Create the list box row container
        let row = gtk::ListBoxRow::new();

        // Create the edit button
        let edit_button = gtk::Button::new_from_icon_name(
            Some("document-edit-symbolic"),
            gtk::IconSize::Button.into(),
        );
        edit_button.connect_clicked(
            clone!(edit_action, position, overview, row => move |_| {
                // Try to get access to the edit action element
                if let Ok(edit_action) = edit_action.try_borrow() {
                    // Load the edit action element
                    edit_action.load_action(position, &overview, &row);
                }
            }),
        );

        // Create the grid and add the items to the grid
        let grid = gtk::Grid::new();
        grid.attach(&overview, 0, 0, 1, 1);
        grid.attach(&edit_button, 1, 0, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);
        row.add(&grid);
        row.show_all();

        // Add the new action to the list
        action_list.add(&row);
    }
}
