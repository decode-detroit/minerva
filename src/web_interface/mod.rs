// Copyright (c) 2020-2021 Decode Detroit
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

//! A module to create the web interface to interface to connect the web UI
//! and endpoints to the program.

// Import crate definitions
use crate::definitions::*;

// Define private submodules
mod web_definitions;

// Import the web definitions
use self::web_definitions::*;

// Import Tokio and warp features
use tokio::sync::{mpsc, oneshot};
use warp::{http, Filter};
use warp::ws::{WebSocket, Message};

// Import stream-related features
use async_stream::stream;
use futures_util::StreamExt;

// Import serde feaures
use serde::de::DeserializeOwned;

// Import constants
use crate::USER_STYLE_SHEET;

/// A structure to contain the web interface and handle all updates to the
/// to the interface.
///
pub struct WebInterface {
    index_access: IndexAccess, // access point for the item index
    style_access: StyleAccess, // access point for the style sheet
    web_send: WebSend,         // send line to the system interface
}

// Implement key Web Interface functionality
impl WebInterface {
    /// A function to create a new web interface. The send channel should
    /// connect directly to the system interface.
    ///
    pub fn new(index_access: IndexAccess, style_access: StyleAccess, web_send: WebSend) -> Self {
        // Return the new web interface and runtime handle
        WebInterface {
            index_access,
            style_access,
            web_send,
        }
    }

