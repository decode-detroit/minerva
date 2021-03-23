// Copyright (c) 2017-2021 Decode Detroit
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

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::time::Duration;
use std::sync::{Arc, Mutex};

// Import tokio features
use tokio::sync::mpsc;
use tokio::runtime::Handle;
use tokio::time::sleep;

/// An internal struct to hold the coming events and associated updates.
///
/// The most soon-to-trigger events (i.e. those with the smallest amount of time
/// remaining) are at the back of the list (closest to pop and push). The events
/// then increase in delay toward the front of the list. This results in slow
/// addition of new events but quick removal.
///
#[derive(Clone)]
struct ComingEvents {
    list: Arc<Mutex<Vec<ComingEvent>>>, // a threadsafe vector to hold the coming events
    interface_send: InternalSend, // the general update line for passing current events back to the rest of the system
}

// Implement key features for the coming events
impl ComingEvents {
    /// A function to create a new, empty ComingEvents structure.
    ///
    fn new(interface_send: InternalSend) -> ComingEvents {
        ComingEvents {
            list: Arc::new(Mutex::new(Vec::new())),
            interface_send,
        }
    }

    /// A method to update the rest of the system with the current events in
    /// the queue
    ///
    async fn send_current(&self) {
        // Make a copy of the list and send it
        let list = match self.list.lock() {
            Ok(list) => list.clone(),
            _ => Vec::new(), // inelegant failure handling
        };
        self.interface_send.send_coming_events(list).await;
    }

