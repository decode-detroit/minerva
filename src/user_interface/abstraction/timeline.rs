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

//! A module to create the timeline abstraction that generates default
//! content and allows easier interaction and manipulation with the timeline.
//! This module links indirectly to the system interface and sends any updates
//! to the application window through gtk widgets.

// Import the relevant structures into the correct namespace
use super::super::super::system_interface::{
    DisplayControl, DisplayDebug, DisplayWith, LabelHidden, EventChange, ItemPair, SystemSend, UpcomingEvent,
};
use super::super::utils::clean_text;

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

// Import the eternal time library
extern crate time;

// Import FNV HashMap
extern crate fnv;
use self::fnv::FnvHashMap;

// Import GTK, GDK, and Cairo libraries
extern crate cairo;
extern crate gdk;
extern crate gio;
extern crate gtk;
use self::gtk::prelude::*;

// Import module constants
const TIMELINE_LIMIT: usize = 16; // maximum character width of timeline names
const MINUTES_LIMIT: f64 = 300.0; // maximum number of minutes in an adjustment
const FONT_SIZE: f64 = 12.0; // font size in pixels
const LABEL_ADJUSTMENT: f64 = 120.0; // width of timeline labels in pixels

/// An internal structure to hold the events currently in the queue. This allows
/// easier modification of the individual events as needed.
///
#[derive(Clone, Debug)]
struct TimelineEvent {
    event: ItemPair,     // the name and id of the event associated with this event
    start_time: Instant, // the original start time of the event
    delay: Duration,     // the delay of the event (relative to the original start time)
    unique_id: String,   // a unique identifier, composed from both the event id and the start_time
    location: f64,       // the location on the timeline in pixels
    updated: bool,       // a flag to indicate that this event has been updated
}

// Implement key structure features
impl TimelineEvent {
    /// A function to create a new timeline event. This method provides a regular
    /// (and reliable) method of creating a unique id.
    ///
    fn new(event: ItemPair, start_time: Instant, delay: Duration) -> TimelineEvent {
        // Create the unique identifier from the event id and the start_time
        let unique_id = TimelineEvent::new_unique_id(&event, &start_time);

        // Return the new Timeline event
        TimelineEvent {
            event,
            start_time,
            delay,
            unique_id,
            location: 0.0,
            updated: true,
        }
    }

    /// A function to create the a unique id from an event itempair and start time.
    /// This is the method used inside ::new().
    ///
    fn new_unique_id(event: &ItemPair, start_time: &Instant) -> String {
        format!("{}{:?}", event.id(), start_time)
    }

    /// A method to return the amount of time remaining in the event, as a touple
    /// of minutes and seconds (both as f64 to match with gtk::SpinButton
    /// expectations).
    ///
    fn remaining(&self) -> Option<(f64, f64)> {
        // Find the amount of time remaining
        let remaining = match self.delay.checked_sub(self.start_time.elapsed()) {
            Some(time) => time,
            None => return None,
        };

        // Extract the minutes and seconds and return it
        let seconds = remaining.as_secs() % 60;
        let minutes = remaining.as_secs() / 60;
        Some((minutes as f64, seconds as f64))
    }
}

/// A structure to create and modify the adjust button on the timeline.
///
#[derive(Clone, Debug)]
struct TimelineAdjustment {
    system_send: SystemSend, // the reply line for the interface
    timeline_events: Rc<RefCell<FnvHashMap<String, TimelineEvent>>>, // the hashmap of timeline events to modify with the adjust button
}