    /// A method to listen for connections from the internet
    ///
    pub async fn run(&mut self, mut interface_receive: mpsc::Receiver<InterfaceUpdate>) {
        // Create a channel for sending new listener handles
        let (listener_send, mut listener_recv): (mpsc::Sender<mpsc::Sender<Result<Message, warp::Error>>>, mpsc::Receiver<mpsc::Sender<Result<Message, warp::Error>>>) = mpsc::channel(128);
        
        // Spin up a thread to pass messages to all the web sockets
        tokio::spawn(async move {
            // Create a list of listeners
            let mut listeners = Vec::new();
            
            // Loop until failure of one of the channels
            loop {
                // Check for updates on any line
                tokio::select! {
                    // A new listener handle
                    Some(new_listener) = listener_recv.recv() => {
                        // Add the tx line to the listeners
                        listeners.push(new_listener);
                    }

                    // Updates to the user interface
                    Some(update) = interface_receive.recv() => {
                        // For every listener, send the update
                        for listener in listeners.iter() {
                            // Send a message with the new entries
                            listener.send(update.clone().into()).await.unwrap_or(()); // FIXME Discard broken channels
                        }
                    }
                }     
            }
        });

        // Spin up a thread for the limited access port (64635)
        let clone_send = self.web_send.clone();
        tokio::spawn(async move {
            // Create the limited cue event filter
            let limited_cue_event = warp::post()
                .and(warp::path("cueEvent"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_json::<LimitedCueEvent>())
                .and_then(WebInterface::handle_request);

            // Serve this route on a separate port
            warp::serve(limited_cue_event).run(([127, 0, 0, 1], 64635)).await;
        });

        // Spin up a thread for the run port (64636)
        let clone_send = self.web_send.clone();
        let clone_index = self.index_access.clone();
        tokio::spawn(async move {
            // Create the websocket filter
            let listen = warp::path("listen")
                .and(WebInterface::with_clone(listener_send.clone()))
                .and(warp::ws())
                .map(|sender, ws: warp::ws::Ws| {
                    // This will call the function if the handshake succeeds.
                    ws.on_upgrade(move |socket| WebInterface::add_listener(sender, socket))
                });

            // Create the all event change filter
            let all_event_change = warp::post()
                .and(warp::path("allEventChange"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_json::<AllEventChange>())
                .and_then(WebInterface::handle_request);

            // Create the all scenes filter
            let all_scenes = warp::get()
                .and(warp::path("allScenes"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_clone(UserRequest::Detail {detail_type: DetailType::AllScenes} ))
                .and_then(WebInterface::handle_request);

            // Create the all stop filter
            let all_stop = warp::post()
                .and(warp::path("allStop"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_clone(UserRequest::AllStop))
                .and_then(WebInterface::handle_request);

            // Create the broadcast event filter
            let broadcast_event = warp::post()
                .and(warp::path("broadcastEvent"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_json::<BroadcastEvent>())
                .and_then(WebInterface::handle_request);

            // Create the clear queue filter
            let clear_queue = warp::post()
                .and(warp::path("clearQueue"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_clone(UserRequest::ClearQueue))
                .and_then(WebInterface::handle_request);

            // Create the close filter
            let close = warp::post()
                .and(warp::path("close"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_clone(UserRequest::Close))
                .and_then(WebInterface::handle_request);

            // Create the config file filter
            let config_file = warp::post()
                .and(warp::path("configFile"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_json::<ConfigFile>())
                .and_then(WebInterface::handle_request);

            // Create the cue event filter
            let cue_event = warp::post()
                .and(warp::path("cueEvent"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_json::<FullCueEvent>())
                .and_then(WebInterface::handle_request);

                // Create the debug mode filter
            let debug_mode = warp::post()
                .and(warp::path("debugMode"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_json::<DebugMode>())
                .and_then(WebInterface::handle_request);

            // Create the error log filter
            let error_log = warp::post()
                .and(warp::path("errorLog"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_json::<ErrorLog>())
                .and_then(WebInterface::handle_request);

            // Create the event change filter
            let event_change = warp::post()
                .and(warp::path("eventChange"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_json::<EventChange>())
                .and_then(WebInterface::handle_request);

            // Create the game log filter
            let game_log = warp::post()
                .and(warp::path("gameLog"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_json::<GameLog>())
                .and_then(WebInterface::handle_request);

            // Create the item information filter
            let get_item = warp::get()
                .and(warp::path("getItem"))
                .and(WebInterface::with_clone(clone_index))
                .and(warp::path::param::<GetItem>())
                .and(warp::path::end())
                .and_then(WebInterface::handle_get_item);

            // Create the get style filter
            let get_styles = warp::get()
                .and(warp::path("getStyles")) // Allow javascript filename scrambling to defeat the cache
                .and(warp::fs::file(USER_STYLE_SHEET)); // Reference the temporary file created by the system interface
                // FIXME This filter is OS-specific and may fail on OSX and Windows

            // Create the get status filter
            let get_type = warp::get()
                .and(warp::path("getType"))
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(warp::path::param::<GetType>())
                .and(warp::path::end())
                .and_then(WebInterface::handle_request);

            // Create the process event filter
            let process_event = warp::post()
                .and(warp::path("processEvent"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_json::<ProcessEvent>())
                .and_then(WebInterface::handle_request);

            // Create the scene change filter
            let scene_change = warp::post()
                .and(warp::path("sceneChange"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_json::<SceneChange>())
                .and_then(WebInterface::handle_request);

            // Create the config file filter
            let status_change = warp::post()
                .and(warp::path("statusChange"))
                .and(warp::path::end())
                .and(WebInterface::with_clone(clone_send.clone()))
                .and(WebInterface::with_json::<StatusChange>())
                .and_then(WebInterface::handle_request);

            // Create the main page filter
            let run_page = warp::get()
                .and(warp::fs::dir("./public_run/")); 

            // Combine the filters
            let run_routes = listen
                .or(all_event_change)
                .or(all_scenes)
                .or(all_stop)
                .or(broadcast_event)
                .or(clear_queue)
                .or(close)
                .or(config_file)
                .or(cue_event)
                .or(debug_mode)
                .or(error_log)
                .or(event_change)
                .or(game_log)
                .or(get_item)
                .or(get_styles)
                .or(get_type)
                .or(process_event)
                .or(scene_change)
                .or(status_change)
                .or(run_page);

            // Serve this route on a separate port
            warp::serve(run_routes).run(([127, 0, 0, 1], 64636)).await;
        });

        // Create the all items filter
        let all_items = warp::get()
            .and(warp::path("allItems"))
            .and(warp::path::end())
            .and(WebInterface::with_clone(self.index_access.clone()))
            .and_then(WebInterface::handle_all_items);

        // Create the all scenes filter
        let all_scenes = warp::get()
            .and(warp::path("allScenes"))
            .and(warp::path::end())
            .and(WebInterface::with_clone(self.web_send.clone()))
            .and(WebInterface::with_clone(UserRequest::Detail {detail_type: DetailType::AllScenes} ))
            .and_then(WebInterface::handle_request);

        // Create the config file filter
        let config_file = warp::post()
            .and(warp::path("configFile"))
            .and(warp::path::end())
            .and(WebInterface::with_clone(self.web_send.clone()))
            .and(WebInterface::with_json::<ConfigFile>())
            .and_then(WebInterface::handle_request);

        // Create the edit filter
        let edit = warp::post()
            .and(warp::path("edit"))
            .and(warp::path::end())
            .and(WebInterface::with_clone(self.web_send.clone()))
            .and(WebInterface::with_json::<Edit>())
            .and_then(WebInterface::handle_request);

        // Create the get config path filter
        let get_config_path = warp::get()
            .and(warp::path("getConfigPath"))
            .and(warp::path::end())
            .and(WebInterface::with_clone(self.web_send.clone()))
            .and(WebInterface::with_clone(UserRequest::ConfigPath))
            .and_then(WebInterface::handle_request);

        // Create the get connections filter
        let get_connections = warp::get()
            .and(warp::path("getConnections"))
            .and(warp::path::end())
            .and(WebInterface::with_clone(self.web_send.clone()))
            .and(WebInterface::with_clone(UserRequest::Detail { detail_type: DetailType::Connections }))
            .and_then(WebInterface::handle_request);

        // Create the get event filter
        let get_event = warp::get()
            .and(warp::path("getEvent"))
            .and(WebInterface::with_clone(self.web_send.clone()))
            .and(warp::path::param::<GetEvent>())
            .and(warp::path::end())
            .and_then(WebInterface::handle_request);

        // Create the item information filter
        let get_item = warp::get()
            .and(warp::path("getItem"))
            .and(WebInterface::with_clone(self.index_access.clone()))
            .and(warp::path::param::<GetItem>())
            .and(warp::path::end())
            .and_then(WebInterface::handle_get_item);

        // Create the get scene filter
        let get_scene = warp::get()
            .and(warp::path("getScene"))
            .and(WebInterface::with_clone(self.web_send.clone()))
            .and(warp::path::param::<GetScene>())
            .and(warp::path::end())
            .and_then(WebInterface::handle_request);

        // Create the get status filter
        let get_status = warp::get()
            .and(warp::path("getStatus"))
            .and(WebInterface::with_clone(self.web_send.clone()))
            .and(warp::path::param::<GetStatus>())
            .and(warp::path::end())
            .and_then(WebInterface::handle_request);

        // Create the get style filter
        let get_styles = warp::get()
            .and(warp::path("getStyles")) // Allow javascript filename scrambling to defeat the cache
            .and(warp::fs::file(USER_STYLE_SHEET)); // Reference the temporary file created by the system interface
            // FIXME This filter is OS-specific and may fail on OSX and Windows

        // Create the get status filter
        let get_type = warp::get()
            .and(warp::path("getType"))
            .and(WebInterface::with_clone(self.web_send.clone()))
            .and(warp::path::param::<GetType>())
            .and(warp::path::end())
            .and_then(WebInterface::handle_request);

        // Create the redraw filter FIXME is this needed anymore?
        let redraw = warp::post()
            .and(warp::path("redraw"))
            .and(warp::path::end())
            .and(WebInterface::with_clone(self.web_send.clone()))
            .and(WebInterface::with_clone(UserRequest::Redraw))
            .and_then(WebInterface::handle_request);

        // Create the save config filter FIXME verify filenames
        let save_config = warp::post()
            .and(warp::path("saveConfig"))
            .and(warp::path::end())
            .and(WebInterface::with_clone(self.web_send.clone()))
            .and(WebInterface::with_json::<SaveConfig>())
            .and_then(WebInterface::handle_request);

        // Create the save styles filter FIXME verify content
        let save_style = warp::post()
            .and(warp::path("saveStyles"))
            .and(warp::path::end())
            .and(WebInterface::with_clone(self.style_access.clone()))
            .and(WebInterface::with_json::<SaveStyles>())
            .and_then(WebInterface::handle_save_styles);

        // Create the main page filter
        let edit_page = warp::get()
            .and(warp::fs::dir("./public_edit/")); 

        // Combine the filters
        let edit_routes = all_items
            .or(all_scenes)
            .or(config_file)
            .or(edit)
            .or(get_config_path)
            .or(get_connections)
            .or(get_event)
            .or(get_item)
            .or(get_scene)
            .or(get_status)
            .or(get_styles)
            .or(get_type)
            .or(save_config)
            .or(save_style)
            .or(edit_page);

        // Handle incoming requests on the edit port
        warp::serve(edit_routes).run(([127, 0, 0, 1], 64637)).await;
    }

    /// A function to handle incoming requests
    /// 
    async fn handle_request<R>(
        web_send: WebSend,
        request: R,
    ) -> Result<impl warp::Reply, warp::Rejection>
    where
        R: Into<UserRequest>,
    {
        // Send the message and wait for the reply
        let (reply_to, rx) = oneshot::channel();
        web_send.send(reply_to, request.into()).await;

        // Wait for the reply
        if let Ok(reply) = rx.await {
            // If the reply is a success
            if reply.is_success() {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&reply),
                    http::StatusCode::OK,
                ));

            // Otherwise, note the error
            } else {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&reply),
                    http::StatusCode::BAD_REQUEST,
                ));
            }

        // Otherwise, note the error
        } else {
            return Ok(warp::reply::with_status(
                warp::reply::json(&WebReply::failure("Unable to process request.")),
                http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    }

    /// A function to handle all item requests (processed by the index)
    /// 
    async fn handle_all_items(
        index_access: IndexAccess,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // Get the item pairs from the index
        let items = index_access
            .get_all()
            .await;

        // Return the item pair (even if it is the default)
        return Ok(warp::reply::with_status(
            warp::reply::json(&WebReply::Items {
                is_valid: true,
                items,
            }),
            http::StatusCode::OK,
        ));
    }

    /// A function to handle get item requests (processed by the index)
    /// 
    async fn handle_get_item(
        index_access: IndexAccess,
        get_item: GetItem,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // Get the item pair from the index
        let item_pair = index_access
            .get_pair(&ItemId::new_unchecked(get_item.id))
            .await;

        // Return the item pair (even if it is the default)
        return Ok(warp::reply::with_status(
            warp::reply::json(&WebReply::Item {
                is_valid: true,
                item_pair,
            }),
            http::StatusCode::OK,
        ));
    }

    /// A function to handle saving an updated stylesheet
    async fn handle_save_styles(
        style_access: StyleAccess,
        styles: SaveStyles,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // Send each new style to the style sheet
        style_access.add_styles(styles.new_styles).await;

        // Indicate success
        return Ok(warp::reply::with_status(
            warp::reply::json(&WebReply::success()),
            http::StatusCode::CREATED,
        ));
    }

    /// A function to add a new websocket listener
    /// 
    async fn add_listener(sender: mpsc::Sender<mpsc::Sender<Result<Message, warp::Error>>>, socket: WebSocket) {
        // Split the socket into a sender and receiver
        let (ws_tx, mut ws_rx) = socket.split();

        // Use an unbounded channel to handle buffering and flushing of messages
        let (tx, mut rx) = mpsc::channel(128);
        let stream = stream! {
            while let Some(item) = rx.recv().await {
                yield item;
            }
        };

        // Forward messages until the line is dropped
        tokio::spawn(
            // Forward received messages
            stream.forward(ws_tx)
        );
        
        // Send the tx line to the listener list
        if let Err(_) = sender.send(tx).await {
            // Drop the connection on failure
            return;
        }

        // Wait for the line to be dropped (ignore incoming messages)
        while let Some(_) = ws_rx.next().await {}
    }

    // A function to extract a helper type from the body of the message
    fn with_json<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
    where T: Send + DeserializeOwned {
        // When accepting a body, we want a JSON body (reject large payloads)
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    // A function to add the web send to the filter
    fn with_clone<T>(
        item: T,
    ) -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Clone
    where T: Send + Clone {
        warp::any().map(move || item.clone())
    }
}
