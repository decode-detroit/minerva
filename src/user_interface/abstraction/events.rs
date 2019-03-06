// Copyright (c) 2019 Decode Detroit
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
use super::super::utils::clean_text;
use super::super::super::system_interface::{SystemSend, TriggerEvent, StatusChange, EventGroup, EventWindow, ItemId, ItemPair, DisplayType, DisplayControl, DisplayWith, DisplayDebug, LabelHidden, Hidden};

// Import standard library features
use std::rc::Rc;
use std::cell::RefCell;
use std::u32::MAX as u32MAX;

// Import GTK and GDK libraries
extern crate gtk;
extern crate gdk;
use self::gtk::prelude::*;

// Define module constants
const BUTTON_LIMIT: usize = 20; // maximum character width of normal buttons
const STORY_BUTTON_LIMIT: usize = 40; // maximum character width of story buttons


/// A structure to hold all the event groups in the default interface.
///
/// This structure allows easier modification of the gtk event group interfaces
/// to simplify interaction between the interface and the underlying program.
///
#[derive(Clone, Debug)]
pub struct EventAbstraction {
    window: gtk::ScrolledWindow, // the scrolled window to hold the grid
    grid: gtk::Grid, // the grid to hold the underlying elements
    groups: Vec<EventGroupAbstraction>, // a vector of the current event groups
    side_panel: gtk::Grid, // the container to hold the side panel events
    side_group: Option<EventGroupAbstraction>, // side panel group (for ungrouped events)
    last_change: Rc<RefCell<Option<ItemId>>>, // a flag to track state changes and prevent double triggering
}

// Implement key features for the Event Abstraction
impl EventAbstraction {
    
    /// A function to create a new Event Abstration instance.
    ///
    /// Notifications of new events that are triggered will be posted to the
    /// status bar with a context id of 0.
    ///
    pub fn new() -> EventAbstraction {
    
        // Create the event grid for holding all the available events
        let grid = gtk::Grid::new();
        
        // Set the features of the grid
        grid.set_column_homogeneous(false); // set the row and column heterogeneous
        grid.set_row_homogeneous(false);
        grid.set_column_spacing(10); // add some space between the columns
        grid.set_row_spacing(0);
        grid.set_margin_top(10); // add some space on the margins
        grid.set_margin_bottom(10);
        grid.set_margin_left(10);
        grid.set_margin_right(10);
        
        // Create the scrolled window and add the grid
        let window = gtk::ScrolledWindow::new(None, None);
        window.add(&grid);
        window.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
        
        // Format the window
        window.set_hexpand(true);
        window.set_vexpand(true);
        window.set_halign(gtk::Align::Fill);
        window.set_valign(gtk::Align::Fill);
        
        // Create the side panel grid and set the features
        let side_panel = gtk::Grid::new();
        side_panel.set_column_homogeneous(false); // set the row and column heterogeneous
        side_panel.set_row_homogeneous(false);
        side_panel.set_column_spacing(10); // add some space between the columns
        side_panel.set_row_spacing(10);
        side_panel.set_margin_top(10); // add some space on the margins
        side_panel.set_margin_bottom(10);
        side_panel.set_margin_left(10);
        side_panel.set_margin_right(10);
        side_panel.set_vexpand(true); // adjust the expansion parameters of the grid
        side_panel.set_hexpand(false);
        
        // Return the new Event Abstraction
        EventAbstraction {
            window,
            grid,
            groups: Vec::new(), // an empty list of event groups
            side_panel,
            side_group: None, // an empty side panel event group
            last_change: Rc::new(RefCell::new(None)), // an empty list of pending state changes
        }
    }
    
    /// A method to return a reference to the top element of the interface,
    /// currently the top scrolled window.
    ///
    pub fn get_top_element(&self) -> &gtk::ScrolledWindow {
        &self.window
    }
    
    /// A method to return the side panel container for special events
    ///
    pub fn get_side_panel(&self) -> &gtk::Grid {
        &self.side_panel
    }

    /// A method to clear the old event groups and event grids to create a fresh
    /// event abstraction.
    ///
    pub fn clear(&mut self) {
        
        // Remove all the the children from the primary grid
        let to_remove = self.grid.get_children();
        for item in to_remove {
            self.grid.remove(&item);
        }
        
        // Replace the side panel grid
        let to_remove = self.side_panel.get_children();
        for item in to_remove {
            self.side_panel.remove(&item);
        }
        
        // Clear the group list and the side group to clear dangling references
        self.groups = Vec::new();
        self.side_group = None;
    }
    
