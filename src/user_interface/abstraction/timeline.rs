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
    AllEventChange, DisplayControl, DisplayDebug, DisplayWith, EventChange, ItemPair, LabelControl,
    LabelHidden, SystemSend, UpcomingEvent,
};
use super::super::utils::clean_text;
use super::{LARGE_FONT, NORMAL_FONT};

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
const TIMELINE_LIMIT: usize = 26; // maximum character width of timeline names
const TIMELINE_LIMIT_SHORT: usize = 21; // maximum character width of timeline names
const MINUTES_LIMIT: f64 = 300.0; // maximum number of minutes in an adjustment
const LABEL_SIZE: f64 = 240.0; // the size of the event labels in pixels
const NOW_LOCATION: f64 = 0.02; // the location of "now" on the timeline, from the left
const TIMELINE_HEIGHT: f64 = 80.0; // the height of the timeline
const CLICK_PRECISION: f64 = 20.0; // the click precision for selecting events in pixels
const HIGHLIGHT_WIDTH: f64 = 16.0; // the width of an event when highlighted

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
    in_focus: bool,      // a flag to indicate that this event has been clicked on
}

// Implement key structure features
impl TimelineEvent {
    /// A function to create a new timeline event. This method provides a regular
    /// (and reliable) method of creating a unique id.
    ///
    fn new(event: UpcomingEvent) -> TimelineEvent {
        // Create the unique identifier from the event id and the start_time
        let unique_id = TimelineEvent::new_unique_id(&event.event, &event.start_time);

        // Return the new Timeline event
        TimelineEvent {
            event: event.event,
            start_time: event.start_time,
            delay: event.delay,
            unique_id,
            location: 0.0,
            in_focus: false,
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
        let minutes = remaining.as_secs() / 60;
        let seconds = remaining.as_secs() % 60;
        Some((minutes as f64, seconds as f64))
    }

    /// A method to return the precise amount of time remaining in the event
    /// as a float number of fractional seconds.
    ///
    fn remaining_precise(&self) -> Option<f64> {
        // Find the amount of time remaining
        let remaining = match self.delay.checked_sub(self.start_time.elapsed()) {
            Some(time) => time,
            None => return None,
        };

        // Extract the exact number of seconds remaining
        Some(remaining.as_secs() as f64 + (remaining.subsec_nanos() as f64 / 1000000000.0))
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
                ("Cancel", gtk::ResponseType::Cancel),
                ("Confirm", gtk::ResponseType::Ok),
            ],
        );
        dialog.set_position(gtk::WindowPosition::Center);

        // Get a copy of the available events
        let events = match self.timeline_events.try_borrow() {
            Ok(events) => events,
            Err(_) => return,
        };

        // Chreate the cancel checkbox
        let cancel_checkbox = gtk::CheckButton::new_with_label("Cancel Event");

        // Create the new spin buttons for minutes and seconds
        let minute_adjustment = gtk::Adjustment::new(0.0, 0.0, MINUTES_LIMIT, 1.0, 1.0, 1.0);
        let minutes = gtk::SpinButton::new(Some(&minute_adjustment), 1.0, 0);
        let second_adjustment = gtk::Adjustment::new(0.0, 0.0, 60.0, 1.0, 1.0, 1.0);
        let seconds = gtk::SpinButton::new(Some(&second_adjustment), 1.0, 0);

        // Create the event selection dropdown and populate it
        let selection = gtk::ComboBoxText::new();
        selection.append(Some("all"), "Adjust All Events");
        for (_, event) in events.iter() {
            selection.append(Some(event.unique_id.as_str()), &event.event.description());
        }

        // Create the time label
        let time_label = gtk::Label::new(Some(" Minutes "));

        // Set the connection change parameters
        let clone_events = self.timeline_events.clone();
        selection.connect_changed(
            clone!(minutes, seconds, time_label, cancel_checkbox => move |dropdown| {

                // Get a copy of the available events
                let events = match clone_events.try_borrow() {
                    Ok(events) => events,
                    Err(_) => return,
                };

                // Identify the selected ID and adjust accordingly
                if let Some(id) = dropdown.get_active_id() {

                    // If all events are selected
                    if id == "all" {
                        // Change the labels
                        cancel_checkbox.set_label("Subtract Time");
                        time_label.set_text("  Add Minutes  ");

                        // Set the time to zero
                        minutes.set_value(0.0);
                        seconds.set_value(0.0);
                    } else

                    // Identify the selected event
                    if let Some(event) = events.get(id.as_str()) {
                        // Reset the labels
                        cancel_checkbox.set_label("Cancel Event");
                        time_label.set_text(" Minutes ");

                        // Use the information to update the minute and second values
                        if let Some((min, sec)) = event.remaining() {
                            minutes.set_value(min);
                            seconds.set_value(sec);
                        }
                    }
                }
            }),
        );

