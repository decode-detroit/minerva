// Copyright (c) 2021 Decode Detroit
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

//! A module to create handy definitions for the web interface.
//! These definitions are not used elsewhere in the program.

// Import crate definitions
use crate::definitions::*;

// Import standard library features
use std::str::FromStr;
use std::path::PathBuf;
use std::time::Duration;
use std::num::ParseIntError;

// Import Chrono features
use chrono::NaiveDateTime;

/// Helper data types to formalize request structure
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllEventChange {
    adjustment_secs: u64,
    adjustment_nanos: u64,
    is_negative: bool,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastEvent {
    id: u32,
    data: Option<u32>,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFile { 
    filename: String,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitedCueEvent {
    id: u32,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullCueEvent {
    id: u32,
    secs: u64,
    nanos: u64,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugMode {
    is_debug: bool,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edit {
    modifications: Vec<Modification>,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorLog {
    filename: String,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventChange {
    event_id: ItemId,
    start_time: NaiveDateTime,
    new_delay: Option<Duration>,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameLog {
    filename: String,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetEvent {
    id: u32,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetItem {
    pub id: u32,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetScene {
    id: u32,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetStatus {
    id: u32,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetType {
    id: u32,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEvent {
    event_id: u32,
    check_scene: bool,
    broadcast: bool,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveConfig {
    filename: String,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveStyles {
    pub new_styles: StyleMap,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneChange {
    scene_id: u32,
}
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusChange {
    status_id: u32,
    state_id: u32,
}

// Implement FromStr for helper data types
impl FromStr for GetEvent {
    // Interpret errors as ParseIntError
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse as a u32 and return the result
        let id = s.parse::<u32>()?;
        Ok(GetEvent { id })
    }
}
impl FromStr for GetItem {
    // Interpret errors as ParseIntError
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse as a u32 and return the result
        let id = s.parse::<u32>()?;
        Ok(GetItem { id })
    }
}
impl FromStr for GetScene {
    // Interpret errors as ParseIntError
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse as a u32 and return the result
        let id = s.parse::<u32>()?;
        Ok(GetScene { id })
    }
}
impl FromStr for GetStatus {
    // Interpret errors as ParseIntError
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse as a u32 and return the result
        let id = s.parse::<u32>()?;
        Ok(GetStatus { id })
    }
}
impl FromStr for GetType {
    // Interpret errors as ParseIntError
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse as a u32 and return the result
        let id = s.parse::<u32>()?;
        Ok(GetType { id })
    }
}

// Implement from for the helper data types
impl From<AllEventChange> for UserRequest {
    fn from(all_event_change: AllEventChange) -> Self {
        // Create the duration
        let adjustment = Duration::from_secs(all_event_change.adjustment_secs)
            + Duration::from_nanos(all_event_change.adjustment_nanos);

        // Return the request
        UserRequest::AllEventChange {
            adjustment,
            is_negative: all_event_change.is_negative,
        }
    }
}
impl From<BroadcastEvent> for UserRequest {
    fn from(broadcast_event: BroadcastEvent) -> Self {
        UserRequest::BroadcastEvent {
            event_id: ItemId::new_unchecked(broadcast_event.id),
            data: broadcast_event.data,
        }
    }
}
impl From<ConfigFile> for UserRequest {
    fn from(config_file: ConfigFile) -> Self {
        UserRequest::ConfigFile {
            filepath: Some(PathBuf::from(config_file.filename)),
        }
    }
}
impl From<LimitedCueEvent> for UserRequest {
    fn from(cue_event: LimitedCueEvent) -> Self {
        // Return the request
        UserRequest::CueEvent {
            event_delay: EventDelay::new(None, ItemId::new_unchecked(cue_event.id)),
        }
    }
}
impl From<FullCueEvent> for UserRequest {
    fn from(cue_event: FullCueEvent) -> Self {
        // Create the duration
        let delay;
        if cue_event.secs != 0 || cue_event.nanos != 0 {
            delay =
                Some(Duration::from_secs(cue_event.secs) + Duration::from_nanos(cue_event.nanos));
        } else {
            delay = None;
        }

        // Return the request
        UserRequest::CueEvent {
            event_delay: EventDelay::new(delay, ItemId::new_unchecked(cue_event.id)),
        }
    }
}
impl From<DebugMode> for UserRequest {
    fn from(debug_mode: DebugMode) -> Self {
        UserRequest::DebugMode(debug_mode.is_debug)
    }
}
impl From<Edit> for UserRequest {
    fn from(edit: Edit) -> Self {
        UserRequest::Edit {
            modifications: edit.modifications,
        }
    }
}
impl From<ErrorLog> for UserRequest {
    fn from(error_log: ErrorLog) -> Self {
        UserRequest::ErrorLog {
            filepath: PathBuf::from(error_log.filename),
        }
    }
}
impl From<EventChange> for UserRequest {
    fn from(event_change: EventChange) -> Self {
        UserRequest::EventChange {
            event_id: event_change.event_id,
            start_time: event_change.start_time,
            new_delay: event_change.new_delay,
        }
    }
}
impl From<GameLog> for UserRequest {
    fn from(game_log: GameLog) -> Self {
        UserRequest::GameLog {
            filepath: PathBuf::from(game_log.filename),
        }
    }
}
impl From<GetEvent> for UserRequest {
    fn from(get_event: GetEvent) -> Self {
        UserRequest::Detail {
            detail_type: DetailType::Event {
                item_id: ItemId::new_unchecked(get_event.id),
            }
        }
    }
}
impl From<GetScene> for UserRequest {
    fn from(get_scene: GetScene) -> Self {
        UserRequest::Detail {
            detail_type: DetailType::Scene {
                item_id: ItemId::new_unchecked(get_scene.id),
            }
        }
    }
}
impl From<GetStatus> for UserRequest {
    fn from(get_status: GetStatus) -> Self {
        UserRequest::Detail {
            detail_type: DetailType::Status {
                item_id: ItemId::new_unchecked(get_status.id),
            }
        }
    }
}
impl From<GetType> for UserRequest {
    fn from(get_type: GetType) -> Self {
        UserRequest::Detail {
            detail_type: DetailType::Type {
                item_id: ItemId::new_unchecked(get_type.id),
            }
        }
    }
}
impl From<ProcessEvent> for UserRequest {
    fn from(process_event: ProcessEvent) -> Self {
        UserRequest::ProcessEvent {
            event: ItemId::new_unchecked(process_event.event_id),
            check_scene: process_event.check_scene,
            broadcast: process_event.broadcast,
        }
    }
}
impl From<SaveConfig> for UserRequest {
    fn from(save_config: SaveConfig) -> Self {
        UserRequest::SaveConfig {
            filepath: PathBuf::from(save_config.filename),
        }
    }
}
impl From<SceneChange> for UserRequest {
    fn from(scene_change: SceneChange) -> Self {
        UserRequest::SceneChange {
            scene: ItemId::new_unchecked(scene_change.scene_id),
        }
    }
}
impl From<StatusChange> for UserRequest {
    fn from(status_change: StatusChange) -> Self {
        UserRequest::StatusChange {
            status: ItemId::new_unchecked(status_change.status_id),
            state: ItemId::new_unchecked(status_change.state_id),
        }
    }
}