    /// A method to clear the old event grid and load the provided window into
    /// the event grid, updating the Event Abstraction in the process.
    ///
    /// This method will send the triggered event id as a System Update on the
    /// system send line.
    ///
    pub fn update_window(&mut self, current_scene: ItemPair, mut window: EventWindow, system_send: &SystemSend) {
    
        // Empty the old event grid
        self.clear();
        
        // Copy the available groups into new group abstractions
        let mut groups_raw = Vec::new();
        for group in window.drain(..) {
        
            // Try to load the group into a group abstraction
            if let Some(grp_abstraction) = EventGroupAbstraction::new(group, system_send, self.last_change.clone()) {     
        
                // If it has an id, add it to the group list
                if let Some(_) = grp_abstraction.group_id {
                    
                    // Identify the group priority
                    match grp_abstraction.priority.clone() {
                        
                        // If the priority is defined, set it
                        Some(number) => groups_raw.push((number, grp_abstraction)),
                        
                        // Otherwise, set the maximum default priority
                        None => groups_raw.push((u32MAX, grp_abstraction)),
                    }
                
                // Otherwise place it in the default group
                } else {
                    self.side_group = Some(grp_abstraction);
                }
            }
        }
        
        // Reorder the groups to follow priority
        groups_raw.sort_by_key(|pair| {
            let &(ref priority, _) = pair;
            priority.clone()
        });
        
        // Strip the raw groups to remove priority
        for (_, group) in groups_raw.drain(..) {
            self.groups.push(group);
        }
        
        // Add the current scene detail
        let current_title = gtk::Label::new(Some("Current Scene:"));
        current_title.set_property_xalign(0.5);
        current_title.set_margin_left(60);
        current_title.set_margin_right(10);
        current_title.show();
        self.side_panel.attach(&current_title, 0, 1, 1, 1);
        let current = gtk::Label::new(None);
        decorate_label(&current, &current_scene.description, current_scene.display);
        current.set_property_xalign(0.5);
        current.set_margin_left(10);
        current.set_margin_right(40);
        current.show();
        self.side_panel.attach(&current, 1, 1, 1, 1);

        // Add title and control separators
        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
        separator.set_halign(gtk::Align::Fill);
        separator.set_hexpand(true);
        separator.show();
        self.side_panel.attach(&separator, 0, 2, 2, 1);
        
        // Try to attach the side panel group to the side panel grid
        if let Some(ref group) = self.side_group {
        
            // Create the found side panel group
            let grid = group.as_grid();
            grid.set_halign(gtk::Align::Center);
            self.side_panel.attach(&grid, 0, 3, 2, 1);
        }
        
        // Attach all the regular event groups to the event grid
        let mut count: i32 = 0;
        for group in self.groups.iter() {
            
            // Attach the found group to the grid
            self.grid.attach(&group.as_grid(), count, 0, 1, 1);
            count = count + 1;
        }
    }
    
    /// A method to update the state of a particular event group based on
    /// the provided group id and state.
    ///
    /// # Note
    ///
    /// This function will update the state of any group with a matching id.
    /// In theory, there should only be one event group that matches the id.
    ///
    pub fn update_state(&mut self, group_id: ItemPair, state: ItemPair) {
    
        // Find the correct group
        for group in self.groups.iter() {
        
            // Update the state any group with the matching id
            if let Some(id) = group.group_id {
                
                // Update the state if there is a match
                if group_id.get_id() == id {
                
                    // Try to add the state to change flag
                    if let Ok(mut last_change) = self.last_change.try_borrow_mut() {
                        
                        // Add the state id to the flag    
                        *last_change = Some(state.get_id());
                    }
                
                    // Update the dropdown (if it exists)
                    let id_str: &str = &state.id().to_string();
                    if let Some(ref dropdown) = group.state_selection {
                        dropdown.set_active_id(id_str);
                    }
                }
            }
        }
    }
}


/// An internal structure to hold an individual event group in the default interface.
///
/// This structure allows easier modification of the gtk event group interfaces
/// to simplify interaction between the interface and the underlying program.
///
#[derive(Clone, Debug)]
struct EventGroupAbstraction {
    group_id: Option<ItemId>, // the id for the group for rapid identification
    priority: Option<u32>, // the priority for the group for ordering on the screen
    header: gtk::Label, // the label attached to the header for the group
    state_selection: Option<gtk::ComboBoxText>, // the dropdown for switching between states (if it exists)
    buttons: Vec<gtk::Button>, // the event buttons for the event group
}

// Implement key features for the Event Group Abstraction
impl EventGroupAbstraction {
    
