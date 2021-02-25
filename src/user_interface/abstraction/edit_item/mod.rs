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
mod edit_event;
mod edit_scene;
mod edit_status;

// Import the relevant structures into the correct namespace
use self::edit_event::EditEvent;
use self::edit_scene::EditScene;
use self::edit_status::EditStatus;
use crate::definitions::{
    DisplayComponent, DisplayControl, DisplayDebug, DisplayWith, Edit, EditItemElement,
    Hidden, InterfaceUpdate, ItemDescription, ItemId, ItemPair,
    LabelControl, LabelHidden, Modification, ReplyType, Request, RequestType, Status,
    SyncSystemSend,
};
use super::super::utils::{clean_text, color_label};

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

// Import the serde_yaml library
use serde_yaml;

// Import FNV Hash Set
use fnv::FnvHashSet;

// Import GTK and GDK libraries
use gdk;
use gtk;
use self::gtk::prelude::*;

// Define module constants
const LABEL_LIMIT: usize = 30; // maximum character width of labels
const ITEM_START: u32 = 1000; // starting number for new items


/// A structure to hold all editing components.
///
/// This structure consists of a list of all possible items,
/// and two copies of the edit item overviews for editing
/// individual items.
#[derive(Clone, Debug)]
pub struct EditWindow {
    scroll_window: gtk::ScrolledWindow,            // the scroll window to hold the underlying elements
    system_send: SyncSystemSend,              // a copy of the system send line
    item_list: ItemList,                  // the list of all possible items
    edit_item_left: EditItemAbstraction,   // the left overview to edit an item
    edit_item_right: EditItemAbstraction, // the right overview to edit an item
}

// Implement key features for the EditWindow
impl EditWindow {
    /// A function to create a new instance of the Edit Window. This
    /// function loads the default widgets into the interface and returns
    /// a new copy to allow insertion into higher-level elements.
    ///
    pub fn new(
        window: &gtk::ApplicationWindow,
        system_send: &SyncSystemSend,
        interface_send: &mpsc::Sender<InterfaceUpdate>,
    ) -> EditWindow {
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

        // Create a scrolling window to hold the top-level grid
        let scroll_window = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        scroll_window.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);

        // Format the scrolling window
        scroll_window.set_hexpand(true);
        scroll_window.set_vexpand(true);

        // Add the grid to the scroll window
        scroll_window.add(&grid);

        // Create the item list that holds the buttons with all item data
        let item_list = ItemList::new(system_send);

        // Create an EditItemAbstraction copy for the left side
        let edit_item_left = EditItemAbstraction::new(
            window,
            system_send,
            interface_send,
            true // left side
        );

        // Create an EditItemAbstraction copy for the right side
        let edit_item_right = EditItemAbstraction::new(
            window,
            system_send,
            interface_send,
            false // right side
        );

        // Attach the edit elements to the grid
        grid.attach(item_list.get_top_element(), 0, 0, 1, 2);
        grid.attach(edit_item_left.get_top_element(), 1, 0, 1, 1);
        grid.attach(edit_item_right.get_top_element(), 2, 0, 1, 1);
        grid.show();
        scroll_window.show();

        // Return a copy of the Edit window
        EditWindow {
            scroll_window,
            system_send: system_send.clone(),
            item_list,
            edit_item_left,
            edit_item_right,
        }
    }

    /// A method to return a reference to the top element of the interface,
    /// currently grid.
    ///
    pub fn get_top_element(&self) -> &gtk::ScrolledWindow {
        &self.scroll_window
    }

    // A function to refresh the entire edit window
    //
    pub fn refresh_all(&self) {
        // Refresh the available items
        self.system_send.send(Request {
            reply_to: DisplayComponent::ItemList,
            request: RequestType::Items,
        });

        // Try to get a copy of the current id for the left editor
        if let Ok(current_id) = self.edit_item_left.current_id.try_borrow() {
            // If a current id is specified
            if let Some(ref id) = *current_id {
               // Refresh the current item
               EditItemAbstraction::refresh_item(id.clone(), true, &self.system_send);
            }
        }

        // Try to get a copy of the current id for the right editor
        if let Ok(current_id) = self.edit_item_right.current_id.try_borrow() {
            // If a current id is specified
            if let Some(ref id) = *current_id {
               // Refresh the current item
               EditItemAbstraction::refresh_item(id.clone(), false, &self.system_send);
            }
        }
    }

    /// A method to process information updates received from the system
    ///
    pub fn update_info(&self, reply_to: DisplayComponent, reply: ReplyType) {
        // Unpack reply_to
        match reply_to.clone() {
            // Unpack the reply
            DisplayComponent::ItemList => {
                if let ReplyType::Items { items } = reply {
                    self.item_list.update_info(items);
                }
            }

            DisplayComponent::EditItemOverview { is_left, .. } => {
                match is_left {
                    // Send to the left side
                    true => self.edit_item_left.update_info(reply_to, reply),

                    // Send to the right side
                    false => self.edit_item_right.update_info(reply_to, reply),
                }
            }

            DisplayComponent::EditActionElement { is_left, .. } => {
                match is_left {
                    // Send to the left side
                    true => self.edit_item_left.update_info(reply_to, reply),

                    // Send to the right side
                    false => self.edit_item_right.update_info(reply_to, reply),
                }
            }

            DisplayComponent::EditMultiStateStatus { is_left, .. } => {
                match is_left {
                    // Send to the left side
                    true => self.edit_item_left.update_info(reply_to, reply),

                    // Send to the right side
                    false => self.edit_item_right.update_info(reply_to, reply),
                }
            }

            DisplayComponent::EditCountedStateStatus { is_left, .. } => {
                match is_left {
                    // Send to the left side
                    true => self.edit_item_left.update_info(reply_to, reply),

                    // Send to the right side
                    false => self.edit_item_right.update_info(reply_to, reply),
                }
            }
            _ => unreachable!(),
        }
    }

}




