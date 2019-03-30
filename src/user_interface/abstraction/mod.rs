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

//! A module to create the user interface abstraction that generates default
//! content and allows easier interaction and manipulation with the interface.
//! system interface. This module links indirectly to the system interface and
//! sends any updates to the application window through gtk widgets.

// Define private submodules
mod control;
mod events;
mod special_windows;
mod timeline;

// Import the relevant structures into the correct namespace
use self::control::ControlAbstraction;
use self::events::EventAbstraction;
use self::special_windows::{
    EditDialog, EditEventDialog, InfoDialog, JumpDialog, StatusDialog, TriggerDialog,
};
use self::timeline::TimelineAbstraction;
use super::super::system_interface::{
    EventDetail, EventWindow, FullStatus, ItemPair, Notification, SystemSend, UpcomingEvent,
};
use super::utils::clean_text;

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;

// Import GTK and GDK libraries
extern crate gdk;
extern crate gio;
extern crate gtk;
use self::gio::SimpleAction;
use self::gtk::prelude::*;

// Import the external time library
extern crate time;

// Define module constants
const NOTIFY_LIMIT: usize = 60; // maximum character width of the notifications

/// A structure to hold all the features of the default interface.
///
/// This structure allows easier modification of the gtk interface to simplify
/// interaction between the interface and the underlying program.
///
#[derive(Clone, Debug)]
pub struct InterfaceAbstraction {
    primary_grid: gtk::Grid, // the top-level grid that contains the control and event grids
    timeline: TimelineAbstraction, // the timeline abstraction that holds the timeline of upcoming events
    control: ControlAbstraction, // the control abstraction that holds the universal system controls
    events: EventAbstraction, // the event abstraction that holds the events selectable in the current scene
    status_bar: gtk::Statusbar, // the status bar where one line notifications are posted
    edit_dialog: EditDialog,  // the edit dialog for the system to confirm a change to edit mode
    status_dialog: StatusDialog, // the status dialog for the system to change individual statuses
    jump_dialog: JumpDialog,  // the jump dialog for the system to switch between individual scenes
    trigger_dialog: TriggerDialog, // the trigger dialog for the system to trigger a custom event
    info_dialog: InfoDialog,  // the information dialog for displaying information about items
    edit_event_dialog: EditEventDialog, // the edit event dialog for the editing event details
    is_debug: bool,           // a flag to indicate whether or not the program is in debug mode
}

impl InterfaceAbstraction {
    /// A function to create a return a new interface abstraction to hold the
    /// gtk elements of the program.
    ///
    /// # Note
    ///
    /// These elements are in the process of rapid iteration and should not be
    /// relied upon for any future stability (even names may change to improve
    /// clarity).
    ///
    pub fn new(
        system_send: &SystemSend,
        window: &gtk::ApplicationWindow,
        edit_mode: Rc<RefCell<bool>>,
    ) -> InterfaceAbstraction {
        // Create the top-level element of the program, a grid to hold other elements
        let primary_grid = gtk::Grid::new();

        // Set the features of the primary grid
        primary_grid.set_column_homogeneous(false); // Allow everything to adjust
        primary_grid.set_row_homogeneous(false);
        primary_grid.set_column_spacing(10); // add some space between the rows and columns
        primary_grid.set_row_spacing(10);
        primary_grid.set_margin_left(10); // add some space on the left and right side
        primary_grid.set_margin_right(10);

        // Set the interrior items to fill the available space
        primary_grid.set_valign(gtk::Align::Fill);
        primary_grid.set_halign(gtk::Align::Fill);

        // Create the timeline abstraction and add it to the primary grid
        let timeline = TimelineAbstraction::new(system_send, window);
        primary_grid.attach(timeline.get_top_element(), 0, 0, 3, 1);

        // Create the control abstraction and add it to the primary grid
        let control = ControlAbstraction::new(system_send);
        primary_grid.attach(control.get_top_element(), 0, 3, 1, 1);

        // Create a horizontal and vertical separator
        let separator_vertical = gtk::Separator::new(gtk::Orientation::Vertical);
        let separator_horizontal = gtk::Separator::new(gtk::Orientation::Horizontal);

        // Configure the separators and add them to the primary grid
        separator_vertical.set_valign(gtk::Align::Fill);
        separator_vertical.set_vexpand(true);
        separator_horizontal.set_halign(gtk::Align::Fill);
        separator_horizontal.set_hexpand(true);
        primary_grid.attach(&separator_vertical, 1, 1, 1, 3);
        primary_grid.attach(&separator_horizontal, 0, 4, 3, 1);

        // Create the status bar and add it to the primary grid
        let status_bar = gtk::Statusbar::new();
        status_bar.set_property_height_request(30);
        status_bar.set_vexpand(false);
        status_bar.set_hexpand(true);
        status_bar.set_halign(gtk::Align::Fill);
        primary_grid.attach(&status_bar, 0, 5, 3, 1);

        // Create the event abstraction and add it to the primary grid
        let events = EventAbstraction::new();
        primary_grid.attach(events.get_top_element(), 2, 1, 1, 3);

        // Add the title and the side panel
        let title = gtk::Label::new(None);
        #[cfg(feature = "theater-speak")]
        title.set_markup("<span color='#338DD6' size='14000'>Story Information</span>");
        #[cfg(not(feature = "theater-speak"))]
        title.set_markup("<span color='#338DD6' size='14000'>Game Information</span>");
        title.set_property_xalign(0.5);
        title.show();
        primary_grid.attach(&title, 0, 1, 1, 1);
        primary_grid.attach(events.get_side_panel(), 0, 2, 1, 1);

        // Create the special windows for the user interface
        let edit_dialog = EditDialog::new(edit_mode, window);
        let status_dialog = StatusDialog::new();
        let jump_dialog = JumpDialog::new();
        let trigger_dialog = TriggerDialog::new();
        let info_dialog = InfoDialog::new(window);
        let edit_event_dialog = EditEventDialog::new(window);

        // Return a copy of the interface abstraction
        InterfaceAbstraction {
            primary_grid,
            timeline,
            control,
            events,
            status_bar,
            edit_dialog,
            status_dialog,
            jump_dialog,
            trigger_dialog,
            info_dialog,
            edit_event_dialog,
            is_debug: false,
        }
    }

