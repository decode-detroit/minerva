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

//! A module to create the user interface to interact with the underlying
//! system interface. This module links directly to the system interface and
//! sends any updates to the application window.

// Define public submodules
#[macro_use]
pub mod utils;

// Define private submodules
mod abstraction;
mod menu;

// Import the relevant structures into the correct namespace
use self::abstraction::InterfaceAbstraction;
use self::menu::MenuAbstraction;
use super::system_interface::{
    DebugMode, Description, DetailToModify, EditMode, InterfaceUpdate, Notify, SystemSend,
    SystemUpdate, UpdateConfig, UpdateNotifications, UpdateQueue, UpdateStatus, UpdateWindow,
};

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

// Import GTK and GDK libraries
extern crate gdk;
extern crate gio;
extern crate gtk;
use self::gio::{ActionExt, SimpleAction};
use self::gtk::prelude::*;

// Define user interface constants
const REFRESH_RATE: u32 = 100; // the display refresh rate in milliseconds

// Import program constants
use super::WINDOW_TITLE; // the window title

/// A structure to contain the user interface and handle all updates to the
/// to the interface.
///
#[derive(Clone, Debug)]
pub struct UserInterface {
    interface_abstraction: Rc<RefCell<InterfaceAbstraction>>, // the interface abstraction instance for the program, wrapped in a refcell and rc for multi-referencing
    edit_mode: Rc<RefCell<bool>>, // a flag to indicate whether edit mode it active
    system_send: SystemSend, // the system update sender for the system interface, included here for easy access from the menu and other closures
    interface_send: mpsc::Sender<InterfaceUpdate>, // the interface update sender for the user interface, included here for easy access from the menu and other closures
    window: gtk::ApplicationWindow,                // the gkt application window
}

// Implement key UserInterface functionality
impl UserInterface {
    /// A function to create a new, blank instance of the user interface. The
    /// window provided to the function should be the top-level window for the
    /// program.
    ///
    pub fn new(
        application: &gtk::Application,
        window: &gtk::ApplicationWindow,
        system_send: SystemSend,
        interface_send: mpsc::Sender<InterfaceUpdate>,
        interface_receive: mpsc::Receiver<InterfaceUpdate>,
    ) -> UserInterface {
        // Create a new interface abstraction and add the top element to the window
        let edit_mode = Rc::new(RefCell::new(false));
        let interface_abstraction =
            InterfaceAbstraction::new(&system_send, window, edit_mode.clone());
        window.add(interface_abstraction.get_top_element());

        // Wrap the interface abstraction in a rc and refcell
        let interface_abstraction = Rc::new(RefCell::new(interface_abstraction));

        // Create the User Interface with the abstraction reference
        let user_interface = UserInterface {
            interface_abstraction,
            edit_mode,
            system_send,
            interface_send,
            window: window.clone(),
        };

        // Create the menu bar for the window
        MenuAbstraction::build_menu(application, window, &user_interface);

        // Launch the interface monitoring interrupt, currently set to ten times a second
        let update_interface = clone!(user_interface => move || {
            user_interface.check_updates(&interface_receive);
            gtk::Continue(true) // continue looking for updates indefinitely
        });
        gtk::timeout_add(REFRESH_RATE, update_interface); // triggers once every 100ms

        // Return the new UserInterface
        user_interface
    }

    /// A method to try to send a system update to the rest of the system. This
    /// method ignores a failed send.
    ///
    pub fn send(&self, update: SystemUpdate) {
        self.system_send.send(update);
    }

    /// A method to swap between normal and debug versions of the interface.
    /// When set to true, this method exposes the clear queue button, and
    /// changes the notification level to debug.
    ///
    pub fn select_debug(&self, debug_interface: bool) {
        // Send a notification to the underlying system to change mode
        self.send(DebugMode(debug_interface));

        // Attempt to get a mutable copy of the interface abstraction
        let mut interface = match self.interface_abstraction.try_borrow_mut() {
            Ok(interface) => interface,

            // If unable, exit immediately
            Err(_) => return,
        };

        // Change the timeline format, exposing the clear queue button. And
        // change the notification level.
        interface.select_debug(debug_interface);
    }