// Implement key structure features
impl TimelineAdjustment {
    /// A method to create a new dialog for the adjustment button. This dialog
    /// will modify the remaining duration of the events in the timeline.
    ///
    fn new_dialog(&self, window: &gtk::ApplicationWindow, unique_str: Option<String>) {
        // Create the new dialog
        let dialog = gtk::Dialog::new_with_buttons(
            Some("Adjust Timeline Event"),
            Some(window),
            gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT,
            &[
                ("Cancel", gtk::ResponseType::Cancel.into()),
                ("Confirm", gtk::ResponseType::Ok.into()),
            ],
        );
        dialog.set_position(gtk::WindowPosition::Center);

        // Get a copy of the available events
        let events = match self.timeline_events.try_borrow() {
            Ok(events) => events,
            Err(_) => return,
        };

        // Create the new spin buttons for minutes and seconds
        let minute_adjustment = gtk::Adjustment::new(0.0, 0.0, MINUTES_LIMIT, 1.0, 1.0, 1.0);
        let minutes = gtk::SpinButton::new(&minute_adjustment, 1.0, 0);
        let second_adjustment = gtk::Adjustment::new(0.0, 0.0, 60.0, 1.0, 1.0, 1.0);
        let seconds = gtk::SpinButton::new(&second_adjustment, 1.0, 0);

        // Create the event selection dropdown and populate it
        let selection = gtk::ComboBoxText::new();
        for (_, event) in events.iter() {
            selection.append(event.unique_id.as_str(), &event.event.description());
        }

        // Set the connection change parameters
        let clone_events = self.timeline_events.clone();
        selection.connect_changed(clone!(minutes, seconds => move |dropdown| {

            // Get a copy of the available events
            let events = match clone_events.try_borrow() {
                Ok(events) => events,
                Err(_) => return,
            };

            // Identify and forward the selected id
            if let Some(id_str) = dropdown.get_active_id() {

                // Identify the selected event
                if let Some(event) = events.get(&id_str) {

                    // Use the information to update the minute and second values
                    if let Some((min, sec)) = event.remaining() {
                        minutes.set_value(min);
                        seconds.set_value(sec);
                    }
                }
            }
        }));

        // Change to the provided selection, if specified
        if let Some(id_str) = unique_str {
            selection.set_active_id(id_str.as_str());
        }

        // Access the content area and add the spin buttons
        let content = dialog.get_content_area();
        let grid = gtk::Grid::new();
        content.add(&grid);

        // Add the event label and the spin buttons
        grid.attach(&gtk::Label::new(Some(" Event: ")), 0, 0, 1, 1);
        grid.attach(&selection, 1, 0, 3, 1);
        grid.attach(&gtk::Label::new(Some(" Minutes ")), 0, 1, 1, 1);
        grid.attach(&minutes, 1, 1, 1, 1);
        grid.attach(&gtk::Label::new(Some(" Seconds ")), 2, 1, 1, 1);
        grid.attach(&seconds, 3, 1, 1, 1);

        // Add some space between the rows and columns
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_left(10);
        grid.set_margin_right(10);

        // Connect the close event for when the dialog is complete
        let timeline_events = self.timeline_events.clone();
        let system_send = self.system_send.clone();
        dialog.connect_response(clone!(selection, minutes, seconds => move |modal, reply| {

            // Notify the system of the event change
            let response: i32 = gtk::ResponseType::Ok.into();
            if reply == response {

                // Try to find the information about the events
                let events = match timeline_events.try_borrow() {
                    Ok(events) => events,
                    Err(_) => return, // give up if the event list couldn't be accessed
                };

                // Identify and forward the selected event
                if let Some(id_str) = selection.get_active_id() {

                    // Look for the corresponding event
                    if let Some(event) = events.get(&id_str) {

                        // Use that information to create the new duration
                        let mut new_delay = Duration::from_secs((minutes.get_value() as u64) * 60 + (seconds.get_value() as u64));
                        new_delay += event.start_time.elapsed();

                        // Send an event update to the system
                        system_send.send(EventChange { event_id: event.event.get_id(), start_time: event.start_time.clone(), new_delay, });
                    }
                }
            }

            // Close the window either way
            modal.destroy();
        }));

        // Show the dialog and return it
        dialog.show_all();
    }
}