    /// A method to load an additional coming event.
    ///
    /// # Caution
    ///
    /// This method should only be called from within the background thread.
    /// Otherwise the thread may not process an event properly that has a shorter
    /// delay than existing events
    ///
    async fn load_event(&mut self, event: ComingEvent) {
        // Get access to the list
        if let Ok(mut list) = self.list.lock() {
            // Calculate the remaining time before the event triggers
            if let Some(event_remaining) = event.remaining() {
                // Find the correct spot in the queue
                let mut index = 0;
                for coming in list.iter() {
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
                list.insert(index, event);

            // If the event had no time left, put it at the back of the list
            } else {
                list.push(event);
            }
        }

        // Update the system
        self.send_current().await;
    }

    /// A method to clear the events in the queue.
    ///
    async fn clear(&mut self) {
        // Get access to the list
        if let Ok(mut list) = self.list.lock() {
            *list = Vec::new();
        }
        
        // Send the update
        self.send_current().await;
    }

    /// A method that returns a copy of the last coming event in the list,
    /// if it exists.
    ///
    fn last(&self) -> Option<ComingEvent> {
        // Get access to the list
        if let Ok(list) = self.list.lock() {
            // Return the last entry
            return match list.last() {
                Some(event) => Some(event.clone()),
                None => None,
            }
        }
        None
    }

    /// A method that removes the last event in the list if it matches the
    /// provided coming event. Returns the event if they match and None
    /// otherwise.
    ///
    async fn pop_if(&mut self, test_event: &ComingEvent) -> Option<ComingEvent> {
        // If an event was found, compare it
        let mut result = false;
        if let Some(event) = self.last() {
            // Compare the id and the start time with the test event
            result = event.compare_with(test_event);
        }

        // If the event is correct, pop it from the list and notify the system
        if result {
            let tmp = match self.list.lock() {
                Ok(mut list) => list.pop(), // technically could be None, but isn't because of the logic above
                _ => None,
            };
            
            // Send the update
            self.send_current().await;
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
        // Get access to the list
        if let Ok(list) = self.list.lock() {
            // Look through the list for a matching event
            for coming in list.iter().rev() {
                // If the event ids match
                if coming.event_id == *event_id {
                    // Return the corresponding remaining duration
                    return coming.remaining();
                }
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
    async fn withdraw(&mut self, new_event: ComingEvent) -> Option<ComingEvent> {
        // Get access to the list
        if let Ok(mut list) = self.list.lock() {
            // Look for and remove the requested event (based on the drain_filter code)
            let mut index = 0;
            while index != list.len() {
                // If the event was found, 
                if list[index].compare_with(&new_event) {
                    // Break at this index point
                    break;
                }

                // Otherwise, keep looking
                index += 1;
            }
            
            // If the event wasn't found, return None
            if index == list.len() {
                return None;
            }
            
            // Otherwise, remove the event from the list
            list.remove(index);
        }
        
        // Send the update
        self.send_current().await;

        // Return the new event
        return Some(new_event);
    }

    /// A method to remove any events that match the event id from the list.
    ///
    /// # Errors
    ///
    /// If the requested event id does not exist in the queue, this method will
    /// fail silently.
    ///
    async fn cancel(&mut self, event_id: ItemId) {
        // Get access to the list
        let mut is_changed = false;
        if let Ok(mut list) = self.list.lock() {
            // Look for and remove any events that match the requested id
            let mut index = 0;
            while index != list.len() {
                // If the event was found, remove it, and return the provided event
                if list[index].event_id == event_id {
                    // Remove the old event and update the flag
                    list.remove(index);
                    // Do not increment, as the index has now changed by one

                    // Note the change
                    is_changed = true;

                // Otherwise, keep looking
                } else {
                    index += 1;
                }
            }
        }
        
        // If changed, update the current events
        if is_changed {
            self.send_current().await;
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
    interface_send: InternalSend, // the general update line for passing current events back to the rest of the system
    coming_events: ComingEvents, // the data queue to be modified by the background process and system handler process
}

// Implement the Queue methods
impl Queue {
    /// A function to create a new queue.
    ///
    /// This function returns a new queue which will send all triggered events
    /// back up the reply_line. The new implementation of the queue launches a
    /// background thread to monitor updates.
    ///
    pub fn new(interface_send: InternalSend) -> Queue {
        // Create a new channel pair to send updates to the background queue
        let (queue_load, queue_receive) = mpsc::channel(128);

        // Create the new queue data
        let coming_events = ComingEvents::new(interface_send.clone());
        let coming_clone = coming_events.clone();

        // Launch the background process with the queue data
        let general_clone = interface_send.clone();
        Handle::current().spawn(async {
            // Run the queue background process indefinitely
            Queue::run_loop(general_clone, queue_receive, coming_clone).await;
        });

        // Return the Queue
        Queue {
            queue_load,
            interface_send,
            coming_events,
        }
    }

    /// An internal function to run the queue in an infinite loop. This function
    /// should be launched in a new background thread for the queue.
    ///
    async fn run_loop(
        interface_send: InternalSend,
        mut queue_receive: mpsc::Receiver<ComingEvent>,
        mut coming_events: ComingEvents,
    ) {
        // Run the background process indefinitely
        loop {
            // Check for the next coming event
            let next_event = coming_events.last();
            match next_event {
                // If there isn't a coming event
                None => {
                    // Wait indefinitely for new events on the queue receive line
                    match queue_receive.recv().await {
                        // Process an upcoming event
                        Some(event) => {
                            coming_events.load_event(event).await;
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
                            // Remove the last event from the list
                            let last_event = coming_events.pop_if(&event).await;
                            
                            // Send it if it matches what we expected. Otherwise, do nothing.
                            if let Some(event_now) = last_event {
                                interface_send.send_event(event_now.id(), true, true).await;
                            }
                        }

                        // If there is some time remaining, wait for a message to arrive or the time to pass
                        Some(delay) => {
                            // Create the new sleep
                            let sleep = sleep(delay);
                            
                            // Act on the first to return
                            tokio::select! {
                                // If an event is received before the delay expires
                                Some(new_event) = queue_receive.recv() => {
                                    // Process the new upcoming event
                                    coming_events.load_event(new_event).await;
                                }
                                
                                // If the delay expires instead
                                _ = sleep => {
                                    // Remove the last event from the list
                                    let last_event = coming_events.pop_if(&event).await;
                                    
                                    // Send it if it matches what we expected. Otherwise, do nothing.
                                    if let Some(event_now) = last_event {
                                        interface_send.send_event(event_now.id(), true, true).await;
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
            None => self.interface_send.send_event(event.id(), true, true).await,
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
        // Try to get the delay of the provided event id
        self.coming_events.remaining(event_id)
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
        // Try to open the coming events
        let possible_event = self.coming_events.withdraw(new_event).await;

        // Check to see if the operation was successful
        if let Some(event) = possible_event {
            // If successful, send the new event to the queue. This also triggers the queue to notice the change.
            self.queue_load.send(event).await.unwrap_or(());
        } // fail silently
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
        // Try to get a copy of the coming events
        let possible_events: Option<Vec<ComingEvent>> = match self.coming_events.list.lock() {
            // Remove all the events from the list and return them
            Ok(mut list) => Some(list.drain(..).collect()),
            _ => None,
        };
        
        // If collecting the events was successful
        if let Some(mut events) = possible_events {
            // If the adjustment is positive
            if !is_negative {
                // Add time to all the events
                for event in events.drain(..) {
                    // Load the new event into the Queue
                    self.queue_load
                        .send(ComingEvent {
                            start_time: event.start_time,
                            delay: event.delay + adjustment,
                            event_id: event.id(),
                        })
                        .await.unwrap_or(());
                }

            // Otherwise, try to subtract time from the events
            } else {
                // Try to subtract time from all the events
                for event in events.drain(..) {
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
                                        start_time: event.start_time,
                                        delay,
                                        event_id: event.id(),
                                    })
                                    .await.unwrap_or(());
                            }
                        }
                    }
                }
            }
        
        // Otherwise, raise an error that the queue has failed
        } else {
            update!(err &self.interface_send => "Internal Failure Of The Event Queue.");
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
    pub async fn cancel_event(&mut self, new_event: ComingEvent) {
        // Try to withdraw the existing event from the queue
        self.coming_events.withdraw(new_event).await; // Queue will automatically detect the change
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
    pub async fn cancel_all(&mut self, event_id: ItemId) {
        // Cancel any matching events in the queue
        self.coming_events.cancel(event_id).await; // Queue will automatically detect the change
    }

    /// A method to clear any events in the queue.
    ///
    /// # Note
    ///
    /// While unlikely, this function must wait for the background process to
    /// release the lock on the queue. If the background process hangs, this
    /// function may hang as well.
    ///
    pub async fn clear(&mut self) {
        // Clear all events in the queue
        self.coming_events.clear().await;
    }
}

// Tests of the queue module
#[cfg(test)]
mod tests {
    use super::*;

    // Simple test of running the queue module
    #[tokio::test]
    async fn queue_events() {
        // Import libraries for testing
        use crate::definitions::{InternalSend, InternalUpdate};
        use std::time::Duration;
        use tokio::time::sleep;

        // Create a channel for receiving messages from the queue
        let (tx, mut rx) = InternalSend::new();

        // Create a new message queue
        let mut queue = Queue::new(tx);

        // Load some events into the queue
        queue.add_event(EventDelay::new(
            Some(Duration::from_millis(20)),
            ItemId::new(20).unwrap(),
        )).await;
        queue.add_event(EventDelay::new(
            Some(Duration::from_millis(60)),
            ItemId::new(60).unwrap(),
        )).await;
        queue.add_event(EventDelay::new(
            Some(Duration::from_millis(40)),
            ItemId::new(40).unwrap(),
        )).await;
        queue.add_event(EventDelay::new(
            Some(Duration::from_millis(100)),
            ItemId::new(100).unwrap(),
        )).await;
        queue.add_event(EventDelay::new(
            Some(Duration::from_millis(80)),
            ItemId::new(80).unwrap(),
        )).await;

        // Create the test vector
        let reference = vec![
            InternalUpdate::ProcessEvent{ event: ItemId::new_unchecked(20), check_scene: true, broadcast: true },
            InternalUpdate::ProcessEvent { event: ItemId::new_unchecked(40), check_scene: true, broadcast: true },
            InternalUpdate::ProcessEvent { event: ItemId::new_unchecked(60), check_scene: true, broadcast: true },
            InternalUpdate::ProcessEvent { event: ItemId::new_unchecked(80), check_scene: true, broadcast: true },
            InternalUpdate::ProcessEvent { event: ItemId::new_unchecked(100), check_scene: true, broadcast: true },
        ];

        // Print and check the messages received (wait at most half a second)
        let mut received = Vec::new();
        loop {
            tokio::select! {
                // Try to find the test updates
                Some(update) = rx.recv() => {
                    // Log any new addition that is ProcessEvent type
                    if let &InternalUpdate::ProcessEvent { .. } = &update {
                        received.push(update);
                    }
                    
                    // Check if the received vector matches the test vector
                    if reference == received {
                        return;
                    }
                }

                // Only wait half a second
                _ = sleep(Duration::from_millis(500)) => {
                    break;
                }
            }
        }

        // Print debugging help if the script failed
        println!(
            "===================DEBUG==================\n\nEXPECTED\n{:?}\n\nOUTPUT\n{:?}",
            reference, received
        );

        // If they were not found, fail the test
        panic!("Failed test vector comparison.");
    }
}
