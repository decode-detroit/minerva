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
    DataType, DisplayComponent, ItemId, ItemPair, Request, RequestType, SystemSend,
};
use super::super::super::utils::{clean_text, decorate_label};
use super::NORMAL_FONT;

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
    system_send: SystemSend,                // a reference to the system send line
    current_id: Rc<RefCell<Option<ItemId>>>, // the wrapped current item id
    events_list: gtk::ListBox,               // a list box to hold the events in the scene
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

                // Add the button to the list box
                widget.add(&event_button);
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
        // Attach the checkbox and scroll window to the grid
        grid.attach(&detail_checkbox, 0, 0, 1, 1);
        grid.attach(&events_scroll, 0, 1, 1, 1);

        // Make the grid visible but hide the scroll window by default
        grid.show_all();
        events_scroll.hide();

        EditScene{
            grid,
            system_send: system_send.clone(),
            current_id: Rc::new(RefCell::new(None)),
            events_list,
            events,
        }
    }

    /// A method to update the list of all events in the scene
    ///
    pub fn update_info(&self, events: Vec<ItemPair>) {
        // Iterate through the item pairs in the events vector
        for item_pair in events {
            // Create a button with the event description
            let event_button = gtk::Button::new_with_label(&item_pair.description());
            // Add the button to the list box
            self.events_list.add(&event_button);
        }
        // Show all the buttons in the grid
        self.grid.show_all();
    }

    pub fn set_current_id(&mut self, current_id: Rc<RefCell<Option<ItemId>>>) {
        // Set the current id
        self.current_id = current_id;

        // Unwrap and extract the current id
        if let Ok(scene_id) = self.current_id.try_borrow() {
            if let Some(id) = *scene_id {
                // Send a request for the items in the current scene
                self.system_send.send(Request {
                    reply_to: DisplayComponent::EditScene,
                    request: RequestType::Events { scene_id: id, },
                });
            }
        }
    }

    // A method to return the top element
    //
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }
}
