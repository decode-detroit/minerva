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

//! A module to load and play audio files on this device

// Import the relevant structures into the correct namespace
use super::{EventConnection, ItemId, ReadResult};

// Import standard library features
use std::path::PathBuf;
#[cfg(feature = "audio")]
use std::fs::File;
#[cfg(feature = "audio")]
use std::io::BufReader;

// Import the Rust audio module
#[cfg(feature = "audio")]    
use rodio::{Device, DeviceTrait, Sink};

// Import FNV HashMap
use fnv::FnvHashMap;

// Import the failure features
use failure::Error;

/// A struct to define a single audio track to play
///
/// # Note
///
/// Devices are enumated by the ALSA library, and generally follow the format
/// "front:CARD=Device,DEV=0". If the specified device is not found or is not
/// available, the audio will play on the default audio device instead.
///
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioCue {
    path: PathBuf,                  // the location of the audio file to play
    device_name: String,            // the device name, where "default" indicates the default device
    stop_sounds: bool,              // a flag to stop other sounds playing on the same device
    volume: f32,                    // the volume of the source, where 1 is default
    // TODO Add additional features in the future
    // E.G. the ability to append sound, the ability to pause or resume sound
}

/// A type to store a hashmap of event ids and Audio Cues
///
pub type AudioMap = FnvHashMap<ItemId, AudioCue>;

/// A structure to hold a device and all corresponding sinks
///
#[cfg(feature = "audio")]
struct AudioDevice {
    device: Device,                 // the audio device
    sinks: Vec<Sink>,               // all the sinks that are playing on this device
}

/// A structure to hold and manipulate the connection to the audio backend
///
pub struct AudioOut {
    #[cfg(feature = "audio")]
    all_stop_audio: Vec<AudioCue>,        // a vector of audio cues for all stop
    audio_map: AudioMap,                  // the map of event ids to audio cues
    #[cfg(feature = "audio")]
    devices: FnvHashMap<String, AudioDevice>,   // a map of active audio devices and corresponding sinks
}

// Implement key functionality for the Audio Out structure
impl AudioOut {
    /// A function to create a new instance of the AudioOut, active version
    ///
    #[cfg(feature = "audio")]
    pub fn new(all_stop_audio: Vec<AudioCue>, audio_map: AudioMap) -> Result<AudioOut, Error> {
        // Return the complete module
        Ok(AudioOut {
            all_stop_audio,
            audio_map,
            devices: FnvHashMap::default(),
        })
    }

    /// A function to create a new instance of the AudioOut, inactive version
    ///
    #[cfg(not(feature = "audio"))]
    pub fn new(_all_stop_audio: Vec<AudioCue>, audio_map: AudioMap) -> Result<AudioOut, Error> {
        // Return a partial module
        Ok(AudioOut {
            audio_map,
        })
    }
    
    // A helper function to correctly add a new audio cue
    #[cfg(feature = "audio")]
    fn add_cue(active_devices: &mut FnvHashMap<String, AudioDevice>, audio_cue: AudioCue) -> Result<(), Error> {
        // Try to open the specified audio file
        let file = match File::open(audio_cue.path) {
            Ok(file) => file,
            _ => return Err(format_err!("Unable to open selected audio file.")),
        };
        
        // If the device is already active
        if let Some(audio_device) = active_devices.get_mut(&audio_cue.device_name) {
            // Stop any playing sounds, as directed
            if audio_cue.stop_sounds {
                AudioOut::stop_sinks(audio_device);
            }
            
            // Try to play the file as a new source
            if let Ok(sink) = rodio::play_once(&audio_device.device, BufReader::new(file)) {
                // Set the sink volume and save it
                sink.set_volume(audio_cue.volume);
                audio_device.sinks.push(sink);
                
                // Return success
                return Ok(());
            
            // Throw an error that the file failed to play
            } else {
                return Err(format_err!("Unable to open selected audio file."));
            }
        
        // If the device was not found
        } else {
            // Unpack the specified device name
            if "default" != audio_cue.device_name {
                // Check to see if the device name is valid
                if let Ok(devices) = rodio::devices() {
                    // Check to see if the name matches
                    for device in devices {
                        if let Ok(name) = device.name() {
                            if name == audio_cue.device_name {
                                // Verify that the device is valid for output
                                if let Ok(_) = device.supported_output_formats() {
                                    // Try to play the file as a new source
                                    if let Ok(sink) = rodio::play_once(&device, BufReader::new(file)) {
                                        // Set the sink volume
                                        sink.set_volume(audio_cue.volume);
                                    
                                        // Create the new audio device
                                        let audio_device = AudioDevice {
                                            device,
                                            sinks: vec!(sink),
                                        };
                                        
                                        // Add the audio device to the device list
                                        active_devices.insert(audio_cue.device_name, audio_device);
                                        
                                        // Return success
                                        return Ok(());
                                    
                                    // Throw an error with the file
                                    } else {
                                        return Err(format_err!("Unable to play audio file."));
                                    }
                                }
                            }
                        }
                    } // If this search does not succeed, continue to the default device below
                // Return on error
                } else {
                    return Err(format_err!("Error opening audio devices list."));
                }
            }
            
            // Otherwise, use the default device
            if let Some(audio_device) = active_devices.get_mut("default") {
                // Stop any playing sounds, as directed
                if audio_cue.stop_sounds {
                    AudioOut::stop_sinks(audio_device);
                }
                
                // Try to play the file as a new source
                if let Ok(sink) = rodio::play_once(&audio_device.device, BufReader::new(file)) {                  
                    // Set the sink volume and save it
                    sink.set_volume(audio_cue.volume);
                    audio_device.sinks.push(sink);
                    
                    // Return success
                    return Ok(());
                
                // Throw an error with the file
                } else {
                    return Err(format_err!("Unable to play audio file."));
                }
            
            // Create the default device if it doesn't exist
            } else {
                // Try to open the default device
                if let Some(device) = rodio::default_output_device() {
                    // Try to play the file as a new source
                    if let Ok(sink) = rodio::play_once(&device, BufReader::new(file)) {
                        // Set the sink volume
                        sink.set_volume(audio_cue.volume);
                        
                        // Create the new audio device
                        let audio_device = AudioDevice {
                            device,
                            sinks: vec!(sink),
                        };
                                            
                        // Add the device to the device list
                        active_devices.insert("default".to_string(), audio_device);
                        
                        // Return success
                        return Ok(());
                    
                    // Throw an error with the file
                    } else {
                        return Err(format_err!("Unable to play audio file."));
                    }
                
                // Return an error
                } else {
                    return Err(format_err!("Error opening default audio device."));
                }
            }
        }
    }
    
