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
use super::system_interface::ItemId;

// Import tokio and warp modules
use tokio::runtime::{Handle, Runtime};
use tokio::sync::{mpsc, oneshot};
use warp::{http, Filter};


/// A structure to hold a reply from the system interface
///
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct WebReply {
    is_valid: bool, // An indication if the request was a success
}

// Implement key features of the web reply
impl WebReply {
    /// A function to return a new, successful web reply
    ///
    pub fn success() -> WebReply {
        WebReply {
            is_valid: true,
        }
    }
    
    /// A function to return a new, failed web reply
    ///
    pub fn failure() -> WebReply {
        WebReply {
            is_valid: false,
        }
    }
}

/// A helpder type definition for the web_sender
type WebSend = mpsc::Sender<(ItemId, oneshot::Sender<WebReply>)>;

/// A structure to contain the web interface and handle all updates to the
/// to the interface.
///
pub struct WebInterface {
    runtime: Runtime,       // the tokio runtime
    web_send: WebSend,   // send line from the web interface
}

// Implement key Web Interface functionality
impl WebInterface {
    /// A function to create a new web interface. The send channel should
    /// connect directly to the system interface.
    ///
    pub fn new(web_send: &WebSend) -> (WebInterface, Handle) {
        // Create the runtime
        let runtime = Runtime::new().expect("Unable To Start Tokio Runtime.");
        let handle = runtime.handle().clone();
        
        // Return the new web interface and runtime handle
        (WebInterface {
          runtime,
          web_send: web_send.clone(),
        }, handle)
    }

    /// A method to listen for connections from the internet
    ///
    pub fn run(&mut self) {
        // Create the index filter
        let readme = warp::get()
            .and(warp::path::end())
            .and(warp::fs::file("./index.html"));

        // Create the trigger event filter
        let trigger_event = warp::post()
            .and(warp::path("triggerEvent"))
            .and(warp::path::end())
            .and(WebInterface::with_sender(self.web_send.clone()))
            .and(WebInterface::json_body())
            .and_then(WebInterface::send_message);

        // Combine the filters
        let routes = readme.or(trigger_event);
        
        // Handle incoming requests
        self.runtime.block_on(async {
            warp::serve(routes)
                .run(([127, 0, 0, 1], 64637))
                .await;
        });
    }
    
    /// A function to extract the json body
    fn json_body() -> impl Filter<Extract = (ItemId,), Error = warp::Rejection> + Clone {
        // When accepting a body, we want a JSON body (reject huge payloads)
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    fn with_sender(web_send: WebSend) -> impl Filter<Extract = (WebSend,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || web_send.clone())
    }

    /// A helper function to send a message to the system thread
    async fn send_message(mut web_send: WebSend, item_id: ItemId) -> Result<impl warp::Reply, warp::Rejection> {
        // Send the message and wait for the reply
        let (reply_line, rx) = oneshot::channel();
        web_send.send((item_id, reply_line)).await.unwrap_or(());
        
        // Wait for the reply
        let new_item = rx.await.unwrap_or(WebReply::failure());
        Ok(warp::reply::with_status(
            warp::reply::json(&new_item),
            http::StatusCode::CREATED,
        ))
    }
}