    /// A method to return a reference to the top element of the interface,
    /// currently primary grid.
    ///
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.primary_grid
    }

    /// A method to change the timeline and the notification level to (or from)
    /// debug. If notification level is not debug, only updates, warnings, and
    /// errors will be displayed.
    ///
    pub fn select_debug(&mut self, debug: bool) {
        self.timeline.select_debug(debug);
        self.control.debug_notifications(debug);
        self.is_debug = debug;
    }

    // Methods to update the timeline abstraction
    //
    /// A method to update the timebar within the timeline
    ///
    pub fn update_timeline(&self) {
        self.timeline.update();
    }
    //
    /// A method to update the timeline of coming events
    ///
    pub fn update_events(&mut self, events: Vec<UpcomingEvent>) {
        self.timeline.update_events(events);
    }

    // Methods to update the control abstraction
    //
    /// A method to update the notifications in the control abstraction
    ///
    pub fn update_notifications(&mut self, notifications: Vec<Notification>) {
        self.control.update_notifications(notifications);
    }

    // Methods to update the event abstraction
    //
    /// A method to clear the existing events in the main window.
    ///
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
    //
    /// A method to update the current event window based on new events
    ///
    pub fn update_window(
        &mut self,
        current_scene: ItemPair,
        window: EventWindow,
        system_send: &SystemSend,
    ) {
        self.events
            .update_window(current_scene, window, system_send);
    }

    /// A method to update the status bar
    ///
    pub fn notify(&mut self, new_text: &str) {
        // Remove any old messages from the status bar
        self.status_bar.pop(0);

        // Add the time to the event description
        let now = time::now();
        let timestr = now.strftime("%a %T").unwrap_or_else(|_| now.asctime()); // Fallback on other time format
        let message = format!(
            "\t\t{} — {}",
            timestr,
            clean_text(&new_text, NOTIFY_LIMIT, false, false, false)
        );

        // Add the new notification to the status bar
        self.status_bar.push(0, &message);

        // If in debug mode, also print it to stdio
        if self.is_debug {
            println!("{}", message);
        }
    }

    /// A method to update the state of a particular status
    ///
    pub fn update_state(&mut self, status_id: ItemPair, new_state: ItemPair) {
        // Update both the event abstraction and the status dialog
        self.events
            .update_state(status_id.clone(), new_state.clone());
        self.status_dialog.update_state(status_id, new_state);
    }

    /// A method to launch the edit dialog
    ///
    pub fn launch_edit(&self, system_send: &SystemSend, checkbox: &SimpleAction) {
        self.edit_dialog.launch(system_send, checkbox);
    }

    // Methods to update the status dialog
    //
    /// A method to launch the status dialog
    ///
    pub fn launch_status(&self, window: &gtk::ApplicationWindow, system_send: &SystemSend) {
        self.status_dialog.launch(window, system_send);
    }
    //
    /// A method to update the statuses in the status dialog
    ///
    pub fn update_full_status(&mut self, new_status: FullStatus) {
        self.status_dialog.update_full_status(new_status);
    }

    // Methods to update the jump dialog
    //
    /// A method to launch the jump dialog
    ///
    pub fn launch_jump(&self, window: &gtk::ApplicationWindow, system_send: &SystemSend) {
        self.jump_dialog.launch(window, system_send);
    }
    //
    /// A method to update the scenes in the jump dialog
    ///
    pub fn update_scenes(&mut self, scenes: Vec<ItemPair>) {
        self.jump_dialog.update_scenes(scenes);
    }

    /// A method to launch the trigger event dialog
    ///
    pub fn launch_trigger(&self, window: &gtk::ApplicationWindow, system_send: &SystemSend) {
        self.trigger_dialog.launch(window, system_send);
    }

    /// A method to launch the information window
    ///
    pub fn launch_info(&self, item_information: &ItemPair) {
        self.info_dialog.launch(item_information);
    }

    /// A method to launch the edit event dialog
    ///
    pub fn launch_edit_event(
        &self,
        system_send: &SystemSend,
        event_id: Option<ItemPair>,
        event_detail: Option<EventDetail>,
    ) {
        self.edit_event_dialog
            .launch(system_send, event_id, event_detail);
    }
}