    /// A method to swap between operations and edit versions of the interface.
    /// When set to true, this method clears all upcoming events and switches
    /// the operation of the program to editing the configuration file.
    pub fn select_edit(&self, edit_config: bool, checkbox: &SimpleAction) {
        // If the edit setting was chosen
        if edit_config {
            // Attempt to get a mutable copy of the interface abstraction
            let interface = match self.interface_abstraction.try_borrow() {
                Ok(interface) => interface,

                // If unable, exit immediately
                Err(_) => return,
            };

            // Launch the edit dialog
            interface.launch_edit(checkbox);

        // If the edit setting was not chosen
        } else {
            // Change the internal flag from edit mode
            if let Ok(mut flag) = self.edit_mode.try_borrow_mut() {
                *flag = false;
            }

            // Switch the interface back to operations mode
            self.send(EditMode(false));

            // Return the checkbox to its default state
            checkbox.change_state(&(false).to_variant());

            // Change the window title back to normal
            self.window.set_title(WINDOW_TITLE);
        }
    }

    /// A method to launch the status dialog to modify an individual status
    ///
    pub fn launch_status_dialog(&self, window: &gtk::ApplicationWindow) {
        // Attempt to get a copy of the interface abstraction
        if let Ok(interface) = self.interface_abstraction.try_borrow() {
            // Launch the dialog
            interface.launch_status(window);
        }
    }

    /// A method to launch the jump dialog to change between individual scenes
    ///
    pub fn launch_jump_dialog(&self, window: &gtk::ApplicationWindow) {
        // Attempt to get a copy of the interface abstraction
        if let Ok(interface) = self.interface_abstraction.try_borrow() {
            // Launch the dialog
            interface.launch_jump(window);
        }
    }

    /// A method to launch the trigger dialog to change between individual scenes
    ///
    pub fn launch_trigger_dialog(&self, window: &gtk::ApplicationWindow) {
        // Attempt to get a copy of the interface abstraction
        if let Ok(interface) = self.interface_abstraction.try_borrow() {
            // Launch the dialog
            interface.launch_trigger(window);
        }
    }

    /// A method to laundh the new event dialog to edit event details
    /// Only available in edit mode.
    ///
    pub fn launch_new_event_dialog(&self) {
        // Attempt to get a copy of the interface abstraction
        if let Ok(interface) = self.interface_abstraction.try_borrow() {
            // Launch the dialog
            interface.launch_edit_event(None, None);
        }
    }

    /// A method to listen for modifications to the user interface.
    ///
    /// This method listens on the provided interface_update line for any changes
    /// to the interface. The method then processes any/all of these updates
    /// in the order that they were received.
    ///
    pub fn check_updates(&self, interface_update: &mpsc::Receiver<InterfaceUpdate>) {
        // Attempt to get a mutable copy of the interface abstraction
        let mut interface = match self.interface_abstraction.try_borrow_mut() {
            Ok(interface) => interface,

            // If unable, exit immediately
            Err(_) => return,
        };

        // Update the time-sensitive elements of the interface
        interface.refresh_all();

        // Look for any updates and act upon them
        loop {
            // Check to see if there are any more updatess
            let update = match interface_update.try_recv() {
                Ok(update) => update,
                _ => return, // exit when there are no updates left
            };

            // Unpack the updates of every type
            match update {
                // Update the available scenes in the scene selection and the available statuses
                UpdateConfig {
                    scenes,
                    full_status,
                } => {
                    // Update the special dialogs
                    interface.update_scenes(scenes);
                    interface.update_full_status(full_status);

                    // Clear the existing events from the main window
                    interface.clear_events();
                }

                // Update the current event window
                UpdateWindow {
                    current_scene,
                    window,
                } => interface.update_window(current_scene, window),

                // Update the state of a particular status
                UpdateStatus {
                    status_id,
                    new_state,
                } => interface.update_state(status_id, new_state),

                // Update the notifications in the notification window
                UpdateNotifications { notifications } => {
                    interface.update_notifications(notifications)
                }

                // Update the events in the timeline area
                UpdateQueue { events } => interface.update_events(events),

                // Show a one line notification in the status bar
                Notify { message } => interface.notify(&message),

                // Show the item description in a new window
                Description { item_information } => interface.launch_info(&item_information),

                // Launch the event detail modification window
                DetailToModify {
                    event_id,
                    event_detail,
                } => interface.launch_edit_event(
                    Some(event_id),
                    Some(event_detail),
                ),
                // FIXME Add other modification tools here
            }
        }
    }
}