/// A transparent structure to hold the timeline start time and duration
///
#[derive(Clone, Debug)]
struct TimelineInfo {
    pub start: Instant,     // the current start time for the timeline
    pub duration: Duration, // the current duration for the timeline
    pub is_stale: bool,     // a flag to indicate that there are no longer any active events
    pub is_debug: bool,     // a flag to indicate if the timeline is in debug mode
}

/// A structure to hold the timeline and queue elements in the default interface.
///
/// This structure allows easier modification of the timeline and queue elements
/// to simplify interaction between the interface and the underlying system.
///
#[derive(Clone, Debug)]
pub struct TimelineAbstraction {
    grid: gtk::Grid,                 // the top level grid for the timeline display
    timeline_area: gtk::DrawingArea, // the timeline draw area for upcoming events
    timeline_events: Rc<RefCell<FnvHashMap<String, TimelineEvent>>>, // a hash map which stores references to the events currently in the timeline
    timeline_info: Rc<RefCell<TimelineInfo>>, // the timeline start time and duration
}

// Implement key features for the Timeline
impl TimelineAbstraction {
    /// A function to create a new instance of the timeline. This function loads
    /// all the default widgets into the interface and returns a new copy to
    /// allow insertion into higher levels.
    ///
    pub fn new(system_send: &SystemSend, window: &gtk::ApplicationWindow) -> TimelineAbstraction {
        // Create the timeline title
        let timeline_title = gtk::Label::new(None);
        timeline_title.set_markup("<span color='#338DD6' size='14000'>Timeline</span>");
        timeline_title.set_property_xalign(0.5);
        timeline_title.set_margin_top(20);
        timeline_title.set_margin_bottom(10);
        timeline_title.set_hexpand(true);
        timeline_title.show();

        // Create timeline drawing area
        let timeline_area = gtk::DrawingArea::new();
        timeline_area.show_all();

        // Format the area to be the correct size
        timeline_area.set_property_height_request(50);
        timeline_area.set_hexpand(true);
        timeline_area.set_vexpand(false);
        timeline_area.set_halign(gtk::Align::Fill);

        // Create a new set of timeline events
        let timeline_events = Rc::new(RefCell::new(FnvHashMap::default()));

        // Create a new instance of timeline info
        let timeline_info = Rc::new(RefCell::new(TimelineInfo {
            start: Instant::now(),
            duration: Duration::from_secs(0),
            is_stale: true,
            is_debug: false,
        }));

        // Connect the draw function for the timeline area
        timeline_area.connect_draw(clone!(timeline_events, timeline_info => move |area, event| {
            TimelineAbstraction::draw_timeline(timeline_events.clone(), timeline_info.clone(), area, event)
        }));

        // Create the timeline adjustment
        let adjustment = TimelineAdjustment {
            timeline_events: timeline_events.clone(),
            system_send: system_send.clone(),
        };

        // Connect the button press events to the timeline area
        timeline_area.add_events(gdk::EventMask::BUTTON_PRESS_MASK.bits() as i32);
        timeline_area.connect_button_press_event(clone!(window => move |area, press| {

            // Get the drawable width
            let width = area.get_allocation().width as f64;

            // Get the event position
            let (x, _) = press.get_position();

            // Try to get a copy of the timeline events
            let events_clone = adjustment.timeline_events.clone();
            let events = match events_clone.try_borrow() {
                Ok(events) => events,
                Err(_) => return Inhibit(false),
            };

            // Check to see if the click overlaps with any of the events
            let mut event_id = None;
            for (_, event) in events.iter() {
                if x > ((event.location * width) - LABEL_ADJUSTMENT) && x < (event.location * width) {

                    // If it's a match with an event
                    match event_id {

                        // If one has not already been found, set the event id
                        None => event_id = Some(event.unique_id.clone()),

                        // If one was already found
                        Some(_) => {

                            // Reset the event id
                            event_id = None;

                            // End the search
                            break;
                        },
                    }
                }
            }

            // Launch an adjustment with the found id
            adjustment.new_dialog(&window, event_id);

            // Allow the signal to propagate
            Inhibit(false)
        }));

        // Locate the different elements in the grid
        let grid = gtk::Grid::new();
        grid.attach(&timeline_title, 0, 0, 1, 1);
        grid.attach(&timeline_area, 0, 1, 1, 1);
        grid.show_all();

        // Return the new timeline
        TimelineAbstraction {
            grid,
            timeline_area,
            timeline_events,
            timeline_info,
        }
    }

