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
    DataType, EventAction, EventDelay, Hidden, ItemId, ItemDescription, ItemPair,
    StatusChange, StatusDescription,
};
use super::super::super::utils::{clean_text, decorate_label};
use super::NORMAL_FONT;

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
extern crate glib;
use self::gtk::prelude::*;
use self::gtk::GridExt;

// Define and import constants
const MINUTES_LIMIT: f64 = 10080.0; // maximum input time for a delayed event (one week)


/// A structure to contain the dialog for editing an individual event action.
/// This dialog is launched from the edit view.
///
#[derive(Clone, Debug)]
pub struct EditActionDialog;

// Implement key features of the EditActionDialog
impl EditActionDialog {
    /// A function to create and launch a new edit action dialog
    ///
    pub fn launch(window: &gtk::ApplicationWindow, event_actions: &Rc<RefCell<FnvHashMap<usize, EventAction>>>, position: usize, overview: &gtk::Label) {
        // Try to get the current event actions
        let actions = match event_actions.try_borrow() {
            Ok(actions) => actions,
            
            // If unable, return immediately
            _ => return,
        };
        
        // Try to extract the correct action
        let action = match actions.get(&position) {
            Some(action) => action,
            
            // If unable, return immediately
            _ => return,
        };
        
        // Create the new dialog
        let dialog = gtk::Dialog::new_with_buttons(
            Some("Edit Action"),
            Some(window),
            gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
            &[
                ("Cancel", gtk::ResponseType::Cancel.into()),
                ("Confirm", gtk::ResponseType::Ok.into()),
            ],
        );
        dialog.set_position(gtk::WindowPosition::Center);
        
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
        let edit_new_scene = EditNewScene::new();
        let edit_modify_status = EditModifyStatus::new();
        let edit_queue_event = EditQueueEvent::new();
        let edit_cancel_event = EditCancelEvent::new();
        let edit_save_data = EditSaveData::new();
        let edit_send_data = EditSendData::new();
        let edit_grouped_event = EditGroupedEvent::new();
        
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
            if let Some(detail_str) = dropdown.get_active_id() {
                // Change the action stack to the requested variation
                action_stack.set_visible_child_full(&detail_str, gtk::StackTransitionType::None);
            }
        }));

        // Load the selected action
        match action {
            // the NewScene variant
            EventAction::NewScene { new_scene } => {
                action_selection.set_active_id(Some("newscene"));
                edit_new_scene.load_action(new_scene);
            }
            
            // the ModifyStatus variant
            EventAction::ModifyStatus { status_id, new_state } => {
                action_selection.set_active_id(Some("modifystatus"));
                edit_modify_status.load_action(status_id, new_state);
            }
            
            // the QueueEvent variant
            EventAction::QueueEvent { event } => {
                action_selection.set_active_id(Some("queueevent"));
                edit_queue_event.load_action(event);
            }
            
            // the CancelEvent variant
            EventAction::CancelEvent { event } => {
                action_selection.set_active_id(Some("cancelevent"));
                edit_cancel_event.load_action(event);
            }
            
            // the SaveData variant
            EventAction::SaveData { data } => {
                action_selection.set_active_id(Some("savedata"));
                edit_save_data.load_action(data);
            }
            
            // the SendData variant
            EventAction::SendData { data } => {
                action_selection.set_active_id(Some("senddata"));
                edit_send_data.load_action(data);
            }
            
            // the GroupedEvent variant
            EventAction::GroupedEvent { status_id, event_map } => {
                action_selection.set_active_id(Some("groupedevent"));
                edit_grouped_event.load_action(status_id, event_map);
            }
        }
        
        // Access the content area and add main grid
        let content = dialog.get_content_area();
        let grid = gtk::Grid::new();
        content.add(&grid);

        // Add the dropdown and the action stack
        grid.attach(&action_selection, 0, 1, 1, 1);
        grid.attach(&action_stack, 0, 2, 1, 1);
        grid.set_column_spacing(10); // add some space
        grid.set_row_spacing(10);

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);

        // Connect the close event for when the dialog is complete
        dialog.connect_response(clone!(event_actions, overview => move |modal, id| {
            // If the selection confirmed the change
            if id == gtk::ResponseType::Ok {
                // Try to get a mutable copy of the event actions
                let mut actions = match event_actions.try_borrow_mut() {
                    Ok(actions) => actions,
                    
                    // If unable, return immediately
                    _ => return,
                };
                
                // Try to extract the correct action
                let action = match actions.get_mut(&position) {
                    Some(action) => action,
                    
                    // If unable, return immediately
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
            }

            // Close the dialog on success or cancel
            modal.destroy();
        }));

        // Show the dialog and return
        dialog.show_all();
    }

}

