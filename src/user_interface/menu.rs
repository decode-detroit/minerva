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

// Import the relevant structures into the correct namespace
use super::UserInterface;
use super::super::system_interface::{Close, ConfigFile, SaveConfig, GameLog, ErrorLog, ClearQueue};

// Import GTK and GDK libraries
extern crate gtk;
extern crate gio;
extern crate gdk_pixbuf;
use self::gio::prelude::*;
use self::gtk::{AboutDialogExt, DialogExt, GtkApplicationExt, GtkWindowExt, ToVariant, WidgetExt, FileChooserExt};


/// A structure to hold all the features of the default menu.
///
/// This structure allows easier modification of the gtk menu to simplify
/// interaction between the menu and the rest of the interface.
///
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct MenuAbstraction;

impl MenuAbstraction {
    
    /// A function to build a new default menu for the application.
    ///
    pub fn build_menu(application: &gtk::Application, window: &gtk::ApplicationWindow, user_interface: &UserInterface) {
    
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
        let status_section = gio::Menu::new();
        let edit_section = gio::Menu::new();
        let modify_section = gio::Menu::new();
        
        // Organize the file section of the menu
        config_section.append("Choose Configuration", "app.config");
        config_section.append("Choose Game Log", "app.gamelog");
        config_section.append("Choose Error Log", "app.errorlog");
        quit_section.append("Quit", "app.quit");
        file_menu.append_item(&gio::MenuItem::new_section(None, &config_section));
        file_menu.append_item(&gio::MenuItem::new_section(None, &quit_section));
        
        // Organize the edit section of the menu
        settings_section.append("_Fullscreen", "app.fullscreen");
        settings_section.append("_Debug Mode", "app.debug_mode");
        status_section.append("Jump To ...", "app.jump");
        status_section.append("Modify Status", "app.status");
        status_section.append("Trigger Event", "app.trigger");
        status_section.append("Clear Timeline", "app.clear");
        run_menu.append_item(&gio::MenuItem::new_section(None, &settings_section));
        run_menu.append_item(&gio::MenuItem::new_section(None, &status_section));
        
        // Organize the run section of the menu
        edit_section.append("_Edit Mode", "app.edit_mode");
        edit_section.append("Save Config", "app.save_config");
        modify_section.append("New Scene", "app.new_scene");
        modify_section.append("New Status", "app.new_status");
        modify_section.append("New Event", "app.new_event");
        edit_menu.append_item(&gio::MenuItem::new_section(None, &edit_section));
        edit_menu.append_item(&gio::MenuItem::new_section(None, &modify_section));
        
        // Organize the help section of the menu
        help_menu.append("Help", "app.help");
        help_menu.append("About", "app.about");

        // Add the sub menus to the menu bar
        menu_bar.append_submenu("File", &file_menu);
        menu_bar.append_submenu("Run", &run_menu);
        menu_bar.append_submenu("Edit", &edit_menu);
        menu_bar.append_submenu("Help", &help_menu);

        // Set the menu bar
        application.set_menubar(&menu_bar);
        
        // Create the config dialog action
        let config = gio::SimpleAction::new("config", None);
        config.connect_activate(clone!(window, user_interface => move |_, _| {
        
            // Creaate and launch a new config chooser dialog
            let dialog = gtk::FileChooserDialog::new(Some("Choose Configuration"), Some(&window), gtk::FileChooserAction::Open);
            dialog.set_position(gtk::WindowPosition::Center);
            
            // Connect the close event for when the dialog is complete
            dialog.add_button("Cancel", gtk::ResponseType::Cancel.into());
            dialog.add_button("Confirm", gtk::ResponseType::Ok.into());
            dialog.connect_response(clone!(user_interface => move |chooser, id| {
            
                // Notify the system of the new configuration file
                let response: i32 = gtk::ResponseType::Ok.into();
                if id == response {
                    if let Some(filepath) = chooser.get_filename() {
                        user_interface.send(ConfigFile { filepath: Some(filepath), });
                    }
                }

                // Close the window either way
                chooser.destroy();
            }));
            
            // Show the dialog
            dialog.show_all();
        }));
        
        // Create the game log dialog action
        let gamelog = gio::SimpleAction::new("gamelog", None);
        gamelog.connect_activate(clone!(window, user_interface => move |_, _| {
        
            // Creaate and launch a new config chooser dialog
            let dialog = gtk::FileChooserDialog::new(Some("Choose Game Log File"), Some(&window), gtk::FileChooserAction::Save);
            dialog.set_position(gtk::WindowPosition::Center);
            
            // Connect the close event for when the dialog is complete
            dialog.add_button("Cancel", gtk::ResponseType::Cancel.into());
            dialog.add_button("Confirm", gtk::ResponseType::Ok.into());
            dialog.connect_response(clone!(user_interface => move |chooser, id| {
            
                // Notify the system of the new configuration file
                let response: i32 = gtk::ResponseType::Ok.into();
                if id == response {
                    if let Some(filepath) = chooser.get_filename() {
                        user_interface.send(GameLog { filepath, });
                    }
                }
                
                // Close the window either way
                chooser.destroy();
            }));
            
            // Show the dialog
            dialog.show_all();
        }));
        
        // Create the error log dialog action
        let errorlog = gio::SimpleAction::new("errorlog", None);
        errorlog.connect_activate(clone!(window, user_interface => move |_, _| {
        
            // Creaate and launch a new config chooser dialog
            let dialog = gtk::FileChooserDialog::new(Some("Choose Error Log File"), Some(&window), gtk::FileChooserAction::Save);
            dialog.set_position(gtk::WindowPosition::Center);
            
            // Connect the close event for when the dialog is complete
            dialog.add_button("Cancel", gtk::ResponseType::Cancel.into());
            dialog.add_button("Confirm", gtk::ResponseType::Ok.into());
            dialog.connect_response(clone!(user_interface => move |chooser, id| {
            
                // Notify the system of the new configuration file
                let response: i32 = gtk::ResponseType::Ok.into();
                if id == response {
                    if let Some(filepath) = chooser.get_filename() {
                        user_interface.send(ErrorLog { filepath, });
                    }
                }
                
                // Close the window either way
                chooser.destroy();
            }));
            
            // Show the dialog
            dialog.show_all();
        }));
        
        // Create the quit action
        let quit = gio::SimpleAction::new("quit", None);
        quit.connect_activate(clone!(user_interface, window => move |_, _| {
            
            // Tell the system interface to close
            user_interface.send(Close);
            
            // Close the window for the program
            window.destroy();
        }));

        // Create the fullscreen action
        let fullscreen = gio::SimpleAction::new_stateful("fullscreen", None, &true.to_variant());
        window.fullscreen(); // default to fullscreen
        fullscreen.connect_activate(clone!(window => move |checkbox, _| {
        
            // Update the fullscreen status of the window
            let mut is_fullscreen = false;
            if let Some(state) = checkbox.get_state() {
            
                // Default to false if unable to get the current state of checkbox
                is_fullscreen = state.get().unwrap_or(false);
                
                // Set the window to be fullscreen (to the opposite of the current state
                if !is_fullscreen {
                    window.fullscreen();
                } else {
                    window.unfullscreen();
                }
            }
            
            // Invert the checkbox state ourselves because of gio innerworkings
            checkbox.change_state(&(!is_fullscreen).to_variant());
        }));
        
        // Create the debug mode action
        let debug = gio::SimpleAction::new_stateful("debug_mode", None, &false.to_variant());
        debug.connect_activate(clone!(user_interface => move |checkbox, _| {
        
            // Update the debug status of the program
            let mut is_debug = false;
            if let Some(state) = checkbox.get_state() {
            
                // Default to false if unable to get the current state of checkbox
                is_debug = state.get().unwrap_or(false);
                
                // Update the rest of the interface (to the opposite of the current state)
                user_interface.select_debug(!is_debug);
            }
            
            // Invert the checkbox state ourselves because of gio innerworkings
            checkbox.change_state(&(!is_debug).to_variant());
        }));
        
        // Create the jump to dialog action
        let jump = gio::SimpleAction::new("jump", None);
        jump.connect_activate(clone!(window, user_interface => move |_, _| {
            
            // Launch the jump dialog
            user_interface.launch_jump_dialog(&window);
        }));
                
        // Create the modify status dialog action
        let status = gio::SimpleAction::new("status", None);
        status.connect_activate(clone!(window, user_interface => move |_, _| {
            
            // Launch the status modification dialog
            user_interface.launch_status_dialog(&window);
        }));
          
        // Create the clear timeline action
        let clear = gio::SimpleAction::new("clear", None);
        clear.connect_activate(clone!(user_interface => move |_, _| {
            
            // Launch the trigger event dialog
            user_interface.send(ClearQueue);
        }));
        
        // Create the trigger event to dialog action
        let trigger = gio::SimpleAction::new("trigger", None);
        trigger.connect_activate(clone!(window, user_interface => move |_, _| {
            
            // Launch the trigger event dialog
            user_interface.launch_trigger_dialog(&window);
        }));
        
        // Create the edit mode action
        let edit = gio::SimpleAction::new_stateful("edit_mode", None, &false.to_variant());
        edit.connect_activate(clone!(user_interface => move |checkbox, _| {
        
            // Update the debug status of the program
            if let Some(state) = checkbox.get_state() {
            
                // Default to false if unable to get the current state of checkbox
                let is_edit = state.get().unwrap_or(false);
                
                // Update the rest of the interface (to the opposite of the current state)
                user_interface.select_edit(!is_edit, &checkbox);
            }
        }));
        
        // Create the game log dialog action
        let saveconfig = gio::SimpleAction::new("save_config", None);
        saveconfig.connect_activate(clone!(window, user_interface, edit => move |_, _| {
        
            // Check if we're in edit mode
            if let Some(state) = edit.get_state() {
                
                // Get the current state of the checkbox
                let is_edit = state.get().unwrap_or(false);
                if is_edit {        

                    // Creaate and launch a new save config chooser dialog
                    let dialog = gtk::FileChooserDialog::new(Some("Save Config To File"), Some(&window), gtk::FileChooserAction::Save);
                    dialog.set_position(gtk::WindowPosition::Center);
                    
                    // Connect the close event for when the dialog is complete
                    dialog.add_button("Cancel", gtk::ResponseType::Cancel.into());
                    dialog.add_button("Confirm", gtk::ResponseType::Ok.into());
                    dialog.connect_response(clone!(user_interface => move |chooser, id| {
                    
                        // Notify the system of the new configuration file
                        let response: i32 = gtk::ResponseType::Ok.into();
                        if id == response {
                            if let Some(filepath) = chooser.get_filename() {
                                user_interface.send(SaveConfig { filepath, });
                            }
                        }
                        
                        // Close the window either way
                        chooser.destroy();
                    }));
                    
                    // Show the dialog
                    dialog.show_all();
                
                // If not in edit mode, prompt the user to switch to edit mode
                } else {
                
                    // Prompt the use to switch to edit mode
                    user_interface.select_edit(true, &edit);
                }
            }
        }));
        
        // Create the new event dialog action
        let newevent = gio::SimpleAction::new("new_event", None);
        newevent.connect_activate(clone!(user_interface, edit => move |_, _| {
            
            // Check if we're in edit mode
            if let Some(state) = edit.get_state() {
                
                // Get the current state of the checkbox
                let is_edit = state.get().unwrap_or(false);
                if is_edit {                
                
                    // Launch the edit event dialog
                    user_interface.launch_new_event_dialog();
                
                // If not in edit mode, prompt the user to switch to edit mode
                } else {
                
                    // Prompt the use to switch to edit mode
                    user_interface.select_edit(true, &edit);
                }
            }
        }));
        
        // Create the new status dialog action
        let newstatus = gio::SimpleAction::new("new_status", None);
        newstatus.connect_activate(clone!(user_interface, edit => move |_, _| {
            
            // Check if we're in edit mode
            if let Some(state) = edit.get_state() {
                
                // Get the current state of the checkbox
                let is_edit = state.get().unwrap_or(false);
                if is_edit {                
                
                    // Launch the status dialog
                    // FIXME user_interface.launch_new_status_dialog(&window);
                
                // If not in edit mode, prompt the user to switch to edit mode
                } else {
                
                    // Prompt the use to switch to edit mode
                    user_interface.select_edit(true, &edit);
                }
            }
        }));
        
        // Create the new scene dialog action
        let newscene = gio::SimpleAction::new("new_scene", None);
        newscene.connect_activate(clone!(user_interface, edit => move |_, _| {
            
            // Check if we're in edit mode
            if let Some(state) = edit.get_state() {
                
                // Get the current state of the checkbox
                let is_edit = state.get().unwrap_or(false);
                if is_edit {                
                               
                    // Launch the scene dialog
                    // FIXME user_interface.launch_new_scene_dialog(&window);
                
                // If not in edit mode, prompt the user to switch to edit mode
                } else {
                
                    // Prompt the use to switch to edit mode
                    user_interface.select_edit(true, &edit);
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
            dialog.set_authors(&["Patton Doyle","and Team Decode"]);
            dialog.set_website_label(Some("www.ComedyElectronics.com"));
            dialog.set_website(Some("http://www.ComedyElectronics.com"));
            dialog.set_title("About Minerva");
            dialog.set_logo_icon_name("Minerva Control Panel");
            dialog.set_program_name("Minerva Control Panel");
            dialog.set_version(env!("CARGO_PKG_VERSION"));
            dialog.set_license_type(gtk::License::Gpl30);
            
            // Try to add the software logo
            match gdk_pixbuf::Pixbuf::new_from_file(::LOGO_WIDE) {
            
                // Add the logo if successful
               Ok(ref pixbuf) => dialog.set_logo(pixbuf),
                _ => (),
            }
         
            // Set the closure and destroy parameters
            dialog.set_transient_for(Some(&window));
            dialog.run();
            dialog.destroy();
        }));
        
        // Add the actions to the application
        application.add_action(&config);
        application.add_action(&gamelog);
        application.add_action(&errorlog);
        application.add_action(&quit);
        application.add_action(&fullscreen);
        application.add_action(&debug);
        application.add_action(&edit);
        application.add_action(&saveconfig);
        application.add_action(&newevent);
        application.add_action(&newstatus);
        application.add_action(&newscene);
        application.add_action(&status);
        application.add_action(&jump);
        application.add_action(&trigger);
        application.add_action(&clear);
        application.add_action(&help);
        application.add_action(&about);
    }
}


// Tests of the abstraction module
#[cfg(test)]
mod tests {
    use super::*;
    
    // FIXME Define tests of this module
    #[test]
    fn test_menu() {
        
        // FIXME: Implement this
        unimplemented!();
    }
}