    /// A method to return a reference to the top element of the interface,
    /// currently grid.
    ///
    pub fn get_top_element(&self) -> &gtk::Grid {
        &self.grid
    }

    /// A method to switch between the debug version of the timeline.
    ///
    pub fn select_debug(&self, debug: bool) {
        // Try to get a mutable copy of timeline info
        let mut timeline_info = match self.timeline_info.try_borrow_mut() {
            Ok(info) => info,
            Err(_) => return,
        };

        // Change the current debug setting
        timeline_info.is_debug = debug;
    }

    /// A method to update the timeline and the queue of coming events.
    ///
    pub fn update_events(&mut self, mut events: Vec<UpcomingEvent>) {
        // Try to get a mutable copy of timeline info
        let mut timeline_info = match self.timeline_info.try_borrow_mut() {
            Ok(info) => info,
            Err(_) => return,
        };

        // Try to get a mutable copy of timeline events
        let mut timeline_events = match self.timeline_events.try_borrow_mut() {
            Ok(events) => events,
            Err(_) => return,
        };

        // Clean out a stale timeline
        if timeline_info.is_stale && (events.len() > 0) {
            *timeline_events = FnvHashMap::default();
            timeline_info.start = Instant::now();
            timeline_info.duration = Duration::from_secs(0);
        }

        // Mark the timeline as stale if there are no new events
        if events.len() > 0 {
            timeline_info.is_stale = false;
        } else {
            timeline_info.is_stale = true;
            self.timeline_area.queue_draw();
            return;
        }

        // Calculate the new start and end time for the timeline
        for event in events.iter() {
            // Adjust the start time if it is earlier than the current one
            if event.start_time < timeline_info.start {
                timeline_info.duration =
                    timeline_info.duration + (timeline_info.start - event.start_time); // modify the old duration to match the current start time
                timeline_info.start = event.start_time;
            }

            // Adjust the duration if it is later than the current one
            if (event.start_time + event.delay) > (timeline_info.start + timeline_info.duration) {
                timeline_info.duration = (event.start_time + event.delay) - timeline_info.start;
            }
        }

        // Mark all the timeline events as old
        for (_, event) in timeline_events.iter_mut() {
            event.updated = false;
        }

        // Pass the events into the timeline events
        for event in events.drain(..) {
            // Check to see if the event is already present
            let mut existing = false; // to work around the borrow checker
            if let Some(existing_event) = timeline_events.get_mut(&TimelineEvent::new_unique_id(
                &event.event,
                &event.start_time,
            )) {
                existing = true; // to work around the borrow checker

                // If so, update the event delay
                existing_event.delay = event.delay;
                existing_event.updated = true;
            }

            // If the event was not found
            if !existing {
                // Add the new event to the timeline events
                let new_event = TimelineEvent::new(event.event, event.start_time, event.delay);
                timeline_events.insert(new_event.unique_id.clone(), new_event);
            }
        }

        // Tell the timeline area to redraw itself
        self.timeline_area.queue_draw();
    }

    /// A method to trigger a redraw of the timeline
    ///
    pub fn update(&self) {
        self.timeline_area.queue_draw();
    }