    // A helper function to stop and remove any sinks associated with this device
    #[cfg(feature = "audio")]
    fn stop_sinks(audio_device: &mut AudioDevice) {
        // Remove each sink from the device
        for sink in audio_device.sinks.drain(..) {
            // Stop playing audio on the sink
            sink.stop();
        }
    }
    
    // A helper function to clear any empty sinks
    #[cfg(feature = "audio")]
    fn clean_sinks(devices: &mut FnvHashMap<String, AudioDevice>) {
        // TODO: Consider replacing with experimental function drain_filter
        // Look through each device
        for (_, audio_device) in devices.iter_mut() {
            // Remove any sinks that are empty
            let mut index = 0;
            while index != audio_device.sinks.len() {
                // Check to see if the sink is empty
                match audio_device.sinks[index].empty() {
                    // Remove empty sinks
                    true => { audio_device.sinks.remove(index); }

                    // Just increment otherwise
                    _ => { index += 1 }
                }
            }
        }
    }
}

// Implement the event connection trait for Audio Out
impl EventConnection for AudioOut {
    /// A method to receive a new event, empty for this connection type
    ///
    fn read_events(&mut self) -> Vec<ReadResult> {
        Vec::new() // return an empty vector
    }

    /// A method to send a new event to the audio connection, active version
    ///
    #[cfg(feature = "audio")]
    fn write_event(&mut self, id: ItemId, _data1: u32, _data2: u32) -> Result<(), Error> {
        // Clean any empty sinks
        AudioOut::clean_sinks(&mut self.devices);
        
        // Check to see if the event is all stop
        if id == ItemId::all_stop() {            
            // Run all of the all stop audio, ignoring errors
            for audio_cue in self.all_stop_audio.iter() {
                // Add the audio cue
                AudioOut::add_cue(&mut self.devices, audio_cue.clone()).unwrap_or(());
            }

        // Check to see if the event is in the audio map
        } else {
            if let Some(audio_cue) = self.audio_map.get(&id) {
                
                AudioOut::add_cue(&mut self.devices, audio_cue.clone())?;
            }
        }

        // If the event wasn't found or was processed correctly, indicate success
        Ok(())
    }
    
    /// A method to send a new event to the audio connection, inactive version
    ///
    #[cfg(not(feature = "audio"))]
    fn write_event(&mut self, id: ItemId, _data1: u32, _data2: u32) -> Result<(), Error> {
        // Check to see if the event is in the map
        if let Some(_) = self.audio_map.get(&id) {
            return Err(format_err!("Program compiled without audio support. See documentation."));
        } else {
            return Ok(());
        }
    }

    /// A method to echo an event to the audio connection
    fn echo_event(&mut self, id: ItemId, data1: u32, data2: u32) -> Result<(), Error> {
        self.write_event(id, data1, data2)
    }
}

// Tests of the AudioOut module
#[cfg(test)]
mod tests {
    use super::*;

    // Import the library items for the testing function
    use std::thread;
    use std::time::{Duration, Instant};

    // Test the function by
    fn main() {
        unimplemented!();
    }
}