        // Change to the provided selection, if specified
        if let Some(id_str) = unique_str {
            selection.set_active_id(Some(id_str.as_str()));

        // Otherwise set it to all
        } else {
            selection.set_active_id(Some("all"));
        }

        // Access the content area and add the spin buttons
        let content = dialog.get_content_area();
        let grid = gtk::Grid::new();
        content.add(&grid);

        // Add the event label and the spin buttons
        grid.attach(&gtk::Label::new(Some(" Event: ")), 0, 0, 1, 1);
        grid.attach(&selection, 1, 0, 3, 1);
        grid.attach(&time_label, 0, 1, 1, 1);
        grid.attach(&minutes, 1, 1, 1, 1);
        grid.attach(&gtk::Label::new(Some(" Seconds ")), 2, 1, 1, 1);
        grid.attach(&seconds, 3, 1, 1, 1);
        grid.attach(&cancel_checkbox, 4, 1, 1, 1);

        // Add some space between the rows and columns
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        // Add some space on all the sides
        grid.set_margin_top(10);
        grid.set_margin_bottom(10);
        grid.set_margin_start(10);
        grid.set_margin_end(10);

        // Connect the close event for when the dialog is complete
        let timeline_events = self.timeline_events.clone();
        let system_send = self.system_send.clone();
        dialog.connect_response(clone!(selection, minutes, seconds, cancel_checkbox => move |modal, reply| {

            // Notify the system of the event change
            if reply == gtk::ResponseType::Ok {

                // Try to find the information about the events
                let events = match timeline_events.try_borrow() {
                    Ok(events) => events,
                    Err(_) => return, // give up if the event list couldn't be accessed
                };

                // Identify and forward the selected event
                if let Some(id) = selection.get_active_id() {

                    // If set to the all event
                    if id == "all" {
                        // Use the information to create the time adjustment
                        let adjustment = Duration::from_secs((minutes.get_value() as u64) * 60 + (seconds.get_value() as u64));

                        // Send an all event update to the system
                        system_send.send(AllEventChange {
                            adjustment,
                            is_negative: cancel_checkbox.get_active(),
                        });

                    // Look for the corresponding event
                    } else if let Some(event) = events.get(id.as_str()) {
                        // If the event was selected to be canceled
                        if cancel_checkbox.get_active() {
                            // Send an event update to the system
                            system_send.send(EventChange {
                                event_id: event.event.get_id(),
                                start_time: event.start_time.clone(),
                                new_delay: None,
                            });

                        // Otherwise
                        } else {
                            // Use that information to create the new duration
                            let mut new_delay = Duration::from_secs((minutes.get_value() as u64) * 60 + (seconds.get_value() as u64));
                            new_delay += event.start_time.elapsed();

                            // Send an event update to the system
                            system_send.send(EventChange {
                                event_id: event.event.get_id(),
                                start_time: event.start_time.clone(),
                                new_delay: Some(new_delay),
                            });
                        }
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
    pub duration: Duration,     // the current duration for the timeline
    pub font_size: u32,         // the current font size for the timeline
    pub label_limit: usize,     // the current character limit for timeline flags
    pub is_high_contrast: bool, // a flag for the display contrast mode
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
        timeline_title.set_markup("<span color='#338DD6' size='16000'>Timeline</span>");
        timeline_title.set_hexpand(true);
        timeline_title.set_halign(gtk::Align::Center);

        // Create a new set of timeline events
        let timeline_events = Rc::new(RefCell::new(FnvHashMap::default()));

        // Create a new instance of timeline info
        let timeline_info = Rc::new(RefCell::new(TimelineInfo {
            duration: Duration::from_secs(3600),
            font_size: NORMAL_FONT,
            label_limit: TIMELINE_LIMIT,
            is_high_contrast: false,
        }));

        // Create the duration buttons
        let duration_twohours = gtk::Button::new_with_label("2 Hrs");
        let duration_onehour = gtk::Button::new_with_label("60 Min");
        let duration_tenmins = gtk::Button::new_with_label("10 Min");
        let duration_onemin = gtk::Button::new_with_label("60 Secs");

        // Format the buttons
        duration_twohours.set_halign(gtk::Align::End);
        duration_onehour.set_halign(gtk::Align::End);
        duration_tenmins.set_halign(gtk::Align::End);
        duration_onemin.set_halign(gtk::Align::End);

        // Connect the clicked functions to change the timeline duration
        duration_twohours.connect_clicked(clone!(timeline_info => move |_| {
            if let Ok(mut info) = timeline_info.try_borrow_mut() {
                info.duration = Duration::from_secs(7200);
            }
        }));
        duration_onehour.connect_clicked(clone!(timeline_info => move |_| {
            if let Ok(mut info) = timeline_info.try_borrow_mut() {
                info.duration = Duration::from_secs(3600);
            }
        }));
        duration_tenmins.connect_clicked(clone!(timeline_info => move |_| {
            if let Ok(mut info) = timeline_info.try_borrow_mut() {
                info.duration = Duration::from_secs(600);
            }
        }));
        duration_onemin.connect_clicked(clone!(timeline_info => move |_| {
            if let Ok(mut info) = timeline_info.try_borrow_mut() {
                info.duration = Duration::from_secs(60);
            }
        }));

        // Create timeline drawing area
        let timeline_area = gtk::DrawingArea::new();
        timeline_area.show_all();

        // Format the area to be the correct size
        timeline_area.set_property_height_request(TIMELINE_HEIGHT as i32);
        timeline_area.set_hexpand(true);
        timeline_area.set_vexpand(false);
        timeline_area.set_halign(gtk::Align::Fill);

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
        timeline_area.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
        timeline_area.connect_button_press_event(clone!(window => move |area, press| {

            // Get the drawable width
            let width = area.get_allocation().width as f64;

            // Get the event position
            let (x, _) = press.get_position();

            // Try to get a copy of the timeline events
            let mut event_id = None;
            let events_clone = adjustment.timeline_events.clone();
            match events_clone.try_borrow_mut() {
                Err(_) => return Inhibit(false),

                // If the events were able to be extracted
                Ok(mut events) => {
                    // Check to see if the click overlaps with one of the events
                    for (_, event) in events.iter() {
                        if x > ((event.location * width) - (CLICK_PRECISION / 2.0)) && x < ((event.location * width) + (CLICK_PRECISION / 2.0)) {
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

                    // Check to see if the event is in focus
                    let mut launch = false;
                    if let Some(id) = event_id.clone() {
                        if let Some(mut event) = events.get_mut(&id) {
                            if event.in_focus {
                                // Launch the adjustment window
                                launch = true;

                            // Otherwise put it in focus and do not launch the window
                            } else {
                                event.in_focus = true;
                            }
                        }
                        // Make sure all the other events are not in focus
                        for (other_id, mut event) in events.iter_mut() {
                            if id != *other_id {
                                event.in_focus = false;
                            }
                        }

                    // If no event was found
                    } else {
                        // Make sure no events are in focus
                        for (_, mut event) in events.iter_mut() {
                            event.in_focus = false;
                        }

                        // Check to see if the location is near now
                        if x > ((NOW_LOCATION * width) - (CLICK_PRECISION / 2.0)) && x < ((NOW_LOCATION * width) + (CLICK_PRECISION / 2.0)) {
                            // Launch the window with no event
                            launch = true;
                        }
                    }

                    // Leave if we're not planning the lauch the window
                    if !launch {
                        return Inhibit(false);
                    }
                },
            }

            // Open the adjustment window
            adjustment.new_dialog(&window, event_id);

            // Allow the signal to propagate
            Inhibit(false)
        }));

        // Locate the different elements in the grid
        let grid = gtk::Grid::new();
        grid.set_column_spacing(10);
        grid.set_column_homogeneous(false);
        grid.set_row_homogeneous(false);
        grid.attach(&timeline_title, 0, 0, 1, 1);
        grid.attach(&duration_twohours, 1, 0, 1, 1);
        grid.attach(&duration_onehour, 2, 0, 1, 1);
        grid.attach(&duration_tenmins, 3, 0, 1, 1);
        grid.attach(&duration_onemin, 4, 0, 1, 1);
        grid.attach(&timeline_area, 0, 1, 5, 1);
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

    /// A method to update the timeline and the queue of coming events.
    ///
    pub fn update_events(&mut self, mut events: Vec<UpcomingEvent>) {
        // Try to get a mutable copy of timeline events
        let mut timeline_events = match self.timeline_events.try_borrow_mut() {
            Ok(events) => events,
            Err(_) => return,
        };

        // Copy the old timeline events, and clear the timeline
        let old_events = timeline_events.clone();
        *timeline_events = FnvHashMap::default();

        // Pass the events into the timeline events
        for event in events.drain(..) {
            // Convert each to a new timeline event
            let mut new_event = TimelineEvent::new(event);

            // Check to see if the event already existed in the timeline
            if let Some(existing) = old_events.get(&new_event.unique_id) {
                // Copy the focus
                new_event.in_focus = existing.in_focus;
            }

            // Add the new event to the timeline
            timeline_events.insert(new_event.unique_id.clone(), new_event);
        }

        // Tell the timeline area to redraw itself
        self.timeline_area.queue_draw();
    }

    /// A method to trigger a redraw of the timeline
    ///
    pub fn refresh(&self) {
        self.timeline_area.queue_draw();
    }

    /// A method to select the font size of the timeline
    ///
    pub fn select_font(&mut self, is_large: bool) {
        // Set the new font size
        let font_size = match is_large {
            false => NORMAL_FONT,
            true => LARGE_FONT,
        };

        // Set the new label character limit
        let label_limit = match is_large {
            false => TIMELINE_LIMIT,
            true => TIMELINE_LIMIT_SHORT,
        };

        // Try to get a mutable copy of timeline info
        if let Ok(mut info) = self.timeline_info.try_borrow_mut() {
            info.font_size = font_size;
            info.label_limit = label_limit;
        }
    }

    /// A method to select the color contrast of the timeline
    ///
    pub fn select_contrast(&mut self, is_hc: bool) {
        // Try to get a mutable copy of timeline info
        if let Ok(mut info) = self.timeline_info.try_borrow_mut() {
            info.is_high_contrast = is_hc;
        }
    }

    /// A function to draw the timeline with any events
    ///
    fn draw_timeline(
        timeline_events: Rc<RefCell<FnvHashMap<String, TimelineEvent>>>,
        timeline_info: Rc<RefCell<TimelineInfo>>,
        area: &gtk::DrawingArea,
        cr: &cairo::Context,
    ) -> Inhibit {
        // Draw the background dark grey
        cr.set_source_rgb(0.05, 0.05, 0.05);
        cr.paint();

        // Get and set the size of the window allocation
        let allocation = area.get_allocation();
        let (width, height) = (allocation.width as f64, allocation.height as f64); // create shortcuts
        cr.scale(width, height);

        // Try to get a copy of timeline info
        let info = match timeline_info.try_borrow() {
            Ok(info) => info,
            Err(_) => return Inhibit(false),
        };

        // Set the font size and ratio
        cr.set_font_matrix(cairo::Matrix {
            xx: ((info.font_size / 800) as f64 / width),
            yy: ((info.font_size / 800) as f64 / height),
            xy: 0.0,
            yx: 0.0,
            x0: 0.0,
            y0: 0.0,
        });

        // Draw the base line for the timeline
        cr.set_source_rgb(0.4, 0.4, 0.4);
        cr.set_line_width(2.0 / height); // 2 pixels wide
        cr.move_to(0.0, 0.75);
        cr.line_to(1.0, 0.75);
        cr.stroke();

        // Draw the current time line
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.set_line_width(2.0 / width); // 2 pixels wide
        cr.move_to(NOW_LOCATION, 0.0);
        cr.line_to(NOW_LOCATION, 1.0);
        cr.stroke();

        // Add the current time to the timeline
        let time = time::now();
        cr.move_to(NOW_LOCATION, 0.95);
        cr.show_text(
            format!(" {:02}:{:02}:{:02}", time.tm_hour, time.tm_min, time.tm_sec).as_str(),
        );

        // Calculate the time subdivisions
        let duration_secs = info.duration.as_secs();
        cr.set_source_rgb(0.5, 0.5, 0.5);
        cr.set_line_width(1.0 / width); // 1 pixel wide

        // Change the markers based on the duration
        let spacing;
        let increment;
        let label;
        // If the time is less than two minutes
        if duration_secs < 120 {
            // Use a 10 second marker
            spacing = (1.0 - NOW_LOCATION) / (duration_secs as f64 / 10.0);
            increment = 10.0;
            label = "sec";

        // If the time is between two and twenty minutes
        } else if duration_secs < 1200 {
            // Use a 1 minute marker
            spacing = (1.0 - NOW_LOCATION) / (duration_secs as f64 / 60.0);
            increment = 1.0;
            label = "min";

        // Otherwise, use a fixe minute marker
        } else {
            spacing = (1.0 - NOW_LOCATION) / (duration_secs as f64 / 300.0);
            increment = 5.0;
            label = "min";
        }

        // Draw divisions going to the right
        let mut count = 1.0;
        while (count * spacing) < 1.0 {
            // Calculate the offset
            let offset = NOW_LOCATION + (count * spacing);

            // Draw the subdivision
            cr.move_to(offset, 0.0);
            cr.line_to(offset, 1.0);
            cr.stroke();

            // Add a time label
            cr.move_to(offset, 0.95);
            cr.show_text(format!(" {} {}", count * increment, label).as_str());

            // Increment the count
            count = count + 1.0;
        }

        // Set default color and line width
        cr.set_source_rgb(0.9, 0.9, 0.9);
        cr.set_line_width(2.0 / width); // 2 pixels wide

        // Try to get a mutable copy of timeline events
        let mut events = match timeline_events.try_borrow_mut() {
            Ok(events) => events,
            Err(_) => return Inhibit(false),
        };

        // Locate all the events in the timeline
        let mut ordered_events = Vec::new();
        for (_, event) in events.iter_mut() {
            // Calculate the total time remaining for the event
            let mut location = 0.0; // default location is "now"
            if let Some(remaining) = event.remaining_precise() {
                // Calculate the location of the event on the timeline
                location = remaining
                    / (info.duration.as_secs() as f64
                        + (info.duration.subsec_nanos() as f64 / 1000000000.0));
            }

            // Correct for the now location
            event.location = (location * (1.0 - NOW_LOCATION)) + NOW_LOCATION;

            // Copy the event into the ordered list
            ordered_events.push(event.clone());
        }

        // Reorder the events to follow location, highest to lowest
        ordered_events.sort_by_key(|event| ((1.0 - event.location.clone()) * 1000.0) as u64);

        // Draw any events for the timeline
        for event in ordered_events.iter() {
            // Try to draw the event
            TimelineAbstraction::draw_event(
                cr,
                event,
                width,
                info.label_limit,
                info.is_high_contrast,
            );

            // Reset the color after the event
            cr.set_source_rgb(0.9, 0.9, 0.9);
        }

        // Allow the signal to propagate (probably not necessary)
        Inhibit(false)
    }

    /// A function to draw a new event on the timeline.
    ///
    /// If the event is in focus, this function also draws a flag to describe
    /// the event in brief detail
    ///
    fn draw_event(
        cr: &cairo::Context,
        event: &TimelineEvent,
        width: f64,
        label_limit: usize,
        is_high_contrast: bool,
    ) {
        // Extract the event location
        let location = event.location;

        // Check that the location is visible
        if (location < 0.0) | (location > 1.0) {
            return;
        }

        // Change the event color, visibility, and line width, if specified
        let mut line_width = 2.0; // 2 pixels wide

        // Skip if in high_contrast mode
        if !is_high_contrast {
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

                    // TODO Replace when let chains feature becomes stable
                    // If the highlight is specified
                    if let Some((red, green, blue)) = highlight {
                        // If there are under ten seconds remaining
                        if let Some((min, sec)) = event.remaining() {
                            // Flash the highlight color and line width
                            if (min == 0.0) & (sec < 15.0) & (sec as u32 % 2 == 0) {
                                cr.set_source_rgb(
                                    red as f64 / 255.0,
                                    green as f64 / 255.0,
                                    blue as f64 / 255.0,
                                );
                                line_width = HIGHLIGHT_WIDTH;
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

                    // TODO Replace when let chains feature becomes stable
                    // If the highlight is specified
                    if let Some((red, green, blue)) = highlight {
                        // If there are under ten seconds remaining
                        if let Some((min, sec)) = event.remaining() {
                            // Flash the highlight color and line width
                            if (min == 0.0) & (sec < 15.0) & (sec as u32 % 2 == 0) {
                                cr.set_source_rgb(
                                    red as f64 / 255.0,
                                    green as f64 / 255.0,
                                    blue as f64 / 255.0,
                                );
                                line_width = HIGHLIGHT_WIDTH;
                            }
                        }
                    }
                }

                // Catch the display debug variant
                DisplayDebug {
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

                    // TODO Replace when let chains feature becomes stable
                    // If the highlight is specified
                    if let Some((red, green, blue)) = highlight {
                        // If there are under ten seconds remaining
                        if let Some((min, sec)) = event.remaining() {
                            // Flash the highlight color and line width
                            if (min == 0.0) & (sec < 15.0) & (sec as u32 % 2 == 0) {
                                cr.set_source_rgb(
                                    red as f64 / 255.0,
                                    green as f64 / 255.0,
                                    blue as f64 / 255.0,
                                );
                                line_width = HIGHLIGHT_WIDTH;
                            }
                        }
                    }
                }

                // Catch the label control variant
                LabelControl {
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

                    // TODO Replace when let chains feature becomes stable
                    // If the highlight is specified
                    if let Some((red, green, blue)) = highlight {
                        // If there are under ten seconds remaining
                        if let Some((min, sec)) = event.remaining() {
                            // Flash the highlight color and line width
                            if (min == 0.0) & (sec < 15.0) & (sec as u32 % 2 == 0) {
                                cr.set_source_rgb(
                                    red as f64 / 255.0,
                                    green as f64 / 255.0,
                                    blue as f64 / 255.0,
                                );
                                line_width = HIGHLIGHT_WIDTH;
                            }
                        }
                    }
                }

                // Catch the label hidden variant
                LabelHidden {
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

                    // TODO Replace when let chains feature becomes stable
                    // If the highlight is specified
                    if let Some((red, green, blue)) = highlight {
                        // If there are under ten seconds remaining
                        if let Some((min, sec)) = event.remaining() {
                            // Flash the highlight color and line width
                            if (min == 0.0) & (sec < 15.0) & (sec as u32 % 2 == 0) {
                                cr.set_source_rgb(
                                    red as f64 / 255.0,
                                    green as f64 / 255.0,
                                    blue as f64 / 255.0,
                                );
                                line_width = HIGHLIGHT_WIDTH;
                            }
                        }
                    }
                }

                // If the event is hidden, change nothing
                _ => (),
            }
        }

        // Draw the vertical line
        cr.set_line_width(line_width / width);
        cr.move_to(location, 0.0);
        cr.line_to(location, 0.75);
        cr.stroke();

        // If the event is in focus
        if event.in_focus {
            // Draw the label flag to the right
            cr.set_line_width(1.0 / TIMELINE_HEIGHT); // 1 pixel
            cr.move_to(location, 0.01);
            cr.line_to(location + (LABEL_SIZE / width), 0.01);
            cr.stroke();
            cr.move_to(location, 0.72);
            cr.line_to(location + (LABEL_SIZE / width), 0.72);
            cr.stroke();
            cr.set_line_width(1.0 / width); // 1 pixel
            cr.move_to(location + (LABEL_SIZE / width), 0.0);
            cr.line_to(location + (LABEL_SIZE / width), 0.72);
            cr.stroke();

            // Draw the background of the flag
            cr.set_source_rgb(0.05, 0.05, 0.05);
            cr.set_line_width(0.7);
            cr.move_to(location + (2.0 / width), 0.36);
            cr.line_to(location + ((LABEL_SIZE - 2.0) / width), 0.36);
            cr.stroke();

            // Write the event text and the remaining time
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.move_to(location + (4.0 / width), 0.22);
            let text = clean_text(&event.event.description(), label_limit, false, false, false);
            cr.show_text(&text.as_str());

            // If there is a remaining time, add that as well
            if let Some((min, sec)) = event.remaining() {
                cr.move_to(location + (4.0 / width), 0.45);
                cr.show_text(format!("In {:02}:{:02}", min, sec).as_str());

                // Show what time the event will happen
                let now = time::now();
                let minute =
                    (((now.tm_sec as f64 + sec) / 60.0 + now.tm_min as f64 + min) % 60.0) as u64;
                let hour = (((now.tm_sec as f64 + sec) / 3600.0
                    + (now.tm_min as f64 + min) / 60.0
                    + now.tm_hour as f64)
                    % 24.0) as u64;
                cr.move_to(location + (4.0 / width), 0.68);
                cr.show_text(format!("At {:02}:{:02}", hour, minute).as_str());
            }
        }
    }
}