    /// A function to create a new abstraction from the provided EventGroup.
    ///
    fn new(event_group: EventGroup, system_send: &SystemSend, last_change: Rc<RefCell<Option<ItemId>>>) -> Option<EventGroupAbstraction> {
        
        // If there is an id, create the header and id and add it to the group
        let group_id;
        let priority;
        let button_width;
        #[cfg(feature = "theater-speak")]
        let header = gtk::Label::new(Some("Story Controls"));
        #[cfg(not(feature = "theater-speak"))]
        let header = gtk::Label::new(Some("Game Controls"));
        match event_group.group_id.clone() {
        
            // Create the group_id and header for the particular group
            Some(id_pair) => {
                group_id = Some(id_pair.get_id());
                priority = decorate_label(&header, &id_pair.description, id_pair.display);
                button_width = BUTTON_LIMIT;
            },
        
            // Create the default id and header
            None => {
                group_id = None;
                priority = None;
                button_width = STORY_BUTTON_LIMIT; // allow larger buttons
            },
        }
        
        // Create the status selection dropdown (if the status exists)
        let mut state_selection = None;
        if let Some(_) = event_group.group_state {
        
            // Create the new state selection
            let selection = gtk::ComboBoxText::new();

            // Add each of the available states to the dropdown
            for state_pair in event_group.allowed_states.iter() {
                let id_str: &str = &state_pair.id().to_string();
                selection.append(id_str, &state_pair.description());
            }
            
            // Set the dropdown to the current status state, if it exists
            if let Some(state) = event_group.group_state {
                let id_str: &str = &state.id().to_string();
                selection.set_active_id(id_str);
            }
            
            // Try to extract the valid event_group status id
            if let Some(status_id) = event_group.group_id {
            
                // Connect the function to trigger when the state selection changes
                selection.connect_changed(clone!(system_send, last_change => move |dropdown| {
                    
                    // Identify and forward the selected state
                    if let Some(id_str) = dropdown.get_active_id() {
                        
                        // Try to parse the requested id
                        if let Ok(id_number) = id_str.parse::<u32>() {
                        
                            // Try to compose the id into an item
                            if let Some(state) = ItemId::new(id_number) {
                                
                                // Check to see if this was an internal change
                                if let Ok(mut change) = last_change.try_borrow_mut() {
                                    
                                    // If the state matches the flag
                                    if *change == Some(state) {
                                      
                                        // Reset the flag and return before sending to the system
                                        *change = None;
                                        return
                                    }
                                }
                                
                                // Send the new state of the status to the underlying system
                                system_send.send(StatusChange { status_id: status_id.get_id(), state });
                            }
                        }
                    }
                }));
            }
            
            // Set the new state selection
            state_selection = Some(selection);
        }
        
        // Create a new button for each of the group events
        let mut buttons_raw = Vec::new();
        for event in event_group.group_events {
            
            // Create a new button
            let button_label = gtk::Label::new(None);
            let button_markup = clean_text(&event.description(), button_width, true, false, true);
            
            // Set the markup based on the requested color and extract the priority
            let button_priority = decorate_label(&button_label, &button_markup, event.display);
            
            // Set the features of the new label and place it on the button
            button_label.show();
            button_label.set_property_xalign(0.5);
            let button = gtk::Button::new();
            button.add(&button_label);
            
            // Set the features of the new button
            button.set_size_request(80, 30);
            button.set_margin_right(10);
            button.set_margin_left(10);
            
            // Create the new button action and connect it
            button.connect_clicked(clone!(system_send => move |_| {
                
                // Send the event trigger to the underlying system
                system_send.send(TriggerEvent { event: event.get_id()});
            }));
            
            // Add the priority and button to the list
            match button_priority {
                
                // Use the priority, if provided
                Some(number) => buttons_raw.push((number, button)),
                
                // Otherwise, default to the maximum possible
                None => buttons_raw.push((u32MAX, button)),
            }
        }
        
        // Reorder the buttons to follow priority
        buttons_raw.sort_by_key(|pair| {
            let &(ref priority, _) = pair;
            priority.clone()
        });
        
        // Strip the raw buttons to remove priority
        let mut buttons = Vec::new();
        for (_, button) in buttons_raw.drain(..) {
            buttons.push(button);
        }
        
        // If there are some buttons in the abstraction
        if buttons.len() > 0 {
            
            // Return the new group abstraction
            return Some(EventGroupAbstraction {
                group_id,
                priority,
                header,
                state_selection,
                buttons,
            });
        }
        
        // Otherwise, return nothing
        None 
    }
    
