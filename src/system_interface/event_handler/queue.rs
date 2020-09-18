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

//! A module to queue upcoming events. This module uses a background process
//! to have any number of future events triggered at once. The timing of this
//! module has been repaired since the original version and _should_ guarantee
//! that events with a longer delay always arrive later than earlier events.

// Import the relevant structures into the correct namespace
use super::super::GeneralUpdate;
use super::event::{EventDelay, EventUpdate};
use super::item::ItemId;

// Import other standard library features
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

// Import tokio features
use tokio::sync::mpsc;
use tokio::runtime::Handle;
use tokio::time;

/// A struct to allow easier manipulation of coming events.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ComingEvent {
    pub start_time: Instant, // the original start time of the event
    pub delay: Duration,     // delay between the start time and the trigger time for the event
    pub event_id: ItemId,    // id of the event to launch
}

// Implement the Coming Event features
impl ComingEvent {
    /// A function to return a new ComingEvent by consuming Duration and
    /// ItemId.
    ///
    pub fn new(delay: Duration, event_id: ItemId) -> ComingEvent {
        ComingEvent {
            start_time: Instant::now(),
            delay,
            event_id,
        }
    }

    /// A method to return a copy of the event id.
    ///
    pub fn id(&self) -> ItemId {
        self.event_id.clone()
    }

    /// A method to calculate the amount of time remaining before the event
    /// triggers. Returns None if the event should already have occured.
    ///
    pub fn remaining(&self) -> Option<Duration> {
        self.delay.checked_sub(self.start_time.elapsed())
    }

    /// A method to compare the start time and event id of two coming events.
    /// The method returns true iff both values are equal.
    ///
    pub fn compare_with(&self, other: &ComingEvent) -> bool {
        (self.event_id == other.event_id) & (self.start_time == other.start_time)
    }
}

/// An internal struct to hold the coming events and associated updates.
///
/// The most soon-to-trigger events (i.e. those with the smallest amount of time
/// remaining) are at the back of the list (closest to pop and push). The events
/// then increase in delay toward the front of the list. This results in slow
/// addition of new events but quick removal.
///
struct ComingEvents {
    list: Vec<ComingEvent>,        // a vector to hold the coming events
    general_update: GeneralUpdate, // the general update line for passing current events back to the rest of the system
}

// Implement key features for the coming events
impl ComingEvents {
    /// A function to create a new, empty ComingEvents structure.
    ///
    fn new(general_update: GeneralUpdate) -> ComingEvents {
        ComingEvents {
            list: Vec::new(),
            general_update,
        }
    }

    /// A method to update the rest of the system with the current events in
    /// the queue
    ///
    fn send_current(&self) {
        self.general_update.send_coming_events(self.list.clone());
    }

    /// A method to load an additional coming event.
    ///
    /// # Caution
    ///
    /// This method should only be called from within the background thread.
    /// Otherwise the thread may not process an event properly that has a shorter
    /// delay than existing events
    ///
    fn load_event(&mut self, event: ComingEvent) {
        // Calculate the remaining time before the event triggers
        if let Some(event_remaining) = event.remaining() {
            // Find the correct spot in the queue
            let mut index = 0;
            for coming in self.list.iter() {
                // Calculate the remaining time for this particular coming event
                if let Some(coming_remaining) = coming.remaining() {
                    // If event delay is larger than coming event, put new event in front
                    if event_remaining > coming_remaining {
                        break;
                    }
                }

                // Otherwise, increment
                index += 1;
            }

            // Load the event at the appropriate point in the queue
            self.list.insert(index, event);

        // If the event had no time left, put it at the back of the list
        } else {
            self.list.push(event);
        }

        // Update the system
        self.send_current();
    }

    /// A method to clear the events in the queue.
    ///
    fn clear(&mut self) {
        self.list = Vec::new(); // drop the old list
        self.send_current();
    }

    /// A method that returns a copy of the last coming event in the list,
    /// if it exists.
    ///
    fn last(&self) -> Option<ComingEvent> {
        match self.list.last() {
            Some(event) => Some(event.clone()),
            None => None,
        }
    }

    /// A method that removes the last event in the list if it matches the
    /// provided coming event. Returns the event if they match and None
    /// otherwise.
    ///
    fn pop_if(&mut self, test_event: &ComingEvent) -> Option<ComingEvent> {
        // If an event was found, compare it
        let mut result = false;
        if let Some(event) = self.list.last() {
            // Compare the id and the start time with the test event
            result = event.compare_with(test_event);
        }

        // If the event is correct, pop it from the list and notify the system
        if result {
            let tmp = self.list.pop(); // technically could be None, but isn't because of the logic above
            self.send_current();
            return tmp;
        }

        // Otherwise return None
        None
    }

