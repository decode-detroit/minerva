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
    DataType, DisplayComponent, EditActionElement, EventAction, Event, EventDelay,
    ItemId, ItemPair, Request, RequestType, Status, SyncSystemSend,
};

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

// Import FNV HashMap
use fnv;
use self::fnv::FnvHashMap;

// Import GTK and GDK libraries
use gdk;
use gtk;
use self::gtk::prelude::*;
use self::gtk::GridExt;

// Define and import constants
const MINUTES_LIMIT: f64 = 10080.0; // maximum input time for a delayed event (one week)


/// A compound structure to store the state and event ids, as well as
/// the elements where they are displayed, for a select event
///
#[derive(Clone, Debug)]
pub struct EventGrouping {
    state_id: ItemId,
    event_id: ItemId,
    state_label: gtk::Label,
    event_label: gtk:: Button,
}


// Create a structure for editing the event
#[derive(Clone, Debug)]
pub struct EditEvent {
    grid: gtk::Grid,                   // the main grid for this element
    edit_action: Rc<RefCell<EditAction>>, // a wrapped dialog to edit the current action
    edit_action_window: gtk::Grid,    // the window holding the edit action
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
    pub fn new(system_send: &SyncSystemSend, is_left: bool) -> EditEvent {
        // Create the grid
        let grid = gtk::Grid::new();

        // Construct the checkbox for the event
        let event_checkbox = gtk::CheckButton::with_label("Item Corresponds To An Event");

        // Create the empty event actions
        let event_actions = Rc::new(RefCell::new(FnvHashMap::default()));

        // Create the starting next position
        let next_position = Rc::new(RefCell::new(0));

        // Create the action list for the events
        let action_list = gtk::ListBox::new();
        action_list.set_selection_mode(gtk::SelectionMode::None);

        // Create a new edit action dialog
        let tmp_edit_action = EditAction::new(system_send, &event_actions, is_left);
        let edit_action_window = tmp_edit_action.get_top_element().clone();
        let edit_action = Rc::new(RefCell::new(tmp_edit_action));

        // Create a button to add actions to the list
        let add_button = gtk::Button::from_icon_name(
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
            edit_action_window
        => move | checkbox | {
            // Make the elements invisible if the box isn't checked
            if checkbox.get_active() {
                action_window.show();
                add_button.show();
                edit_action_window.show();
            } else {
                action_window.hide();
                add_button.hide();
                edit_action_window.hide();
            }
        }));