    /// A function to draw the timeline with any events
    ///
    fn draw_timeline(
        timeline_events: Rc<RefCell<FnvHashMap<String, TimelineEvent>>>,
        timeline_info: Rc<RefCell<TimelineInfo>>,
        area: &gtk::DrawingArea,
        cr: &cairo::Context,
    ) -> Inhibit {
        // Get and set the size of the window allocation
        let allocation = area.get_allocation();
        let (width, height) = (allocation.width as f64, allocation.height as f64); // create shortcuts
        cr.scale(width, height);

        // Draw the bottom line for the timeline
        cr.set_source_rgb(0.4, 0.4, 0.4);
        cr.set_line_width(2.0 / height); // 2 pixels wide
        cr.move_to(0.0, 1.0);
        cr.line_to(1.0, 1.0);
        cr.stroke();

        // Set default color and line width
        cr.set_source_rgb(0.9, 0.9, 0.9);
        cr.set_line_width(2.0 / width); // 2 pixels wide

        // Set the font size and ratio
        cr.set_font_matrix(cairo::Matrix {
            xx: (FONT_SIZE / width),
            yy: (FONT_SIZE / height),
            xy: 0.0,
            yx: 0.0,
            x0: 0.0,
            y0: 0.0,
        });

        // Try to get a mutable copy of timeline info
        let info = match timeline_info.try_borrow() {
            Ok(info) => info,
            Err(_) => return Inhibit(false),
        };

        // Try to get a mutable copy of timeline events
        let mut events = match timeline_events.try_borrow_mut() {
            Ok(events) => events,
            Err(_) => return Inhibit(false),
        };

        // Locate all the events in the timeline
        let mut event_locations = Vec::new();
        for (_, event) in events.iter_mut() {
            // Calculate the total time for the event
            let total_time = (event.start_time + event.delay) - info.start;

            // Calculate the location of the event on the timeline
            event.location = (total_time.as_secs() as f64
                + (total_time.subsec_nanos() as f64 / 1000000000.0))
                / (info.duration.as_secs() as f64
                    + (info.duration.subsec_nanos() as f64 / 1000000000.0));

            // Add the event location to the tally
            event_locations.push(event.location.clone());
        }

        // Draw any events for the timeline
        let mut placeholders = Vec::new();
        for (_, event) in events.iter_mut() {
            // Try to draw the event
            if TimelineAbstraction::draw_event(
                cr,
                timeline_info.clone(),
                event,
                width,
                &event_locations,
            ) {
                cr.set_source_rgb(0.9, 0.9, 0.9); // reset the color, just in case

            // Otherwise, draw a placeholder
            } else {
                // Extract the event location
                let location = event.location;

                // Made sure that this placeholder does not overlap others
                let label_width = LABEL_ADJUSTMENT as f64 / width;
                let location_width = location - label_width;
                let mut found = false;
                for spot in placeholders.iter() {
                    // Check to see if this placeholder overlaps
                    if (location > (*spot - label_width) && location < *spot)
                        || (location_width > (*spot - label_width) && location_width < *spot)
                    {
                        // Note that one was found and quit
                        found = true;
                        break;
                    }
                }

                // Don't add another placeholder if one was found
                if !found {
                    // Note the location of this placeholder
                    placeholders.push(location);

                    // Draw the vertical line
                    cr.set_line_width(2.0 / width); // 2 pixels wide
                    cr.move_to(location, 0.0);
                    cr.line_to(location, 1.0);
                    cr.stroke();

                    // Draw the horizonal line
                    cr.set_line_width(2.0 / 50.0); // 2 pixels wide
                    cr.move_to(location - (LABEL_ADJUSTMENT / width), 0.0);
                    cr.line_to(location, 0.0);
                    cr.stroke();

                    // Write the event text and the remaining time
                    cr.move_to(location - (LABEL_ADJUSTMENT / width), 0.3);
                    cr.show_text("Multiple Events");

                    // If there is remaining time and the timeline isn't stale, add that as well
                    if !info.is_stale {
                        if let Some((min, sec)) = event.remaining() {
                            cr.move_to(location - (LABEL_ADJUSTMENT / width), 0.6);
                            cr.show_text(format!(" In {:02}:{:02}", min, sec).as_str());
                        }
                    }
                }
            }
        }

        // If the timeline is not stale
        if !info.is_stale {
            // Draw the current time bar
            cr.set_source_rgb(0.0, 1.0, 0.0);
            let elapsed = info.start.elapsed();
            let location = (elapsed.as_secs() as f64
                + (elapsed.subsec_nanos() as f64 / 1000000000.0))
                / (info.duration.as_secs() as f64
                    + (info.duration.subsec_nanos() as f64 / 1000000000.0));
            cr.set_line_width(2.0 / width); // 2 pixels wide
            cr.move_to(location, 0.0);
            cr.line_to(location, 1.0);
            cr.stroke();

            // Add the current time to the timeline
            let time = time::now();
            cr.move_to(location, 0.9);
            cr.show_text(
                format!(" {:02}:{:02}:{:02}", time.tm_hour, time.tm_min, time.tm_sec).as_str(),
            );
        }

        // Allow the signal to propagate (probably not necessary)
        Inhibit(false)
    }