/// A structure to contain the the item editing funcitonality.
///
/// This structure automatically detects if an item corresponds to an event,
/// status, or scene (or a combination of those items) and allows the user to
/// modify all the details associated with that item.
#[derive(Clone, Debug)]
pub struct EditItemAbstraction {
    grid: gtk::Grid,                               // the grid to hold underlying elements
    system_send: SyncSystemSend,                   // a copy of the system send line
    interface_send: mpsc::Sender<InterfaceUpdate>, // a copy of the interface send line
    current_id: Rc<RefCell<Option<ItemId>>>,       // the wrapped current item id
    is_left: bool,                                 // whether the element is on the left or right
    item_description: gtk::Entry,                 // the description of the item being edited
    edit_overview: Rc<RefCell<EditOverview>>,      // the wrapped edit overview section
    edit_event: Rc<RefCell<EditEvent>>,            // the wrapped edit event section
    edit_scene: Rc<RefCell<EditScene>>,            // the wrapped edit scene section
    edit_status: Rc<RefCell<EditStatus>>,          // the wrapped edit status section
}

// Implement key features for the EditItemAbstraction
impl EditItemAbstraction {
    /// A function to create a new instance of the Edit Item Abstraction. This
    /// function loads all the default widgets into the interface and returns
    /// a new copy to allow insertion into higher-level elements.
    ///
    pub fn new(
        window: &gtk::ApplicationWindow,
        system_send: &SyncSystemSend,
        interface_send: &mpsc::Sender<InterfaceUpdate>,
        is_left: bool,
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

        // Create the grid that holds all the edit item options
        let edit_grid = gtk::Grid::new();

        // Create the edit scene window
        let edit_scene = EditScene::new(window);

        // Create the edit title
        let edit_title = gtk::Label::new(Some("Drop Item Here"));
        edit_title.set_size_request(-1, 30);

        // Connect the drag destination to edit_title
        drag!(dest edit_title);

        // Set the callback function when data is received
        let current_id = Rc::new(RefCell::new(None));
        edit_title.connect_drag_data_received(clone!(system_send, current_id, is_left => move |_, _, _, _, selection_data, _, _| {
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

                // Refresh the current data
                EditItemAbstraction::refresh_item(item_pair.get_id(), is_left, &system_send)
            }
        }));

        // Create the scrollable window for the edit item fields
        let edit_window = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types

        // Set the scrollable window to scroll up/down
        edit_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Add the edit grid as a child of the scrollable window
        edit_window.add(&edit_grid);

        // Format the scrolling window
        edit_window.set_hexpand(true);
        edit_window.set_vexpand(true);
        edit_window.set_halign(gtk::Align::Fill);
        edit_window.set_valign(gtk::Align::Fill);

        // Create the edit overview
        let edit_overview = EditOverview::new(system_send, is_left);

        // Create the edit event
        let edit_event = EditEvent::new(system_send, is_left);

        // Create the edit status
        let edit_status = EditStatus::new(system_send, is_left);

        // Create the save button
        let save = gtk::Button::with_label("  Save Changes  ");

        // Create the entry for the item description
        let overview_label = gtk::Label::new(Some("Item Description:"));
        let item_description = gtk::Entry::new();
        item_description.set_placeholder_text(Some("Enter Item Description Here"));

