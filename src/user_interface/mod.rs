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
    ChangeSettings, DebugMode, DetailToModify, DisplaySetting, EditMode, InterfaceUpdate,
    LaunchWindow, Notify, Redraw, SystemSend, SystemUpdate, UpdateConfig, UpdateNotifications,
    UpdateQueue, UpdateStatus, UpdateWindow, WindowType,
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
#[derive(Clone)]
pub struct UserInterface {
    interface_abstraction: Rc<RefCell<InterfaceAbstraction>>, // the interface abstraction instance for the program, wrapped in a refcell and rc for multi-referencing
    edit_mode: Rc<RefCell<bool>>, // a flag to indicate whether edit mode is active
    system_send: SystemSend, // the system update sender for the system interface, included here for easy access from the menu and other closures
    menu_abstraction: Rc<RefCell<MenuAbstraction>>, // the program menu abstraction, wrapped in a refcell and rc for multi-referencing
    window: gtk::ApplicationWindow,                 // the gtk application window
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
            InterfaceAbstraction::new(&system_send, &interface_send, window, edit_mode.clone());
        window.add(interface_abstraction.get_top_element());

        // Wrap the interface abstraction in a rc and refcell
        let interface_abstraction = Rc::new(RefCell::new(interface_abstraction));

        // Create the menu bar for the window
        let menu = MenuAbstraction::build_menu(application, window, &system_send, &interface_send);

        // Wrap the menu abstraction in a rc and refcell
        let menu_abstraction = Rc::new(RefCell::new(menu));

        // Create the User Interface with the abstraction reference
        let user_interface = UserInterface {
            interface_abstraction,
            edit_mode,
            system_send,
            menu_abstraction,
            window: window.clone(),
        };

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

    /// A method to swap between operations and edit versions of the interface.
    /// When set to true, this method clears all upcoming events and switches
    /// the operation of the program to editing the configuration file.
    pub fn select_edit(&self, edit_config: bool, checkbox: &SimpleAction) {
        // If the edit setting was chosen
        if edit_config {
            // Attempt to get a copy of the interface abstraction
            if let Ok(interface) = self.interface_abstraction.try_borrow() {
                // Launch the edit dialog
                interface.launch_edit(checkbox);
            }

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

    /// A method to launch the new event dialog to edit event details
    /// Only available in edit mode. FIXME
    ///
    pub fn launch_new_event_dialog(&self) {
        ();
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
                    statuses,
                    window,
                    key_map,
                } => {
                    // Update the current event window
                    interface.update_window(current_scene, statuses, window);
                    
                    // Update the keyboard shortcuts
                    interface.update_shortcuts(key_map);
                }

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

                // Launch the requested special window
                LaunchWindow { window_type } => {
                    // Sort for the window type
                    match window_type {
                        // Launch the jump dialog
                        WindowType::Jump(scene) => interface.launch_jump(scene),

                        // Launch the status dialog
                        WindowType::Status(status) => interface.launch_status(status),

                        // Launch the jump dialog
                        WindowType::Shortcuts => interface.launch_shortcuts(),

                        // Launch the trigger dialog
                        WindowType::Trigger(event) => interface.launch_trigger(event),
                        
                        // Launch the prompt string dialog
                        WindowType::PromptString(event) => interface.launch_prompt_string(event),
                    }
                }

                // Change the internal setting of the user interface
                ChangeSettings { display_setting } => {
                    // Attempt to get a mutable copy of the menu abstraction
                    let mut menu = match self.menu_abstraction.try_borrow_mut() {
                        Ok(menu) => menu,

                        // If unable, exit immediately
                        Err(_) => return,
                    };

                    // Sort for the display setting
                    match display_setting {
                        // Change the fullscreen mode of the display
                        DisplaySetting::FullScreen(is_fullscreen) => {
                            // Set the menu checkbox
                            menu.set_fullscreen(is_fullscreen);

                            // Change the window fullscreen setting
                            if is_fullscreen {
                                self.window.fullscreen();
                            } else {
                                self.window.unfullscreen();
                            }
                        }

                        // Change the debug mode of the display
                        DisplaySetting::DebugMode(is_debug) => {
                            // Set the menu checkbox
                            menu.set_debug(is_debug);

                            // Update the interface and trigger a redraw.
                            interface.select_debug(is_debug);
                            self.send(DebugMode(is_debug));
                            self.send(Redraw);
                        }

                        // Change the font size of the display
                        DisplaySetting::LargeFont(is_large) => {
                            // Set the menu checkbox
                            menu.set_font(is_large);

                            // Update the interface and trigger a redraw
                            interface.select_font(is_large);
                            self.send(Redraw);
                        }

                        // Change the color mode of the display
                        DisplaySetting::HighContrast(is_hc) => {
                            // Set the menu checkbox
                            menu.set_contrast(is_hc);

                            // Update the interface and trigger a redraw
                            interface.select_contrast(is_hc);
                            self.send(Redraw);
                        }
                    }
                }

                // Show a one line notification in the status bar
                Notify { message } => interface.notify(&message),

                // Launch the event detail modification window FIXME
                DetailToModify {
                    event_id,
                    event_detail,
                } => (),
            }
        }
    }
}
