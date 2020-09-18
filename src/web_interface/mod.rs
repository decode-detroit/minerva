// Copyright (c) 2020 Decode Detroit
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

//! A module to create the web interface to interact with the underlying
//! system interface. This module links directly to the system interface.

// Import the relevant structures into the correct namespace
use super::system_interface::{ItemId, ItemPair, DescriptiveScene, Status, Event, DisplaySetting}; // FIXME WindowType , Notification, UpcomingEvent};

// Import tokio and warp modules
use tokio::runtime::{Handle, Runtime};
use tokio::sync::{mpsc, oneshot};
use warp::{http, Filter};

/// A type to cover all web api replies
/// 
#[derive(Clone, Serialize, Deserialize)]
pub enum WebReply {
    // A variant for replies with no specific content
    Generic {
        is_valid: bool, // a flag to indicate the result of the request
        message: String, // a message describing the success or failure
    },
    
    // A variant that contains the complete item list
    AllItems {
        is_valid: bool, // a flag to indicate the result of the request
        all_items: Option<Vec<ItemPair>>, // the list of all items, if found
    },

    // A variant that contains scene detail
    Scene {
        is_valid: bool, // a flag to indicate the result of the request
        scene: Option<DescriptiveScene>, // the scene detail, if found
    },

    // A variant that contains status detail
    Status {
        is_valid: bool, // a flag to indicate the result of the request
        status: Option<Status>, // the status detail, if found
    },

    // A variant that contains event detail
    Event {
        is_valid: bool, // a flag to indicate the result of the request
        event: Option<Event>, // the event detail, if found
    },

    // A variant that contains item detail
    Item {
        is_valid: bool, // a flag to indicate the result of the request
        item_pair: Option<ItemPair>, // the item pair, if found
    },
}

// Implement key features of the web reply
impl WebReply {
    /// A function to return a new, successful web reply
    ///
    pub fn success() -> WebReply {
        WebReply::Generic {
            is_valid: true,
            message: "Request completed.".to_string(),
        }
    }
    
    /// A function to return a new, failed web reply
    ///
    pub fn failure<S>(reason: S) -> WebReply
        where S: Into<String>
    {
    
        WebReply::Generic {
            is_valid: false,
            message: reason.into(),
        }
    }
}

/// A structure to pass server-side updates to the user interface
/// 
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebUpdate {// FIXME change to an enum
    display_setting: Option<DisplaySetting>, // variable for to changes the display settings
    //launch_window: Option<WindowType>, // FIXME launch a special window
    notice: Option<String>, // a notice to post briefly
    // FIXME notifications: Option<Vec<Notification>>, // formatted system notifications
    // FIXME upcoming_events: Option<Vec<UpcomingEvent>>, // a list of upcoming events for the timeline
    /*
    UpdateConfig {
        scenes: Vec<ItemPair>,
        full_status: FullStatus,
    },
    UpdateWindow {
        current_scene: ItemPair,
        statuses: Vec<ItemPair>,
        window: EventWindow,
        key_map: KeyMap,
    },
    UpdateStatus {
        status_id: ItemPair, // the group to update
        new_state: ItemPair, // the new state of the group
    },
    */
}

/// A helper type definition for the web_sender
type WebSend = mpsc::Sender<(ItemId, oneshot::Sender<WebReply>)>;

/// A structure to contain the web interface and handle all updates to the
/// to the interface.
///
pub struct WebInterface {
    system_send: SystemSend, // send line to the system interface
}

// Implement key Web Interface functionality
impl WebInterface {
    /// A function to create a new web interface. The send channel should
    /// connect directly to the system interface.
    ///
    pub fn new(system_send: &SystemSend) -> WebInterface {
        // Return the new web interface and runtime handle
        WebInterface {
          system_send: system_send.clone(),
        }
    }

    /// A method to listen for connections from the internet
    ///
    pub async fn run(&mut self) {
        // Create the index filter
        let readme = warp::get()
            .and(warp::path::end())
            .and(warp::fs::file("./index.html"));

        // Create the trigger event filter
        let trigger_event = warp::post()
            .and(warp::path("triggerEvent"))
            .and(warp::path::end())
            .and(WebInterface::with_sender(self.system_send.clone()))
            .and(WebInterface::json_body())
            .and_then(WebInterface::send_message);

        // Combine the filters
        let routes = readme.or(trigger_event);
        
        // Handle incoming requests
        warp::serve(routes)
            .run(([127, 0, 0, 1], 64637))
            .await;
    }
    
    /// A function to extract the json body
    fn json_body() -> impl Filter<Extract = (ItemId,), Error = warp::Rejection> + Clone {
        // When accepting a body, we want a JSON body (reject huge payloads)
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    fn with_sender(system_send: SystemSend) -> impl Filter<Extract = (SystemSend,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || system_send.clone())
    }

    /// A helper function to send a message to the system thread
    async fn send_message(mut system_send: SystemSend, item_id: ItemId) -> Result<impl warp::Reply, warp::Rejection> {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        system_send.send(WebRequest {
            item_id,
            reply_line,
        }).await;
        
        // Wait for the reply
        let new_item = rx.await.unwrap_or(ItemId::all_stop()); // FIXME Not a great fallback
        Ok(warp::reply::with_status(
            warp::reply::json(&new_item),
            http::StatusCode::CREATED,
        ))
    }
}
