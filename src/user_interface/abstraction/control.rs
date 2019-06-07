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
    AllStop, Current, Error, Notification, SystemSend, Update, Warning,
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

/// A structure to hold the control grid elements in the default interface.
///
/// This structure allows easier modification of the gtk control grid elements
/// to simplify interaction between the interface and the underlying program.
///
#[derive(Clone, Debug)]
pub struct ControlAbstraction {
    grid: gtk::Grid,               // the grid to hold the underlying elements
    notification_area: gtk::Label, // the notification area for system notifications
    is_debug_mode: bool, // a flag to indicate whether debug-level notifications should be displayed (not retroctive)
}

// Implement key features for the Control Abstraction
impl ControlAbstraction {
    /// A function to create a new instance of the Control Abstraction. This
    /// function loads all the default widgets into the interface and returns
    /// a new copy to allow insertion into higher-level elements.
    ///
    pub fn new(system_send: &SystemSend) -> ControlAbstraction {
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

        // Create the notification area
        let notification_area = gtk::Label::new(Some("System notifications appear here."));
        notification_area.set_halign(gtk::Align::Start);
        notification_area.set_valign(gtk::Align::Start);
        notification_area.set_property_height_request(400);
        notification_area.set_hexpand(true);
        notification_area.set_vexpand(true);
        notification_area.set_halign(gtk::Align::Fill);
        notification_area.set_valign(gtk::Align::Fill);

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

        // Return the new Control Abstraction
        ControlAbstraction {
            grid,
            notification_area,
            is_debug_mode: false,
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

        // Simply unpack each of the notification lines, the most recent at the top
        let mut markup = String::new();
        for (i, update) in notifications.drain(..).enumerate() {
            // Cap the list at the update number
            if i >= UPDATE_NUMBER {
                break;
            }

            // Unpack the notification
            markup = markup + &self.unpack_notification(update);
        }

        // Change the notification area
        self.notification_area.set_markup(&markup);
    }

    /// An internal method to unpack a notification into properly formatted
    /// Pango markup that highlights warnings and errors.
    ///
    fn unpack_notification(&self, notification: Notification) -> String {
        // Unpack the notification based on its variant
        match notification {
            // Highlight the error variant in bold with red
            Error { message, time } => {
                // Format the time appropriately
                let timestr = time.strftime("%a %T").unwrap_or_else(|_| time.asctime()); // Fallback on other time format

                // Combine the message and the time
                format!(
                    "{} — <span color='#FF3333'><b>Error: {}</b></span>\n",
                    timestr,
                    clean_text(&message, UPDATE_LIMIT, true, true, true)
                )
            }

            // Highlight the warning variant with yellow
            Warning { message, time } => {
                // Format the time appropriately
                let timestr = time.strftime("%a %T").unwrap_or_else(|_| time.asctime()); // Fallback on other time format

                // Combine the message and the time
                format!(
                    "{} — <span color='#FFEE44'>Warning: {}</span>\n",
                    timestr,
                    clean_text(&message, UPDATE_LIMIT, true, true, true)
                )
            }

            // Add a prefix to the current variant and highlight with blue
            Current { message, time } => {
                // Format the time appropriately
                let timestr = time.strftime("%a %T").unwrap_or_else(|_| time.asctime()); // Fallback on other time format

                // Combine the message and the time
                return format!(
                    "{} — Event: <span color='#338DD6'>{}</span>\n",
                    timestr,
                    clean_text(&message, UPDATE_LIMIT, true, true, true)
                );
            }

            // Leave the other update unformatted
            Update { message, time } => {
                // Format the time appropriately
                let timestr = time.strftime("%a %T").unwrap_or_else(|_| time.asctime()); // Fallback on other time format

                // Combine the message and the time
                format!(
                    "{} — {}\n",
                    timestr,
                    clean_text(&message, UPDATE_LIMIT, true, true, true)
                )
            }
        }
    }
}