        // Add the button below the data list
        grid.attach(&event_checkbox, 0, 0, 1, 1);
        grid.attach(&action_window, 0, 1, 1, 1);
        grid.attach(&add_button, 0, 2, 1, 1);
        grid.attach(&edit_action_window, 1, 1, 1, 2);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);
        grid.show_all();
                
        // Default to unchecked
        event_checkbox.set_active(false);
        action_window.hide();
        add_button.hide();
        edit_action_window.hide();

        // Create and return the trigger events variant
        EditEvent {
            grid,
            edit_action,
            edit_action_window,
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
        // Hide the edit action window
        self.edit_action_window.hide();
        
        // Remove the existing event actions
        if let Ok(mut actions) = self.event_actions.try_borrow_mut() {
            actions.clear();
        }

        // Clear the existing list of actions
        for item in self.action_list.get_children() {
            unsafe {
                item.destroy();
            }
        }
        
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
    pub fn update_info(&self, variant: EditActionElement, status: Option<Status>) {
        // Try to get access the edit action dialog
        if let Ok(dialog) = self.edit_action.try_borrow() {
            dialog.update_info(variant, status);
        }
    }

    // A method to update the description of an item
    pub fn update_description(&self, action_type: EditActionElement, description: ItemPair) {
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
                EventAction::CueEvent { .. } => overview.set_text("Cue Event"),
                EventAction::CancelEvent { .. } => overview.set_text("Cancel Event"),
                EventAction::SaveData { .. } => overview.set_text("Save Data"),
                EventAction::SendData { .. } => overview.set_text("Send Data"),
                EventAction::SelectEvent { .. } => overview.set_text("Select Event"),
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
        let edit_button = gtk::Button::from_icon_name(
            Some("document-edit-symbolic"),
            gtk::IconSize::Button.into(),
        );
        edit_button.connect_clicked(
            clone!(edit_action, position, overview, row => move |_| {
                // Try to get access to the edit action element
                if let Ok(mut edit_action) = edit_action.try_borrow_mut() {
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
    edit_cue_event: Rc<RefCell<EditCueEvent>>,              // the wrapped EditCueEvent structure
    edit_cancel_event: Rc<RefCell<EditCancelEvent>>,            // the wrapped EditCancelEvent structure
    edit_save_data: Rc<RefCell<EditSaveData>>,                  // the wrapped EditSaveData structure
    edit_send_data: Rc<RefCell<EditSendData>>,                  // the wrapped EditSendData structure
    edit_select_event: Rc<RefCell<EditSelectEvent>>,          // the wrapped EditSelectEvent structure
    event_actions: Rc<RefCell<FnvHashMap<usize, EventAction>>>, // a wrapped hash map of event actions
    is_left: bool,                                              // whether the element is on the left
}

// Implement key features of the EditAction
impl EditAction {
    /// A function to create a new instance of the EditAction
    ///
    fn new(
        system_send: &SyncSystemSend,
        event_actions: &Rc<RefCell<FnvHashMap<usize, EventAction>>>,
        is_left: bool,
    ) -> EditAction {
        // Create a dropdown for the action selection
        let action_selection = gtk::ComboBoxText::new();

        // Add each of the available action types to the dropdown
        action_selection.append(Some("newscene"), "New Scene");
        action_selection.append(Some("modifystatus"), "Modify Status");
        action_selection.append(Some("cueevent"), "Cue Event");
        action_selection.append(Some("cancelevent"), "Cancel Event");
        action_selection.append(Some("savedata"), "Save Data");
        action_selection.append(Some("senddata"), "Send Data");
        action_selection.append(Some("selectevent"), "Select Event");

        // Create the different edit windows for the action types
        let edit_new_scene = EditNewScene::new(system_send, is_left);
        let edit_modify_status = EditModifyStatus::new(system_send, is_left);
        let edit_cue_event = EditCueEvent::new(system_send, is_left);
        let edit_cancel_event = EditCancelEvent::new(system_send, is_left);
        let edit_save_data = EditSaveData::new(system_send, is_left);
        let edit_send_data = EditSendData::new(system_send, is_left);
        let edit_select_event = EditSelectEvent::new(system_send, is_left);

        // Create the action stack
        let action_stack = gtk::Stack::new();

        // Add the edit types to the action stack
        action_stack.add_named(edit_new_scene.get_top_element(), "newscene");
        action_stack.add_named(edit_modify_status.get_top_element(), "modifystatus");
        action_stack.add_named(edit_cue_event.get_top_element(), "cueevent");
        action_stack.add_named(edit_cancel_event.get_top_element(), "cancelevent");
        action_stack.add_named(edit_save_data.get_top_element(), "savedata");
        action_stack.add_named(edit_send_data.get_top_element(), "senddata");
        action_stack.add_named(edit_select_event.get_top_element(), "selectevent");

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
        action_selection.show();
        action_stack.show();
        grid.hide();

        EditAction {
            grid,
            action_selection,
            edit_new_scene: Rc::new(RefCell::new(edit_new_scene)),
            edit_modify_status: Rc::new(RefCell::new(edit_modify_status)),
            edit_cue_event: Rc::new(RefCell::new(edit_cue_event)),
            edit_cancel_event: Rc::new(RefCell::new(edit_cancel_event)),
            edit_save_data: Rc::new(RefCell::new(edit_save_data)),
            edit_send_data: Rc::new(RefCell::new(edit_send_data)),
            edit_select_event: Rc::new(RefCell::new(edit_select_event)),
            event_actions: event_actions.clone(),
            is_left,
        }
    }

    /// A method to load a new action
    ///
    fn load_action(&mut self, position: usize, overview: &gtk::Label, row: &gtk::ListBoxRow) {
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

        // Try to get a copy of the edit cue event
        let edit_cue_event = match self.edit_cue_event.try_borrow() {
            Ok(edit_cue) => edit_cue,
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

        // Try to get a copy of the edit select event
        let mut edit_select_event = match self.edit_select_event.try_borrow_mut() {
            Ok(edit_select) => edit_select,
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

            // the CueEvent variant
            EventAction::CueEvent { event } => {
                self.action_selection.set_active_id(Some("CueEvent"));
                edit_cue_event.load_action(event);
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

            // the SelectEvent variant
            EventAction::SelectEvent {
                status_id,
                event_map,
            } => {
                self.action_selection.set_active_id(Some("selectevent"));
                edit_select_event.load_action(status_id, event_map);
            }
        }

        // Create the button to save an action
        let save_button = gtk::Button::with_label("Save");

        // Create the button to delete an action
        let delete_button = gtk::Button::with_label("Delete");

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
            unsafe {
                row.destroy();
            }

            // Delete the save and delete buttons
            unsafe {
                save_button.destroy();
                delete_button.destroy();
            }

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
            edit_cue_event,
            edit_cancel_event,
            edit_save_data,
            edit_send_data,
            edit_select_event => move |_| {

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

                        // the CueEvent variant
                        "cueevent" => {
                            // Update the action label and action
                            overview.set_text("Cue Event");
                            *action = edit_cue_event.pack_action();
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

                        // The SelectEvent variant
                        "selectevent" => {
                            // Update the action label and action
                            overview.set_text("Select Event");
                            *action = edit_select_event.pack_action();
                        }

                        _ => unreachable!(),
                    }

                // If no selection was made, exit prematurely
                } else {
                    return;
                }
            // Delete the save and delete buttons
            unsafe {
                save_button.destroy();
                delete_button.destroy();
            }

            // Hide the grid to prevent editing
            grid.hide();
        }));

        // Add the save and delete buttons to the grid
        self.grid.attach(&save_button, 0, 2, 1, 1);
        self.grid.attach(&delete_button, 1, 2, 1, 1);
        save_button.show();
        delete_button.show();
    }

    /// A method to return the top element
    ///
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    /// A method to update the description of an item
    ///
    pub fn update_description(&self, action_type: EditActionElement, description: ItemPair) {
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

            EditActionElement::EditCueEvent => {
                // Get a copy of the edit cue event element
                if let Ok(edit_cue_event) = self.edit_cue_event.try_borrow() {
                    edit_cue_event.update_description(description)
                }
            }

            EditActionElement::EditCancelEvent => {
                // Get a copy of the edit cancel event element
                if let Ok(edit_cancel_event) = self.edit_cancel_event.try_borrow() {
                    edit_cancel_event.update_description(description)
                }
            }

            EditActionElement::EditSaveData => {
                // Get a copy of the edit save data element
                if let Ok(edit_save_data) = self.edit_save_data.try_borrow() {
                    edit_save_data.update_description(description)
                }
            }

            EditActionElement::EditSendData => {
                // Get a copy of the edit save data element
                if let Ok(edit_send_data) = self.edit_send_data.try_borrow() {
                    edit_send_data.update_description(description)
                }
            }

            EditActionElement::SelectEventDescription { position, is_event } => {
                // Get a copy of the edit select event element
                if let Ok(edit_select_event) = self.edit_select_event.try_borrow() {
                    edit_select_event.update_description(position, is_event, description)
                }
            }

            _ => unreachable!(),
        }
    }


    /// A method to pass the status to the structure that requested it
    ///
    fn update_info(&self, variant: EditActionElement, status: Option<Status>) {
        // Check who requested the status
        match variant {
            // The edit select event variant
            EditActionElement::SelectEventStates => {
                // Update the info in the edit select event
                if let Ok(mut edit_select_event) = self.edit_select_event.try_borrow_mut() {
                    edit_select_event.update_info(status);
                }
            }

            EditActionElement::EditModifyStatus { .. } => {
                // Update the states in the edit modify status
                if let Ok(edit_modify_status) = self.edit_modify_status.try_borrow() {
                    edit_modify_status.update_states(status);
                }
            }

            _ => unreachable!(),
        }
    }
}

// Create the new scene variant
#[derive(Clone, Debug)]
struct EditNewScene {
    grid: gtk::Grid,                      // the main grid for this element
    system_send: SyncSystemSend,              // a copy of the system send line
    description: gtk::Button,             // the description of the scene
    scene: Rc<RefCell<ItemId>>,           // the wrapped data associated with the scene
    is_left: bool,                        // whether the edit element is left or right
}

// Implement key features for Edit New Scene
impl EditNewScene {
    // A function to create a new scene variant
    //
    fn new(system_send: &SyncSystemSend, is_left: bool) -> EditNewScene {
        // Create the top level grid
        let grid = gtk::Grid::new();

        // Add a button with a label to the grid
        let description = gtk::Button::with_label("Scene: None");

        // Create the data associated with the scene
        let scene = Rc::new(RefCell::new(ItemId::all_stop()));

        // Set up the description to act as a drag source and destination
        drag!(source description);
        drag!(dest description);

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
        // Load the scene data
        if let Ok(mut scene_data) = self.scene.try_borrow_mut() {
            *scene_data = new_scene.clone();
        }

        // Request the description associated with the id
        self.system_send.send(Request {
            reply_to: DisplayComponent::EditActionElement { is_left: self.is_left, variant: EditActionElement::EditNewScene },
            request: RequestType::Description { item_id: new_scene.clone() },
        });
    }

    /// A method to update the description of the scene
    ///
    pub fn update_description(&self, description: ItemPair) {
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
    system_send: SyncSystemSend,                      // a copy of the system send line
    status_description: gtk::Button,              // the status description display
    status_data: Rc<RefCell<ItemId>>,             // the wrapped status data
    state_dropdown: gtk::ComboBoxText,            // the state description dropdown
    state_data: Rc<RefCell<ItemId>>,              // the wrapped state data
    state_database: Rc<RefCell<FnvHashMap<String, ItemId>>>, // the wrapped state description/id database
    is_left: bool,                                // whether the element is on the left
}

impl EditModifyStatus {
    // A function to ceate a modify status variant
    //
    fn new(system_send: &SyncSystemSend, is_left: bool) -> EditModifyStatus {
        // Create the grid for the modify status variant
        let grid = gtk::Grid::new();

        // Set up the labels and data
        let status_description = gtk::Button::with_label("Status: None");
        let status_data = Rc::new(RefCell::new(ItemId::all_stop()));
        let state_label = gtk::Label::new(Some("State:"));
        let state_dropdown = gtk::ComboBoxText::new();
        let state_data = Rc::new(RefCell::new(ItemId::all_stop()));

        // Create the database to store the valid states
        let state_database: Rc<RefCell<FnvHashMap<String, ItemId>>> = Rc::new(RefCell::new(FnvHashMap::default()));

        // Set up the status description to act as a drag source and destination
        drag!(source status_description);
        drag!(dest status_description);

        // Set the callback function when data is received
        status_description.connect_drag_data_received(clone!(status_data, system_send, is_left => move |widget, _, _, _, selection_data, _, _| {
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

                // Request the state data associated with the status
                system_send.send(Request {
                    reply_to: DisplayComponent::EditActionElement {
                        is_left,
                        variant: EditActionElement::EditModifyStatus { is_status: false }
                    },
                    request: RequestType::Status { item_id: item_pair.get_id() },
                });

                // Serialize the item pair data
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));


        // Set the callback function when the state dropdown is changed
        state_dropdown.connect_changed(clone!(state_database, state_data => move |dropdown| {
            // Identify the selected state
            if let Some(current_state) = dropdown.get_active_id() {
                // Look up the state in the database
                if let Ok(state_db) = state_database.try_borrow() {
                    if let Some(state_id) = state_db.get(&current_state.to_string()) {
                        // Store the current state in the state data
                        if let Ok(mut state_data) = state_data.try_borrow_mut() {
                            *state_data = state_id.clone();
                        }
                    }
                }
            }
        }));

        // Place everything into the grid
        grid.attach(&status_description, 0, 0, 2, 1);
        grid.attach(&state_label, 0, 1, 1, 1);
        grid.attach(&state_dropdown, 1, 1, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the EditModifyStatus
        grid.show_all();
        EditModifyStatus {
            grid,
            system_send: system_send.clone(),
            status_description,
            status_data,
            state_dropdown,
            state_data,
            state_database,
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
        // Clear the state dropdown and database
        self.clear();

        // Load the status data
        if let Ok(mut status_data) = self.status_data.try_borrow_mut() {
            *status_data = status_id.clone();
        }

        // Load the state data
        if let Ok(mut state_data) = self.state_data.try_borrow_mut() {
            *state_data = new_state.clone();
        }

        // Request the description associated with the status
        self.system_send.send(Request {
            reply_to: DisplayComponent::EditActionElement {
                is_left: self.is_left,
                variant: EditActionElement::EditModifyStatus { is_status: true }
            },
            request: RequestType::Description { item_id: status_id.clone() },
        });

        // Request the state data associated with the status
        self.system_send.send(Request {
            reply_to: DisplayComponent::EditActionElement {
                is_left: self.is_left,
                variant: EditActionElement::EditModifyStatus { is_status: false }
            },
            request: RequestType::Status { item_id: status_id.clone() },
        });
    }

    // A method to update the descriptions of states associated with a status
    //
    pub fn update_states(&self, status: Option<Status>) {
        // Unpack the status
        if let Some(status) = status {
            // Go through each allowed state and request its description
            for state_id in status.allowed().drain(..) {
                self.system_send.send(Request {
                    reply_to: DisplayComponent::EditActionElement {
                        is_left: self.is_left,
                        variant: EditActionElement::EditModifyStatus { is_status: false }
                    },
                    request: RequestType::Description { item_id: state_id.clone() },
                });
            }
        }
    }

    // A method to update the description of the status or state
    pub fn update_description(&self, is_status: bool, item_pair: ItemPair) {
        match is_status {
            // If the description is for the status, update the status label
            true => self.status_description.set_label(&format!("Status: {}", item_pair.description)),

            // If the description is for the state, add it to the dropdown and the state database
            false => {
                // Update the user interface
                self.state_dropdown.append(Some(&item_pair.description), &item_pair.description);

                // Update the associated data
                if let Ok(mut state_database) = self.state_database.try_borrow_mut() {
                    state_database.insert(item_pair.clone().description, item_pair.clone().get_id());
                }
            }
        }
    }

    // A method to clear all the listed states in the state dropdown and database
    pub fn clear(&self) {
        // Remove all the dropdown elements
        self.state_dropdown.remove_all();

        // Clear the database
        if let Ok(mut state_db) = self.state_database.try_borrow_mut() {
            state_db.clear();
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

// Create the cue event variant
//
#[derive(Clone, Debug)]
struct EditCueEvent {
    grid: gtk::Grid,                  // the main grid for this element
    system_send: SyncSystemSend,          // a copy of the system send line
    event_description: gtk::Button,   // the event description display
    event_data: Rc<RefCell<ItemId>>,  // the data associated with the event
    minutes_spin: gtk::SpinButton,    // the minutes spin button
    millis_spin: gtk::SpinButton,     // the milliseconds spin button
    is_left: bool,                    // whether the element is on the left
}

impl EditCueEvent {
    // A function to ceate a cue event variant
    //
    fn new(system_send: &SyncSystemSend, is_left: bool) -> EditCueEvent {
        // Create the top-level grid
        let grid = gtk::Grid::new();

        // Create the labels and spin buttons
        let event_description = gtk::Button::with_label("Event: None");
        let minutes_label = gtk::Label::new(Some("Delay: Minutes"));
        let minutes_spin = gtk::SpinButton::with_range(0.0, MINUTES_LIMIT, 1.0);
        let millis_label = gtk::Label::new(Some("Milliseconds"));
        let millis_spin = gtk::SpinButton::with_range(0.0, 60000.0, 1.0);

        // Create the id to hold the event data
        let event_data = Rc::new(RefCell::new(ItemId::all_stop()));

        // Set up the event description to act as a drag source and destination
        drag!(source event_description);
        drag!(dest event_description);

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

        // Create and return the cue event variant
        grid.show_all();
        EditCueEvent {
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
        // Load the event data
        if let Ok(mut event_data) = self.event_data.try_borrow_mut() {
            *event_data = event_delay.id();
        }

        // Request the description associated with the event id
        self.system_send.send(Request {
            reply_to: DisplayComponent::EditActionElement {
                is_left: self.is_left,
                variant: EditActionElement::EditCueEvent,
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
    pub fn update_description(&self, description: ItemPair) {
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
                EventAction::CueEvent {
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
    system_send: SyncSystemSend,          // a copy of the system send line
    event_description: gtk::Button,   // the event description
    event_data: Rc<RefCell<ItemId>>,  // the data associated with the event
    is_left: bool                     // whether the element is on the left or right
}

impl EditCancelEvent {
    // A function to ceate a cancel event variant
    //
    fn new(system_send: &SyncSystemSend, is_left: bool) -> EditCancelEvent {
        // Create the top level grid
        let grid = gtk::Grid::new();

        // Create the button to hold the event description
        let event_description = gtk::Button::with_label("Event: None");

        // Create the variable to hold the event data
        let event_data = Rc::new(RefCell::new(ItemId::all_stop()));

        // Set up the event description to act as a drag source and destination
        drag!(source event_description);
        drag!(dest event_description);

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
        // Load the event data
        if let Ok(mut event_data) = self.event_data.try_borrow_mut() {
            *event_data = event.clone();
        }

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
    pub fn update_description(&self, description: ItemPair) {
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
    grid: gtk::Grid,                 // the main grid for this element
    system_send: SyncSystemSend,         // a copy of the system send line
    data_type: gtk::ComboBoxText,    // the data type dropdown
    event_description: gtk::Button,  // the event description
    event_data: Rc<RefCell<ItemId>>, // the data associated with the event
    minutes_spin: gtk::SpinButton,   // the minutes spin button
    millis_spin: gtk::SpinButton,    // the milliseconds spin button
    string_entry: gtk::Entry,        // the entry for the hardcoded string
    is_left: bool,                   // whether the element is on the left or right
}

impl EditSaveData {
    // A function to ceate a save data variant
    //
    fn new(system_send: &SyncSystemSend, is_left: bool) -> EditSaveData {
        // Create the dropdown selection for the data type
        let data_type = gtk::ComboBoxText::new();

        // Add each of the available data types to the dropdown
        data_type.append(Some("timeuntil"), "Time until an event will occur");
        data_type.append(
            Some("timepasseduntil"),
            "Time passed since an event was cued",
        );
        data_type.append(Some("staticstring"), "A hardcoded string of data");
        data_type.append(Some("userstring"), "A user-provided string");

        // Add the entry boxes and data for the different data types
        let event_description = gtk::Button::with_label("Event: None");
        let event_data = Rc::new(RefCell::new(ItemId::all_stop()));
        let minutes_label = gtk::Label::new(Some("Time: Minutes"));
        let minutes_spin = gtk::SpinButton::with_range(0.0, MINUTES_LIMIT, 1.0);
        let millis_label = gtk::Label::new(Some("Milliseconds"));
        let millis_spin = gtk::SpinButton::with_range(0.0, 60000.0, 1.0);
        let string_label = gtk::Label::new(Some("Data:"));
        let string_entry = gtk::Entry::new();
        string_entry.set_placeholder_text(Some("Enter Data Here"));

        // Set up the event description to as a drag source and destination
        drag!(source event_description);
        drag!(dest event_description);

        // Set the callback function when data is received
        event_description.connect_drag_data_received(clone!(event_data => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the current description
                widget.set_label(&format!("Event: {}", item_pair.description));

                // Update the event data
                if let Ok(mut event) = event_data.try_borrow_mut() {
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


        // Connect the function to trigger when the data type changes
        data_type.connect_changed(clone!(
            event_description,
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
                        event_description.show();
                        minutes_label.hide();
                        minutes_spin.hide();
                        millis_label.hide();
                        millis_spin.hide();
                        string_label.hide();
                        string_entry.hide();
                    }

                    // The time passed until variant
                    "timepasseduntil" => {
                        event_description.show();
                        minutes_label.show();
                        minutes_spin.show();
                        millis_label.show();
                        millis_spin.show();
                        string_label.hide();
                        string_entry.hide();
                    }

                    // The static string variant
                    "staticstring" => {
                        event_description.hide();
                        minutes_label.hide();
                        minutes_spin.hide();
                        millis_label.hide();
                        millis_spin.hide();
                        string_label.show();
                        string_entry.show();
                    }

                    // The user string variant
                    _ => {
                        event_description.hide();
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
        grid.attach(&event_description, 0, 1, 2, 1);
        grid.attach(&minutes_label, 0, 2, 1, 1);
        grid.attach(&minutes_spin, 0, 3, 1, 1);
        grid.attach(&millis_label, 1, 2, 1, 1);
        grid.attach(&millis_spin, 1, 3, 1, 1);
        grid.attach(&string_label, 0, 4, 1, 1);
        grid.attach(&string_entry, 1, 4, 2, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the save data variant
        grid.show_all();
        EditSaveData {
            grid,
            system_send: system_send.clone(),
            data_type,
            event_description,
            event_data,
            minutes_spin,
            millis_spin,
            string_entry,
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
    fn load_action(&self, data: &DataType) {
        // Match the data type
        match data {
            // The TimeUntil variant
            &DataType::TimeUntil { ref event_id } => {
                // Change the dropdowm
                self.data_type.set_active_id(Some("timeuntil"));

                // Set the event data
                if let Ok(mut event_data) = self.event_data.try_borrow_mut() {
                    *event_data = event_id.clone();
                }

                // Request the description associated with the id
                self.system_send.send(Request {
                    reply_to: DisplayComponent::EditActionElement {
                        is_left: self.is_left,
                        variant: EditActionElement::EditSaveData
                    },
                    request: RequestType::Description { item_id: event_id.clone() },
                });
            }

            // The TimePassedUntil variant
            &DataType::TimePassedUntil {
                ref event_id,
                ref total_time,
            } => {
                // Change the dropdowm
                self.data_type.set_active_id(Some("timepasseduntil"));

                // Set the event data
                if let Ok(mut event_data) = self.event_data.try_borrow_mut() {
                    *event_data = event_id.clone();
                }

                // Request the description associated with the id
                self.system_send.send(Request {
                    reply_to: DisplayComponent::EditActionElement {
                        is_left: self.is_left,
                        variant: EditActionElement::EditSaveData
                    },
                    request: RequestType::Description { item_id: event_id.clone() },
                });

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

    // A method to update the description of the event
    pub fn update_description(&self, description: ItemPair) {
        // Update the event label
        self.event_description.set_label(&format!("Event: {}", description.description));
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Extract the dropdown and corresponding data
        if let Some(data_type) = self.data_type.get_active_id() {
            let data = match data_type.as_str() {
                // The TimeUntil variant
                "timeuntil" => {
                    // Get the event id
                    match self.event_data.try_borrow() {
                        Ok(event_data) => {
                            // Return the EventAction
                            DataType::TimeUntil {
                                event_id: event_data.clone(),
                            }
                        }

                        _ => unreachable!(),
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

                    // Get the event id
                    match self.event_data.try_borrow() {
                        Ok(event_data) => {
                            // Return the EventAction
                            DataType::TimePassedUntil {
                                event_id: event_data.clone(),
                                total_time: time,
                            }
                        }

                        _ => unreachable!(),
                    }


                }

                // The StaticString variant
                "staticstring" => {
                    // Extract the string
                    DataType::StaticString { string: self.string_entry.get_text().to_string(), }
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
    system_send: SyncSystemSend,       // a copy of the system send line
    data_type: gtk::ComboBoxText,  // the data type dropdown
    event_description: gtk::Button,// the event description display
    event_data: Rc<RefCell<ItemId>>, // the wrapped event data
    minutes_spin: gtk::SpinButton, // the minutes spin button
    millis_spin: gtk::SpinButton,  // the milliseconds spin button
    string_entry: gtk::Entry,      // the entry for the hardcoded string
    is_left: bool,
}

impl EditSendData {
    // A function to ceate a send data variant
    //
    fn new(system_send: &SyncSystemSend, is_left: bool) -> EditSendData {
        // Create the dropdown selection for the data type
        let data_type = gtk::ComboBoxText::new();

        // Add each of the available data types to the dropdown
        data_type.append(Some("timeuntil"), "Time until an event will occur");
        data_type.append(
            Some("timepasseduntil"),
            "Time passed since an event was cued",
        );
        data_type.append(Some("staticstring"), "A hardcoded string of data");
        data_type.append(Some("userstring"), "A user-provided string");

        // Add the entry boxes for the different data types
        let event_description = gtk::Button::with_label("Event to track");
        let minutes_label = gtk::Label::new(Some("Time: Minutes"));
        let minutes_spin = gtk::SpinButton::with_range(0.0, MINUTES_LIMIT, 1.0);
        let millis_label = gtk::Label::new(Some("Milliseconds"));
        let millis_spin = gtk::SpinButton::with_range(0.0, 60000.0, 1.0);
        let string_label = gtk::Label::new(Some("Data:"));
        let string_entry = gtk::Entry::new();
        string_entry.set_placeholder_text(Some("Enter Data Here"));

        // Add the variable to hold the event data
        let event_data = Rc::new(RefCell::new(ItemId::all_stop()));

        // Set up the event spin as a drag source and destination
        drag!(source event_description);
        drag!(dest event_description);

        // Set the callback function when data is received
        event_description.connect_drag_data_received(clone!(event_data => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the current description
                widget.set_label(&format!("Event: {}", item_pair.description));

                // Update the event data
                if let Ok(mut event) = event_data.try_borrow_mut() {
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

        // Connect the function to trigger when the data type changes
        data_type.connect_changed(clone!(
            event_description,
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
                        event_description.show();
                        minutes_label.hide();
                        minutes_spin.hide();
                        millis_label.hide();
                        millis_spin.hide();
                        string_label.hide();
                        string_entry.hide();
                    }

                    // The time passed until variant
                    "timepasseduntil" => {
                        event_description.show();
                        minutes_label.show();
                        minutes_spin.show();
                        millis_label.show();
                        millis_spin.show();
                        string_label.hide();
                        string_entry.hide();
                    }

                    // The static string variant
                    "staticstring" => {
                        event_description.hide();
                        minutes_label.hide();
                        minutes_spin.hide();
                        millis_label.hide();
                        millis_spin.hide();
                        string_label.show();
                        string_entry.show();
                    }

                    // The user string variant
                    _ => {
                        event_description.hide();
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
        grid.attach(&event_description, 0, 1, 2, 1);
        grid.attach(&minutes_label, 0, 2, 1, 1);
        grid.attach(&minutes_spin, 0, 3, 1, 1);
        grid.attach(&millis_label, 1, 2, 1, 1);
        grid.attach(&millis_spin, 1, 3, 1, 1);
        grid.attach(&string_label, 0, 4, 1, 1);
        grid.attach(&string_entry, 1, 4, 2, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the save data variant
        grid.show_all();
        EditSendData {
            grid,
            system_send: system_send.clone(),
            data_type,
            event_description,
            event_data,
            minutes_spin,
            millis_spin,
            string_entry,
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
    fn load_action(&self, data: &DataType) {
        // Match the data type
        match data {
            // The TimeUntil variant
            &DataType::TimeUntil { ref event_id } => {
                // Change the dropdowm
                self.data_type.set_active_id(Some("timeuntil"));

                // Set the event data
                if let Ok(mut event_data) = self.event_data.try_borrow_mut() {
                    *event_data = event_id.clone();
                }

                // Request the description associated with the id
                self.system_send.send(Request {
                    reply_to: DisplayComponent::EditActionElement {
                        is_left: self.is_left,
                        variant: EditActionElement::EditSendData
                    },
                    request: RequestType::Description { item_id: event_id.clone() },
                });
            }

            // The TimePassedUntil variant
            &DataType::TimePassedUntil {
                ref event_id,
                ref total_time,
            } => {
                // Change the dropdowm
                self.data_type.set_active_id(Some("timepasseduntil"));

                // Set the event data
                if let Ok(mut event_data) = self.event_data.try_borrow_mut() {
                    *event_data = event_id.clone();
                }

                // Request the description associated with the id
                self.system_send.send(Request {
                    reply_to: DisplayComponent::EditActionElement {
                        is_left: self.is_left,
                        variant: EditActionElement::EditSendData
                    },
                    request: RequestType::Description { item_id: event_id.clone() },
                });

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

    // A method to update the description of the event
    pub fn update_description(&self, description: ItemPair) {
        // Update the event label
        self.event_description.set_label(&format!("Event: {}", description.description));
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Extract the dropdown and corresponding data
        if let Some(data_type) = self.data_type.get_active_id() {
            let data = match data_type.as_str() {
                // The TimeUntil variant
                "timeuntil" => {
                    // Get the event id
                    match self.event_data.try_borrow() {
                        Ok(event_data) => {
                            // Return the EventAction
                            DataType::TimeUntil {
                                event_id: event_data.clone(),
                            }
                        }

                        _ => unreachable!(),
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

                    // Get the event id
                    match self.event_data.try_borrow() {
                        Ok(event_data) => {
                            // Return the EventAction
                            DataType::TimePassedUntil {
                                event_id: event_data.clone(),
                                total_time: time,
                            }
                        }

                        _ => unreachable!(),
                    }
                }

                // The StaticString variant
                "staticstring" => {
                    DataType::StaticString { string: self.string_entry.get_text().to_string(), }
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

// Create the select event variant
//
#[derive(Clone, Debug)]
struct EditSelectEvent {
    grid: gtk::Grid,                                         // the main grid for this element
<<<<<<< HEAD
    system_send: SystemSend,                                 // a copy of the system send line
    select_event_list: gtk::ListBox,                        // the list for events in this variant
    select_events: Rc<RefCell<FnvHashMap<usize, EventGrouping>>>, // a database for the select events
=======
    system_send: SyncSystemSend,                                 // a copy of the system send line
    grouped_event_list: gtk::ListBox,                        // the list for events in this variant
    grouped_events: Rc<RefCell<FnvHashMap<usize, EventGrouping>>>, // a database for the grouped events
>>>>>>> Rough Patch for a Fully Asyncronous backend
    next_event: Rc<RefCell<usize>>,                          // the next available event location
    status_description: gtk::Button,                         // the status description display
    status_data: Rc<RefCell<ItemId>>,                        // the wrapped status data
    is_left: bool,                                           // whether the element is to the left or right
}

impl EditSelectEvent {
    // A function to create a select event variant
    //
    fn new(system_send: &SyncSystemSend, is_left: bool) -> EditSelectEvent {
        // Create the list for the trigger events variant
        let select_event_list = gtk::ListBox::new();
        select_event_list.set_selection_mode(gtk::SelectionMode::None);

        // Create the status description and data
        let status_description = gtk::Button::with_label("Status: None");
        let status_data = Rc::new(RefCell::new(ItemId::all_stop()));

        // Set up the status description as a drag source and destination
        drag!(source status_description);
        drag!(dest status_description);

        // Set the callback function when data is received
        status_description.connect_drag_data_received(clone!(system_send, is_left, status_data
        => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the current description
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

                // Send a request to get the data associated with the status
                system_send.send(Request {
                    reply_to: DisplayComponent::EditActionElement { is_left, variant: EditActionElement::SelectEventStates },
                    request: RequestType::Status { item_id: item_pair.get_id(), },
                });
            }
        }));


        // Create the scrollable window for the list
        let select_window = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        select_window.add(&select_event_list);
        select_window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        select_window.set_hexpand(true);
        select_window.set_size_request(-1, 120);


        // Add the status above and button below the event list
        let grid = gtk::Grid::new();
        grid.attach(&status_description, 0, 0, 1, 1);
        grid.attach(&select_window, 0, 1, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the select event variant
        grid.show_all();
        EditSelectEvent {
            grid,
            system_send: system_send.clone(),
            select_event_list,
            select_events: Rc::new(RefCell::new(FnvHashMap::default())),
            next_event: Rc::new(RefCell::new(0)),
            status_description,
            status_data,
            is_left
        }
    }

    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load an action
    fn load_action(&mut self, status_id: &ItemId, event_map: &FnvHashMap<ItemId, ItemId>) {
        // Clear the database and the rows
        self.clear();

        // Send the request to update the status description
        self.system_send.send(Request {
            reply_to: DisplayComponent::EditActionElement {
                is_left: self.is_left,
                variant: EditActionElement::SelectEventDescription { position: None, is_event: false } },
            request: RequestType::Description { item_id: status_id.clone() },
        });

        // Add each event in the map to the list
        for (ref state_id, ref event_id) in event_map.iter() {
            self.add_event(state_id, Some(event_id));
        }
    }

    // A method to update the listed states in the select event
    fn update_info(&mut self, possible_status: Option<Status>) {
        // Clear the ListBox
        self.clear();

        // Check to see if the status is valid
        if let Some(status) = possible_status {
            // Show the states associated with the valid status
            for state_id in status.allowed() {
                self.add_event(&state_id, None);
            }
        } else {
            // Create the label that will replace the list if the spin value is not valid
            let invalid_label = gtk::Label::new(Some("Not a valid status."));

            // Display "not a valid status"
            &self.select_event_list.add(&invalid_label);
            invalid_label.show();
        }
    }

    // A method to clear all the listed states in the ListBox
    pub fn clear(&self) {
        // Remove all the user interface elements
        let to_remove = self.select_event_list.get_children();
        for item in to_remove {
            unsafe {
                item.destroy();
            }
        }
        // Empty the database
        if let Ok(mut events) = self.select_events.try_borrow_mut() {
            events.clear();
        }
    }

    // A method to add a select event to the list
    pub fn add_event(
        &mut self,
        state_id: &ItemId,
        possible_event_id: Option<&ItemId>
    ){

        // Try to get a mutable copy of the next_event
        let position = match self.next_event.try_borrow_mut() {
            Ok(mut position) => {
                let tmp = position.clone();
                *position = *position + 1;
                tmp
            }

            // If unable, exit immediately
            _ => return,
        };

        // Create the grid to hold the state and event labels
        let state_grid = gtk::Grid::new();

        // Create a state description for the list
        let state_label = gtk::Label::new(Some("  State: Loading ...  "));
        state_label.set_size_request(80, 30);
        state_label.set_hexpand(false);
        state_label.set_vexpand(false);

        // Create a event button to hold the even description
        let event_label = gtk::Button::with_label("Event: None");
        event_label.set_size_request(80, 30);
        event_label.set_hexpand(false);
        event_label.set_vexpand(false);

        // Check to see if an event id is given
        let event_id = match possible_event_id {
            Some(event_id) => event_id.clone(),
            None => ItemId::all_stop(),
        };

        // Add the state id and event id to the database
        if let Ok(mut events) = self.select_events.try_borrow_mut() {
            events.insert(position, EventGrouping {
                state_id: state_id.clone(),
                event_id: event_id.clone(),
                state_label: state_label.clone(),
                event_label: event_label.clone(),
            });
        }

        // Request the event description
        self.system_send.send(Request {
            reply_to: DisplayComponent::EditActionElement {
                is_left: self.is_left,
                variant: EditActionElement::SelectEventDescription{
                    position: Some(position),
                    is_event: true,
                },
            },
            request: RequestType::Description { item_id: event_id.clone() },
        });

        // Request the state description
        self.system_send.send(Request {
            reply_to: DisplayComponent::EditActionElement {
                is_left: self.is_left,
                variant: EditActionElement::SelectEventDescription{
                    position: Some(position),
                    is_event: false,
                },
            },
            request: RequestType::Description { item_id: state_id.clone() },
        });

        // Set up the event label to act as a drag source and destination
        drag!(dest event_label);
        drag!(source event_label);

        // Set the callback function when data is received
        let select_events = self.select_events.clone();
        event_label.connect_drag_data_received(clone!(position => move |widget, _, _, _, selection_data, _, _| {
            // Try to extract the selection data
            if let Some(string) = selection_data.get_text() {
                // Convert the selection data to an ItemPair
                let item_pair: ItemPair = match serde_yaml::from_str(string.as_str()) {
                    Ok(item_pair) => item_pair,
                    _ => return,
                };

                // Update the event description
                widget.set_label(&format!("Event: {}", item_pair.description));

                // Borrow a mutable copy of the database
                if let Ok(mut events) = select_events.try_borrow_mut() {
                    // Clone the current database entry
                    let possible_grouping = events.get(&position).map(|value| value.clone());
                    if let Some(current_event_grouping) = possible_grouping {
                        // Update the event id in the current entry
                        events.insert(
                            position,
                            EventGrouping {
                                state_id: current_event_grouping.state_id.clone(),
                                event_id: item_pair.get_id(),
                                state_label: current_event_grouping.state_label.clone(),
                                event_label: current_event_grouping.event_label.clone(),
                            }
                        );
                    }
                }

                // Serialize the item pair data
                widget.connect_drag_data_get(clone!(item_pair => move |_, _, selection_data, _, _| {
                    if let Ok(data) = serde_yaml::to_string(&item_pair) {
                        selection_data.set_text(data.as_str());
                    }
                }));
            }
        }));

        // Add all the items to the group grid
        state_grid.attach(&state_label, 0, 0, 1, 1);
        state_grid.attach(&event_label, 1, 0, 1, 1);

        // Add the new grid to the list
        state_grid.show_all();
        self.select_event_list.add(&state_grid);
    }

    // A method to update the description of the status
    pub fn update_description(&self, position: Option<usize>, is_event: bool, description: ItemPair) {
        match position {
            // If no position is given, update the status label
            None => self.status_description.set_label(&format!("Status: {}", description.description)),

            // If a position is given, extract the data associated with the position
            Some(position) => {
                // Unwrap the database
                if let Ok(select_events) = self.select_events.try_borrow() {
                    if let Some(event_grouping) = select_events.get(&position) {
                        // Check if the description to update is for the event or state
                        match is_event {
                            // Update the event description
                            true => {
                                event_grouping.event_label.set_label(&format!("Event: {}", description.description))
                            },

                            // Update the state description
                            false => {
                                event_grouping.state_label.set_text(&format!("  State: {}  ", description.description))
                            },
                        }
                    }
                }
            }
        }
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Create the event vector
        let event_groupings = match self.select_events.try_borrow() {
            Ok(events) => {
                // Create a hash map to store the data
                let mut event_map = FnvHashMap::default();

                // Copy all the elements into hash map
                for grouping in events.values() {
                    event_map.insert(grouping.state_id, grouping.event_id);
                }

                event_map
            },

            _ => FnvHashMap::default(),
        };

        // Extract the status id
        match self.status_data.try_borrow() {
            Ok(status_data) => {
                // Return the completed Event Action
                EventAction::SelectEvent {
                    status_id: *status_data,
                    event_map: event_groupings,
                }
            },

            _ => unreachable!(),
        }
    }
}
