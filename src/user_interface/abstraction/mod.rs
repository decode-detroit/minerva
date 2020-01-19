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
    EditDialog, JumpDialog, PromptStringDialog, ShortcutsDialog, StatusDialog,
    TriggerDialog,
};
use self::timeline::TimelineAbstraction;
use super::super::system_interface::{
    EventWindow, FullStatus, Hidden, InterfaceUpdate, ItemPair, KeyMap,
    Notification, StatusDescription, SystemSend, UpcomingEvent,
};
use super::utils::clean_text;

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

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
const SMALL_FONT: u32 = 10000; // equivalent to 14pt
const NORMAL_FONT: u32 = 12000; // equivalent to 12pt
const LARGE_FONT: u32 = 14000; // equivalent to 10pt

/// A structure to hold all the features of the default interface.
///
/// This structure allows easier modification of the gtk interface to simplify
/// interaction between the interface and the underlying program.
///
pub struct InterfaceAbstraction {
    system_send: SystemSend, // a copy of system send held in the interface abstraction
    interface_send: mpsc::Sender<InterfaceUpdate>, // a copy of the interface send
    primary_grid: gtk::Grid, // the top-level grid that contains the control and event grids
    full_status: Rc<RefCell<FullStatus>>, // user interface storage of the current full status of the system (for use by the event labels and the status dialog)
    current_window: (ItemPair, Vec<ItemPair>, EventWindow), // user interface storage of the current event window
    timeline: TimelineAbstraction, // the timeline abstraction that holds the timeline of upcoming events
    control: ControlAbstraction, // the control abstraction that holds the universal system controls
    events: EventAbstraction, // the event abstraction that holds the events selectable in the current scene
    notification_bar: gtk::Statusbar, // the status bar where one line notifications are posted
    edit_dialog: EditDialog,  // the edit dialog for the system to confirm a change to edit mode
    jump_dialog: JumpDialog,  // the jump dialog for the system to switch between individual scenes
    status_dialog: StatusDialog, // the status dialog for the system to change individual statuses
    shortcuts_dialog: ShortcutsDialog,  // the shortcuts dialog to show the current keyboard shortcuts
    trigger_dialog: TriggerDialog, // the trigger dialog for the system to trigger a custom event
    prompt_string_dialog: PromptStringDialog, // the prompty string dialog to solicit information from the user
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
        interface_send: &mpsc::Sender<InterfaceUpdate>,
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
        primary_grid.set_margin_start(10); // add some space on the left and right side
        primary_grid.set_margin_end(10);

        // Set the interrior items to fill the available space
        primary_grid.set_valign(gtk::Align::Fill);
        primary_grid.set_halign(gtk::Align::Fill);

        // Create the timeline abstraction and add it to the primary grid
        let timeline = TimelineAbstraction::new(system_send, window);
        primary_grid.attach(timeline.get_top_element(), 0, 0, 3, 1);

        // Create the control abstraction and add it to the primary grid
        let control = ControlAbstraction::new(system_send, interface_send);
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

        // Create the notification bar and add it to the primary grid
        let notification_bar = gtk::Statusbar::new();
        notification_bar.set_vexpand(false);
        notification_bar.set_hexpand(true);
        notification_bar.set_halign(gtk::Align::Fill);
        primary_grid.attach(&notification_bar, 0, 5, 3, 1);

        // Create the event abstraction and add it to the primary grid
        let events = EventAbstraction::new();
        primary_grid.attach(events.get_top_element(), 2, 1, 1, 3);

        // Create the side panel scrolling window
        let side_scroll = gtk::ScrolledWindow::new(
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
            Some(&gtk::Adjustment::new(0.0, 0.0, 100.0, 0.1, 100.0, 100.0)),
        ); // Should be None, None, but the compiler has difficulty inferring types
        side_scroll.add(events.get_side_panel());
        side_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

        // Add the title and the side panel
        let title = gtk::Label::new(None);
        title.set_markup("<span color='#338DD6' size='14000'>Game Information</span>");
        title.set_halign(gtk::Align::Center);
        title.show();

        primary_grid.attach(&title, 0, 1, 1, 1);
        primary_grid.attach(&side_scroll, 0, 2, 1, 1);

        // Create internal storage for the full status of the system
        let full_status = Rc::new(RefCell::new(FullStatus::default()));

        // Create the special windows for the user interface
        let edit_dialog = EditDialog::new(edit_mode, window);
        let jump_dialog = JumpDialog::new(window);
        let status_dialog = StatusDialog::new(full_status.clone(), window);
        let shortcuts_dialog = ShortcutsDialog::new(system_send, window);
        let trigger_dialog = TriggerDialog::new(window);
        let prompt_string_dialog = PromptStringDialog::new(window);