        // Attach all elements to the edit grid
        edit_grid.attach(edit_event.get_top_element(), 0, 0, 2, 1);
        edit_grid.attach(edit_scene.get_top_element(), 0, 1, 2, 1);
        edit_grid.attach(edit_status.get_top_element(), 0, 2, 2, 1);
        edit_grid.attach(edit_overview.get_top_element(), 0, 3, 2, 1);

        // Attach the edit window and other elements to the top-level grid
        grid.attach(&edit_title, 0, 0, 2, 1);
        grid.attach(&save, 2, 0, 2, 1);
        grid.attach(&overview_label, 0, 1, 1, 1);
        grid.attach(&item_description, 1, 1, 3, 1);
        grid.attach(&edit_window, 0, 2, 4, 1);
        edit_title.show();
        save.show();
        overview_label.show();
        item_description.show();
        edit_window.show();
        edit_grid.show();

        // Connect the save button click callback
        let edit_overview = Rc::new(RefCell::new(edit_overview));
        let edit_event = Rc::new(RefCell::new(edit_event));
        let edit_scene = Rc::new(RefCell::new(edit_scene));
        let edit_status = Rc::new(RefCell::new(edit_status));
        save.connect_clicked(clone!(
            system_send,
            current_id,
            item_description,
            edit_overview,
            edit_event,
            edit_scene,
            edit_status,
            is_left
        => move |_| {
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

            // Try to borrow the edit event
            let event = match edit_event.try_borrow() {
                Ok(event) => event,
                _ => return,
            };

            // Try to borrow the edit status
            let status = match edit_status.try_borrow() {
                Ok(status) => status,
                _ => return,
            };

            // Try to borrow the edit scene
            let scene = match edit_scene.try_borrow() {
                Ok(scene) => scene,
                _ => return,
            };

            // Check to make sure there is a current id
            let item_id = match *current_id {
                Some(id) => id,
                _ => return, // FIXME warn the user
            };

            // Get the text in the description entry box
            let tmp_description = item_description.get_text();

            // Collect the information and save it
            let item_pair = ItemPair::from_item(item_id, overview.pack_description(tmp_description.to_string()));
            let mut modifications = vec!(Modification::ModifyItem { item_pair });

            // Update (or delete) the event
            modifications.push(Modification::ModifyEvent {
                item_id,
                event: event.pack_event()
            });

            // Update (or delete) the status
            modifications.push(Modification::ModifyStatus {
                item_id,
                status: status.pack_status()
            });

            // Update (or delete) the scene
            modifications.push(Modification::ModifyScene {
                item_id,
                scene: scene.pack_scene()
            });

            // Save the edit to the configuration
            system_send.send(Edit { modifications });

            // Refresh the item list
            system_send.send(Request {
                reply_to: DisplayComponent::ItemList,
                request: RequestType::Items,
            });

            // Refresh the item
            EditItemAbstraction::refresh_item(item_id, is_left, &system_send);
        }));

        // Add some space on all the sides and show the components
        grid.set_column_spacing(100); // Add some space between the left and right
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        grid.show();

