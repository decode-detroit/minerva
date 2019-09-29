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

//! A module to create the control abstraction that generates default
//! content and allows easier interaction and manipulation with the interface.
//! system interface. This module links indirectly to the system interface and
//! sends any updates to the application window through gtk widgets.

// Import the relevant structures into the correct namespace
use super::super::super::system_interface::{
    AllStop, Current, Error, Notification, GetDescription, TriggerEvent, SystemSend, Update, Warning, ItemId
};
use super::super::utils::clean_text;

// Import GTK and GDK libraries
extern crate gdk;
extern crate gtk;
use self::gtk::prelude::*;

// Import the eternal time library
extern crate time;

// Define module constants
const UPDATE_LIMIT: usize = 40; // maximum character width of updates
const UPDATE_NUMBER: usize = 20; // maximum number of updates to display

/// A structure to contain the dialog for triggering a custom event. A near
/// of the special windows trigger dialog, copied here for convenience
///
#[derive(Clone, Debug)]
struct TriggerDialog {
    window: gtk::ApplicationWindow,
    system_send: SystemSend,
}

// Implement key features for the trigger dialog
impl TriggerDialog {
    /// A function to create a new trigger dialog structure.
    ///
    pub fn new(window: &gtk::ApplicationWindow, system_send: &SystemSend) -> TriggerDialog {
        TriggerDialog { window: window.clone(), system_send: system_send.clone() }
    }

    /// A method to launch the new edit dialog
    ///
    pub fn launch(&self, event_id: Option<ItemId>) {
        // Create the new dialog
        let dialog = gtk::Dialog::new_with_buttons(
            Some("Trigger Custom Event"),
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
        let label = gtk::Label::new(Some(" Warning: Triggering A Custom Event May Cause Undesired Behaviour. "));
        label.set_halign(gtk::Align::Center);
        //label.set_hexpand(true);
        grid.attach(&label, 0, 0, 3, 1);
        
        // Description label for the current event FIXME
        //let description = gtk::Label::new(Some("");

        // Create the event selection
        let event_spin = gtk::SpinButton::new_with_range(1.0, 536870911.0, 1.0);
        
        // If an id was specified, use it
        if let Some(id) = event_id {
            event_spin.set_value(id.id() as f64);
        }
        
        // Create the checkbox
        let event_checkbox = gtk::CheckButton::new_with_label("Check Scene");
        let event_lookup =
            gtk::Button::new_from_icon_name("edit-find", gtk::IconSize::Button.into());
        let send_clone = self.system_send.clone();
        event_lookup.connect_clicked(clone!(event_spin => move |_| {
            send_clone.send(GetDescription { item_id: ItemId::new_unchecked(event_spin.get_value() as u32) });
        }));
        grid.attach(&event_spin, 0, 1, 1, 1);
        grid.attach(&event_checkbox, 1, 1, 1, 1);
        grid.attach(&event_lookup, 2, 1, 1, 1);

        // Add some space between the rows and columns
        grid.set_column_spacing(20);
        grid.set_row_spacing(30);

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);
        //grid.set_hexpand(true);
        grid.set_halign(gtk::Align::Center);

        // Connect the close event for when the dialog is complete
        let send_clone = self.system_send.clone();
        dialog.connect_response(clone!(event_spin, event_checkbox => move |modal, id| {

            // Notify the system of the event change
            if id == gtk::ResponseType::Ok {

                // Send the selected event to the system
                send_clone.send(TriggerEvent { event: ItemId::new_unchecked(event_spin.get_value() as u32), checkscene: event_checkbox.get_active()});
            }

            // Close the window either way
            modal.destroy();
        }));

        // Show the dialog and return
        dialog.show_all();
    }
}

/// A structure to hold the control grid elements in the default interface.
///
/// This structure allows easier modification of the gtk control grid elements
/// to simplify interaction between the interface and the underlying program.
///
#[derive(Clone, Debug)]
pub struct ControlAbstraction {
    grid: gtk::Grid,               // the grid to hold the underlying elements
    notification_area_list: gtk::ListBox, // the notification area list for system notifications
    is_debug_mode: bool, // a flag to indicate whether debug-level notifications should be displayed (not retroctive)
    trigger_dialog: TriggerDialog, // a struct to conveniently launch a trigger dialog
}