    /// A method to determine the amount of time remaining before an event
    /// is triggered.
    ///
    /// # Errors
    ///
    /// If the requested event does not exist in the queue, this method will
    /// return None.
    ///
    fn remaining(&self, event_id: &ItemId) -> Option<Duration> {
        // Look through the list for a matching event
        for coming in self.list.iter().rev() {
            // If the event ids match
            if coming.event_id == *event_id {
                // Return the corresponding remaining duration
                return coming.remaining();
            }
        }

        // Otherwise, indicate the event wasn't found
        None
    }

    /// A method to remove the requested event from the list, change its delay
    /// to the provided Duration, return it to the caller.
    ///
    /// # Errors
    ///
    /// If the requested event does not exist in the queue, this method will
    /// return None.
    ///
    fn withdraw(&mut self, new_event: ComingEvent) -> Option<ComingEvent> {
        // Look for and remove the requested event (based on the drain_filter code)
        let mut index = 0;
        while index != self.list.len() {
            // If the event was found, remove it, and return the provided event
            if self.list[index].compare_with(&new_event) {
                // Remove the old event and update the flag
                self.list.remove(index);
                self.send_current();

                // Return the new event
                return Some(new_event);
            }

            // Otherwise, keep looking
            index += 1;
        }

        // If the event wasn't found, return None
        None
    }

    /// A method to remove any events that match the event id from the list.
    ///
    /// # Errors
    ///
    /// If the requested event id does not exist in the queue, this method will
    /// fail silently.
    ///
    fn cancel(&mut self, event_id: ItemId) {
        // Look for and remove any events that match the requested id
        let mut index = 0;
        while index != self.list.len() {
            // If the event was found, remove it, and return the provided event
            if self.list[index].event_id == event_id {
                // Remove the old event and update the flag
                self.list.remove(index);
                // Do not increment, as the index has now changed by one

                // Update the current events
                self.send_current();

            // Otherwise, keep looking
            } else {
                index += 1;
            }
        }
    }
}

/// A struct to hold a queue of future events. This struct launches a
/// separate daemon to preserve ordering of the events and minimize the spread
/// of unnecessary threads. This version preserves the proper order of the
/// events.
///
pub struct Queue {
    queue_load: mpsc::Sender<ComingEvent>, // the queue loading line that sends additional items to the daemon
    general_update: GeneralUpdate, // the general update line for passing current events back to the rest of the system
    coming_events: Arc<Mutex<ComingEvents>>, // the data queue to be modified by the background process and system handler process
}

// Implement the Queue methods
impl Queue {
    /// A function to create a new queue.
    ///
    /// This function returns a new queue which will send all triggered events
    /// back up the reply_line. The new implementation of the queue launches a
    /// background thread to monitor updates.
    ///
    pub fn new(handle: Handle, general_update: GeneralUpdate) -> Queue {
        // Create a new channel pair to send updates to the background queue
        let (queue_load, queue_receive) = mpsc::channel(128);

        // Create the new queue data
        let coming_events = Arc::new(Mutex::new(ComingEvents::new(general_update.clone())));
        let coming_clone = coming_events.clone();

        // Launch the background process with the queue data
        let general_clone = general_update.clone();
        handle.spawn(async {
            // Run the queue background process indefinitely
            Queue::run_loop(general_clone, queue_receive, coming_clone).await;
        });

        // Return the Queue
        Queue {
            queue_load,
            general_update,
            coming_events,
        }
    }