        // Return the new Control Abstraction
        EditItemAbstraction {
            grid,
            system_send: system_send.clone(),
            interface_send: interface_send.clone(),
            current_id,
            is_left,
            item_description,
            edit_overview,
            edit_event,
            edit_scene,
            edit_status,
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
    pub fn _load_item(&mut self, id: Option<ItemId>) {
        // Change the current item id
        match self.current_id.try_borrow_mut() {
            Ok(mut current_id) => *current_id = id,
            _ => return,
        }

        // Refresh all the item components
        if let Some(item_id) = id {
            EditItemAbstraction::refresh_item(item_id, self.is_left, &self.system_send);
        }
    }

    // A function to refresh the components of the current item
    //
    fn refresh_item(item_id: ItemId, is_left: bool, system_send: &SyncSystemSend) {
        // Request new data for each component
        system_send.send(Request {
            reply_to: DisplayComponent::EditItemOverview { is_left, variant: EditItemElement::ItemDescription },
            request: RequestType::Description { item_id },
        });
        system_send.send(Request {
            reply_to: DisplayComponent::EditItemOverview { is_left, variant: EditItemElement::Details },
            request: RequestType::Event { item_id },
        });
        system_send.send(Request {
            reply_to: DisplayComponent::EditItemOverview { is_left, variant: EditItemElement::Details },
            request: RequestType::Scene { item_id, },
        });
        system_send.send(Request {
            reply_to: DisplayComponent::EditItemOverview { is_left, variant: EditItemElement::Details },
            request: RequestType::Status { item_id, },
        });
    }

    /// A method to process information updates received from the system
    ///
    pub fn update_info(&self, reply_to: DisplayComponent, reply: ReplyType) {
        // Unpack reply_to
        match reply_to {
            // Unpack the reply
            DisplayComponent::EditItemOverview { variant, .. } => {
                match reply {
                    // The description variant
                    ReplyType::Description { description } => {
                        // If the type is an item description
                        if variant == EditItemElement::ItemDescription {
                            // Load the description into the text entry
                            self.item_description.set_text(&description.description);
                        }

                        // Try to borrow the edit overview
                        if let Ok(edit_overview) = self.edit_overview.try_borrow() {
                            edit_overview.load_description(variant, description);
                        }
                    }

                    // The event variant
                    ReplyType::Event { event } => {
                        // Try to borrow the edit event
                        if let Ok(edit_event) = self.edit_event.try_borrow() {
                            edit_event.load_event(event);
                        }
                    }

                    // The scene variant
                    ReplyType::Scene { scene } => {
                        // Try to borrow the edit scene
                        if let Ok(edit_scene) = self.edit_scene.try_borrow() {
                            edit_scene.update_info(scene);
                        }
                    }

                    // The status variant
                    ReplyType::Status { status } => {
                        // Match the variant
                        match variant {
                            // The details variant
                            EditItemElement::Details => {
                                // Try to borrow the edit status
                                if let Ok(mut edit_status) = self.edit_status.try_borrow_mut() {
                                    edit_status.load_status(status);
                                }
                            },

                            // The status variant
                            EditItemElement::Status { state } => {
                                // Try to borrow the edit overview
                                if let Ok(edit_overview) = self.edit_overview.try_borrow() {
                                    edit_overview.load_status(status, state);
                                }
                            }

                            _ => unreachable!(),
                        }
                    }

                    _ => {
                        unreachable!();
                    }
                }
            }

            DisplayComponent::EditActionElement { variant, .. } => {
                match reply {
                    // The status variant
                    ReplyType::Status { status } => {
                        // Try to borrow the edit event
                        if let Ok(edit_event) = self.edit_event.try_borrow() {
                            // Update the status info
                            edit_event.update_info(variant, status);
                        }
                    }

                    // The description variant
                    ReplyType::Description { description } => {
                        // Try to borrow the edit event
                        if let Ok(edit_event) = self.edit_event.try_borrow() {
                            // Update the description
                            edit_event.update_description(variant, description);
                        }
                    }

                    _ => unreachable!(),
                }
            }


            DisplayComponent::EditMultiStateStatus { position, .. } => {
                if let ReplyType::Description { description } = reply {
                    // Try to borrow the edit status
                    if let Ok(edit_status) = self.edit_status.try_borrow() {
                        edit_status.update_multistate_description(description, position);
                    }
                }
            }

            DisplayComponent::EditCountedStateStatus { state_type, .. } => {
                if let ReplyType::Description { description } = reply {
                    // Try to borrow the edit status
                    if let Ok(edit_status) = self.edit_status.try_borrow() {
                        edit_status.update_countedstate_description(description, state_type);
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
    grid: gtk::Grid,                        // the main grid for this element
    next_item: Rc<RefCell<u32>>,            // the number for the next-created item, not necessarily unique
    id_list: Rc<RefCell<FnvHashSet<u32>>>,  // the list of existing ids
    items_list: gtk::ListBox,               // the list of item buttons
}

// Implement key features of the ItemList
impl ItemList {
    /// A function to create a new ItemList
    ///
    fn new(system_send: &SyncSystemSend) -> ItemList {
        // Add the top-level grid
        let grid = gtk::Grid::new();
        grid.set_row_spacing(10); // add some internal space

        // Create the item list title and attach it to the grid
        let grid_title = gtk::Label::new(Some("  All Items  "));
        grid.attach(&grid_title, 0, 0, 1, 1);

        // Create the new item count tracker and id list
        let next_item = Rc::new(RefCell::new(ITEM_START));
        let id_list = Rc::new(RefCell::new(FnvHashSet::default()));

        // Create the New Item button and attach it to the grid
        let new = gtk::Button::with_label("New Item");
        grid.attach(&new, 1, 0, 1, 1);

        // Connect the new item button
        new.connect_clicked(clone!(
            system_send,
            next_item,
            id_list
        => move |_| {
            // Try to borrow the the next id
            let mut next_id = match next_item.try_borrow_mut() {
                Ok(id) => id,
                _ => return,
            };

            // Try to borrow the the list of ids
            let mut list = match id_list.try_borrow_mut() {
                Ok(list) => list,
                _ => return,
            };

            // Increment until an open id is found
            let mut new_id = *next_id;
            while let Some(_) = list.get(&new_id) {
                new_id += 1;
            }

            // Save the new starting id and add it to the list
            *next_id = new_id + 1;
            list.insert(new_id);

            // Create the item pair and save the modification
            let item_pair = ItemPair::new_blank(new_id);
            let modifications = vec!(Modification::ModifyItem { item_pair });

            // Save the new id to the configuration
            system_send.send(Edit { modifications });

            // Refresh the item list
            system_send.send(Request {
                reply_to: DisplayComponent::ItemList,
                request: RequestType::Items,
            });
        }));

        // Add the top separator for the item list
        let items_separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        items_separator.set_hexpand(true);
        items_separator.set_halign(gtk::Align::Fill);
        grid.attach(&items_separator, 0, 1, 2, 1);
        
        // Create the scrolling window that contains the list box
        let items_scroll = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        items_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window and attach it to the grid
        items_scroll.set_vexpand(true);
        items_scroll.set_valign(gtk::Align::Fill);
        grid.attach(&items_scroll, 0, 2, 2, 1);

        // Create the list box to hold the item buttons
        let items_list = gtk::ListBox::new();
        items_list.set_selection_mode(gtk::SelectionMode::None);
        items_scroll.add(&items_list);
        
        // Show all the elements of the grid
        grid.show_all();

        // Return the completed structure
        ItemList { grid, next_item, id_list, items_list }
    }
    
    /// A method to return the top element
    ///
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }
    
    /// A method to clear the current item list
    ///
    fn clear(&self) {
        // Remove all the item list buttons
        let to_remove = self.items_list.get_children();
        for item in to_remove {
            unsafe {
                item.destroy();
            }
        }
    }

    /// A method to make a button for each item in the configuration file
    ///
    fn update_info(&self, items: Vec<ItemPair>) {
        // Try to borrow the the list of ids
        let mut list = match self.id_list.try_borrow_mut() {
            Ok(list) => list,
            _ => return,
        };
        
        // Clear the item list
        self.clear();
        
        // Iterate through the item pairs in the items vector
        for item_pair in items {
            // Add the id to the id_list
            list.insert(item_pair.id()); // if it is already present, nothing happens
            
            // Create the label to hold the data
            let item_label = gtk::Label::new(None);
            let item_markup = clean_text(&item_pair.description, LABEL_LIMIT, true, false, true);
            color_label(
                &item_label,
                &item_markup,
                item_pair.display,
                12000, // font size FIXME should be a variable
            );

            // Add the label to a button
            let item_button = gtk::Button::new();
            item_button.add(&item_label);

            // Make the label a drag source
            drag!(source item_button);

            // Serialize the item pair data
            item_button.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                if let Ok(data) = serde_yaml::to_string(&item_pair) {
                    selection_data.set_text(data.as_str());
                }
            }));

            // Show the label, button and add the button to the list box
            item_label.show();
            item_button.show();
            self.items_list.add(&item_button);
        }
    }
}


// Create a structure for editing the item description of the item
#[derive(Clone, Debug)]
struct EditOverview {
    grid: gtk::Grid,                      // the main grid for this element
    system_send: SyncSystemSend,          // a copy of the system send line
    display_type: gtk::ComboBoxText,      // the display type selection for the event
    group_checkbox: gtk::CheckButton,     // the checkbox for group id
    group_description: gtk::Label,        // the description of the group
    group_data: Rc<RefCell<Option<ItemId>>>,      // the data associated with the group
    position_checkbox: gtk::CheckButton,  // the position checkbox
    position: gtk::SpinButton,            // the spin selection for position
    color_checkbox: gtk::CheckButton,     // the color checkbox
    color: gtk::ColorButton,              // the color selection button
    highlight_checkbox: gtk::CheckButton, // the highlight checkbox
    highlight: gtk::ColorButton,          // the highlight selection button
    spotlight_checkbox: gtk::CheckButton, // the spotlight checkbox
    spotlight: gtk::SpinButton,           // the spin selection for spotlight number
    highstate_checkbox: gtk::CheckButton, // the highlight state checkbox
    highstate_status_description: gtk::Label,       // the description of the status
    highstate_status_data: Rc<RefCell<ItemId>>,     // the data associated with the status
    highstate_state_dropdown: gtk::ComboBoxText,    // a dropdown of the valid states
    is_left: bool,                        // whether the element is on the left or right
}

// Implement key features of the Edit Overview
impl EditOverview {
    /// A function to create a new edit overview
    ///
    fn new(system_send: &SyncSystemSend, is_left: bool) -> EditOverview {
        // Add the display type dropdown
        let display_settings_label = gtk::Label::new(Some("Display Settings"));
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

        // Create the group options
        let group_checkbox = gtk::CheckButton::with_label("Show In Control Area");
        // The button to hold the label
        let group_button = gtk::Button::new();
        let group_description = gtk::Label::new(None);
        group_description.set_markup("Group: None");

        // Add the label to the button
        group_button.add(&group_description);

        // Create the variable to hold the group data
        let group_data = Rc::new(RefCell::new(None));

        // Set up the group description button to act as a drag source and destination
        drag!(source group_button);
        drag!(dest group_button);

        // Set the callback function when data is received
        group_button.connect_drag_data_received(clone!(group_data, group_description => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the current description
                group_description.set_markup(&format!("Group: {}", item_pair.description));

                // Update the group data
                if let Ok(mut group) = group_data.try_borrow_mut() {
                    *group = Some(item_pair.get_id());
                }

                // Serialize the item pair data
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));

        // Connect the checkbox callback function to change label text
        group_checkbox.connect_toggled(clone!(group_description => move | checkbox | {
            // Strikethrough the text when checkbox is selected
            if checkbox.get_active() {
                // Remove the markup
                let label_markup = clean_text(&group_description.get_text(), LABEL_LIMIT, true, false, true);
                // Display the label text with strikethrough
                group_description.set_markup(&format!("<s>{}</s>", label_markup));
            } else {
                // Remove the markup
                let label_markup = clean_text(&group_description.get_text(), LABEL_LIMIT, true, false, true);
                // Display the label text without strikethrough
                group_description.set_markup(&label_markup);
            }
        }));

        // Create the position option
        let position_checkbox = gtk::CheckButton::with_label("Display Position");
        let position_label = gtk::Label::new(None);
        position_label.set_markup("<s>Position Number:</s>");
        let position = gtk::SpinButton::with_range(1.0, 536870911.0, 1.0);
        position_checkbox.connect_toggled(clone!(position_label => move | checkbox | {
            // Strikethrough the text when checkbox not selected
            if checkbox.get_active() {
                position_label.set_markup("Position Number:");
            } else {
                position_label.set_markup("<s>Position Number:</s>");
            }
        }));

        // Create the color option
        let color_checkbox = gtk::CheckButton::with_label("Custom Text Color");
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
        let highlight_checkbox = gtk::CheckButton::with_label("Custom Text Highlight");
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
        let spotlight_checkbox = gtk::CheckButton::with_label("Spotlight Changes");
        let spotlight_label = gtk::Label::new(None);
        spotlight_label.set_markup("<s>Flash Cycles:</s>");
        let spotlight = gtk::SpinButton::with_range(1.0, 536870911.0, 1.0);
        spotlight_checkbox.connect_toggled(clone!(spotlight_label => move | checkbox | {
            // Strikethrough the text when checkbox not selected
            if checkbox.get_active() {
                spotlight_label.set_markup("Flash Cycles:");
            } else {
                spotlight_label.set_markup("<s>Flash Cycles:</s>");
            }
        }));

        // Create the highlight state options
        let highstate_checkbox = gtk::CheckButton::with_label("Status-Based Highlighting");
        // Create the button to hold the label for the status
        let highstate_status_button = gtk::Button::new();
        let highstate_status_description = gtk::Label::new(None);
        highstate_status_description.set_markup("<s>Status: None</s>");

        // Add the label to the button
        highstate_status_button.add(&highstate_status_description);

        // Create the variable to hold the status data
        let highstate_status_data = Rc::new(RefCell::new(ItemId::all_stop()));

        // Create the elements associated with the state
        let highstate_state_dropdown = gtk::ComboBoxText::new();
        let state_label = gtk::Label::new(None);
        state_label.set_markup("<s>State:</s>");

        // Set up the status button to act as a drag source and destination
        drag!(source highstate_status_button);
        drag!(dest highstate_status_button);

        // Set the callback function when data is received
        highstate_status_button.connect_drag_data_received(clone!(
            highstate_status_data,
            highstate_status_description,
            system_send,
            is_left
        => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the current description
                highstate_status_description.set_markup(&format!("Status: {}", item_pair.description));

                // Update the status data
                if let Ok(mut status) = highstate_status_data.try_borrow_mut() {
                    *status = item_pair.get_id();
                }

                // Request the allowed states
                system_send.send(Request {
                    reply_to: DisplayComponent::EditItemOverview {
                        is_left,
                        variant: EditItemElement::Status { state: None },
                    },
                    request: RequestType::Status { item_id: item_pair.get_id() }
                });

                // Serialize the item pair data
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));

        // Connect the checkbox callback function to change label text
        highstate_checkbox.connect_toggled(clone!(highstate_status_description, state_label => move | checkbox | {
            // Strikethrough the text when checkbox is selected
            if checkbox.get_active() {
                // Display the state label without strikethrough
                state_label.set_markup("State:");

                // Remove the markup
                let label_markup = clean_text(&highstate_status_description.get_text(), LABEL_LIMIT, true, false, true);
                // Display the label text without strikethrough
                highstate_status_description.set_markup(&label_markup);
            } else {
                // Display the state label with strikethrough
                state_label.set_markup("<s>State:</s>");

                // Remove the markup
                let label_markup = clean_text(&highstate_status_description.get_text(), LABEL_LIMIT, true, false, true);
                // Display the label text with strikethrough
                highstate_status_description.set_markup(&format!("<s>{}</s>", label_markup));
            }
        }));

        // Compose the display grid
        let display_grid = gtk::Grid::new();
        display_grid.attach(&group_checkbox, 0, 0, 1, 1);
        display_grid.attach(&group_button, 1, 0, 2, 1);
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
        display_grid.attach(&highstate_status_button, 1, 5, 2, 1);
        display_grid.attach(&state_label, 1, 6, 1, 1);
        display_grid.attach(&highstate_state_dropdown, 2, 6, 1, 1);
        display_grid.set_column_spacing(10); // Add some space
        display_grid.set_row_spacing(10);
        display_grid.set_halign(gtk::Align::End);

        // Connect the function to trigger display type changes
        display_type.connect_changed(clone!(
            group_checkbox,
            group_button,
            group_description,
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
            highstate_status_button,
            highstate_state_dropdown,
            state_label
        => move |dropdown| {
            // Identify the selected display type
            if let Some(display_type) = dropdown.get_active_id() {
                // Match the selection and change the visible options
                match display_type.as_str() {
                    // the DisplayControl variant
                    "displaycontrol" => {
                        group_checkbox.hide();
                        group_button.hide();
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
                        highstate_status_button.show();
                        highstate_state_dropdown.show();
                        state_label.show();
                    }

                    // the DisplayWith variant
                    "displaywith" => {
                        group_checkbox.hide();
                        group_button.show();
                        // Remove the markup
                        let label_markup = clean_text(&group_description.get_text(), LABEL_LIMIT, true, false, true);
                        // Display the label text without strikethrough
                        group_description.set_markup(&label_markup);
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
                        highstate_status_button.show();
                        highstate_state_dropdown.show();
                        state_label.show();
                    }

                    // the DisplayDebug variant
                    "displaydebug" => {
                        group_checkbox.show();
                        group_button.show();
                        // Remove the markup
                        let label_markup = clean_text(&group_description.get_text(), LABEL_LIMIT, true, false, true);
                        // Display the label text with strikethrough
                        group_description.set_markup(&format!("<s>{}</s>", label_markup));
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
                        highstate_status_button.show();
                        highstate_state_dropdown.show();
                        state_label.show();
                    }

                    // the LabelControl variant
                    "labelcontrol" => {
                        group_checkbox.hide();
                        group_button.hide();
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
                        highstate_status_button.show();
                        highstate_state_dropdown.show();
                        state_label.show();
                    }

                    // the LabelHidden variant
                    "labelhidden" => {
                        group_checkbox.hide();
                        group_button.hide();
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
                        highstate_status_button.show();
                        highstate_state_dropdown.show();
                        state_label.show();
                    }

                    // the Hidden variant
                    _ => {
                        group_checkbox.hide();
                        group_button.hide();
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
                        highstate_status_button.hide();
                        highstate_state_dropdown.hide();
                        state_label.hide();
                    }
                }
            }
        }));

        // Create the edit overview grid and populate it
        let grid = gtk::Grid::new();
        grid.attach(&display_settings_label, 0, 0, 3, 1);
        grid.attach(&display_type_label, 0, 1, 1, 1);
        grid.attach(&display_type, 1, 1, 2, 1);
        grid.attach(&display_grid, 0, 2, 3, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);
        grid.show_all();

        // Create and return the edit overview
        EditOverview {
            grid,
            system_send: system_send.clone(),
            display_type,
            group_checkbox,
            group_description,
            group_data,
            position_checkbox,
            position,
            color_checkbox,
            color,
            highlight_checkbox,
            highlight,
            spotlight_checkbox,
            spotlight,
            highstate_checkbox,
            highstate_status_description,
            highstate_status_data,
            highstate_state_dropdown,
            is_left,
        }
    }

    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load an item description into the edit overview
    //
    fn load_description(&self, variant: EditItemElement, description: ItemPair) {
        match variant {
            EditItemElement::ItemDescription => {
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
                        group_id,
                        position,
                        color,
                        highlight,
                        highlight_state,
                        spotlight,
                    } => {
                        // Change the visible options
                        self.display_type.set_active_id(Some("displaydebug"));

                        // Save the available elements
                        new_group = group_id;
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
                        
                        // Set the group data
                        if let Ok(mut group_data) = self.group_data.try_borrow_mut() {
                            *group_data = Some(id.clone());
                        }

                        // Send a request to update the group description
                        self.system_send.send(Request {
                            reply_to: DisplayComponent::EditItemOverview {
                                is_left: self.is_left,
                                variant: EditItemElement::Group,
                            },
                            request: RequestType::Description { item_id: id.clone() }
                        });
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
                        // Set the status data
                        if let Ok(mut status_data) = self.highstate_status_data.try_borrow_mut() {
                            *status_data = new_status.clone();
                        }
                        // Send a request to update the status description
                        self.system_send.send(Request {
                            reply_to: DisplayComponent::EditItemOverview {
                                is_left: self.is_left,
                                variant: EditItemElement::Status { state: None },
                            },
                            request: RequestType::Description { item_id: new_status.clone() }
                        });

                        // Send a request to get the states associated with the status
                        self.system_send.send(Request {
                            reply_to: DisplayComponent::EditItemOverview {
                                is_left: self.is_left,
                                variant: EditItemElement::Status { state: Some(new_state.clone()) },
                            },
                            request: RequestType::Status { item_id: new_status.clone() }
                        });
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
            },

            // Update the group description
            EditItemElement::Group => self.group_description.set_text(&format!("Group: {}", description.description)),

            // Update the status description
            EditItemElement::Status { .. } => self.highstate_status_description.set_text(&format!("Status: {}", description.description)),

            // Update the state description in the dropdown
            EditItemElement::State => {
                // Add the decription to the dropdown with the item id as the id
                self.highstate_state_dropdown.append(Some(&description.id().to_string()), &description.description) // FIXME This probably doesn't work
            },

            _ => unreachable!(),
        }
    }

    // A method to update the descriptions of states associated with a status
    //
    pub fn load_status(&self, status: Option<Status>, default_state: Option<ItemId>) {
        // Clear the state dropdown of the previous states
        self.clear();

        // Unpack the status
        if let Some(status) = status {
            // Go through each allowed state and request its description
            for state_id in status.allowed().drain(..) {
                self.system_send.send(Request {
                    reply_to: DisplayComponent::EditItemOverview {
                        is_left: self.is_left,
                        variant: EditItemElement::State,
                    },
                    request: RequestType::Description { item_id: state_id.clone() },
                });
            }

            // Set the dropdown menu to the default state, if one was given
            if let Some(state) = default_state {
                self.highstate_state_dropdown.set_active_id(Some(&state.id().to_string()));
            }
        }
    }

    // A method to clear all the listed states in the state dropdown
    pub fn clear(&self) {
        // Remove all the dropdown elements
        self.highstate_state_dropdown.remove_all();
    }


    // A method to pack the item description
    //
    fn pack_description(&self, tmp_description: String) -> ItemDescription {
        // Create default placeholders for the display settings
        let mut possible_group = None;
        let mut position = None;
        let mut color = None;
        let mut highlight = None;
        let mut highlight_state = None;
        let mut spotlight = None;

        // Extract the group id, if available
        if let Ok(group_data) = self.group_data.try_borrow() {
            possible_group = *group_data;
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
            // Extract the status data
            if let Ok(status_data) = self.highstate_status_data.try_borrow() {
                // Get the id associated with the selected state
                if let Some(state_data) = self.highstate_state_dropdown.get_active_id() {
                    // Set the highlight state
                    highlight_state = Some((
                        *status_data,
                        ItemId::new_unchecked(state_data.parse::<u32>().unwrap()),
                    ));
                }
            }
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
            "displaywith" => {
                // If a group was selected, return DisplayWith
                if let Some(group_id) = possible_group {
                    DisplayWith {
                        group_id,
                        position,
                        color,
                        highlight,
                        highlight_state,
                        spotlight,
                    }
                
                // Fallback to DisplayControl
                } else {
                    DisplayControl {
                        position,
                        color,
                        highlight,
                        highlight_state,
                        spotlight,
                    }
                }
            } 

            // Compose the DisplayDebug type
            "displaydebug" => {
                // If the group checkbox is selected
                if self.group_checkbox.get_active() {
                    // Make sure the possible group is none
                    possible_group = None;
                }
                DisplayDebug {
                    group_id: possible_group,
                    position,
                    color,
                    highlight,
                    highlight_state,
                    spotlight,
                }
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
