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

//! A module to create a macro and several functions that simplify the steps of
//! creating and updating the user interface.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::cell::RefCell;
use std::rc::Rc;
use std::u32::MAX as U32_MAX;

// Import GTK library
use glib;
use gtk;
use gtk::prelude::*;

// Define module constants
const FLASH_RATE: u32 = 700;

/// A macro to make moving clones into closures more convenient
///
/// This macro allows the user to easily and quickly clone any items in a
/// closure that would normally have to be manually cloned. This makes the
/// functions for individual connections much more elegant.
///
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

/// A function to clean any text provided by the user.
//
/// This function cleans any user- or system- provided text (the built-in
/// function with GTK+ glib::markup_escape_texts is not available in the
/// gtk-rs library). This function also truncates the text at the provided
/// max length to prevent the notifications from overflowing the available
/// space. Additional text (beyond max length) will be pushed to the next
/// line with an indentation if indent is true.
///
pub fn clean_text(
    raw_text: &str,
    max_length: usize,
    newline: bool,
    indent: bool,
    clean: bool,
) -> String {
    // If simply truncating withtout a newline
    let mut clean_text = String::new();
    if !newline {
        // Just copy the raw chars to the clean_text
        for (i, character) in raw_text.chars().enumerate() {
            // Stop at the maximum length
            if i > max_length {
                break;
            }

            // Otherwise add it to the clean text
            clean_text.push(character);
        }

    // Otherwise, prevent splitting words whenever possible
    } else {
        // Divide the new text into sections of whitespace
        let mut lines = 1;
        for word in raw_text.split_whitespace() {
            // If the size of the word exceeds our allotment, truncate it
            let size = word.chars().count();
            let count = clean_text.chars().count();
            if size > max_length {
                // Truncate the string at the first viable location
                let mut index = (max_length * lines) - count;
                loop {
                    // Only cut at a valid character boundary
                    match word.get(..index) {
                        // Add the truncated word
                        Some(truncated) => {
                            clean_text.push_str(truncated);
                            clean_text.push_str("\n... ");

                            // Increment the line count before moving on
                            lines += 1;
                            break;
                        }

                        // Try again with a longer word
                        None => index += 1,
                    }
                }

            // If the end of the word would be beyond our allotment, add a newline
            } else if (size + count) > (max_length * lines) {
                // Add the newline, tab if requested, and the word
                if indent {
                    clean_text.push_str("\n\t\t");
                } else {
                    clean_text.push('\n');
                }
                clean_text.push_str(word);
                clean_text.push(' ');

                // Increment the line count
                lines += 1;

            // Otherwise, just add the word and a space
            } else {
                clean_text.push_str(word);
                clean_text.push(' ');
            }
        }
    }

    // Return now if not cleaning
    if !clean {
        return clean_text;
    }

    // Replace any of the offending characters
    let mut final_text = String::new();
    for character in clean_text.chars() {
        // Catch and replace dangerous characters
        let mut safe_character: &str = &format!("{}", character);
        match character {
            // Replace the ampersand
            '&' => safe_character = "&amp;",

            // Replace the less than
            '<' => safe_character = "&lt;",

            // Replace the greater than
            '>' => safe_character = "&gt;",

            // Replace null character with nothing
            '\0' => safe_character = "",

            // Pass along any other characters
            _ => (),
        }

        // Add it to the final text
        final_text.push_str(safe_character);
    }
    final_text
}

/// A helper function to properly decorate a label. The function
/// sets the markup for the existing label and returns the position from
/// the DisplayType, if it exists.
///
/// This function assumes that the text has already been cleaned and sized.
///
pub fn decorate_label(
    label: &gtk::Label,
    text: &str,
    display: DisplayType,
    full_status: &FullStatus,
    font_size: u32,
    high_contrast: bool,
    spotlight_expiration: Option<Rc<RefCell<u32>>>,
) -> Option<u32> {
    // Decorate based on the display type
    match display {
        // Match the display control variant
        DisplayControl {
            color,
            highlight,
            highlight_state,
            spotlight,
            position,
        }
        | DisplayWith {
            color,
            highlight,
            highlight_state,
            spotlight,
            position,
            ..
        }
        | DisplayDebug {
            color,
            highlight,
            highlight_state,
            spotlight,
            position,
            ..
        }
        | LabelControl {
            color,
            highlight,
            highlight_state,
            spotlight,
            position,
        }
        | LabelHidden {
            color,
            highlight,
            highlight_state,
            spotlight,
            position,
        } => {
            // Define the default markup
            let mut markup = format!("<span size='{}'>{}</span>", font_size, text);

            // If high contrast mode, just set the size and return the position
            if high_contrast {
                label.set_markup(&markup);
                return position;
            }

            // Set the markup color, if specified
            if let Some((red, green, blue)) = color {
                markup = format!(
                    "<span color='#{:02X}{:02X}{:02X}' size='{}'>{}</span>",
                    red, green, blue, font_size, text
                );
            }
            // Set the markup (using the default above if no color was specified)
            label.set_markup(&markup);

            // Set the highlight color, if specified (overrides default color)
            if let Some((red, green, blue)) = highlight {
                // Create the highlight markup
                let highlight_markup = format!(
                    "<span color='#{:02X}{:02X}{:02X}' size='{}'>{}</span>",
                    red, green, blue, font_size, text
                );

                // Check to see if the highlight state is specified
                if let Some((status_id, state_id)) = highlight_state {
                    // Find the corresponding status
                    if let Some(&StatusDescription { ref current, .. }) = full_status.get(
                        &ItemPair::from_item(status_id, ItemDescription::new("", Hidden)),
                    ) {
                        // If the current id matches the state id
                        if state_id == current.get_id() {
                            // Set the label to the highlight color
                            label.set_markup(&highlight_markup);
                        }
                    }
                }

                // Set the spotlight color, if specified (overrides highlight color)
                if let Some(count) = spotlight {
                    // Ignore this option if spotlight is not relevant for this label
                    if let Some(expiration) = spotlight_expiration {
                        // Change the count on the expiration if it is u32::MAX
                        if let Ok(mut current) = expiration.try_borrow_mut() {
                            if *current == U32_MAX {
                                *current = count * 2; // once each for on and off
                            }
                        }

                        // Launch a recurring message
                        let spotlight_update = clone!(label, markup, highlight_markup, expiration => move || {
                            spotlight_label(label.clone(), markup.clone(), highlight_markup.clone(), expiration.clone())
                        });
                        glib::timeout_add_local(FLASH_RATE, spotlight_update);
                    }
                }
            }

            // Return the position
            return position;
        }

        // Otherwise, use the default color and position
        Hidden => {
            label.set_markup(&format!("<span size='{}'>{}</span>", font_size, text));
            return None;
        }
    }
}