    /// An internal function to run the queue in an infinite loop. This function
    /// should be launched in a new background thread for the queue.
    ///
    async fn run_loop(
        general_update: GeneralUpdate,
        mut queue_receive: mpsc::Receiver<ComingEvent>,
        coming_events: Arc<Mutex<ComingEvents>>,
    ) {
        // Run the background process indefinitely
        loop {
            // Check for the next coming event
            let next_event = coming_events.lock().unwrap().last();
            match next_event {
                // If there isn't a coming event
                None => {
                    // Wait indefinitely for new events on the queue receive line
                    match queue_receive.recv().await {
                        // Process an upcoming event
                        Some(event) => {
                            coming_events.lock().unwrap().load_event(event);
                        }

                        // Terminate the process if there was an error
                        _ => break,
                    }
                }

                // Otherwise, wait for this event or a new event on the line
                Some(event) => {
                    // Look to see how much time is remaining on the newest event
                    match event.remaining() {
                        // If there is no time remaining, launch the event
                        None => {
                            // Remove the last event from the list and send it if it matches what we expected. Otherwise, do nothing.
                            if let Some(event_now) = coming_events.lock().unwrap().pop_if(&event) {
                                general_update.send_event(event_now.id(), true, true);
                            }
                        }

                        // If there is some time remaining, wait for a message to arrive or the time to pass
                        Some(delay) => {
                            // Create the new async delay object
                            let mut async_delay = time::delay_for(delay);
                            
                            // Act on the first to return
                            tokio::select! {
                                // If an event is received before the delay expires
                                Some(new_event) = queue_receive.recv() => {
                                    // Process the new upcoming event
                                    coming_events.lock().unwrap().load_event(new_event);
                                }
                                
                                // If the delay expires instead
                                _ = &mut async_delay => {
                                    // Remove the last event from the list
                                    if let Some(event_now) = coming_events.lock().unwrap().pop_if(&event) {
                                        // Send it if it matches what we expected.
                                        general_update.send_event(event_now.id(), true, true);
                                    }
                                    // Otherwise, do nothing.
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// A method to add a new event to the queue.
    ///
    /// This function adds the new event to the existing queue. This event may
    /// preceed existing events in the queue.
    ///
    pub async fn add_event(&mut self, event: EventDelay) {
        // Sort between delayed events and static events
        match event.delay() {
            // Load delayed events into the queue
            Some(delay) => {
                // Create a coming event and send it to the queue
                let coming = ComingEvent::new(delay, event.id());
                self.queue_load.send(coming).await.unwrap_or(());
            }

            // Immediately return any events that have no delay
            None => self.general_update.send_event(event.id(), true, true).await,
        }
    }

    /// A method to check the remaining time until an event is triggered. If
    /// multiple events with the same id are in the queue, the remaining time
    /// until the earliest event (the one with the shortest delay) is provided.
    ///
    /// # Errors
    ///
    /// If the provided event id does not exist in the queue, this method will
    /// fail silently.
    ///
    /// # Note
    ///
    /// While unlikely, this function must wait for the background process to
    /// release the lock on the queue. If the background process hangs, this
    /// function may hang as well.
    ///
    pub async fn event_remaining(&self, event_id: &ItemId) -> Option<Duration> {
        // Open the coming events
        match self.coming_events.lock() {
            Ok(events) => {
                // Try to get the delay of the provided event id
                return events.remaining(event_id);
            }

            // Raise an error if the queue has failed
            _ => {
                update!(err &self.general_update => "Internal Failure Of The Event Queue.");
                return None;
            }
        }
    }

    /// A method to adjust the remaining delay in a specific upcoming event. If
    /// this new delay is longer than the amount of time remaining before the
    /// event would trigger, the event will trigger immediately and be removed
    /// from the queue.
    ///
    /// # Errors
    ///
    /// If the provided event id does not exist in the queue, this method will
    /// fail silently.
    ///
    /// # Note
    ///
    /// While unlikely, this function must wait for the background process to
    /// release the lock on the queue. If the background process hangs, this
    /// function may hang as well.
    ///
    pub async fn adjust_event(&mut self, new_event: ComingEvent) {
        // Open the coming events
        match self.coming_events.lock() {
            Ok(mut events) => {
                // Try to withdraw the existing event from the queue
                if let Some(event) = events.withdraw(new_event) {
                    // If successful, send the new event to the queue. This also triggers the queue to notice the change.
                    self.queue_load.send(event).await.unwrap_or(());
                }
            }

            // Raise an error if the queue has failed
            _ => {
                update!(err &self.general_update => "Internal Failure Of The Event Queue.");
            }
        }
    }

    /// A method to adjust the remaining delay for all the events in the queue.
    ///
    /// # Notes
    ///
    /// This method will drop any events that should have happened in the past.
    /// In other words, if is_negative is true and the adjustment is longer
    /// than the last event in the queue, this function is equivalent to
    /// clearing the queue (none of the events will be triggered).
    ///
    /// While unlikely, this function must wait for the background process to
    /// release the lock on the queue. If the background process hangs, this
    /// function may hang as well.
    ///
    pub async fn adjust_all(&mut self, adjustment: Duration, is_negative: bool) {
        // Open the coming events
        match self.coming_events.lock() {
            Ok(mut events) => {
                // If the adjustment is positive
                if !is_negative {
                    // Add time to all the events
                    for event in events.list.iter() {
                        // Load the new event into the Queue
                        self.queue_load
                            .send(ComingEvent {
                                start_time: event.start_time.clone(),
                                delay: event.delay + adjustment,
                                event_id: event.id(),
                            })
                            .await.unwrap_or(());
                    }

                // Otherwise, try to subtract time from the events
                } else {
                    // Try to subtract time from all the events
                    for event in events.list.iter() {
                        // Ignore events that have already happened
                        let remaining = match event.remaining() {
                            Some(time) => time,
                            None => continue,
                        };

                        // Calculate the new delay
                        match remaining.checked_sub(adjustment) {
                            // Drop the event if not enough time left
                            None => continue,
                            Some(_) => {
                                // Calculate the new duration (should always succeed)
                                if let Some(delay) = event.delay.checked_sub(adjustment) {
                                    // Load the new event into the Queue
                                    self.queue_load
                                        .send(ComingEvent {
                                            start_time: event.start_time.clone(),
                                            delay,
                                            event_id: event.id(),
                                        })
                                        .await.unwrap_or(());
                                }
                            }
                        }
                    }
                }

                // Clear the coming events (will be reloaded by the background process)
                events.clear();
            }

            // Raise an error if the queue has failed
            _ => {
                update!(err &self.general_update => "Internal Failure Of The Event Queue.");
            }
        }
    }

    /// A method to cancel a specific upcoming event.
    ///
    /// # Errors
    ///
    /// If the provided event id does not exist in the queue, this method will
    /// fail silently.
    ///
    /// # Note
    ///
    /// While unlikely, this function must wait for the background process to
    /// release the lock on the queue. If the background process hangs, this
    /// function may hang as well.
    ///
    pub async fn cancel_event(&self, new_event: ComingEvent) {
        // Open the coming events
        match self.coming_events.lock() {
            Ok(mut events) => {
                // Try to withdraw the existing event from the queue
                events.withdraw(new_event); // Queue will automatically detect the change
            }

            // Raise an error if the queue has failed
            _ => {
                update!(err &self.general_update => "Internal Failure Of The Event Queue.");
            }
        }
    }

    /// A method to cancel all upcoming instances of an event.
    ///
    /// # Errors
    ///
    /// If the provided event id does not exist in the queue, this method will
    /// fail silently.
    ///
    /// # Note
    ///
    /// While unlikely, this function must wait for the background process to
    /// release the lock on the queue. If the background process hangs, this
    /// function may hang as well.
    ///
    pub async fn cancel_all(&self, event_id: ItemId) {
        // Open the coming events
        match self.coming_events.lock() {
            Ok(mut events) => {
                // Cancel any matching events in the queue
                events.cancel(event_id); // Queue will automatically detect the change
            }

            // Raise an error if the queue has failed
            _ => {
                update!(err &self.general_update => "Internal Failure Of The Event Queue.");
            }
        }
    }

    /// A method to clear any events in the queue.
    ///
    /// # Note
    ///
    /// While unlikely, this function must wait for the background process to
    /// release the lock on the queue. If the background process hangs, this
    /// function may hang as well.
    ///
    pub async fn clear(&self) {
        // Open the coming events
        match self.coming_events.lock() {
            Ok(mut events) => events.clear(),

            // Raise an error if the queue has failed
            _ => {
                update!(err &self.general_update => "Internal Failure Of The Event Queue.");
            }
        }
    }
}

// Tests of the queue module
#[cfg(test)]
mod tests {
    use super::*;

    // Simple test of running the queue module
    #[test]
    fn run_queue() {
        // Import libraries for testing
        use super::super::super::GeneralUpdate;
        use super::super::super::GeneralUpdateType;
        use std::time::Duration;

        // Create a channel for receiving messages from the queue
        let (tx, rx) = GeneralUpdate::new();

        // Create a new message queue
        let queue = Queue::new(tx);

        // Load some events into the queue
        queue.add_event(EventDelay::new(
            Some(Duration::from_millis(20)),
            ItemId::new(20).unwrap(),
        ));
        queue.add_event(EventDelay::new(
            Some(Duration::from_millis(60)),
            ItemId::new(60).unwrap(),
        ));
        queue.add_event(EventDelay::new(
            Some(Duration::from_millis(40)),
            ItemId::new(40).unwrap(),
        ));
        queue.add_event(EventDelay::new(
            Some(Duration::from_millis(100)),
            ItemId::new(100).unwrap(),
        ));
        queue.add_event(EventDelay::new(
            Some(Duration::from_millis(80)),
            ItemId::new(80).unwrap(),
        ));

        // Create the test vector
        let test = vec![
            GeneralUpdateType::Event(ItemId::new_unchecked(20)),
            GeneralUpdateType::Event(ItemId::new_unchecked(40)),
            GeneralUpdateType::Event(ItemId::new_unchecked(60)),
            GeneralUpdateType::Event(ItemId::new_unchecked(80)),
            GeneralUpdateType::Event(ItemId::new_unchecked(100)),
        ];

        // Print and check the messages received (wait at most half a second)
        test_vec!(=rx, test);
    }
}
