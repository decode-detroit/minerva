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

// Import crate definitions
use crate::definitions::*;

// Import other definitions
use super::super::utils::clean_text;
use super::{LARGE_FONT, SMALL_FONT};

// Import GTK and GDK libraries
use self::gtk::prelude::*;
use gtk;

// Define module constants
const UPDATE_LIMIT: usize = 30; // maximum character width of updates
const UPDATE_NUMBER: usize = 50; // maximum number of updates to display

/// A structure to hold the control grid elements in the default interface.
///
/// This structure allows easier modification of the gtk control grid elements
/// to simplify interaction between the interface and the underlying program.
///
#[derive(Clone, Debug)]
pub struct ControlAbstraction {
    grid: gtk::Grid, // the grid to hold the underlying elements
    interface_send: InterfaceSend, // a copy of interface send
    notification_area_list: gtk::ListBox, // the notification area list for system notifications
    is_debug_mode: bool, // a flag to indicate whether debug-level notifications are shown
    is_font_large: bool, // a flag to indicate the font size of the items
    is_high_contrast: bool, // a flag to indicate if the display is high contrast
}

// Implement key features for the Control Abstraction
impl ControlAbstraction {
    /// A function to create a new instance of the Control Abstraction. This
    /// function loads all the default widgets into the interface and returns
    /// a new copy to allow insertion into higher-level elements.
    ///
    pub fn new(
        gtk_send: &GtkSend,
        interface_send: &InterfaceSend,
    ) -> ControlAbstraction {
        // Create the control grid for holding all the universal controls
        let grid = gtk::Grid::new();

        // Set the features of the grid
        grid.set_column_homogeneous(false); // set the row and column heterogeneous
        grid.set_row_homogeneous(false);
        grid.set_row_spacing(10); // add some space between the rows

        // Format the whole grid
        grid.set_width_request(300);
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

        // Create the scrollable window for the list
        let notification_area = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        notification_area.add(&notification_area_list);
        notification_area.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Format the scrolling window
        notification_area.set_height_request(200);
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
        all_stop_label.set_markup("<span size='14000'>All Stop?</span>");
        all_stop_label.set_hexpand(true);
        all_stop_label.set_vexpand(false);

        // Create the all stop confirmation button and format it
        let tmp_label = gtk::Label::new(None);
        tmp_label.set_markup("<span color='#E3240E' size='14000'>Confirm</span>");
        let all_stop_confirm = gtk::Button::new();
        all_stop_confirm.add(&tmp_label);
        all_stop_confirm.set_hexpand(false);
        all_stop_confirm.set_vexpand(false);

        // Create the all stop cancel button and format it
        let tmp_label = gtk::Label::new(None);
        tmp_label.set_markup("<span size='14000'>Cancel</span>");
        let all_stop_cancel = gtk::Button::new();
        all_stop_cancel.add(&tmp_label);
        all_stop_cancel.set_hexpand(false);
        all_stop_cancel.set_vexpand(false);

        // Connect the confirmation hide
        all_stop_confirm.connect_clicked(clone!(gtk_send, stop_stack => move |_| {

            // Send the all stop and close the confirmation
            gtk_send.send(UserRequest::AllStop);
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
            interface_send: interface_send.clone(),
            notification_area_list,
            is_debug_mode: false,
            is_font_large: false,
            is_high_contrast: false,
        }
    }

    /// A method to return a reference to the top element of the interface,
    /// currently grid.
    ///
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    /// A method to select the debug mode of the notifications.
    ///
    pub fn select_debug(&mut self, is_debug: bool) {
        self.is_debug_mode = is_debug;
    }

    /// A method to select the font size of the control items.
    ///
    pub fn select_font(&mut self, is_large: bool) {
        self.is_font_large = is_large;
    }

    /// A method to select the color contrast of the control items.
    ///
    pub fn select_contrast(&mut self, is_hc: bool) {
        self.is_high_contrast = is_hc;
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
                    Notification::Warning { .. } => {
                        notifications.remove(index);
                    }

                    // Ignore Current notifications
                    Notification::Current { .. } => {
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
            match self.notification_area_list.row_at_index(0) {
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
        // Set the font size
        let font_size = match self.is_font_large {
            false => SMALL_FONT,
            true => LARGE_FONT,
        };

        // Unpack the notification based on its variant
        match notification {
            // Highlight the error variant in bold with red
            Notification::Error {
                message,
                time,
                event,
            } => {
                // Format the time appropriately
                let timestr = time.format("%a %T");

                // Assemble a button if there should be one
                match event {
                    // If an event was specified, create a launch info button
                    Some(event_pair) => {
                        // Create a label for the button
                        let tmp_label = gtk::Label::new(None);
                        let markup =
                            format!("<span size='{}'>Trigger The Event Manually</span>", font_size);
                        tmp_label.set_markup(&markup);

                        // Create a button to open the trigger dialog
                        let new_button = gtk::Button::new();
                        new_button.add(&tmp_label);
                        new_button.set_hexpand(false);
                        new_button.set_vexpand(false);

                        // Connect the confirmation reveal
                        let interface_clone = self.interface_send.clone();
                        new_button.connect_clicked(clone!(event_pair => move |_| {
                            interface_clone
                                .sync_send(InterfaceUpdate::LaunchWindow {
                                    window_type: WindowType::Trigger(Some(event_pair.clone())),
                                });
                        }));

                        // Return the message and new button
                        (
                            format!(
                                "<span size='{}'>{} — <span color='#FF3333'><b>Error: {} ({})</b></span></span>",
                                font_size,
                                timestr,
                                clean_text(&message, UPDATE_LIMIT, true, true, true),
                                event_pair.id()
                            ),
                            Some(new_button),
                        )
                    }

                    // Otherwise just return the message
                    None => {
                        (
                            format!(
                                "<span size='{}'>{} — <span color='#FF3333'><b>Error: {}</b></span></span>",
                                font_size,
                                timestr,
                                clean_text(&message, UPDATE_LIMIT, true, true, true)
                            ),
                            None,
                        )
                    }
                }
            }

            // Highlight the warning variant with yellow
            Notification::Warning {
                message,
                time,
                event,
            } => {
                // Format the time appropriately
                let timestr = time.format("%a %T");

                // Assemble a button if there should be one
                match event {
                    // If an event was specified, create a launch info button
                    Some(event_pair) => {
                        // Create a label for the button
                        let tmp_label = gtk::Label::new(None);
                        let markup = format!(
                            "<span size='{}'>Trigger The Event Manually</span>",
                            font_size
                        );
                        tmp_label.set_markup(&markup);

                        // Create a button to open the trigger dialog
                        let new_button = gtk::Button::new();
                        new_button.add(&tmp_label);
                        new_button.set_hexpand(false);
                        new_button.set_vexpand(false);

                        // Connect the confirmation reveal
                        let interface_clone = self.interface_send.clone();
                        new_button.connect_clicked(clone!(event_pair => move |_| {
                            interface_clone
                                .sync_send(InterfaceUpdate::LaunchWindow {
                                    window_type: WindowType::Trigger(Some(event_pair.clone())),
                                });
                        }));

                        // Return the message and new button
                        (
                            format!(
                                "<span size='{}'>{} — <span color='#FFEE44'>Warning: {} ({})</span></span>",
                                font_size,
                                timestr,
                                clean_text(&message, UPDATE_LIMIT, true, true, true),
                                event_pair.id()
                            ),
                            Some(new_button),
                        )
                    }

                    // Otherwise just return the message
                    None => (
                        format!(
                            "<span size='{}'>{} — <span color='#FFEE44'>Warning: {}</span></span>",
                            font_size,
                            timestr,
                            clean_text(&message, UPDATE_LIMIT, true, true, true)
                        ),
                        None,
                    ),
                }
            }

            // Add a prefix to the current variant and highlight with blue
            Notification::Current { message, time } => {
                // Format the time appropriately
                let timestr = time.format("%a %T");

                // Combine the message and the time
                (
                    format!(
                        "<span size='{}'>{} — Event: <span color='#338DD6'>{}</span></span>",
                        font_size,
                        timestr,
                        clean_text(&message, UPDATE_LIMIT, true, true, true)
                    ),
                    None,
                )
            }

            // Leave the other update unformatted
            Notification::Update { message, time } => {
                // Format the time appropriately
                let timestr = time.format("%a %T");

                // Combine the message and the time
                (
                    format!(
                        "<span size='{}'>{} — {}</span>",
                        font_size,
                        timestr,
                        clean_text(&message, UPDATE_LIMIT, true, true, true)
                    ),
                    None,
                )
            }
        }
    }
}
