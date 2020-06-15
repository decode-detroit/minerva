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
    DataType, DisplayComponent, EditActionElement, EventAction, Event, EventDelay, ItemDescription,
    ItemId, ItemPair, Request, RequestType, Status, SystemSend,
};

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

// Import FNV HashMap
extern crate fnv;
use self::fnv::FnvHashMap;

// Import GTK and GDK libraries
extern crate gdk;
extern crate gio;
extern crate gtk;
use self::gtk::prelude::*;
use self::gtk::GridExt;

// Define and import constants
const MINUTES_LIMIT: f64 = 10080.0; // maximum input time for a delayed event (one week)


// Create a structure for editing the event
#[derive(Clone, Debug)]
pub struct EditEvent {
    grid: gtk::Grid,                   // the main grid for this element
    edit_action: Rc<RefCell<EditAction>>, // a wrapped dialog to edit the current action
    event_checkbox: gtk::CheckButton, // the checkbox to indicate an active event
    event_actions: Rc<RefCell<FnvHashMap<usize, EventAction>>>, // a wrapped hash map of actions (may be empty)
    next_position: Rc<RefCell<usize>>, // the next available position in the hash map
    action_list: gtk::ListBox,         // the visible list for event actions
    is_left: bool,                     // whether the element is on the left
}