// Create the new scene variant
#[derive(Clone, Debug)]
struct EditNewScene {
    grid: gtk::Grid,       // the main grid for this element
    spin: gtk::SpinButton, // the spin button for the new scene id
}

// Implement key features for Edit New Scene
impl EditNewScene {
    // A function to create a new scene variant
    //
    fn new() -> EditNewScene {
        // Create the top level grid
        let grid = gtk::Grid::new();

        // Add a label and spin to the grid
        let label = gtk::Label::new(Some("Scene Id"));
        let spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        grid.attach(&label, 0, 0, 1, 1);
        grid.attach(&spin, 1, 0, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the EditNewScene
        grid.show_all();
        EditNewScene {
            grid,
            spin,
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
        self.spin.set_value(new_scene.id() as f64);
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Pack the new scene id into a action
        EventAction::NewScene {
            new_scene: ItemId::new_unchecked(self.spin.get_value() as u32),
        }
    }
}

// Create the modify status variant
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
        let grid = gtk::Grid::new();

        // Add a labels and spins to the grid
        let status_label = gtk::Label::new(Some("Status Id"));
        let status_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let state_label = gtk::Label::new(Some("State Id"));
        let state_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

        // Place everything into the grid
        grid.attach(&status_label, 0, 0, 1, 1);
        grid.attach(&status_spin, 0, 1, 1, 1);
        grid.attach(&state_label, 1, 0, 1, 1);
        grid.attach(&state_spin, 1, 1, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the EditModifyStatus
        grid.show_all();
        EditModifyStatus {
            grid,
            status_spin,
            state_spin,
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
        self.status_spin.set_value(status_id.id() as f64);
        self.state_spin.set_value(new_state.id() as f64);
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        EventAction::ModifyStatus {
            status_id: ItemId::new_unchecked(self.status_spin.get_value() as u32),
            new_state: ItemId::new_unchecked(self.state_spin.get_value() as u32),
        }
    }
}

// Create the queue event variant
//
#[derive(Clone, Debug)]
struct EditQueueEvent {
    grid: gtk::Grid,               // the main grid for this element
    event_spin: gtk::SpinButton,   // the event spin button
    minutes_spin: gtk::SpinButton, // the minutes spin button
    millis_spin: gtk::SpinButton,  // the milliseconds spin button
}

impl EditQueueEvent {
    // A function to ceate a queue event variant
    //
    fn new() -> EditQueueEvent {
        // Create the grid for the queue event variant
        let grid = gtk::Grid::new();

        // Add a labels and spins to the grid
        let grid = gtk::Grid::new();
        let event_label = gtk::Label::new(Some("Event Id"));
        let event_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        let minutes_label = gtk::Label::new(Some("Delay: Minutes"));
        let minutes_spin = gtk::SpinButton::new_with_range(0.0, MINUTES_LIMIT, 1.0);
        let millis_label = gtk::Label::new(Some("Milliseconds"));
        let millis_spin = gtk::SpinButton::new_with_range(0.0, 60000.0, 1.0);

        // Add all the components to the event grid
        grid.attach(&event_label, 0, 0, 1, 1);
        grid.attach(&event_spin, 0, 1, 1, 1);
        grid.attach(&minutes_label, 1, 0, 1, 1);
        grid.attach(&minutes_spin, 1, 1, 1, 1);
        grid.attach(&millis_label, 2, 0, 1, 1);
        grid.attach(&millis_spin, 2, 1, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the queue event variant
        grid.show_all();
        EditQueueEvent {
            grid,
            event_spin,
            minutes_spin,
            millis_spin,
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
        // Set the value of the event id
        self.event_spin.set_value(event_delay.id().id() as f64);

        // Calculate the minutes and seconds of the duration
        if let Some(delay) = event_delay.delay() {
            // May be and empty delay
            let time = delay.as_secs();
            let remainder = time % 60;
            self.minutes_spin.set_value(((time - remainder) / 60) as f64);
            self.millis_spin.set_value(((remainder * 1000) + (delay.subsec_millis() as u64)) as f64);
        
        // Otherwise, set them to zero
        } else {
            self.minutes_spin.set_value(0.0);
            self.millis_spin.set_value(0.0);
        }
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Extract the event id
        let event_id = self.event_spin.get_value() as u32;

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
            event: EventDelay::new(delay, ItemId::new_unchecked(event_id)),
        }
    }
}

// Create the cancel event variant
//
#[derive(Clone, Debug)]
struct EditCancelEvent {
    grid: gtk::Grid,         // the main grid for this element
    spin: gtk::SpinButton,   // the event spin button
}

impl EditCancelEvent {
    // A function to ceate a cancel event variant
    //
    fn new() -> EditCancelEvent {
        // Create the top level grid
        let grid = gtk::Grid::new();

        // Add a label and spin to the grid
        let label = gtk::Label::new(Some("Event Id"));
        let spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        grid.attach(&label, 0, 0, 1, 1);
        grid.attach(&spin, 1, 0, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);
        
        // Create and return the cancel event variant
        grid.show_all();
        EditCancelEvent {
            grid,
            spin,
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
        // Set the value of the event id
        self.spin.set_value(event.id() as f64);
    }

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Extract the event id
        let id = self.spin.get_value() as u32;

        // Return the completed action
        EventAction::CancelEvent { event: ItemId::new_unchecked(id) }
    }
}

// Create the save data variant
//
#[derive(Clone, Debug)]
struct EditSaveData {
    grid: gtk::Grid,              // the main grid for this element
    data_type: gtk::ComboBoxText  // the data type dropdown
}

impl EditSaveData {
    // A function to ceate a save data variant
    //
    fn new() -> EditSaveData {
        // Create the dropdown selection for the data type
        let data_type = gtk::ComboBoxText::new();
        
        // Add each of the available data types to the dropdown
        data_type.append(Some("timeuntil"), "Time until an event will occur");
        data_type.append(Some("timepasseduntil"), "Time passed since an event was queued");
        data_type.append(Some("staticstring"), "A hardcoded string of data");
        data_type.append(Some("userstring"), "A user-provided string");

        // Add the button below the data list
        let grid = gtk::Grid::new();
        grid.attach(&data_type, 0, 0, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the save data variant
        grid.show_all();
        EditSaveData {
            grid,
            data_type,
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
                
                // FIXME Update the fields
            }
            
            // The TimePassedUntil variant
            &DataType::TimePassedUntil { ref event_id, ref total_time } => {
                // Change the dropdowm
                self.data_type.set_active_id(Some("timepasseduntil"));
                
                // FIXME Update the fields
            }
            
            // The StaticString variant
            &DataType::StaticString { ref string } => {
                // Change the dropdown
                self.data_type.set_active_id(Some("staticstring"));
                
                // FIXME Update the fields
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
                    DataType::UserString // FIXME
                }
                
                // The TimePassedUntil variant
                "timepasseduntil" => {
                    DataType::UserString // FIXME
                }
                
                // The StaticString variant
                "staticstring" => {
                    DataType::UserString // FIXME
                }
                
                // The UserString variant
                _ => DataType::UserString,
            };

            // Return the completed action
            return EventAction::SaveData { data };
        
        // If nothing was selected, return UserString by default
        } else {
            return EventAction::SaveData { data: DataType::UserString };
        }
    }
}

// Create the send data variant
//
#[derive(Clone, Debug)]
struct EditSendData {
    grid: gtk::Grid,              // the main grid for this element
    data_type: gtk::ComboBoxText  // the data type dropdown
}

impl EditSendData {
    // A function to ceate a send data variant
    //
    fn new() -> EditSendData {
        // Create the dropdown selection for the data type
        let data_type = gtk::ComboBoxText::new();
        
        // Add each of the available data types to the dropdown
        data_type.append(Some("timeuntil"), "Time until an event will occur");
        data_type.append(Some("timepasseduntil"), "Time passed since an event was queued");
        data_type.append(Some("staticstring"), "A hardcoded string of data");
        data_type.append(Some("userstring"), "A user-provided string");

        // Add the button below the data list
        let grid = gtk::Grid::new();
        grid.attach(&data_type, 0, 0, 1, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the send data variant
        grid.show_all();
        EditSendData {
            grid,
            data_type,
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
                
                // FIXME Update the fields
            }
            
            // The TimePassedUntil variant
            &DataType::TimePassedUntil { ref event_id, ref total_time } => {
                // Change the dropdowm
                self.data_type.set_active_id(Some("timepasseduntil"));
                
                // FIXME Update the fields
            }
            
            // The StaticString variant
            &DataType::StaticString { ref string } => {
                // Change the dropdown
                self.data_type.set_active_id(Some("staticstring"));
                
                // FIXME Update the fields
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
                    DataType::UserString // FIXME
                }
                
                // The TimePassedUntil variant
                "timepasseduntil" => {
                    DataType::UserString // FIXME
                }
                
                // The StaticString variant
                "staticstring" => {
                    DataType::UserString // FIXME
                }
                
                // The UserString variant
                _ => DataType::UserString,
            };

            // Return the completed action
            return EventAction::SendData { data };
        
        // If nothing was selected, return UserString by default
        } else {
            return EventAction::SendData { data: DataType::UserString };
        }
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
        let status_label = gtk::Label::new(Some("Status"));
        let status_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);

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
        group_window.set_hexpand(true);
        group_window.set_size_request(-1, 100);

        // Add the status above and button below the event list
        let grid = gtk::Grid::new();
        grid.attach(&status_label, 0, 0, 1, 1);
        grid.attach(&status_spin, 1, 0, 1, 1);
        grid.attach(&group_window, 0, 1, 2, 1);
        grid.attach(&add_button, 0, 2, 2, 1);
        grid.set_column_spacing(10); // Add some space
        grid.set_row_spacing(10);

        // Create and return the grouped event variant
        grid.show_all();
        EditGroupedEvent {
            grid,
            grouped_event_list,
            status_spin,
        }
    }

    // A method to return the top element
    //
    fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    // A method to load an action
    // FIXME Finish this implementation 
    fn load_action(&self, status_id: &ItemId, event_map: &FnvHashMap<ItemId, ItemId>) {
        // Change the status id
        self.status_spin.set_value(status_id.id() as f64);

        // Add each event in the map to the list
        for (ref state_id, ref event_id) in event_map.iter() {
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

    // A method to pack and return the action
    //
    fn pack_action(&self) -> EventAction {
        // Create the event vector
        let mut event_map = FnvHashMap::default();

        // Extract the status id
        let status_id = ItemId::new_unchecked(self.status_spin.get_value() as u32);

        // FIXME Fill the maps with the grouped events in the list
        /* let mut i: i32 = 0;
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
        }*/

        // Return the completed Event Action
        EventAction::GroupedEvent {
            status_id,
            event_map,
        }
    }
}

