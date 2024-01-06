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
use tokio::fs::read;
use tokio::sync::{mpsc, oneshot};
use warp::ws::{Message, WebSocket};
use warp::{http, Filter};

// Import tracing features
use tracing::{error, info, warn};

// Import stream-related features
use async_stream::stream;
use futures_util::StreamExt;

// Import serde features
use serde::de::DeserializeOwned;

// Import JWT features
use jsonwebtoken as jwt;

// Import rust embed and warp embed features
use rust_embed::RustEmbed;
use warp_embed::embed;

// Define the static resources
#[derive(RustEmbed)]
#[folder = "./web_run_src/build/"]
struct RunWebsite;

#[derive(RustEmbed)]
#[folder = "./web_edit_src/build/"]
struct EditWebsite;

/// A structure to contain the web interface and handle all updates to the
/// to the interface.
///
pub struct WebInterface;

// Implement key Web Interface functionality
impl WebInterface {
    /// A method to launch the web interface.
    /// The interface will listen for connections from the internet and send messages back.
    ///
    pub async fn launch(
        index_access: IndexAccess,
        style_access: StyleAccess,
        web_send: WebSend,
        interface_receive: mpsc::Receiver<InterfaceUpdate>,
        limited_receive: mpsc::Receiver<LimitedUpdate>,
        limited_addr: String,
        run_addr: String,
        edit_addr: String,
        cors_allowed_addr: Option<Vec<String>>,
        possible_cert_path: Option<String>,
        possible_key_path: Option<String>,
    ) {
        // Parse any provided addresses, or use defaults
        let limited_address = limited_addr.parse::<std::net::SocketAddr>();
        let run_address = run_addr.parse::<std::net::SocketAddr>();
        let edit_address = edit_addr.parse::<std::net::SocketAddr>();

        // Create a channel for sending new limited listener handles
        let (limited_listener_send, limited_listener_recv) = mpsc::channel(128);

        // Spin up a thread to pass messages to all the limited web sockets
        let web_clone = web_send.clone();
        tokio::spawn(async move {
            WebInterface::forward_updates(web_clone, limited_listener_recv, limited_receive).await;
        });

        // If limited access address is valid
        if let Ok(address) = limited_address {
            // Spin up a thread for the limited access port
            let clone_send = web_send.clone();
            tokio::spawn(async move {
                // Create the CORS filter to allow a specific origin
                let mut cors;
                if let Some(cors_addresses) = cors_allowed_addr {
                    // Create the header
                    cors = warp::cors();

                    // Add any specified address
                    for address in cors_addresses {
                        cors = cors.allow_origin(address.as_str());
                    }

                    // Add the authentication options
                    cors = cors.allow_headers(vec!["authorization"]);

                    // Specify relevant methods
                    cors = cors.allow_methods(vec!["GET", "POST"]);

                // Otherwise, default to allow any origin
                } else {
                    cors = warp::cors()
                        .allow_any_origin()
                        .allow_methods(vec!["GET", "POST"]);
                }

                // If a TLS certificate and private key were provided FIXME can be cleaned up when let chains are stable
                let mut is_using_tls = false;
                if let (Some(cert_path), Some(private_path)) =
                    (possible_cert_path, possible_key_path)
                {
                    // Try to load the certificate and private keys from the path
                    let possible_cert = read(cert_path).await;
                    let possible_private = read(private_path).await;

                    // If both files loaded successfully FIXME can be cleaned up when let chains are stable
                    if let (Ok(certificate), Ok(private_key)) = (possible_cert, possible_private) {
                        is_using_tls = true; // save successful TLS loading

                        // Try to create the JWT encoding and decoding keys
                        let possible_encoding_key = jwt::EncodingKey::from_rsa_pem(&private_key);
                        let possible_decoding_key = jwt::DecodingKey::from_rsa_pem(&certificate);

                        // If both keys were created successfully FIXME can be cleaned up when let chains are stable
                        if let (Ok(encoding_key), Ok(decoding_key)) =
                            (possible_encoding_key, possible_decoding_key)
                        {   
                            // Share the admin token to the terminal
                            info!(
                                "Limited Cue Admin Token: {}",
                                WebInterface::generate_admin_token(encoding_key.clone())
                            );

                            // Create the options response
                            let cors_options = warp::options().map(warp::reply).with(cors.clone());

                            // Create the authenticated generate token filter
                            let generate_token = warp::get()
                                .and(warp::path("generateToken"))
                                .and(WebInterface::with_clone(encoding_key.clone()))
                                .and(WebInterface::with_clone(decoding_key.clone()))
                                .and(warp::header::<String>("authorization"))
                                .and(warp::path::param::<u64>())
                                .and(warp::path::end())
                                .and_then(WebInterface::generate_token)
                                .with(cors.clone());

                            // Create the websocket filter
                            let limited_listen = warp::path("listen")
                                .and(WebInterface::with_clone(decoding_key.clone()))
                                .and(WebInterface::with_clone(limited_listener_send.clone()))
                                .and(warp::path::param::<String>())
                                .and(warp::ws())
                                .map(|key, sender, token, ws: warp::ws::Ws| {
                                    // This will call the function if the handshake succeeds.
                                    ws.on_upgrade(move |socket| WebInterface::authorize_and_add_listener(key, sender, token, socket))
                                });

                            // Create the authenticated limited cue event filter
                            let limited_cue_event = warp::post()
                                .and(warp::path("cueEvent"))
                                .and(WebInterface::with_clone(decoding_key.clone()))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(warp::header::<String>("authorization"))
                                .and(warp::path::param::<LimitedCueEvent>())
                                .and(warp::path::end())
                                .and_then(WebInterface::authorize_and_handle_request)
                                .with(cors.clone());

                            // Serve this route on a separate port
                            warp::serve(
                                cors_options
                                    .or(generate_token)
                                    .or(limited_listen)
                                    .or(limited_cue_event),
                            )
                            .tls()
                            .cert(certificate)
                            .key(private_key)
                            .run(address)
                            .await;

                        // Fallback to TLS only, no JWT
                        } else {
                            // Throw a warning first
                            warn!("Unable to use JWT authentication: Key format incompatible.");

                            // Create the websocket filter
                            let limited_listen = warp::path("listen")
                                .and(WebInterface::with_clone(limited_listener_send.clone()))
                                .and(warp::ws())
                                .map(|sender, ws: warp::ws::Ws| {
                                    // This will call the function if the handshake succeeds.
                                    ws.on_upgrade(move |socket| WebInterface::add_listener(sender, socket))
                                });

                            // Create the limited cue event filter
                            let limited_cue_event = warp::post()
                                .and(warp::path("cueEvent"))
                                .and(WebInterface::with_clone(clone_send.clone()))
                                .and(warp::path::param::<LimitedCueEvent>())
                                .and(warp::path::end())
                                .and_then(WebInterface::handle_request)
                                .with(cors.clone());

                            // Serve this route on a separate port
                            warp::serve(limited_listen.or(limited_cue_event))
                                .tls()
                                .cert(certificate)
                                .key(private_key)
                                .run(address)
                                .await;
                        }

                    // Fallback to insecure implementation
                    } else {
                        // Throw an error first
                        error!("Unable to serve with TLS: Public or private key file not found.");
                    }
                }

                // Default to no security
                if !is_using_tls {
                    // Create the websocket filter
                    let limited_listen = warp::path("listen")
                        .and(WebInterface::with_clone(limited_listener_send.clone()))
                        .and(warp::ws())
                        .map(|sender, ws: warp::ws::Ws| {
                            // This will call the function if the handshake succeeds.
                            ws.on_upgrade(move |socket| WebInterface::add_listener(sender, socket))
                        });

                    // Create the limited cue event filter
                    let limited_cue_event = warp::post()
                        .and(warp::path("cueEvent"))
                        .and(WebInterface::with_clone(clone_send.clone()))
                        .and(warp::path::param::<LimitedCueEvent>())
                        .and(warp::path::end())
                        .and_then(WebInterface::handle_request)
                        .with(cors);

                    // Serve this route on a separate port
                    warp::serve(limited_listen.or(limited_cue_event))
                        .run(address)
                        .await;
                }
            });
        }

        // Create a channel for sending new listener handles
        let (listener_send, listener_recv) = mpsc::channel(128);

        // Spin up a thread to pass messages to all the web sockets
        let web_clone = web_send.clone();
        tokio::spawn(async move {
            WebInterface::forward_updates(web_clone, listener_recv, interface_receive).await;
        });

        // If run address is valid
        if let Ok(address) = run_address {
            // Spin up a thread for the run port
            let clone_send = web_send.clone();
            let clone_index = index_access.clone();
            let clone_style = style_access.clone();
            tokio::spawn(async move {
                // Create the websocket filter
                let listen = warp::path("listen")
                    .and(WebInterface::with_clone(listener_send.clone()))
                    .and(warp::ws())
                    .map(|sender, ws: warp::ws::Ws| {
                        // This will call the function if the handshake succeeds.
                        ws.on_upgrade(move |socket| WebInterface::add_listener(sender, socket))
                    });

                // Create the all current items filter
                let all_current_items = warp::get()
                    .and(warp::path("allCurrentItems"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(clone_send.clone()))
                    .and(WebInterface::with_clone(UserRequest::Detail {
                        detail_type: DetailType::AllCurrentItems,
                    }))
                    .and_then(WebInterface::handle_request);

                // Create the all event change filter
                let all_event_change = warp::post()
                    .and(warp::path("allEventChange"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(clone_send.clone()))
                    .and(WebInterface::with_json::<AllEventChange>())
                    .and_then(WebInterface::handle_request);

                // Create the all groups filter
                let all_groups = warp::get()
                    .and(warp::path("allGroups"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(clone_send.clone()))
                    .and(WebInterface::with_clone(UserRequest::Detail {
                        detail_type: DetailType::AllGroups,
                    }))
                    .and_then(WebInterface::handle_request);

                // Create the all scenes filter
                let all_scenes = warp::get()
                    .and(warp::path("allScenes"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(clone_send.clone()))
                    .and(WebInterface::with_clone(UserRequest::Detail {
                        detail_type: DetailType::AllScenes,
                    }))
                    .and_then(WebInterface::handle_request);

                // Create the all stop filter
                let all_stop = warp::post()
                    .and(warp::path("allStop"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(clone_send.clone()))
                    .and(WebInterface::with_clone(UserRequest::AllStop))
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

                // Create the event change filter
                let event_change = warp::post()
                    .and(warp::path("eventChange"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(clone_send.clone()))
                    .and(WebInterface::with_json::<EventChange>())
                    .and_then(WebInterface::handle_request);

                // Create the item information filter
                let get_item = warp::get()
                    .and(warp::path("getItem"))
                    .and(WebInterface::with_clone(clone_index))
                    .and(warp::path::param::<GetItem>())
                    .and(warp::path::end())
                    .and_then(WebInterface::handle_get_item);

                // Create the group information filter
                let get_group = warp::get()
                    .and(warp::path("getGroup"))
                    .and(WebInterface::with_clone(clone_send.clone()))
                    .and(warp::path::param::<GetGroup>())
                    .and(warp::path::end())
                    .and_then(WebInterface::handle_request);

                // Create the get style filter
                let get_styles = warp::get()
                    .and(warp::path("getStyles")) // Allow javascript filename scrambling to defeat the cache
                    .and(WebInterface::with_clone(clone_style.clone()))
                    .and_then(WebInterface::handle_get_styles);

                // Create the get status filter
                let get_type = warp::get()
                    .and(warp::path("getType"))
                    .and(WebInterface::with_clone(clone_send.clone()))
                    .and(warp::path::param::<GetType>())
                    .and(warp::path::end())
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
                let run_page = warp::get().and(embed(&RunWebsite));

                // Combine the filters
                let run_routes = listen
                    .or(all_current_items)
                    .or(all_event_change)
                    .or(all_groups)
                    .or(all_scenes)
                    .or(all_stop)
                    .or(clear_queue)
                    .or(close)
                    .or(config_file)
                    .or(cue_event)
                    .or(event_change)
                    .or(get_item)
                    .or(get_group)
                    .or(get_styles)
                    .or(get_type)
                    .or(scene_change)
                    .or(status_change)
                    .or(run_page);

                // Serve this route on a separate port
                warp::serve(run_routes).run(address).await;
            });
        }

        // If the edit address is valid
        if let Ok(address) = edit_address {
            // Spin up a thread for the edit port
            tokio::spawn(async move {
                // Create the websocket filter
                let listen = warp::path("listen")
                    .and(warp::ws())
                    .map(|ws: warp::ws::Ws| {
                        // This will call the function if the handshake succeeds.
                        ws.on_upgrade(move |socket| WebInterface::fake_listener(socket))
                    });

                // Create the all items filter
                let all_items = warp::get()
                    .and(warp::path("allItems"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(index_access.clone()))
                    .and_then(WebInterface::handle_all_items);

                // Create the all groups filter
                let all_groups = warp::get()
                    .and(warp::path("allGroups"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(WebInterface::with_clone(UserRequest::Detail {
                        detail_type: DetailType::AllGroups,
                    }))
                    .and_then(WebInterface::handle_request);

                // Create the all scenes filter
                let all_scenes = warp::get()
                    .and(warp::path("allScenes"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(WebInterface::with_clone(UserRequest::Detail {
                        detail_type: DetailType::AllScenes,
                    }))
                    .and_then(WebInterface::handle_request);

                // Create the close filter
                let close = warp::post()
                    .and(warp::path("close"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(WebInterface::with_clone(UserRequest::Close))
                    .and_then(WebInterface::handle_request);

                // Create the config file filter
                let config_file = warp::post()
                    .and(warp::path("configFile"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(WebInterface::with_json::<ConfigFile>())
                    .and_then(WebInterface::handle_request);

                // Create the edit filter
                let edit = warp::post()
                    .and(warp::path("edit"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(WebInterface::with_json::<Edit>())
                    .and_then(WebInterface::handle_request);

                // Create the get config parameters filter
                let get_config_param = warp::get()
                    .and(warp::path("getConfigParam"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(WebInterface::with_clone(UserRequest::ConfigParameters))
                    .and_then(WebInterface::handle_request);

                // Create the get config path filter
                let get_config_path = warp::get()
                    .and(warp::path("getConfigPath"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(WebInterface::with_clone(UserRequest::ConfigPath))
                    .and_then(WebInterface::handle_request);

                // Create the get event filter
                let get_event = warp::get()
                    .and(warp::path("getEvent"))
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(warp::path::param::<GetEvent>())
                    .and(warp::path::end())
                    .and_then(WebInterface::handle_request);

                // Create the item information filter
                let get_item = warp::get()
                    .and(warp::path("getItem"))
                    .and(WebInterface::with_clone(index_access.clone()))
                    .and(warp::path::param::<GetItem>())
                    .and(warp::path::end())
                    .and_then(WebInterface::handle_get_item);

                // Create the get group filter
                let get_group = warp::get()
                    .and(warp::path("getGroup"))
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(warp::path::param::<GetGroup>())
                    .and(warp::path::end())
                    .and_then(WebInterface::handle_request);

                // Create the get scene filter
                let get_scene = warp::get()
                    .and(warp::path("getScene"))
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(warp::path::param::<GetScene>())
                    .and(warp::path::end())
                    .and_then(WebInterface::handle_request);

                // Create the get status filter
                let get_status = warp::get()
                    .and(warp::path("getStatus"))
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(warp::path::param::<GetStatus>())
                    .and(warp::path::end())
                    .and_then(WebInterface::handle_request);

                // Create the get style filter
                let get_styles = warp::get()
                    .and(warp::path("getStyles")) // Allow javascript filename scrambling to defeat the cache
                    .and(WebInterface::with_clone(style_access.clone()))
                    .and_then(WebInterface::handle_get_styles);

                // Create the get status filter
                let get_type = warp::get()
                    .and(warp::path("getType"))
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(warp::path::param::<GetType>())
                    .and(warp::path::end())
                    .and_then(WebInterface::handle_request);

                // Create the save config filter FIXME verify filenames
                let save_config = warp::post()
                    .and(warp::path("saveConfig"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(web_send.clone()))
                    .and(WebInterface::with_json::<SaveConfig>())
                    .and_then(WebInterface::handle_request);

                // Create the save styles filter FIXME verify content
                let save_style = warp::post()
                    .and(warp::path("saveStyles"))
                    .and(warp::path::end())
                    .and(WebInterface::with_clone(style_access.clone()))
                    .and(WebInterface::with_json::<SaveStyles>())
                    .and_then(WebInterface::handle_save_styles);

                // Create the main page filter
                let edit_page = warp::get().and(embed(&EditWebsite));

                // Combine the filters
                let edit_routes = listen
                    .or(all_items)
                    .or(all_groups)
                    .or(all_scenes)
                    .or(close)
                    .or(config_file)
                    .or(edit)
                    .or(get_config_param)
                    .or(get_config_path)
                    .or(get_event)
                    .or(get_item)
                    .or(get_group)
                    .or(get_scene)
                    .or(get_status)
                    .or(get_styles)
                    .or(get_type)
                    .or(save_config)
                    .or(save_style)
                    .or(edit_page);

                // Handle incoming requests on the edit port
                warp::serve(edit_routes).run(address).await;
            });
        }
    }

    /// A function to pass interface update messages to a websocket
    /// 
    async fn forward_updates<T>(web_send: WebSend, mut listener_recv: mpsc::Receiver<ListenerWithExpiration>, mut interface_receive: mpsc::Receiver<T>) 
    where T: Clone +  Into<Result<Message, warp::Error>>
    {
        // Create a list of listeners
        let mut listeners = Vec::new();

        // Loop until failure of one of the channels
        loop {
            // Check for updates on any line
            tokio::select! {
                // A new websocket handle
                Some(new_listener) = listener_recv.recv() => {
                    // Create a oneshot channel to get the current scene and status
                    let (reply_to, rx) = oneshot::channel();

                    // Send the message and wait for the reply
                    web_send.send(reply_to, UserRequest::CurrentSceneAndStatus).await;

                    // If we got a reply
                    if let Ok(reply) = rx.await {
                        // If the reply is a success
                        if reply.is_success() {
                            // Ensure it's the correct reply
                            match reply.data {
                                WebReplyData::CurrentSceneAndStatus((current_scene, current_status)) => {
                                    // Send the update to the listener (cheat: technically should be InterfaceUpdate some of the time, but they're equivalent)
                                    if let Ok(_) = new_listener.socket.send(LimitedUpdate::CurrentSceneAndStatus { current_scene, current_status }.into()).await {
                                        // If successful, add the tx line to the listeners
                                        listeners.push(new_listener);
                                    }
                                }
                                _ => ()
                            }

                        // Otherwise, just add the listener
                        } else {
                            listeners.push(new_listener);
                        }

                    // Otherwise, just add the listener
                    } else {
                        listeners.push(new_listener);
                    }
                }

                // Updates to the interface
                Some(update) = interface_receive.recv() => {
                    // For every listener, send the update or drop the channel
                    let mut active_listeners = Vec::new();
                    for listener in listeners.drain(..) {
                        // Check if the listener has expired
                        if listener.expiration != 0 && listener.expiration < jwt::get_current_timestamp() {
                            continue; // continue and drop the listener
                        }

                        // Try to send a message with the new entries
                        if let Ok(_) = listener.socket.send(update.clone().into()).await {
                            // If the message was successful, keep the channel
                            active_listeners.push(listener);
                        }
                    }
                    listeners = active_listeners;
                }
            }
        }
    }

    /// A function to generate an administrator JWT authentication token
    ///
    /// # Note
    /// On failure, this function returns a description of the error
    /// instead of the token.
    ///
    fn generate_admin_token(encoding_key: jwt::EncodingKey) -> String {
        // Compose the claims for the token
        let claims = Claims {
            iss: "Minerva-LimitedCue-Admin".to_string(),
            exp: 0, // Expiration is ignored
        };

        // Try to encode the token
        match jwt::encode(
            &jwt::Header::new(jwt::Algorithm::RS256),
            &claims,
            &encoding_key,
        ) {
            Ok(token) => token,
            _ => "Unable to generate admin token.".to_string(),
        }
    }

    /// A function to generate a JWT authentication token
    /// with an expiration time in the future (in seconds)
    ///
    async fn generate_token(
        encoding_key: jwt::EncodingKey,
        decoding_key: jwt::DecodingKey,
        token: String,
        expiration: u64,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // Create the validation requirements for the admin token
        let mut validation = jwt::Validation::new(jwt::Algorithm::RS256);
        validation.set_issuer(&["Minerva-LimitedCue-Admin"]);
        validation.validate_exp = false; // generator token doesn't expire
        match jwt::decode::<Claims>(&token, &decoding_key, &validation) {
            // Return the decoded data
            Ok(_) => (),

            // Return an authentication error
            _ => {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&WebReply::failure("User is not authorized.")),
                    http::StatusCode::FORBIDDEN,
                ));
            }
        };

        // Create the timestamp for the expiration of the new token
        let exp = jwt::get_current_timestamp() + expiration;

        // Compose the claims for the token
        let claims = Claims {
            iss: "Minerva-LimitedCue".to_string(),
            exp,
        };

        // Encode the token
        let token = match jwt::encode(
            &jwt::Header::new(jwt::Algorithm::RS256),
            &claims,
            &encoding_key,
        ) {
            Ok(token) => token,
            Err(_) => {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&WebReply::failure("Unable to generate new token.")),
                    http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        };

        // Return the successful token
        Ok(warp::reply::with_status(
            warp::reply::json(&WebReply {
                is_valid: true,
                data: WebReplyData::Message(token),
            }),
            http::StatusCode::OK,
        ))
    }

    /// A function to check authentication and then handle incoming requests
    ///
    async fn authorize_and_handle_request<R>(
        key: jwt::DecodingKey,
        web_send: WebSend,
        token: String,
        request: R,
    ) -> Result<impl warp::Reply, warp::Rejection>
    where
        R: Into<UserRequest>,
    {
        // Create the validation requirements for the token
        let mut validation = jwt::Validation::new(jwt::Algorithm::RS256);
        validation.set_issuer(&["Minerva-LimitedCue"]);
        match jwt::decode::<Claims>(&token, &key, &validation) {
            // Return the decoded data
            Ok(_) => (),

            // Return an authentication error
            _ => {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&WebReply::failure("User is not authorized.")),
                    http::StatusCode::FORBIDDEN,
                ));
            }
        };

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

    /// A function to check authentication and then add a new websocket
    ///
    async fn authorize_and_add_listener(
        key: jwt::DecodingKey,
        sender: mpsc::Sender<ListenerWithExpiration>,
        token: String,
        socket: WebSocket,
    ) {
        // Create the validation requirements for the token
        let mut validation = jwt::Validation::new(jwt::Algorithm::RS256);
        validation.set_issuer(&["Minerva-LimitedCue"]);
        let token_data = match jwt::decode::<Claims>(&token, &key, &validation) {
            // Return the decoded data
            Ok(data) => data,

            // Return without connecting the socket
            _ => return,
        };
        
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
            stream.forward(ws_tx),
        );

        // Send a listener with no expiration
        if let Err(_) = sender.send(ListenerWithExpiration { socket: tx, expiration: token_data.claims.exp }).await {
            // Drop the connection on failure
            return;
        }

        // Wait for the line to be dropped (ignore incoming messages)
        while ws_rx.next().await.is_some() {}
    }

    /// A function to add a new websocket listener
    ///
    async fn add_listener(
        sender: mpsc::Sender<ListenerWithExpiration>,
        socket: WebSocket,
    ) {
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
            stream.forward(ws_tx),
        );

        // Send a listener with no expiration
        if let Err(_) = sender.send(ListenerWithExpiration { socket: tx, expiration: 0 }).await {
            // Drop the connection on failure
            return;
        }

        // Wait for the line to be dropped (ignore incoming messages)
        while ws_rx.next().await.is_some() {}
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
        let items = index_access.get_all().await;

        // Return the item pair (even if it is the default)
        return Ok(warp::reply::with_status(
            warp::reply::json(&WebReply {
                is_valid: true,
                data: WebReplyData::Items(items),
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
            warp::reply::json(&WebReply {
                is_valid: true,
                data: WebReplyData::Item(item_pair),
            }),
            http::StatusCode::OK,
        ));
    }

    /// A function to handle getting the current stylesheet
    async fn handle_get_styles(
        style_access: StyleAccess,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // Get all the styles from the stylesheet
        let mut rules = style_access.get_all_rules().await;

        // Compose the rules into a string
        let rules_string = rules.drain().map(|(mut selector, rule)| {
            selector += " ";
            selector += &rule;
            return selector;
        }).collect::<Vec<String>>().join("\n");

        // Indicate success
        return Ok(warp::http::Response::builder().header("content-type", "text/css").body(rules_string));
    }

    /// A function to handle saving an updated stylesheet
    async fn handle_save_styles(
        style_access: StyleAccess,
        styles: SaveStyles,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // Send all the new styles to the style sheet
        style_access.add_styles(styles.new_styles).await;

        // Indicate success
        return Ok(warp::reply::with_status(
            warp::reply::json(&WebReply::success()),
            http::StatusCode::CREATED,
        ));
    }

    /// A function to add a fake listener
    ///
    async fn fake_listener(socket: WebSocket) {
        // Split the socket into a sender and receiver
        let (_tx, mut ws_rx) = socket.split();

        // Wait for the line to be dropped (ignore incoming messages)
        while ws_rx.next().await.is_some() {}
    }

    // A function to extract a helper type from the body of the message
    fn with_json<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
    where
        T: Send + DeserializeOwned,
    {
        // When accepting a body, we want a JSON body (reject large payloads)
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    // A function to add the web send to the filter
    fn with_clone<T>(
        item: T,
    ) -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Clone
    where
        T: Send + Clone,
    {
        warp::any().map(move || item.clone())
    }
}