// Implement key features for the Control Abstraction
impl ControlAbstraction {
    /// A function to create a new instance of the Control Abstraction. This
    /// function loads all the default widgets into the interface and returns
    /// a new copy to allow insertion into higher-level elements.
    ///
    pub fn new(system_send: &SystemSend, window: &gtk::ApplicationWindow) -> ControlAbstraction {
        // Create the control grid for holding all the universal controls
        let grid = gtk::Grid::new();

        // Set the features of the grid
        grid.set_column_homogeneous(false); // set the row and column heterogeneous
        grid.set_row_homogeneous(false);
        grid.set_column_spacing(10); // add some space between the rows and columns
        grid.set_row_spacing(10);

        // Format the whole grid
        grid.set_property_width_request(400);
        grid.set_hexpand(false);
        grid.set_vexpand(false);
        grid.set_valign(gtk::Align::Fill);

        // Add the top separator
        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        separator.set_halign(gtk::Align::Fill);
        separator.set_hexpand(true);
        separator.show();
        grid.attach(&separator, 0, 0, 1, 1);

        // Create the notification title
        let notification_title = gtk::Label::new(None);
        notification_title
            .set_markup("<span color='#338DD6' size='14000'>System Notifications</span>");
        notification_title.set_halign(gtk::Align::Center);

        // Create the notification area list
        let notification_area_list = gtk::ListBox::new();
        notification_area_list.set_selection_mode(gtk::SelectionMode::None);
        
        // Format the notification area list FIXME
        //notification_area_list.set_property_height_request(400);
        //notification_area_list.set_hexpand(true);
        //notification_area_list.set_vexpand(true);
        //notification_area_list.set_halign(gtk::Align::Start);
        //notification_area_list.set_valign(gtk::Align::Start);

        // Create the scrollable window for the list
        let notification_area = gtk::ScrolledWindow::new(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0), &gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0));
        // FIXME Broken implementation of ScrolledWindow::new()
        notification_area.add(&notification_area_list);
        notification_area.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        notification_area.set_property_height_request(400);
        notification_area.set_hexpand(true);
        notification_area.set_vexpand(true);
        notification_area.set_halign(gtk::Align::Start);
        notification_area.set_valign(gtk::Align::Start);

        // Add them near the top of the control grid
        grid.attach(&notification_title, 0, 1, 1, 1);
        grid.attach(&notification_area, 0, 2, 1, 1);

        // Create the empty stop stack
        let stop_stack = gtk::Stack::new();

        // Create the all stop button and format it
        let tmp_label = gtk::Label::new(None);
        tmp_label.set_markup("<span color='#E3240E' size='14000'><b>ALL STOP</b></span>");
        let all_stop_button = gtk::Button::new();
        all_stop_button.add(&tmp_label);
        all_stop_button.set_size_request(80, 30);
        all_stop_button.set_hexpand(false);
        all_stop_button.set_vexpand(false);

        // Connect the confirmation reveal
        all_stop_button.connect_clicked(clone!(stop_stack => move |_| {
            stop_stack.set_visible_child_full("confirm", gtk::StackTransitionType::SlideUp);
        }));

        // Add the all stop button to the stack
        stop_stack.add_named(&all_stop_button, "button");

        // Create the all stop label and format it
        let all_stop_label = gtk::Label::new(None);
        all_stop_label.set_markup("<span size='12000'>Stop The System?</span>");
        all_stop_label.set_size_request(80, 30);
        all_stop_label.set_hexpand(true);
        all_stop_label.set_vexpand(false);

        // Create the all stop confirmation button and format it
        let tmp_label = gtk::Label::new(None);
        tmp_label.set_markup("<span color='#E3240E' size='12000'>Confirm</span>");
        let all_stop_confirm = gtk::Button::new();
        all_stop_confirm.add(&tmp_label);
        all_stop_confirm.set_size_request(80, 30);
        all_stop_confirm.set_hexpand(false);
        all_stop_confirm.set_vexpand(false);

        // Create the all stop cancel button and format it
        let tmp_label = gtk::Label::new(None);
        tmp_label.set_markup("<span size='12000'>Cancel</span>");
        let all_stop_cancel = gtk::Button::new();
        all_stop_cancel.add(&tmp_label);
        all_stop_cancel.set_size_request(80, 30);
        all_stop_cancel.set_hexpand(false);
        all_stop_cancel.set_vexpand(false);

        // Connect the confirmation hide
        all_stop_confirm.connect_clicked(clone!(system_send, stop_stack => move |_| {

            // Send the all stop and close the confirmation
            system_send.send(AllStop);
            stop_stack.set_visible_child_full("button", gtk::StackTransitionType::SlideDown);
        }));

        // Connect the cancel hide
        all_stop_cancel.connect_clicked(clone!(stop_stack => move |_| {

            // Just close the confirmation
            stop_stack.set_visible_child_full("button", gtk::StackTransitionType::SlideDown);
        }));

        // Add the label and both buttons to the confirmation grid
        let confirmation_grid = gtk::Grid::new();
        confirmation_grid.attach(&all_stop_label, 0, 0, 1, 1);
        confirmation_grid.attach(&all_stop_cancel, 1, 0, 1, 1);
        confirmation_grid.attach(&all_stop_confirm, 2, 0, 1, 1);
        confirmation_grid.set_column_spacing(10);

        // Set the button as the default visible element
        stop_stack.set_visible_child_full("button", gtk::StackTransitionType::SlideDown);

        // Add the confirmation grid to the stack and format it
        stop_stack.add_named(&confirmation_grid, "confirm");
        stop_stack.set_margin_top(10);
        stop_stack.set_margin_bottom(10);
        stop_stack.set_hexpand(true);

        // Add the all stop stack at the bottom of the control grid
        grid.attach(&stop_stack, 0, 4, 1, 1);
        
        // Create the trigger dialog window
        let trigger_dialog = TriggerDialog::new(window, system_send);

        // Return the new Control Abstraction
        ControlAbstraction {
            grid,
            notification_area_list,
            is_debug_mode: false,
            trigger_dialog,
        }
    }

    /// A method to return a reference to the top element of the interface,
    /// currently grid.
    ///
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    /// A method to change whether the debug version of the notifications are turned on.
    ///
    pub fn debug_notifications(&mut self, debug: bool) {
        self.is_debug_mode = debug;
    }
    
    /// A method to refresh the notification area to allow for highlighted events
    ///
    pub fn refresh_notifications(&self) {
        // FIXME: This currently does nothing
    }

    /// A method to update the notification area with system notifications
    ///
    pub fn update_notifications(&mut self, mut notifications: Vec<Notification>) {
        // If not debug, prefilter and throw out notifications that are debug level
        // TODO: Consider replacing with experimental function Vec::drain_filter()
        if !self.is_debug_mode {
            let mut index = 0;
            while index != notifications.len() {
                match notifications[index] {
                    // Ignore Warning messages
                    Warning { .. } => {
                        notifications.remove(index);
                    }

                    // Ignore Current notifications
                    Current { .. } => {
                        notifications.remove(index);
                    }

                    // Just increment otherwise
                    _ => {
                        index += 1;
                    }
                }
            }
        }
        
        // Clear the existing notifications in the list
        loop {
            // Iterate through the notifications in the list
            match self.notification_area_list.get_row_at_index(0) {
                // Extract each row and the corresponding notification
                Some(row) => {
                    // As each row is removed, the next row moves to index zero
                    self.notification_area_list.remove(&row);
                }

                // Break when there are no more rows
                None => break,
            }
        }

        // Unpack each of the notification lines, the most recent at the top
        for (i, update) in notifications.drain(..).enumerate() {
            // Cap the list at the update number
            if i >= UPDATE_NUMBER {
                break;
            }

            // Unpack the notification and create a label
            let (markup, button_opt) = &self.unpack_notification(update);
            let notification_label = gtk::Label::new(None);
            notification_label.set_markup(&markup);
            
            // Format and show each label
            notification_label.set_halign(gtk::Align::Start);
            notification_label.show();
            
            // Check to see if there is a button
            match button_opt {
            
                // If so, add the button and the label
                Some(button) => {
                    
                    // Create a new grid
                    let grid = gtk::Grid::new();
                    
                    // Set the features of the grid
                    grid.set_column_homogeneous(false); // set the row and column heterogeneous
                    grid.set_row_homogeneous(false);
                    grid.set_column_spacing(5); // add some space between the columns

                    // Format the whole grid
                    grid.set_hexpand(true);
                    grid.set_vexpand(false);
                    
                    // Add the two items to the grid
                    grid.attach(&notification_label, 0, 0, 1, 1);
                    grid.attach(button, 0, 1, 1, 1);
                    
                    // Show the grid components
                    grid.show_all();
                    
                    // Add it to the notification list
                    self.notification_area_list.add(&grid);
                }
                
                // If not, just add the label
                None => {
                    self.notification_area_list.add(&notification_label);
                }
            }
        }
    }

    /// An internal method to unpack a notification into properly formatted
    /// Pango markup that highlights warnings and errors. The function will
    /// return a button if it should accompany the notification.
    ///
    fn unpack_notification(&self, notification: Notification) -> (String, Option<gtk::Button>) {
        // Unpack the notification based on its variant
        match notification {
            // Highlight the error variant in bold with red
            Error { message, time, event_id } => {
                // Format the time appropriately
                let timestr = time.strftime("%a %T").unwrap_or_else(|_| time.asctime()); // Fallback on other time format
                
                // Extract the id, if specified
                match event_id {
                    // If there's an id, include it in the message
                    Some(id) => {
                        // Combine the message and the time
                        (format!(
                            "{} — <span color='#FF3333'><b>Error: {} ({})</b></span>",
                            timestr,
                            clean_text(&message, UPDATE_LIMIT, true, true, true),
                            id
                        ), None)
                    },
                    
                    // Otherwise, just include the message
                    None => {
                        // Combine the message and the time
                        (format!(
                            "{} — <span color='#FF3333'><b>Error: {}</b></span>",
                            timestr,
                            clean_text(&message, UPDATE_LIMIT, true, true, true)
                        ), None)
                    },
                }
            }

            // Highlight the warning variant with yellow
            Warning { message, time, event_id } => {
                // Format the time appropriately
                let timestr = time.strftime("%a %T").unwrap_or_else(|_| time.asctime()); // Fallback on other time format

                // Assemble a button if there should be one
                let button_opt = match event_id {
                    // If an event was specified, create a launch info button
                    Some(event_id) => {
                        // Create a label for the button
                        let tmp_label = gtk::Label::new(None);
                        tmp_label.set_markup("<span size='10000'>Trigger The Event Anyway</span>");
                    
                        // Create a button to open the trigger dialog
                        let new_button = gtk::Button::new();
                        new_button.add(&tmp_label);
                        new_button.set_hexpand(false);
                        new_button.set_vexpand(false);

                        // Connect the confirmation reveal
                        let trigger_clone = self.trigger_dialog.clone();
                        new_button.connect_clicked(move |_| {
                            trigger_clone.launch(Some(event_id))
                        });
                        
                        // Return the new button
                        Some(new_button)
                    },
                    
                    None => None,
                };

                // Combine the message and the time
                (format!(
                    "{} — <span color='#FFEE44'>Warning: {}</span>",
                    timestr,
                    clean_text(&message, UPDATE_LIMIT, true, true, true)
                ), button_opt)
            }

            // Add a prefix to the current variant and highlight with blue
            Current { message, time } => {
                // Format the time appropriately
                let timestr = time.strftime("%a %T").unwrap_or_else(|_| time.asctime()); // Fallback on other time format

                // Combine the message and the time
                (format!(
                    "{} — Event: <span color='#338DD6'>{}</span>",
                    timestr,
                    clean_text(&message, UPDATE_LIMIT, true, true, true)
                ), None)
            }

            // Leave the other update unformatted
            Update { message, time } => {
                // Format the time appropriately
                let timestr = time.strftime("%a %T").unwrap_or_else(|_| time.asctime()); // Fallback on other time format

                // Combine the message and the time
                (format!(
                    "{} — {}",
                    timestr,
                    clean_text(&message, UPDATE_LIMIT, true, true, true)
                ), None)
            }
        }
    }
}

