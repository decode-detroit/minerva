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

//! A module to create the user interface menu that generates default content
//! and allows easier interaction and manipulation of the menu. This module
//! links directly to the rest of the user interface and sends any updates to
//! the application window through gtk widgets.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

// Import GTK and GDK libraries
use gdk_pixbuf;
use gio;
use gio::prelude::*;
use gtk;
use gtk::prelude::*;

/// A structure to hold all the features of the default menu.
///
/// This structure allows easier modification of the gtk menu to simplify
/// interaction between the menu and the rest of the interface.
///
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct MenuAbstraction {
    fullscreen: gio::SimpleAction, // checkbox for fullscreen
    debug: gio::SimpleAction,      // checkbox for debug mode
    font: gio::SimpleAction,       // checkbox for large font
    contrast: gio::SimpleAction,   // checkbox for high contrast
}

impl MenuAbstraction {
    /// A function to build a new default menu for the application.
    ///
    pub fn build_menu(
        application: &gtk::Application,
        window: &gtk::ApplicationWindow,
        gtk_send: &GtkSend,
        interface_send: &mpsc::Sender<InterfaceUpdate>,
    ) -> MenuAbstraction {
        // Create the menu bar and the different submenus
        let menu_bar = gio::Menu::new();
        let file_menu = gio::Menu::new();
        let edit_menu = gio::Menu::new();
        let run_menu = gio::Menu::new();
        let help_menu = gio::Menu::new();

        // Create the submenu sections
        let config_section = gio::Menu::new();
        let quit_section = gio::Menu::new();
        let settings_section = gio::Menu::new();
        let window_section = gio::Menu::new();
        let edit_section = gio::Menu::new();
        let modify_section = gio::Menu::new();

        // Organize the file section of the menu
        config_section.append(Some("Choose Configuration"), Some("app.config"));
        config_section.append(Some("Choose Game Log"), Some("app.game_log"));
        config_section.append(Some("Choose Error Log"), Some("app.error_log"));
        quit_section.append(Some("Quit"), Some("app.quit"));
        file_menu.append_item(&gio::MenuItem::new_section(None, &config_section));
        file_menu.append_item(&gio::MenuItem::new_section(None, &quit_section));

        // Organize the edit section of the menu
        settings_section.append(Some("_Fullscreen"), Some("app.fullscreen"));
        settings_section.append(Some("_Debug Mode"), Some("app.debug_mode"));
        settings_section.append(Some("_Large Font"), Some("app.large_font"));
        settings_section.append(Some("_High Contrast"), Some("app.contrast"));
        window_section.append(Some("Show Shortcuts"), Some("app.shortcuts"));
        window_section.append(Some("Jump To ..."), Some("app.jump"));
        window_section.append(Some("Modify Status"), Some("app.status"));
        window_section.append(Some("Trigger Event"), Some("app.trigger"));
        window_section.append(Some("Clear Timeline"), Some("app.clear"));
        run_menu.append_item(&gio::MenuItem::new_section(None, &settings_section));
        run_menu.append_item(&gio::MenuItem::new_section(None, &window_section));

        // Organize the run section of the menu
        edit_section.append(Some("_Edit Mode"), Some("app.edit_mode"));
        edit_section.append(Some("Save Config"), Some("app.save_config"));
        modify_section.append(Some("New Scene"), Some("app.new_scene"));
        modify_section.append(Some("New Status"), Some("app.new_status"));
        modify_section.append(Some("New Event"), Some("app.new_event"));
        modify_section.append(Some("List Audio Devices"), Some("app.list_audio"));
        edit_menu.append_item(&gio::MenuItem::new_section(None, &edit_section));
        edit_menu.append_item(&gio::MenuItem::new_section(None, &modify_section));

        // Organize the help section of the menu
        help_menu.append(Some("Help"), Some("app.help"));
        help_menu.append(Some("About"), Some("app.about"));

        // Add the sub menus to the menu bar
        menu_bar.append_submenu(Some("File"), &file_menu);
        menu_bar.append_submenu(Some("Run"), &run_menu);
        menu_bar.append_submenu(Some("Edit"), &edit_menu);
        menu_bar.append_submenu(Some("Help"), &help_menu);

        // Set the menu bar
        application.set_menubar(Some(&menu_bar));

        // Create the config dialog action
        let config = gio::SimpleAction::new("config", None);
        config.connect_activate(clone!(window, gtk_send => move |_, _| {

            // Creaate and launch a new config chooser dialog
            let dialog = gtk::FileChooserDialog::new(Some("Choose Configuration"), Some(&window), gtk::FileChooserAction::Open);
            dialog.set_position(gtk::WindowPosition::Center);

            // Connect the close event for when the dialog is complete
            dialog.add_button("Cancel", gtk::ResponseType::Cancel);
            dialog.add_button("Confirm", gtk::ResponseType::Ok);
            dialog.connect_response(clone!(gtk_send => move |chooser, id| {

                // Notify the system of the new configuration file
                if id == gtk::ResponseType::Ok {
                    if let Some(filepath) = chooser.get_filename() {
                        gtk_send.send(UserRequest::ConfigFile { filepath: Some(filepath), });
                    }
                }

                // Close the window either way
                unsafe {
                    chooser.destroy();
                }
            }));

            // Show the dialog
            dialog.show_all();
        }));

        // Create the game log dialog action
        let game_log = gio::SimpleAction::new("game_log", None);
        game_log.connect_activate(clone!(window, gtk_send => move |_, _| {

            // Creaate and launch a new config chooser dialog
            let dialog = gtk::FileChooserDialog::new(Some("Choose Game Log File"), Some(&window), gtk::FileChooserAction::Save);
            dialog.set_position(gtk::WindowPosition::Center);

            // Connect the close event for when the dialog is complete
            dialog.add_button("Cancel", gtk::ResponseType::Cancel);
            dialog.add_button("Confirm", gtk::ResponseType::Ok);
            dialog.connect_response(clone!(gtk_send => move |chooser, id| {

                // Notify the system of the new configuration file
                if id == gtk::ResponseType::Ok {
                    if let Some(filepath) = chooser.get_filename() {
                        gtk_send.send(UserRequest::GameLog { filepath, });
                    }
                }

                // Close the window either way
                unsafe {
                    chooser.destroy();
                }
            }));

            // Show the dialog
            dialog.show_all();
        }));

        // Create the error log dialog action
        let error_log = gio::SimpleAction::new("error_log", None);
        error_log.connect_activate(clone!(window, gtk_send => move |_, _| {

            // Creaate and launch a new config chooser dialog
            let dialog = gtk::FileChooserDialog::new(Some("Choose Error Log File"), Some(&window), gtk::FileChooserAction::Save);
            dialog.set_position(gtk::WindowPosition::Center);

            // Connect the close event for when the dialog is complete
            dialog.add_button("Cancel", gtk::ResponseType::Cancel);
            dialog.add_button("Confirm", gtk::ResponseType::Ok);
            dialog.connect_response(clone!(gtk_send => move |chooser, id| {

                // Notify the system of the new configuration file
                if id == gtk::ResponseType::Ok {
                    if let Some(filepath) = chooser.get_filename() {
                        gtk_send.send(UserRequest::ErrorLog { filepath, });
                    }
                }

                // Close the window either way
                unsafe {
                    chooser.destroy();
                }
            }));

            // Show the dialog
            dialog.show_all();
        }));

        // Create the quit action
        let quit = gio::SimpleAction::new("quit", None);
        quit.connect_activate(clone!(gtk_send, window => move |_, _| {

            // Tell the system interface to close
            gtk_send.send(UserRequest::Close);

            // Wait 1000 nanoseconds for the process to complete
            thread::sleep(Duration::new(0, 1000));

            // Close the window for the program
            unsafe {
                window.destroy();
            }
        }));

        // Create the fullscreen action
        let fullscreen = gio::SimpleAction::new_stateful("fullscreen", None, &false.to_variant());
        fullscreen.connect_activate(clone!(interface_send => move |checkbox, _| {

            // Update the fullscreen status of the window
            if let Some(state) = checkbox.get_state() {

                // Default to false if unable to get the current state of checkbox
                let is_fullscreen = state.get().unwrap_or(false);

                // Update the interface (to the opposite of the current state)
                interface_send
                    .send(InterfaceUpdate::ChangeSettings {
                        display_setting: DisplaySetting::FullScreen(!is_fullscreen),
                    })
                    .unwrap_or(());
            }
        }));

        // Create the debug mode action
        let debug = gio::SimpleAction::new_stateful("debug_mode", None, &false.to_variant());
        debug.connect_activate(clone!(interface_send => move |checkbox, _| {
            // Update the debug status of the program
            if let Some(state) = checkbox.get_state() {
                // Default to false if unable to get the current state of checkbox
                let is_debug = state.get().unwrap_or(false);

                // Update the interface (to the opposite of the current state)
                interface_send
                    .send(InterfaceUpdate::ChangeSettings {
                        display_setting: DisplaySetting::DebugMode(!is_debug),
                    })
                    .unwrap_or(());
            }
        }));

        // Create the large font action
        let font = gio::SimpleAction::new_stateful("large_font", None, &false.to_variant());
        font.connect_activate(clone!(interface_send => move |checkbox, _| {
            // Update the font size of the program
            if let Some(state) = checkbox.get_state() {
                // Default to false if unable to get the current state of checkbox
                let is_large = state.get().unwrap_or(false);

                // Update the interface (to the opposite of the current state)
                interface_send
                    .send(InterfaceUpdate::ChangeSettings {
                        display_setting: DisplaySetting::LargeFont(!is_large),
                    })
                    .unwrap_or(());
            }
        }));

        // Create the high contrast action
        let contrast = gio::SimpleAction::new_stateful("contrast", None, &false.to_variant());
        contrast.connect_activate(clone!(interface_send => move |checkbox, _| {
            // Update the high contrast state of the program
            if let Some(state) = checkbox.get_state() {
                // Default to false if unable to get the current state of checkbox
                let is_hc = state.get().unwrap_or(false);

                // Update the interface (to the opposite of the current state)
                interface_send
                    .send(InterfaceUpdate::ChangeSettings {
                        display_setting: DisplaySetting::HighContrast(!is_hc),
                    })
                    .unwrap_or(());
            }
        }));

        // Create the jump to dialog action
        let shortcuts = gio::SimpleAction::new("shortcuts", None);
        let interface_clone = interface_send.clone();
        shortcuts.connect_activate(move |_, _| {
            // Launch the shortcuts dialog
            interface_clone
                .send(InterfaceUpdate::LaunchWindow {
                    window_type: WindowType::Shortcuts,
                })
                .unwrap_or(());
        });

        // Create the jump to dialog action
        let jump = gio::SimpleAction::new("jump", None);
        let interface_clone = interface_send.clone();
        jump.connect_activate(move |_, _| {
            // Launch the jump dialog
            interface_clone
                .send(InterfaceUpdate::LaunchWindow {
                    window_type: WindowType::Jump(None),
                })
                .unwrap_or(());
        });

        // Create the modify status dialog action
        let status = gio::SimpleAction::new("status", None);
        let interface_clone = interface_send.clone();
        status.connect_activate(move |_, _| {
            // Launch the status modification dialog
            interface_clone
                .send(InterfaceUpdate::LaunchWindow {
                    window_type: WindowType::Status(None),
                })
                .unwrap_or(());
        });

        // Create the clear timeline action
        let clear = gio::SimpleAction::new("clear", None);
        clear.connect_activate(clone!(gtk_send => move |_, _| {
            // Launch the trigger event dialog
            gtk_send.send(UserRequest::ClearQueue);
        }));

        // Create the trigger event to dialog action
        let trigger = gio::SimpleAction::new("trigger", None);
        let interface_clone = interface_send.clone();
        trigger.connect_activate(move |_, _| {
            // Launch the trigger event dialog
            interface_clone
                .send(InterfaceUpdate::LaunchWindow {
                    window_type: WindowType::Trigger(None),
                })
                .unwrap_or(());
        });

        // Create the edit mode action (toggles availability of the other edit actions)
        let edit = gio::SimpleAction::new_stateful("edit_mode", None, &false.to_variant());

        // Create the save game configuration action
        let save_config = gio::SimpleAction::new("save_config", None);
        save_config.connect_activate(clone!(window, gtk_send, edit => move |_, _| {

            // Check if we're in edit mode
            if let Some(state) = edit.get_state() {

                // Get the current state of the checkbox
                let is_edit = state.get().unwrap_or(false);
                if is_edit {

                    // Creaate and launch a new save config chooser dialog
                    let dialog = gtk::FileChooserDialog::new(Some("Save Config To File"), Some(&window), gtk::FileChooserAction::Save);
                    dialog.set_position(gtk::WindowPosition::Center);

                    // Connect the close event for when the dialog is complete
                    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
                    dialog.add_button("Confirm", gtk::ResponseType::Ok);
                    dialog.connect_response(clone!(gtk_send => move |chooser, id| {

                        // Notify the system of the new configuration file
                        if id == gtk::ResponseType::Ok {
                            if let Some(filepath) = chooser.get_filename() {
                                gtk_send.send(UserRequest::SaveConfig { filepath, });
                            }
                        }

                        // Close the window either way
                        unsafe {
                            chooser.destroy();
                        }
                    }));

                    // Show the dialog
                    dialog.show_all();
                }
            }
        }));

        // Create the new scene dialog action
        let new_scene = gio::SimpleAction::new("new_scene", None);
        new_scene.connect_activate(clone!(edit => move |_, _| {

            // Check if we're in edit mode
            if let Some(state) = edit.get_state() {

                // Get the current state of the checkbox
                let is_edit = state.get().unwrap_or(false);
                if is_edit {

                    // Launch the scene dialog
                    // FIXME user_interface.launch_new_scene_dialog(&window);
                }
            }
        }));

        // Create the new status dialog action
        let new_status = gio::SimpleAction::new("new_status", None);
        new_status.connect_activate(clone!(edit => move |_, _| {
            // Check if we're in edit mode
            if let Some(state) = edit.get_state() {
                // Get the current state of the checkbox
                let is_edit = state.get().unwrap_or(false);
                if is_edit {

                    // Launch the status dialog
                    // FIXME user_interface.launch_new_status_dialog(&window);
                }
            }
        }));

        // Create the new event dialog action
        let new_event = gio::SimpleAction::new("new_event", None);
        new_event.connect_activate(clone!(edit => move |_, _| {
            // Check if we're in edit mode
            if let Some(state) = edit.get_state() {
                // Get the current state of the checkbox
                let is_edit = state.get().unwrap_or(false);
                if is_edit {

                    // Launch the edit event dialog
                    // FIXME user_interface.launch_new_event_dialog();
                }
            }
        }));

        // Create the list audio devices action
        let list_audio = gio::SimpleAction::new("list_audio", None);
        list_audio.connect_activate(|_, _| {
            // Try to run the process
            if let Ok(process) = Command::new("aplay").arg("-L").output() {
                // Try to convert the output
                if let Ok(output) = String::from_utf8(process.stdout) {
                    // Print the result
                    print!("{}", output);

                // Otherwise, alert the user
                } else {
                    println!("Error: Invalid output from 'aplay'."); // FIXME Make this pretty
                }

            // Otherwise, alert the user aplay must be installed
            } else {
                println!("Error: This feature requires 'aplay'.");
            }
        });

        // Connect the edit mode action
        edit.connect_activate(clone!(interface_send, application, save_config, new_event, new_status, new_scene => move |checkbox, _| {

            // Update the edit status of the program
            if let Some(state) = checkbox.get_state() {

                // Swap the current state of the checkbox
                let is_edit = !state.get().unwrap_or(true);

                // Update the rest of the interface (to the opposite of the current state)
                interface_send.send(InterfaceUpdate::EditMode(is_edit)).unwrap_or(());

                // Swap the checkbox state
                checkbox.change_state(&(is_edit).to_variant());

                // Change the availability of the other actions
                if is_edit {
                    // Enable the other actions
                    application.add_action(&save_config);
                    application.add_action(&new_event);
                    application.add_action(&new_status);
                    application.add_action(&new_scene);

                // Otherwise disable the other actions
                } else {
                    application.remove_action("save_config");
                    application.remove_action("new_event");
                    application.remove_action("new_status");
                    application.remove_action("new_scene");
                }
            }
        }));

        // Create the help dialog action
        let help = gio::SimpleAction::new("help", None);
        help.connect_activate(move |_, _| {
            // Send the user to the help page
            gtk::show_uri(None, "http://www.comedyelectronics.com", 0).unwrap_or(());
        });

        // Create the about dialog action
        let about = gio::SimpleAction::new("about", None);
        about.connect_activate(clone!(window => move |_, _| {

            // Create the dialog and set the parameters of the information
            let dialog = gtk::AboutDialog::new();
            dialog.set_authors(&["Patton Doyle","Peter Doyle","Jasmine Powell","and Team Decode"]);
            dialog.set_website_label(Some("www.ComedyElectronics.com"));
            dialog.set_website(Some("http://www.ComedyElectronics.com"));
            dialog.set_title("About Minerva");
            dialog.set_logo_icon_name(Some("Minerva Control Panel"));
            dialog.set_program_name("Minerva Control Panel");
            dialog.set_version(Some(env!("CARGO_PKG_VERSION")));
            dialog.set_license_type(gtk::License::Gpl30);

            // Try to add the software logo
            match gdk_pixbuf::Pixbuf::from_file(super::super::LOGO_WIDE) {

                // Add the logo if successful
               Ok(ref pixbuf) => dialog.set_logo(Some(pixbuf)),
                _ => (),
            }

            // Set the closure and destroy parameters
            dialog.set_transient_for(Some(&window));
            dialog.run();
            unsafe {
                dialog.destroy();
            }
        }));

        // Add the actions to the application
        application.add_action(&config);
        application.add_action(&game_log);
        application.add_action(&error_log);
        application.add_action(&quit);
        application.add_action(&fullscreen);
        application.add_action(&debug);
        application.add_action(&font);
        application.add_action(&contrast);
        application.add_action(&edit);
        application.add_action(&list_audio);
        application.add_action(&shortcuts);
        application.add_action(&jump);
        application.add_action(&status);
        application.add_action(&trigger);
        application.add_action(&clear);
        application.add_action(&help);
        application.add_action(&about);

        // Return the completed menu
        MenuAbstraction {
            fullscreen,
            debug,
            font,
            contrast,
        }
    }

    /// Helper function to change the current state of the fullscreen checkbox
    pub fn set_fullscreen(&mut self, is_fullscreen: bool) {
        self.fullscreen.change_state(&(is_fullscreen).to_variant());
    }

    /// Helper function to change the current state of the debug checkbox
    pub fn set_debug(&mut self, is_debug: bool) {
        self.debug.change_state(&(is_debug).to_variant());
    }

    /// Helper function to change the current state of the font checkbox
    pub fn set_font(&mut self, is_large: bool) {
        self.font.change_state(&(is_large).to_variant());
    }

    /// Helper function to change the current state of the high contrast checkbox
    pub fn set_contrast(&mut self, is_hc: bool) {
        self.contrast.change_state(&(is_hc).to_variant());
    }
}