/// A helper function to properly color label for editing purposes. The function
/// sets the markup for the existing label and returns the position from
/// the DisplayType, if it exists.
///
/// This function assumes that the text has already been cleaned and sized.
///
pub fn color_label(label: &gtk::Label, text: &str, display: DisplayType, font_size: u32) {
    // Decorate based on the display type
    match display {
        // Match the display control variant
        DisplayControl { color, .. }
        | DisplayWith { color, .. }
        | DisplayDebug { color, .. }
        | LabelControl { color, .. }
        | LabelHidden { color, .. } => {
            // Define the default markup
            let mut markup = format!("<span size='{}'>{}</span>", font_size, text);

            // Set the markup color, if specified
            if let Some((red, green, blue)) = color {
                markup = format!(
                    "<span color='#{:02X}{:02X}{:02X}' size='{}'>{}</span>",
                    red, green, blue, font_size, text
                );
            }
            // Set the markup (using the default above if no color was specified)
            label.set_markup(&markup);
        }

        // Otherwise, use the default color and position
        Hidden => {
            label.set_markup(&format!("<span size='{}'>{}</span>", font_size, text));
        }
    }
}

/// A private helper function to properly spotlight the highlight color on a label
/// until the provided expiration is complete. The function sets the markup for
/// the existing label to or from the highlight color until expiration equals
/// one. If expiration equals zero or u32::MAX, the function will return true
/// indefinitely.
///
/// This function assumes that the two provided label markups have been prepared.
///
fn spotlight_label(
    label: gtk::Label,
    default_markup: String,
    highlight_markup: String,
    expiration: Rc<RefCell<u32>>,
) -> Continue {
    // Make sure the label is still visible
    if !label.is_visible() {
        return Continue(false);
    }

    // Try to extract the expiration count
    if let Ok(mut count) = expiration.try_borrow_mut() {
        // Act based on the count of the expiration
        match *count {
            // If the count is zero, set it to u32::Max
            0 => {
                *count = U32_MAX;

                // Set the label to the default markup
                label.set_markup(&default_markup);

                // Return true
                return Continue(true);
            }

            // If the count is u32::MAX, set it back to zero
            U32_MAX => {
                *count = 0;

                // Set the label to the highlight markup
                label.set_markup(&highlight_markup);

                // Return true
                return Continue(true);
            }

            // If the count is one, return false
            1 => {
                // Set the label to the default markup
                label.set_markup(&default_markup);

                // Return true
                return Continue(false);
            }

            // If the count is any other number, check for even/odd
            _ => {
                // Decrease the count
                *count = *count - 1;

                // if count is even
                if (*count % 2) == 0 {
                    // Set the label to the default markup
                    label.set_markup(&default_markup);

                    // Return true
                    return Continue(true);

                // Otherwise
                } else {
                    // Set the label to the highlight markup
                    label.set_markup(&highlight_markup);

                    // Return true
                    return Continue(true);
                }
            }
        }
    }

    // Stop the closure on failure
    Continue(false)
}

/// A macro that allows the user to set a widget as a drag source,
/// or a drag destination
///
macro_rules! drag {
    // Set a widget as a drag source
    (source $widget:expr) => {{
        $widget.drag_source_set(
            gdk::ModifierType::MODIFIER_MASK,
            &vec![gtk::TargetEntry::new(
                "STRING",
                gtk::TargetFlags::SAME_APP,
                10,
            )],
            gdk::DragAction::COPY,
        );
    }};

    // Set a widget as a drag destination
    (dest $widget:expr) => {{
        $widget.drag_dest_set(
            gtk::DestDefaults::ALL,
            &vec![gtk::TargetEntry::new(
                "STRING",
                gtk::TargetFlags::SAME_APP,
                10,
            )],
            gdk::DragAction::COPY,
        );
    }};
}