    /// A method to compose the event group into a scrollable, vertical grid of
    /// buttons, with the name and status of the group at the top.
    ///
    fn as_grid(&self) -> gtk::Grid {
    
        // Create the top level grid for this group
        let grid = gtk::Grid::new();
        
        // Define the formatting for this grid
        grid.set_column_homogeneous(false); // set row and column heterogeneous
        grid.set_row_homogeneous(false);
        grid.set_row_spacing(10); // add some space between the rows
        grid.set_column_spacing(0);
        grid.set_margin_top(10); // add margins on all sides
        grid.set_margin_bottom(10);
        grid.set_margin_left(5);
        grid.set_margin_right(5);
        
        // Create the grid for the buttons
        let button_grid = gtk::Grid::new();
        
        // Define the formatting for the button grid
        button_grid.set_column_homogeneous(false); // set row and column heterogeneous
        button_grid.set_row_homogeneous(false);
        button_grid.set_row_spacing(10); // add some space between the rows
        button_grid.set_column_spacing(0);
        
        // Populate the button grid
        for (number, button) in self.buttons.iter().enumerate() {
            
            // Add each button to the grid
            button_grid.attach(button, 0, number as i32, 1, 1);
        }
        
        // Create the scrollable window for the buttons
        let window = gtk::ScrolledWindow::new(None, None);
        window.add(&button_grid);
        window.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        
        // Format the window
        window.set_hexpand(false);
        window.set_vexpand(true);
        window.set_valign(gtk::Align::Fill);
        
        // Add the label, status, and window to the grid
        grid.attach(&self.header, 0, 0, 1, 1);
        
        // Add the status dropdown if it exists
        if let Some(ref selection) = self.state_selection {
            grid.attach(selection, 0, 1, 1, 1);
            grid.attach(&window, 0, 2, 1, 1);
        
        // Otherwise just add the window
        } else {
            grid.attach(&window, 0, 1, 1, 1);
        }

        // Show all the elements in the group
        self.show_all();
        button_grid.show();
        window.show();
        grid.show();
        
        // Return the new grid
        grid
    }
    
    /// A method to show all the gtk widgets in the group abstraction.
    ///
    fn show_all(&self) {
    
        // Show the header and the label
        self.header.show();
        if let Some(ref selection) = self.state_selection {
            selection.show();
        }
        
        // Show all the event buttons
        for button in self.buttons.iter() {
            button.show();
        }
    }
}


/// An internal helper function to properly decorate a label. The function
/// sets the markup for the existing label and returns the priority from
/// the DisplayType, if it exists.
///
/// This function assumes that the text has already been cleaned and sized.
///
fn decorate_label(label: &gtk::Label, text: &str, display: DisplayType) -> Option<u32> {

    // Decorate based on the display type
    match display {    

        // Match the display control variant
        DisplayControl { color, priority, .. } => {
        
            // Set the markup color, if specified
            if let Some((red, green, blue)) = color {
                label.set_markup(&format!("<span color='#{:02X}{:02X}{:02X}'>{}</span>", red, green, blue, text));
                
            // Default to default text color
            } else {
                label.set_markup(text);
            }
            
            // Return the priority
            return priority;
        },

        // Match the display with variant
        DisplayWith { color, priority, .. } => {
        
            // Set the markup color, if specified
            if let Some((red, green, blue)) = color {
                label.set_markup(&format!("<span color='#{:02X}{:02X}{:02X}'>{}</span>", red, green, blue, text));
                
            // Default to default text color
            } else {
                label.set_markup(text);
            }
            
            // Return the priority
            return priority;
        },
        
        // Match the display debug variant
        DisplayDebug { color, priority, .. } => {
        
            // Set the markup color, if specified
            if let Some((red, green, blue)) = color {
                label.set_markup(&format!("<span color='#{:02X}{:02X}{:02X}'>{}</span>", red, green, blue, text));
                
            // Default to default text color
            } else {
                label.set_markup(text);
            }
            
            // Return the priority
            return priority;
        },
        
        // Set only the color for a hidden label
        LabelHidden { color } => {
            
            // Set the markup color
            let (red, green, blue) = color;
            label.set_markup(&format!("<span color='#{:02X}{:02X}{:02X}'>{}</span>", red, green, blue, text));
            return None;
        },
        
        // Otherwise, use the default color and priority
        Hidden => {
            label.set_markup(text);
            return None;
        },
    }
}

// Tests of the event abstraction module
#[cfg(test)]
mod tests {
    use super::*;
    
    // FIXME Define tests of this module
    #[test]
    fn test_event_abstraction() {
        
        // FIXME: Implement this
        unimplemented!();
    }
}