        // Return a copy of the interface abstraction
        InterfaceAbstraction {
            system_send: system_send.clone(),
            interface_send: interface_send.clone(),
            primary_grid,
            full_status,
            current_window: (
                ItemPair::new_unchecked(1, "", Hidden),
                Vec::new(),
                Vec::new(),
            ),
            timeline,
            control,
            events,
            notification_bar,
            edit_dialog,
            jump_dialog,
            status_dialog,
            shortcuts_dialog,
            trigger_dialog,
            prompt_string_dialog,
            is_debug: false,
        }
    }

    /// A method to return a reference to the top element of the interface,
    /// currently primary grid.
    ///
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.primary_grid
    }

    /// A method to change the notification level to (or from) debug. If
    /// notification level is not debug, only updates, warnings, and errors will
    /// be displayed.
    ///
    pub fn select_debug(&mut self, is_debug: bool) {
        self.control.select_debug(is_debug);
        self.is_debug = is_debug;
    }

    /// A method to change the font size of all items in the interface.
    ///
    pub fn select_font(&mut self, is_large: bool) {
        self.timeline.select_font(is_large);
        self.control.select_font(is_large);
        self.events.select_font(is_large);
    }

    /// A method to change the color contrast of all items in the interface.
    ///
    pub fn select_contrast(&mut self, is_hc: bool) {
        self.timeline.select_contrast(is_hc);
        self.control.select_contrast(is_hc);
        self.events.select_contrast(is_hc);
    }

    /// A method to update the internal statuses in the abstraction and the status dialog
    ///
    pub fn update_full_status(&mut self, new_status: FullStatus) {
        // Try to get a mutable copy of the full status
        if let Ok(mut full_status) = self.full_status.try_borrow_mut() {
            // Copy the new full status into the structure
            *full_status = new_status;
        }
    }

    /// A method to update all time-sensitive elements of the interface
    ///
    pub fn refresh_all(&self) {
        // Refresh the timeline
        self.timeline.refresh();
    }

    /// A method to update the timeline of coming events
    ///
    pub fn update_events(&mut self, events: Vec<UpcomingEvent>) {
        self.timeline.update_events(events);
    }

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
        statuses: Vec<ItemPair>,
        window: EventWindow,
    ) {
        // Save a copy of the current scene and event window
        self.current_window = (current_scene.clone(), statuses.clone(), window.clone());

        // Try to get a copy of the full status
        if let Ok(full_status) = self.full_status.try_borrow() {
            // Update the event window
            self.events.update_window(
                current_scene,
                statuses,
                window,
                &full_status,
                &self.system_send,
                &self.interface_send,
            );
        }
    }

    /// A method to update the status bar
    ///
    pub fn notify(&mut self, new_text: &str) {
        // Remove any old messages from the status bar
        self.notification_bar.pop(0);

        // Add the time to the event description
        let now = time::now();
        let timestr = now.strftime("%a %T").unwrap_or_else(|_| now.asctime()); // Fallback on other time format
        let message = format!(
            "\t\t{} — {}",
            timestr,
            clean_text(&new_text, NOTIFY_LIMIT, false, false, false)
        );

        // Add the new notification to the status bar
        self.notification_bar.push(0, &message);

        // If in debug mode, also print it to stdio
        if self.is_debug {
            println!("{}", message);
        }
    }

    /// A method to update the state of a particular status
    ///
    pub fn update_state(&mut self, status_id: ItemPair, new_state: ItemPair) {
        // Try to get a mutable copy of the full status
        if let Ok(mut full_status) = self.full_status.try_borrow_mut() {
            // Modify the specified id
            if let Some(&mut StatusDescription {
                ref mut current, ..
            }) = full_status.get_mut(&status_id)
            {
                // Change the current status
                *current = new_state;
            }
        }

        // Redraw the event window
        if let Ok(full_status) = self.full_status.try_borrow() {
            let (current_scene, statuses, window) = self.current_window.clone();
            self.events.update_window(
                current_scene,
                statuses,
                window,
                &full_status,
                &self.system_send,
                &self.interface_send,
            );
        }
    }

    /// A method to launch the edit dialog
    ///
    pub fn launch_edit(&self, checkbox: &SimpleAction) {
        self.edit_dialog.launch(&self.system_send, checkbox);
    }

    // Methods to update the jump dialog
    //
    /// A method to launch the jump dialog
    ///
    pub fn launch_jump(&self, scene: Option<ItemPair>) {
        self.jump_dialog.launch(&self.system_send, scene);
    }
    //
    /// A method to update the scenes in the jump dialog
    ///
    pub fn update_scenes(&mut self, scenes: Vec<ItemPair>) {
        self.jump_dialog.update_scenes(scenes);
    }

    // Methods to update the status dialog
    //
    /// A method to launch the status dialog
    ///
    pub fn launch_status(&self, status: Option<ItemPair>) {
        self.status_dialog.launch(&self.system_send, status);
    }
    
    // Methods to update the shortcuts dialog
    //
    /// A method to launch the shortcuts dialog
    ///
    pub fn launch_shortcuts(&self) {
        self.shortcuts_dialog.launch();
    }
    //
    /// A method to update the scenes in the jump dialog
    ///
    pub fn update_shortcuts(&mut self, key_map: KeyMap) {
        self.shortcuts_dialog.update_shortcuts(key_map);
    }

    /// A method to launch the trigger event dialog
    ///
    pub fn launch_trigger(&self, event: Option<ItemPair>) {
        self.trigger_dialog.launch(&self.system_send, event);
    }
    
    /// A method to launch the prompt string dialog
    pub fn launch_prompt_string(&self, event: ItemPair) {
        self.prompt_string_dialog.launch(&self.system_send, event);
    }
}