// Implement key features for Edit Event
impl EditEvent {
    // A function to create a new Edit Detail
    //
    pub fn new(system_send: &SystemSend, is_left: bool) -> EditEvent {
        // Create the grid
        let grid = gtk::Grid::new();

        // Construct the checkbox for the event
        let event_checkbox = gtk::CheckButton::new_with_label("Item Corresponds To An Event");
        event_checkbox.set_active(true);

        // Create the empty event actions
        let event_actions = Rc::new(RefCell::new(FnvHashMap::default()));

        // Create the starting next position
        let next_position = Rc::new(RefCell::new(0));

        // Create the action list for the events
        let action_list = gtk::ListBox::new();
        action_list.set_selection_mode(gtk::SelectionMode::None);

        // Create a new edit action dialog
        let tmp_edit_action = EditAction::new(system_send, &event_actions, is_left);
        grid.attach(tmp_edit_action.get_top_element(), 1, 1, 1, 2);
        let edit_action = Rc::new(RefCell::new(tmp_edit_action.clone()));

        // Create a button to add actions to the list
        let add_button = gtk::Button::new_from_icon_name(
            Some("list-add-symbolic"),
            gtk::IconSize::Button.into(),
        );
        add_button.connect_clicked(
            clone!(edit_action, event_actions, next_position, action_list => move |_| {
                // Add an empty action to the list
                EditEvent::add_action(&edit_action, &event_actions, &next_position, &action_list, None);
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
        action_window.set_size_request(-1, 200);

        // Connect the checkbox to the visibility of the other elements
        event_checkbox.connect_toggled(clone!(
            action_window,
            add_button,
            tmp_edit_action
        => move | checkbox | {
            // Make the elements invisible if the box isn't checked
            if checkbox.get_active() {
                action_window.show();
                add_button.show();
                tmp_edit_action.get_top_element().show();
            } else {
                action_window.hide();
                add_button.hide();
                tmp_edit_action.get_top_element().hide();
            }
        }));

        // Add the button below the data list
        grid.attach(&event_checkbox, 0, 0, 1, 1);
        grid.attach(&action_window, 0, 1, 1, 1);
        grid.attach(&add_button, 0, 2, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the trigger events variant
        EditEvent {
            grid,
            edit_action,
            event_checkbox,
            event_actions,
            next_position,
            action_list,
            is_left
        }
    }

    // A method to return the top element
    //
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load an existing event, or none
    //
    pub fn load_event(&self, event: Option<Event>) {
        // See if an event was specified
        let mut event = match event {
            // If a event was specified, switch the checkbox
            Some(event) => {
                self.event_checkbox.set_active(true);
                event
            }

            // Otherwise, uncheck the checkbox and return
            None => {
                self.event_checkbox.set_active(false);
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
        for action in event.drain(..) {
            EditEvent::add_action(
                &self.edit_action,
                &self.event_actions,
                &self.next_position,
                &self.action_list,
                Some(action),
            );
        }
    }

    // A method to pack the listed actions into an event
    //
    pub fn pack_event(&self) -> Option<Event> {

        // If the checkbox was not selected, return None
        if !self.event_checkbox.get_active() {
            return None;
        }

        // Access the current event
        match self.event_actions.try_borrow_mut() {
            // Return the current event, composed as a vector
            Ok(event) => {
                // Create a vector to hold the actions and a counter
                let mut actions = Vec::new();
                let mut count = 0;

                // Search until we've found all the actions
                while actions.len() < event.len() {
                    // Try to get each element, zero indexed
                    if let Some(action) = event.get(&count) {
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
    pub fn update_info(&self, status: Option<Status>) {
        // Try to get access the edit action dialog
        if let Ok(dialog) = self.edit_action.try_borrow() {
            dialog.update_info(status);
        }
    }

    // A method to update the description of an item
    pub fn update_description(&self, action_type: EditActionElement, description: ItemDescription) {
        // Try to get access to the edit action window
        if let Ok(edit_action) = self.edit_action.try_borrow() {
            edit_action.update_description(action_type, description);
        }
    }

    // A helper function to add an action to the action list
    //
    fn add_action(
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
            // Add a copy of the action to the action list
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

/// A structure to contain the grid for editing an individual event action.
///
#[derive(Clone, Debug)]
struct EditAction {
    grid: gtk::Grid,                                            // a grid to hold the actions
    action_selection: gtk::ComboBoxText,                        // the action selection element
    edit_new_scene: Rc<RefCell<EditNewScene>>,                  // the wrapped EditNewScene structure
    edit_modify_status: Rc<RefCell<EditModifyStatus>>,          // the wrapped EditModifyStatus structure
    edit_queue_event: Rc<RefCell<EditQueueEvent>>,              // the wrapped EditQueueEvent structure
    edit_cancel_event: Rc<RefCell<EditCancelEvent>>,            // the wrapped EditCancelEvent structure
    edit_save_data: Rc<RefCell<EditSaveData>>,                  // the wrapped EditSaveData structure
    edit_send_data: Rc<RefCell<EditSendData>>,                  // the wrapped EditSendData structure
    edit_grouped_event: Rc<RefCell<EditGroupedEvent>>,          // the wrapped EditGroupedEvent structure
    event_actions: Rc<RefCell<FnvHashMap<usize, EventAction>>>, // a wrapped hash map of event actions
    is_left: bool,                                              // whether the element is on the left
}

// Implement key features of the EditAction
impl EditAction {
    /// A function to create a new instance of the EditAction
    ///
    fn new(
        system_send: &SystemSend,
        event_actions: &Rc<RefCell<FnvHashMap<usize, EventAction>>>,
        is_left: bool,
    ) -> EditAction {
        // Create a dropdown for the action selection
        let action_selection = gtk::ComboBoxText::new();

        // Add each of the available action types to the dropdown
        action_selection.append(Some("newscene"), "New Scene");
        action_selection.append(Some("modifystatus"), "Modify Status");
        action_selection.append(Some("queueevent"), "Queue Event");
        action_selection.append(Some("cancelevent"), "Cancel Event");
        action_selection.append(Some("savedata"), "Save Data");
        action_selection.append(Some("senddata"), "Send Data");
        action_selection.append(Some("groupedevent"), "Grouped Event");

        // Create the different edit windows for the action types
        let edit_new_scene = EditNewScene::new(system_send, is_left);
        let edit_modify_status = EditModifyStatus::new(system_send, is_left);
        let edit_queue_event = EditQueueEvent::new(system_send, is_left);
        let edit_cancel_event = EditCancelEvent::new(system_send, is_left);
        let edit_save_data = EditSaveData::new();
        let edit_send_data = EditSendData::new();
        let edit_grouped_event = EditGroupedEvent::new(system_send, is_left);

        // Create the action stack
        let action_stack = gtk::Stack::new();

        // Add the edit types to the action stack
        action_stack.add_named(edit_new_scene.get_top_element(), "newscene");
        action_stack.add_named(edit_modify_status.get_top_element(), "modifystatus");
        action_stack.add_named(edit_queue_event.get_top_element(), "queueevent");
        action_stack.add_named(edit_cancel_event.get_top_element(), "cancelevent");
        action_stack.add_named(edit_save_data.get_top_element(), "savedata");
        action_stack.add_named(edit_send_data.get_top_element(), "senddata");
        action_stack.add_named(edit_grouped_event.get_top_element(), "groupedevent");

        // Connect the function to trigger action selection changes
        action_selection.connect_changed(clone!(action_stack => move |dropdown| {
            // Identify the selected action type
            if let Some(action_str) = dropdown.get_active_id() {
                // Change the action stack to the requested variation
                action_stack.set_visible_child_full(&action_str, gtk::StackTransitionType::None);
            }
        }));

        // Create a grid to hold the actions
        let grid = gtk::Grid::new();

        // Add the dropdown and the action stack
        grid.attach(&action_selection, 0, 0, 2, 1);
        grid.attach(&action_stack, 0, 1, 2, 1);
        grid.set_column_spacing(10); // add some space
        grid.set_row_spacing(10);

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);

        // Make the correct things visible
        grid.show_all();
        grid.hide();

        EditAction {
            grid,
            action_selection,
            edit_new_scene: Rc::new(RefCell::new(edit_new_scene)),
            edit_modify_status: Rc::new(RefCell::new(edit_modify_status)),
            edit_queue_event: Rc::new(RefCell::new(edit_queue_event)),
            edit_cancel_event: Rc::new(RefCell::new(edit_cancel_event)),
            edit_save_data: Rc::new(RefCell::new(edit_save_data)),
            edit_send_data: Rc::new(RefCell::new(edit_send_data)),
            edit_grouped_event: Rc::new(RefCell::new(edit_grouped_event)),
            event_actions: event_actions.clone(),
            is_left,
        }
    }

    /// A method to load a new action
    ///
    fn load_action(&self, position: usize, overview: &gtk::Label, row: &gtk::ListBoxRow) {
        // Show the grid to allow for editing
        self.grid.show();

        // Try to get the current event actions
        let actions = match self.event_actions.try_borrow() {
            Ok(actions) => actions,
            _ => return,
        };

        // Try to extract the correct action
        let action = match actions.get(&position) {
            Some(action) => action,
            _ => return,
        };

        // Try to get a copy of the edit new scene
        let edit_new_scene = match self.edit_new_scene.try_borrow() {
            Ok(edit_scene) => edit_scene,
            _ => return,
        };

        // Try to get a copy of the edit modify status
        let edit_modify_status = match self.edit_modify_status.try_borrow() {
            Ok(edit_status) => edit_status,
            _ => return,
        };

        // Try to get a copy of the edit queue event
        let edit_queue_event = match self.edit_queue_event.try_borrow() {
            Ok(edit_queue) => edit_queue,
            _ => return,
        };

        // Try to get a copy of the edit cancel event
        let edit_cancel_event = match self.edit_cancel_event.try_borrow() {
            Ok(edit_cancel) => edit_cancel,
            _ => return,
        };

        // Try to get a copy of the edit save data
        let edit_save_data = match self.edit_save_data.try_borrow() {
            Ok(edit_save) => edit_save,
            _ => return,
        };

        // Try to get a copy of the edit send data
        let edit_send_data = match self.edit_send_data.try_borrow() {
            Ok(edit_send) => edit_send,
            _ => return,
        };

        // Try to get a copy of the edit grouped event
        let edit_grouped_event = match self.edit_grouped_event.try_borrow() {
            Ok(edit_grouped) => edit_grouped,
            _ => return,
        };

        // Load the selected action
        match action {
            // the NewScene variant
            EventAction::NewScene { new_scene } => {
                self.action_selection.set_active_id(Some("newscene"));
                edit_new_scene.load_action(new_scene);
            }

            // the ModifyStatus variant
            EventAction::ModifyStatus {
                status_id,
                new_state,
            } => {
                self.action_selection.set_active_id(Some("modifystatus"));
                edit_modify_status.load_action(status_id, new_state);
            }

            // the QueueEvent variant
            EventAction::QueueEvent { event } => {
                self.action_selection.set_active_id(Some("queueevent"));
                edit_queue_event.load_action(event);
            }

            // the CancelEvent variant
            EventAction::CancelEvent { event } => {
                self.action_selection.set_active_id(Some("cancelevent"));
                edit_cancel_event.load_action(event);
            }

            // the SaveData variant
            EventAction::SaveData { data } => {
                self.action_selection.set_active_id(Some("savedata"));
                edit_save_data.load_action(data);
            }

            // the SendData variant
            EventAction::SendData { data } => {
                self.action_selection.set_active_id(Some("senddata"));
                edit_send_data.load_action(data);
            }

            // the GroupedEvent variant
            EventAction::GroupedEvent {
                status_id,
                event_map,
            } => {
                self.action_selection.set_active_id(Some("groupedevent"));
                edit_grouped_event.load_action(status_id, event_map);
            }
        }

        // Create the button to save an action
        let save_button = gtk::Button::new_with_label("Save");

        // Create the button to delete an action
        let delete_button = gtk::Button::new_with_label("Delete");

        // Connect the delete button
        let event_actions = self.event_actions.clone();
        let grid = self.grid.clone();
        delete_button.connect_clicked(clone!(
            row,
            position,
            event_actions,
            save_button,
            delete_button => move |_| {
            // Remove the action from the event actions
            match event_actions.try_borrow_mut() {
                Ok(mut actions) => {
                    actions.remove(&position);
                }

                // If unable, exit immediately
                Err(_) => return,
            };

            // Destroy the row (automatically removing it from the action list)
            row.destroy();

            // Delete the save and delete buttons
            save_button.destroy();
            delete_button.destroy();

            // Hide the grid to prevent editing
            grid.hide();
        }));

        // Connect the save button
        let grid = self.grid.clone();
        let action_selection = self.action_selection.clone();
        save_button.connect_clicked(clone!(
            overview,
            save_button,
            delete_button,
            edit_new_scene,
            edit_modify_status,
            edit_queue_event,
            edit_cancel_event,
            edit_save_data,
            edit_send_data,
            edit_grouped_event => move |_| {

                // Try to get a mutable copy of the event actions
                let mut actions = match event_actions.try_borrow_mut() {
                    Ok(actions) => actions,
                    _ => return,
                };

                // Try to extract the correct action
                let action = match actions.get_mut(&position) {
                    Some(action) => action,
                    _ => return,
                };

                // Match the current dropdown selection
                if let Some(selection) = action_selection.get_active_id() {
                    match selection.as_str() {
                        // the NewScene variant
                        "newscene" => {
                            // Update the action label and action
                            overview.set_text("New Scene");
                            *action = edit_new_scene.pack_action();
                        }

                        // the ModifyStatus variant
                        "modifystatus" => {
                            // Update the action label and action
                            overview.set_text("Modify Status");
                            *action = edit_modify_status.pack_action();
                        }

                        // the QueueEvent variant
                        "queueevent" => {
                            // Update the action label and action
                            overview.set_text("Queue Event");
                            *action = edit_queue_event.pack_action();
                        }

                        // the CancelEvent variant
                        "cancelevent" => {
                            // Update the action label and action
                            overview.set_text("Cancel Event");
                            *action = edit_cancel_event.pack_action();

                        }

                        // the SaveData variant
                        "savedata" => {
                            // Update the action label and action
                            overview.set_text("Save Data");
                            *action = edit_save_data.pack_action();
                        }

                        // the SendData variant
                        "senddata" => {
                            // Update the action label and action
                            overview.set_text("Send Data");
                            *action = edit_send_data.pack_action();
                        }

                        // The GroupedEvent variant
                        "groupedevent" => {
                            // Update the action label and action
                            overview.set_text("Grouped Event");
                            *action = edit_grouped_event.pack_action();
                        }

                        _ => unreachable!(),
                    }

                // If no selection was made, exit prematurely
                } else {
                    return;
                }
            // Delete the save and delete buttons
            save_button.destroy();
            delete_button.destroy();

            // Hide the grid to prevent editing
            grid.hide();
        }));

        // Add the save and delete buttons to the grid
        self.grid.attach(&save_button, 0, 2, 1, 1);
        self.grid.attach(&delete_button, 1, 2, 1, 1);
        delete_button.show();
        save_button.show();
    }

    /// A method to return the top element
    ///
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    /// A method to update the description of an item
    ///
    pub fn update_description(&self, action_type: EditActionElement, description: ItemDescription) {
        match action_type {
            EditActionElement::EditNewScene => {
                // Get a copy of the edit new scene element
                if let Ok(edit_new_scene) = self.edit_new_scene.try_borrow() {
                    edit_new_scene.update_description(description)
                }
            }

            EditActionElement::EditModifyStatus { is_status } => {
                // Get a copy of the edit new scene element
                if let Ok(edit_modify_status) = self.edit_modify_status.try_borrow() {
                    edit_modify_status.update_description(is_status, description)
                }
            }

            EditActionElement::EditQueueEvent => {
                // Get a copy of the edit queue event element
                if let Ok(edit_queue_event) = self.edit_queue_event.try_borrow() {
                    edit_queue_event.update_description(description)
                }
            }

            EditActionElement::EditCancelEvent => {
                // Get a copy of the edit cancel event element
                if let Ok(edit_cancel_event) = self.edit_cancel_event.try_borrow() {
                    edit_cancel_event.update_description(description)
                }
            }

            _ => unreachable!(),
        }
    }

    /// A method to pass the status to the EditGroupedEvent structure
    ///
    fn update_info(&self, status: Option<Status>) {
        if let Ok(mut edit_grouped_event) = self.edit_grouped_event.try_borrow_mut() {
            edit_grouped_event.update_info(status);
        }
    }
}

// Create the new scene variant
#[derive(Clone, Debug)]
struct EditNewScene {
    grid: gtk::Grid,                      // the main grid for this element
    system_send: SystemSend,              // a copy of the system send line
    description: gtk::Button,             // the description of the scene
    scene: Rc<RefCell<ItemId>>,           // the wrapped data associated with the scene
    is_left: bool,                        // whether the edit element is left or right
}

// Implement key features for Edit New Scene
impl EditNewScene {
    // A function to create a new scene variant
    //
    fn new(system_send: &SystemSend, is_left: bool) -> EditNewScene {
        // Create the top level grid
        let grid = gtk::Grid::new();

        // Add a button with a label to the grid
        let description = gtk::Button::new_with_label("Scene: None");

        // Create the data associated with the scene
        let scene = Rc::new(RefCell::new(ItemId::all_stop()));

        // Set up the description to act as a drag source
        description.drag_source_set(
            gdk::ModifierType::MODIFIER_MASK,
            &vec![
                gtk::TargetEntry::new("STRING", gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY,
        );

        // Set up the description to receive a dropped item pair
        description.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        description.connect_drag_data_received(clone!(scene => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the description
                widget.set_label(&format!("Scene: {}", item_pair.description));

                // Update the scene data
                if let Ok(mut scene_data) = scene.try_borrow_mut() {
                    *scene_data = item_pair.get_id();
                }

                // Serialize the item pair data
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));

        // Attach the label and spin button
        grid.attach(&description, 0, 0, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the EditNewScene
        grid.show_all();
        EditNewScene {
            grid,
            system_send: system_send.clone(),
            description,
            scene,
            is_left
         }
    }

    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load the action
    //
    fn load_action(&self, new_scene: &ItemId) {
        // Request the description associated with the id
        self.system_send.send(Request {
            reply_to: DisplayComponent::EditActionElement { is_left: self.is_left, variant: EditActionElement::EditNewScene },
            request: RequestType::Description { item_id: new_scene.clone() },
        });
    }

    /// A method to update the description of the scene
    ///
    pub fn update_description(&self, description: ItemDescription) {
        // FIXME: because the default is all_stop, the default description is "No Description."
        self.description.set_label(&format!("Scene: {}", &description.description));
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        match self.scene.try_borrow() {
            // Get a copy of the scene data
            Ok(scene) => {
                // Pack the new scene id into an action
                EventAction::NewScene { new_scene: *scene }
            }

            _ => unreachable!(),
        }
    }
}

// Create the modify status variant
#[derive(Clone, Debug)]
struct EditModifyStatus {
    grid: gtk::Grid,                              // the main grid for this element
    system_send: SystemSend,                      // a copy of the system send line
    status_description: gtk::Button,              // the status description display
    status_data: Rc<RefCell<ItemId>>,             // the wrapped status data
    state_description: gtk::Button,               // the state description display
    state_data: Rc<RefCell<ItemId>>,              // the wrapped state data
    is_left: bool,                                // whether the element is on the left
}

impl EditModifyStatus {
    // A function to ceate a modify status variant
    //
    fn new(system_send: &SystemSend, is_left: bool) -> EditModifyStatus {
        // Create the grid for the modify status variant
        let grid = gtk::Grid::new();

        // Set up the labels and data
        let status_description = gtk::Button::new_with_label("Status: None");
        let status_data = Rc::new(RefCell::new(ItemId::all_stop()));
        let state_description = gtk::Button::new_with_label("State: None");
        let state_data = Rc::new(RefCell::new(ItemId::all_stop()));

        // Set up the status description to act as a drag source
        status_description.drag_source_set(
            gdk::ModifierType::MODIFIER_MASK,
            &vec![
                gtk::TargetEntry::new("STRING", gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY,
        );

        // Set up the status spin button to receive a dropped item pair
        status_description.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        status_description.connect_drag_data_received(clone!(status_data => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the label description
                widget.set_label(&format!("Status: {}", item_pair.description));

                // Update the status data
                if let Ok(mut status) = status_data.try_borrow_mut() {
                    *status = item_pair.get_id();
                }

                // Serialize the item pair data
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));

        // Set up the state description to act as a drag source
        state_description.drag_source_set(
            gdk::ModifierType::MODIFIER_MASK,
            &vec![
                gtk::TargetEntry::new("STRING", gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY,
        );

        // Set up the state spin button to receive a dropped item pair
        state_description.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        state_description.connect_drag_data_received(clone!(state_data => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the current spin button value
                widget.set_label(&format!("State: {}", item_pair.description));

                // Update the status data
                if let Ok(mut state) = state_data.try_borrow_mut() {
                    *state = item_pair.get_id();
                }

                // Serialize the item pair data
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));

        // Place everything into the grid
        grid.attach(&status_description, 0, 0, 1, 1);
        grid.attach(&state_description, 1, 0, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the EditModifyStatus
        grid.show_all();
        EditModifyStatus {
            grid,
            system_send: system_send.clone(),
            status_description,
            status_data,
            state_description,
            state_data,
            is_left,
        }
    }

    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load the action
    //
    fn load_action(&self, status_id: &ItemId, new_state: &ItemId) {
        // Request the description associated with the status
        self.system_send.send(Request {
            reply_to: DisplayComponent::EditActionElement {
                is_left: self.is_left,
                variant: EditActionElement::EditModifyStatus { is_status: true }
            },
            request: RequestType::Description { item_id: status_id.clone() },
        });

        // Request the description associated with the state
        self.system_send.send(Request {
            reply_to: DisplayComponent::EditActionElement {
                is_left: self.is_left,
                variant: EditActionElement::EditModifyStatus { is_status: false }
            },
            request: RequestType::Description { item_id: new_state.clone() },
        });
    }

    // A method to update the description of the status or state
    pub fn update_description(&self, is_status: bool, description: ItemDescription) {
        match is_status {
            // If the description is for the status, update the status label
            true => self.status_description.set_label(&format!("Status: {}", description.description)),

            // If the description is for the state, update the state label
            false => self.state_description.set_label(&format!("State: {}", description.description)),
        }
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Get the status data
        match self.status_data.try_borrow() {
            Ok(status_data) => {
                // Get the state data
                match self.state_data.try_borrow() {
                    Ok(state_data) => {
                        // Return the associated ModifyStatus object
                        EventAction::ModifyStatus {
                            status_id: *status_data,
                            new_state: *state_data,
                        }
                    },

                    _ => unreachable!(),
                }
            }

            _ => unreachable!(),
        }
    }
}

// Create the queue event variant
//
#[derive(Clone, Debug)]
struct EditQueueEvent {
    grid: gtk::Grid,                  // the main grid for this element
    system_send: SystemSend,          // a copy of the system send line
    event_description: gtk::Button,   // the event description display
    event_data: Rc<RefCell<ItemId>>,  // the data associated with the event
    minutes_spin: gtk::SpinButton,    // the minutes spin button
    millis_spin: gtk::SpinButton,     // the milliseconds spin button
    is_left: bool,                    // whether the element is on the left
}

impl EditQueueEvent {
    // A function to ceate a queue event variant
    //
    fn new(system_send: &SystemSend, is_left: bool) -> EditQueueEvent {
        // Create the top-level grid
        let grid = gtk::Grid::new();

        // Create the labels and spin buttons
        let event_description = gtk::Button::new_with_label("Event: None");
        let minutes_label = gtk::Label::new(Some("Delay: Minutes"));
        let minutes_spin = gtk::SpinButton::new_with_range(0.0, MINUTES_LIMIT, 1.0);
        let millis_label = gtk::Label::new(Some("Milliseconds"));
        let millis_spin = gtk::SpinButton::new_with_range(0.0, 60000.0, 1.0);

        // Create the id to hold the event data
        let event_data = Rc::new(RefCell::new(ItemId::all_stop()));

        // Set up the event description to act as a drag source
        event_description.drag_source_set(
            gdk::ModifierType::MODIFIER_MASK,
            &vec![
                gtk::TargetEntry::new("STRING", gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY,
        );

        // Set up the event spin button to receive a dropped item pair
        event_description.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        event_description.connect_drag_data_received(clone!(event_data => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the event description
                widget.set_label(&format!("Event: {}", item_pair.description));

                // Get a copy of the event data
                if let Ok(mut event) = event_data.try_borrow_mut() {
                    // Update the event data
                    *event = item_pair.get_id();
                }

                // Serialize the item pair data
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));

        // Add all the components to the event grid
        grid.attach(&event_description, 0, 0, 2, 1);
        grid.attach(&minutes_label, 0, 1, 1, 1);
        grid.attach(&minutes_spin, 0, 2, 1, 1);
        grid.attach(&millis_label, 1, 1, 1, 1);
        grid.attach(&millis_spin, 1, 2, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the queue event variant
        grid.show_all();
        EditQueueEvent {
            grid,
            system_send: system_send.clone(),
            event_description,
            event_data,
            minutes_spin,
            millis_spin,
            is_left,
        }
    }

    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load the action
    //
    fn load_action(&self, event_delay: &EventDelay) {
        // Request the description associated with the event id
        self.system_send.send(Request {
            reply_to: DisplayComponent::EditActionElement {
                is_left: self.is_left,
                variant: EditActionElement::EditQueueEvent,
            },
            request: RequestType::Description { item_id: event_delay.id() },
        });

        // Calculate the minutes and seconds of the duration
        if let Some(delay) = event_delay.delay() {
            // May be and empty delay
            let time = delay.as_secs();
            let remainder = time % 60;
            self.minutes_spin
                .set_value(((time - remainder) / 60) as f64);
            self.millis_spin
                .set_value(((remainder * 1000) + (delay.subsec_millis() as u64)) as f64);

        // Otherwise, set them to zero
        } else {
            self.minutes_spin.set_value(0.0);
            self.millis_spin.set_value(0.0);
        }
    }

    // A method to update the description of the event
    //
    pub fn update_description(&self, description: ItemDescription) {
        // Update the event label
        self.event_description.set_label(&format!("Event: {}", description.description));
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Borrow the event data
        match self.event_data.try_borrow() {
            Ok(event_data) => {
                // Extract the minute count
                let minutes = self.minutes_spin.get_value() as u32;

                // Extract the millis count
                let millis = self.millis_spin.get_value() as u32;

                // Compose the new delay
                let mut delay = None;
                if (minutes != 0) | (millis != 0) {
                    delay = Some(Duration::from_millis((millis + (minutes * 60000)) as u64));
                }

                // Compose the event delay and return the event action
                EventAction::QueueEvent {
                    event: EventDelay::new(delay, *event_data),
                }
            }

            _ => unreachable!(),
        }
    }
}

// Create the cancel event variant
//
#[derive(Clone, Debug)]
struct EditCancelEvent {
    grid: gtk::Grid,                  // the main grid for this element
    system_send: SystemSend,          // a copy of the system send line
    event_description: gtk::Button,   // the event description
    event_data: Rc<RefCell<ItemId>>,  // the data associated with the event
    is_left: bool                     // whether the element is on the left or right
}

impl EditCancelEvent {
    // A function to ceate a cancel event variant
    //
    fn new(system_send: &SystemSend, is_left: bool) -> EditCancelEvent {
        // Create the top level grid
        let grid = gtk::Grid::new();

        // Create the button to hold the event description
        let event_description = gtk::Button::new_with_label("Event: None");

        // Create the variable to hold the event data
        let event_data = Rc::new(RefCell::new(ItemId::all_stop()));

        // Set up the event description to act as a drag source
        event_description.drag_source_set(
            gdk::ModifierType::MODIFIER_MASK,
            &vec![
                gtk::TargetEntry::new("STRING", gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY,
        );

        // Set up the event description to receive a dropped item pair
        event_description.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        event_description.connect_drag_data_received(clone!(event_data => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the event description
                widget.set_label(&format!("Event: {}", item_pair.description));

                // Get a copy of the event data
                if let Ok(mut event) = event_data.try_borrow_mut() {
                    // Update the event data
                    *event = item_pair.get_id();
                }

                // Serialize the item pair data
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));

        // Attach the description to the grid
        grid.attach(&event_description, 0, 0, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the cancel event variant
        grid.show_all();
        EditCancelEvent {
            grid,
            system_send: system_send.clone(),
            event_description,
            event_data,
            is_left,
        }
    }

    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load the action
    //
    fn load_action(&self, event: &ItemId) {
        // Request the description associated with the id
        self.system_send.send(Request {
            reply_to: DisplayComponent::EditActionElement {
                is_left: self.is_left,
                variant: EditActionElement::EditCancelEvent
            },
            request: RequestType::Description { item_id: event.clone() },
        });
    }

    // A method to update the description of the event
    pub fn update_description(&self, description: ItemDescription) {
        // Update the event label
        self.event_description.set_label(&format!("Event: {}", description.description));
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Get the event data
        match self.event_data.try_borrow() {
            Ok(event_data) => {
                // Return the completed action
                EventAction::CancelEvent {
                    event: *event_data,
                }
            },

            _ => unreachable!(),
        }
    }
}

// Create the save data variant
//
#[derive(Clone, Debug)]
struct EditSaveData {
    grid: gtk::Grid,               // the main grid for this element
    data_type: gtk::ComboBoxText,  // the data type dropdown
    event_spin: gtk::SpinButton,   // the event spin button
    minutes_spin: gtk::SpinButton, // the minutes spin button
    millis_spin: gtk::SpinButton,  // the milliseconds spin button
    string_entry: gtk::Entry,      // the entry for the hardcoded string
}

impl EditSaveData {
    // A function to ceate a save data variant
    //
    fn new() -> EditSaveData {
        // Create the dropdown selection for the data type
        let data_type = gtk::ComboBoxText::new();

        // Add each of the available data types to the dropdown
        data_type.append(Some("timeuntil"), "Time until an event will occur");
        data_type.append(
            Some("timepasseduntil"),
            "Time passed since an event was queued",
        );
        data_type.append(Some("staticstring"), "A hardcoded string of data");
        data_type.append(Some("userstring"), "A user-provided string");

        // Add the entry boxes for the different data types
        let event_label = gtk::Label::new(Some("Event to track"));
        let event_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let minutes_label = gtk::Label::new(Some("Time: Minutes"));
        let minutes_spin = gtk::SpinButton::new_with_range(0.0, MINUTES_LIMIT, 1.0);
        let millis_label = gtk::Label::new(Some("Milliseconds"));
        let millis_spin = gtk::SpinButton::new_with_range(0.0, 60000.0, 1.0);
        let string_label = gtk::Label::new(Some("Data:"));
        let string_entry = gtk::Entry::new();
        string_entry.set_placeholder_text(Some("Enter Data Here"));

        // Set up the event spin button to receive a dropped item pair
        event_spin.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        event_spin.connect_drag_data_received(|widget, _, _, _, selection_data, _, _| {
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


        // Connect the function to trigger when the data type changes
        data_type.connect_changed(clone!(
            event_label,
            event_spin,
            minutes_label,
            minutes_spin,
            millis_label,
            millis_spin,
            string_label,
            string_entry
        => move |dropdown| {
            // Identify the selected data type
            if let Some(data_type) = dropdown.get_active_id() {
                // Match the selection and change the visible options
                match data_type.as_str() {
                    // The time until variant
                    "timeuntil" => {
                        event_label.show();
                        event_spin.show();
                        minutes_label.hide();
                        minutes_spin.hide();
                        millis_label.hide();
                        millis_spin.hide();
                        string_label.hide();
                        string_entry.hide();
                    }

                    // The time passed until variant
                    "timepasseduntil" => {
                        event_label.show();
                        event_spin.show();
                        minutes_label.show();
                        minutes_spin.show();
                        millis_label.show();
                        millis_spin.show();
                        string_label.hide();
                        string_entry.hide();
                    }

                    // The static string variant
                    "staticstring" => {
                        event_label.hide();
                        event_spin.hide();
                        minutes_label.hide();
                        minutes_spin.hide();
                        millis_label.hide();
                        millis_spin.hide();
                        string_label.show();
                        string_entry.show();
                    }

                    // The user string variant
                    _ => {
                        event_label.hide();
                        event_spin.hide();
                        minutes_label.hide();
                        minutes_spin.hide();
                        millis_label.hide();
                        millis_spin.hide();
                        string_label.hide();
                        string_entry.hide();
                    }
                }
            }
        }));

        // Add the buttons below the data list
        let grid = gtk::Grid::new();
        grid.attach(&data_type, 0, 0, 2, 1);
        grid.attach(&event_label, 0, 1, 1, 1);
        grid.attach(&event_spin, 0, 2, 1, 1);
        grid.attach(&minutes_label, 1, 1, 1, 1);
        grid.attach(&minutes_spin, 1, 2, 1, 1);
        grid.attach(&millis_label, 2, 1, 1, 1);
        grid.attach(&millis_spin, 2, 2, 1, 1);
        grid.attach(&string_label, 0, 3, 1, 1);
        grid.attach(&string_entry, 1, 3, 2, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the save data variant
        grid.show_all();
        EditSaveData {
            grid,
            data_type,
            event_spin,
            minutes_spin,
            millis_spin,
            string_entry,
        }
    }

    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load the action
    //
    fn load_action(&self, data: &DataType) {
        // Match the data type
        match data {
            // The TimeUntil variant
            &DataType::TimeUntil { ref event_id } => {
                // Change the dropdowm
                self.data_type.set_active_id(Some("timeuntil"));

                // Update the fields
                self.event_spin.set_value(event_id.id() as f64);
            }

            // The TimePassedUntil variant
            &DataType::TimePassedUntil {
                ref event_id,
                ref total_time,
            } => {
                // Change the dropdowm
                self.data_type.set_active_id(Some("timepasseduntil"));

                // Update the event spin button
                self.event_spin.set_value(event_id.id() as f64);

                // Calculate the minutes and seconds of the total time
                let time = total_time.as_secs();
                let remainder = time % 60;
                self.minutes_spin
                    .set_value(((time - remainder) / 60) as f64);
                self.millis_spin
                    .set_value(((remainder * 1000) + (total_time.subsec_millis() as u64)) as f64);
            }

            // The StaticString variant
            &DataType::StaticString { ref string } => {
                // Change the dropdown
                self.data_type.set_active_id(Some("staticstring"));

                // Update the string entry
                self.string_entry.set_text(&string);
            }

            // The UserString variant
            &DataType::UserString => {
                // Change the dropdown
                self.data_type.set_active_id(Some("userstring"));
            }
        }
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Extract the dropdown and corresponding data
        if let Some(data_type) = self.data_type.get_active_id() {
            let data = match data_type.as_str() {
                // The TimeUntil variant
                "timeuntil" => {
                    DataType::TimeUntil {
                        // Get the event id
                        event_id: ItemId::new_unchecked(self.event_spin.get_value() as u32),
                    }
                }

                // The TimePassedUntil variant
                "timepasseduntil" => {
                    // Extract the minute count
                    let minutes = self.minutes_spin.get_value() as u32;

                    // Extract the millis count
                    let millis = self.millis_spin.get_value() as u32;

                    // Compose the total time
                    let time = Duration::from_millis((millis + (minutes * 60000)) as u64);

                    DataType::TimePassedUntil {
                        event_id: ItemId::new_unchecked(self.event_spin.get_value() as u32),
                        total_time: time,
                    }
                }

                // The StaticString variant
                "staticstring" => {
                    // Extract the string, if there is one
                    if let Some(string) = self.string_entry.get_text() {
                        DataType::StaticString { string: string.to_string(), }
                    } else {
                        DataType::StaticString { string: String::new(), }
                    }
                }

                // The UserString variant
                _ => DataType::UserString,
            };

            // Return the completed action
            return EventAction::SaveData { data };

        // If nothing was selected, return UserString by default
        } else {
            return EventAction::SaveData {
                data: DataType::UserString,
            };
        }
    }
}

// Create the send data variant
//
#[derive(Clone, Debug)]
struct EditSendData {
    grid: gtk::Grid,               // the main grid for this element
    data_type: gtk::ComboBoxText,  // the data type dropdown
    event_spin: gtk::SpinButton,   // the event spin button
    minutes_spin: gtk::SpinButton, // the minutes spin button
    millis_spin: gtk::SpinButton,  // the milliseconds spin button
    string_entry: gtk::Entry,      // the entry for the hardcoded string
}

impl EditSendData {
    // A function to ceate a send data variant
    //
    fn new() -> EditSendData {
        // Create the dropdown selection for the data type
        let data_type = gtk::ComboBoxText::new();

        // Add each of the available data types to the dropdown
        data_type.append(Some("timeuntil"), "Time until an event will occur");
        data_type.append(
            Some("timepasseduntil"),
            "Time passed since an event was queued",
        );
        data_type.append(Some("staticstring"), "A hardcoded string of data");
        data_type.append(Some("userstring"), "A user-provided string");

        // Add the entry boxes for the different data types
        let event_label = gtk::Label::new(Some("Event to track"));
        let event_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let minutes_label = gtk::Label::new(Some("Time: Minutes"));
        let minutes_spin = gtk::SpinButton::new_with_range(0.0, MINUTES_LIMIT, 1.0);
        let millis_label = gtk::Label::new(Some("Milliseconds"));
        let millis_spin = gtk::SpinButton::new_with_range(0.0, 60000.0, 1.0);
        let string_label = gtk::Label::new(Some("Data:"));
        let string_entry = gtk::Entry::new();
        string_entry.set_placeholder_text(Some("Enter Data Here"));

        // Set up the event spin button to receive a dropped item pair
        event_spin.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        event_spin.connect_drag_data_received(|widget, _, _, _, selection_data, _, _| {
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

        // Connect the function to trigger when the data type changes
        data_type.connect_changed(clone!(
            event_label,
            event_spin,
            minutes_label,
            minutes_spin,
            millis_label,
            millis_spin,
            string_label,
            string_entry
        => move |dropdown| {
            // Identify the selected data type
            if let Some(data_type) = dropdown.get_active_id() {
                // Match the selection and change the visible options
                match data_type.as_str() {
                    // The time until variant
                    "timeuntil" => {
                        event_label.show();
                        event_spin.show();
                        minutes_label.hide();
                        minutes_spin.hide();
                        millis_label.hide();
                        millis_spin.hide();
                        string_label.hide();
                        string_entry.hide();
                    }

                    // The time passed until variant
                    "timepasseduntil" => {
                        event_label.show();
                        event_spin.show();
                        minutes_label.show();
                        minutes_spin.show();
                        millis_label.show();
                        millis_spin.show();
                        string_label.hide();
                        string_entry.hide();
                    }

                    // The static string variant
                    "staticstring" => {
                        event_label.hide();
                        event_spin.hide();
                        minutes_label.hide();
                        minutes_spin.hide();
                        millis_label.hide();
                        millis_spin.hide();
                        string_label.show();
                        string_entry.show();
                    }

                    // The user string variant
                    _ => {
                        event_label.hide();
                        event_spin.hide();
                        minutes_label.hide();
                        minutes_spin.hide();
                        millis_label.hide();
                        millis_spin.hide();
                        string_label.hide();
                        string_entry.hide();
                    }
                }
            }
        }));

        // Add the buttons below the data list
        let grid = gtk::Grid::new();
        grid.attach(&data_type, 0, 0, 2, 1);
        grid.attach(&event_label, 0, 1, 1, 1);
        grid.attach(&event_spin, 0, 2, 1, 1);
        grid.attach(&minutes_label, 1, 1, 1, 1);
        grid.attach(&minutes_spin, 1, 2, 1, 1);
        grid.attach(&millis_label, 2, 1, 1, 1);
        grid.attach(&millis_spin, 2, 2, 1, 1);
        grid.attach(&string_label, 0, 3, 1, 1);
        grid.attach(&string_entry, 1, 3, 2, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the save data variant
        grid.show_all();
        EditSendData {
            grid,
            data_type,
            event_spin,
            minutes_spin,
            millis_spin,
            string_entry,
        }
    }


    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load the action
    //
    fn load_action(&self, data: &DataType) {
        // Match the data type
        match data {
            // The TimeUntil variant
            &DataType::TimeUntil { ref event_id } => {
                // Change the dropdowm
                self.data_type.set_active_id(Some("timeuntil"));

                // Update the fields
                self.event_spin.set_value(event_id.id() as f64);
            }

            // The TimePassedUntil variant
            &DataType::TimePassedUntil {
                ref event_id,
                ref total_time,
            } => {
                // Change the dropdowm
                self.data_type.set_active_id(Some("timepasseduntil"));

                // Update the event spin button
                self.event_spin.set_value(event_id.id() as f64);

                // Calculate the minutes and seconds of the total time
                let time = total_time.as_secs();
                let remainder = time % 60;
                self.minutes_spin
                    .set_value(((time - remainder) / 60) as f64);
                self.millis_spin
                    .set_value(((remainder * 1000) + (total_time.subsec_millis() as u64)) as f64);
            }

            // The StaticString variant
            &DataType::StaticString { ref string } => {
                // Change the dropdown
                self.data_type.set_active_id(Some("staticstring"));

                // Update the string entry
                self.string_entry.set_text(&string);
            }

            // The UserString variant
            &DataType::UserString => {
                // Change the dropdown
                self.data_type.set_active_id(Some("userstring"));
            }
        }
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Extract the dropdown and corresponding data
        if let Some(data_type) = self.data_type.get_active_id() {
            let data = match data_type.as_str() {
                // The TimeUntil variant
                "timeuntil" => {
                    DataType::TimeUntil {
                        // Get the event id
                        event_id: ItemId::new_unchecked(self.event_spin.get_value() as u32),
                    }
                }

                // The TimePassedUntil variant
                "timepasseduntil" => {
                    // Extract the minute count
                    let minutes = self.minutes_spin.get_value() as u32;

                    // Extract the millis count
                    let millis = self.millis_spin.get_value() as u32;

                    // Compose the total time
                    let time = Duration::from_millis((millis + (minutes * 60000)) as u64);

                    DataType::TimePassedUntil {
                        event_id: ItemId::new_unchecked(self.event_spin.get_value() as u32),
                        total_time: time,
                    }
                }

                // The StaticString variant
                "staticstring" => {
                    if let Some(string) = self.string_entry.get_text() {
                        DataType::StaticString { string: string.to_string(), }
                    } else {
                        DataType::StaticString { string: String::new(), }
                    }
                }

                // The UserString variant
                _ => DataType::UserString,
            };

            // Return the completed action
            return EventAction::SendData { data };

        // If nothing was selected, return UserString by default
        } else {
            return EventAction::SendData {
                data: DataType::UserString,
            };
        }
    }
}

// Create the grouped event variant
//
#[derive(Clone, Debug)]
struct EditGroupedEvent {
    grid: gtk::Grid,                  // the main grid for this element
    grouped_event_list: gtk::ListBox, // the list for events in this variant
    grouped_events: Rc<RefCell<FnvHashMap<ItemId, ItemId>>>, // a database for the grouped events
    status_spin: gtk::SpinButton,     // the status id for this variant
    is_left: bool,                    // whether the element is to the left or right
}

impl EditGroupedEvent {
    // A function to create a grouped event variant
    //
    fn new(system_send: &SystemSend, is_left: bool) -> EditGroupedEvent {
        // Create the list for the trigger events variant
        let grouped_event_list = gtk::ListBox::new();
        grouped_event_list.set_selection_mode(gtk::SelectionMode::None);

        // Create the status spin
        let status_label = gtk::Label::new(Some("Status"));
        let status_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Set up the status spin button to receive a dropped item pair
        status_spin.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        status_spin.connect_drag_data_received(|widget, _, _, _, selection_data, _, _| {
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


        // Create the scrollable window for the list
        let group_window = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        group_window.add(&grouped_event_list);
        group_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        group_window.set_hexpand(true);
        group_window.set_size_request(-1, 120);

        // Connect the function to trigger when the status spin changes
        status_spin.connect_changed(clone!(system_send, is_left => move |spin| {
            system_send.send(Request {
                reply_to: DisplayComponent::EditActionElement { is_left, variant: EditActionElement::Overview },
                request: RequestType::Status { item_id: ItemId::new_unchecked(spin.get_value() as u32), },
            });
        }));

        // Add the status above and button below the event list
        let grid = gtk::Grid::new();
        grid.attach(&status_label, 0, 0, 1, 1);
        grid.attach(&status_spin, 1, 0, 1, 1);
        grid.attach(&group_window, 0, 1, 2, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the grouped event variant
        grid.show_all();
        EditGroupedEvent {
            grid,
            grouped_event_list,
            grouped_events: Rc::new(RefCell::new(FnvHashMap::default())),
            status_spin,
            is_left
        }
    }

    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load an action
    fn load_action(&self, status_id: &ItemId, event_map: &FnvHashMap<ItemId, ItemId>) {
        // Clear the database and the rows
        self.clear();

        // Change the status id
        self.status_spin.set_value(status_id.id() as f64);

        // Add each event in the map to the list
        for (ref state_id, ref event_id) in event_map.iter() {
            EditGroupedEvent::add_event(
                &self.grouped_events,
                &self.grouped_event_list,
                state_id,
                Some(event_id),
            );
        }
    }

    // A method to update the listed states in the grouped event
    fn update_info(&mut self, possible_status: Option<Status>) {
        // Clear the ListBox
        self.clear();

        // Check to see if the status is valid
        if let Some(status) = possible_status {
            // Show the states associated with the valid status
            for state_id in status.allowed() {
                EditGroupedEvent::add_event(
                    &self.grouped_events,
                    &self.grouped_event_list,
                    &state_id,
                    None,
                );
            }
        } else {
            // Create the label that will replace the list if the spin value is not valid
            let invalid_label = gtk::Label::new(Some("Not a valid status."));

            // Display "not a valid status"
            &self.grouped_event_list.add(&invalid_label);
            invalid_label.show();
        }
    }

    // A method to clear all the listed states in the ListBox
    pub fn clear(&self) {
        // Remove all the user interface elements
        let to_remove = self.grouped_event_list.get_children();
        for item in to_remove {
            item.destroy();
        }
        // Empty the database
        if let Ok(mut events) = self.grouped_events.try_borrow_mut() {
            events.clear();
        }
    }

    // A helper function to add a grouped event to the list
    fn add_event(
        grouped_events: &Rc<RefCell<FnvHashMap<ItemId, ItemId>>>,
        grouped_event_list: &gtk::ListBox,
        state_id: &ItemId,
        event_id: Option<&ItemId>
    ){
        // Check to see if an event id is given
        if let Some(event_id) = event_id {
            // Add the state id and event id to the database
            if let Ok(mut events) = grouped_events.try_borrow_mut() {
                events.insert(state_id.clone(), event_id.clone());
            }
        }

        // Create a state spin box for the list
        let group_grid = gtk::Grid::new();
        let state_label = gtk::Label::new(Some(&format!("State Id: {}", state_id.id())));
        state_label.set_size_request(80, 30);
        state_label.set_hexpand(false);
        state_label.set_vexpand(false);

        // Create a event spin box for the list
        let event_label = gtk::Label::new(Some("Event"));
        event_label.set_size_request(80, 30);
        event_label.set_hexpand(false);
        event_label.set_vexpand(false);
        let event_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        event_spin.set_size_request(100, 30);
        event_spin.set_hexpand(false);

        // Set up the event spin button to receive a dropped item pair
        event_spin.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![
                gtk::TargetEntry::new("STRING",gtk::TargetFlags::SAME_APP, 0),
            ],
            gdk::DragAction::COPY
        );

        // Set the callback function when data is received
        event_spin.connect_drag_data_received(|widget, _, _, _, selection_data, _, _| {
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

        // Update the database whenever the event is changed
        event_spin.connect_changed(clone!(grouped_events, state_id => move |spin| {
            if let Ok(mut events) = grouped_events.try_borrow_mut() {
                events.insert(state_id.clone(), ItemId::new_unchecked(spin.get_value() as u32));
            }
        }));

        // Add all the items to the group grid
        group_grid.attach(&state_label, 0, 0, 1, 1);
        group_grid.attach(&event_label, 1, 0, 1, 1);
        group_grid.attach(&event_spin, 2, 0, 1, 1);

        // Set the value of the grouped event if it was provided
        if let Some(event) = event_id {
            event_spin.set_value(event.id() as f64);
        }

        // Add the new grid to the list
        group_grid.show_all();
        grouped_event_list.add(&group_grid);
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Create the event vector
        let event_map = match self.grouped_events.try_borrow() {
            Ok(events) => events.clone(),
            _ => FnvHashMap::default(),
        };

        // Extract the status id
        let status_id = ItemId::new_unchecked(self.status_spin.get_value() as u32);

        // Return the completed Event Action
        EventAction::GroupedEvent {
            status_id,
            event_map,
        }
    }
}