    /// A function to draw a new event on the timeline.
    ///
    /// If the event overlaps with another event, this function does not draw
    /// the event and returns false. Otherwise, it returns true.
    ///
    fn draw_event(
        cr: &cairo::Context,
        timeline_info: Rc<RefCell<TimelineInfo>>,
        event: &mut TimelineEvent,
        width: f64,
        event_locations: &Vec<f64>,
    ) -> bool {
        // Extract the event location
        let location = event.location;

        // Make sure that the event doesn't overlap other events
        let label_width = LABEL_ADJUSTMENT as f64 / width;
        let location_width = location - label_width;
        for spot in event_locations.iter() {
            // If the location is within the spot label range
            if (location > (*spot - label_width) && location < *spot)
                || (location_width > (*spot - label_width) && location_width < *spot)
            {
                return false; // indicate an overlap
            }
        }

        // Try to get a mutable copy of timeline info
        let info = match timeline_info.try_borrow() {
            Ok(info) => info,
            Err(_) => unreachable!(),
        };

        // Change the event color, visibility, and line width, if specified
        let mut text_visible = true;
        let mut line_width = 2.0; // 2 pixels wide
        match event.event.display {
            // Catch the display control variant
            DisplayControl {
                color, highlight, ..
            } => {
                // If the color is specified
                if let Some((red, green, blue)) = color {
                    cr.set_source_rgb(
                        red as f64 / 255.0,
                        green as f64 / 255.0,
                        blue as f64 / 255.0,
                    );
                }

                // If the timeline isn't stale
                if !info.is_stale {
                    // TODO Replace when let chains feature becomes stable
                    // If the highlight is specified
                    if let Some((red, green, blue)) = highlight {
                        // If there are under ten seconds remaining
                        if let Some((_, sec)) = event.remaining() {
                            // Flash the highlight color and line width
                            if (sec < 10.0) & (sec as u32 % 2 == 1) {
                                cr.set_source_rgb(
                                    red as f64 / 255.0,
                                    green as f64 / 255.0,
                                    blue as f64 / 255.0,
                                );
                                line_width = 4.0;
                            }
                        }
                    }
                }
            }

            // Catch the display with variant
            DisplayWith {
                color, highlight, ..
            } => {
                // If the color is specified
                if let Some((red, green, blue)) = color {
                    cr.set_source_rgb(
                        red as f64 / 255.0,
                        green as f64 / 255.0,
                        blue as f64 / 255.0,
                    );
                }

                // If the timeline isn't stale
                if !info.is_stale {
                    // TODO Replace when let chains feature becomes stable
                    // If the highlight is specified
                    if let Some((red, green, blue)) = highlight {
                        // If there are under ten seconds remaining
                        if let Some((_, sec)) = event.remaining() {
                            // Flash the highlight color and line width
                            if (sec < 10.0) & (sec as u32 % 2 == 1) {
                                cr.set_source_rgb(
                                    red as f64 / 255.0,
                                    green as f64 / 255.0,
                                    blue as f64 / 255.0,
                                );
                                line_width = 4.0;
                            }
                        }
                    }
                }
            }

            // Catch the display debug variant
            DisplayDebug {
                color, highlight, ..
            } => {
                // If we're in debug mode
                if info.is_debug {
                    // If the color is specified
                    if let Some((red, green, blue)) = color {
                        cr.set_source_rgb(
                            red as f64 / 255.0,
                            green as f64 / 255.0,
                            blue as f64 / 255.0,
                        );
                    }

                    // If the timeline isn't stale
                    if !info.is_stale {
                        // TODO Replace when let chains feature becomes stable
                        // If the highlight is specified
                        if let Some((red, green, blue)) = highlight {
                            // If there are under ten seconds remaining
                            if let Some((_, sec)) = event.remaining() {
                                // Flash the highlight color and line width
                                if (sec < 10.0) & (sec as u32 % 2 == 1) {
                                    cr.set_source_rgb(
                                        red as f64 / 255.0,
                                        green as f64 / 255.0,
                                        blue as f64 / 255.0,
                                    );
                                    line_width = 4.0;
                                }
                            }
                        }
                    }

                // Otherwise make the text invisible
                } else {
                    text_visible = false;
                }
            }
            
            // Catch the label hidden variant
            LabelHidden {
                color, highlight,
            } => {
                // If the color is specified
                if let Some((red, green, blue)) = color {
                    cr.set_source_rgb(
                        red as f64 / 255.0,
                        green as f64 / 255.0,
                        blue as f64 / 255.0,
                    );
                }

                // If the timeline isn't stale
                if !info.is_stale {
                    // TODO Replace when let chains feature becomes stable
                    // If the highlight is specified
                    if let Some((red, green, blue)) = highlight {
                        // If there are under ten seconds remaining
                        if let Some((_, sec)) = event.remaining() {
                            // Flash the highlight color and line width
                            if (sec < 10.0) & (sec as u32 % 2 == 1) {
                                cr.set_source_rgb(
                                    red as f64 / 255.0,
                                    green as f64 / 255.0,
                                    blue as f64 / 255.0,
                                );
                                line_width = 4.0;
                            }
                        }
                    }
                }
            }

            // If the event is hidden and the timeline is not in debug mode, hide it
            _ => {
                if !info.is_debug {
                    text_visible = false;
                }
            }
        }

        // Draw the vertical line
        cr.set_line_width(line_width / width);
        cr.move_to(location, 0.0);
        cr.line_to(location, 1.0);
        cr.stroke();

        // If the text should be visible
        if text_visible {
            // Draw the horizonal line
            cr.set_line_width(line_width / 50.0);
            cr.move_to(location - (LABEL_ADJUSTMENT / width), 0.0);
            cr.line_to(location, 0.0);
            cr.stroke();

            // Write the event text and the remaining time
            cr.move_to(location - (LABEL_ADJUSTMENT / width), 0.3);
            let text = clean_text(
                &event.event.description(),
                TIMELINE_LIMIT,
                false,
                false,
                false,
            );
            cr.show_text(&text.as_str());

            // If there is a remaining time and the timeline isn't stale, add that as well
            if !info.is_stale {
                if let Some((min, sec)) = event.remaining() {
                    cr.move_to(location - (LABEL_ADJUSTMENT / width), 0.6);
                    cr.show_text(format!(" In {:02}:{:02}", min, sec).as_str());
                }
            }
        }

        // Indicate success
        true
    }
}

