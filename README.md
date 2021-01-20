# Minerva
#### Interactive Show Control

Quickly configure and control an interactive show, no programming experience necessary.

* **Plug-And-Play**: Control video, audio, DMX, LEDs, microcontrollers, and other common show elements with the connection modules. You can run everthing on a single computer or seamlessly connect multiple computers for large installations.

* **Interactive**: Trigger linked events from the user interface, connected devices, or the web endpoint. Minerva supports multiple protocols for bidirectional communication with microcontrollers (Arduino, RaspberryPi, ESP, and others) and includes a simple framework for new protocols.

* **Reliable**: Minerva is written in pure Rust, a threadsafe language. The software has been extensively tested (in real-world installations) and includes an optional live-backup feature to resume instantly if power is lost.

### In Active Development

Minerva is under very active development (one or two significant updates each year) and we are currently migrating to a web interface. You can find a stable copy of the GTK interface under the [gtk-interface branch](https://github.com/decode-detroit/minerva/tree/gtk-interface) which is locked at version 0.8.3.

## Getting Started

If you're on a 64-bit GNU/Linux system, you can use the the [binary release here](https://github.com/decode-detroit/minerva/releases) and skip down to [Installing Extras](#Installing-Extras) below.

If you're on Windows or Mac, binaries are still a work in progress.

## Compile From Source (Cross-Platform)

If you would like to contribute to Minerva, or if you are on Windows or Mac, you'll need to compile from source. Start with these prerequisites.

### Prerequisites

You'll need Rust and GTK+ to compile and run Minerva.

* Installation of Rust: https://www.rust-lang.org/
* Installation of GTK+: https://www.gtk.org/ (This is usually installed already on GNU/Linux systems. Search for package libgtk-3-0.)

Follow the directions on both websites to download and install these tools before you proceed.

### Compiling

Once you have installed the two prerequities above, clone or download this repository. Then compile and run the program using Cargo (included with Rust):
```
cargo run
```

This will take several minutes to download all the components. You'll be left with a running Minerva instance with an example configuration loaded. You can use
```
cargo run
```

to run Minerva again (it will not recompile this time). This is a debug version (larger file, but otherwise perfectly functional).

To compile a finished copy for deployment, use
```
cargo build --release
```

The completed binary will be located in the automatically generated "target/release" folder with the name "minerva".

Note: This procedure should work on all systems. If you run into an error or bug, let us know!

### Issues Compiling

If you run into issues with glib-2.0 or gdk-3.0, you can run these commands on a Debian-like system:

glib2.0 issue: 
```
sudo apt install libgtk2.0
```

gdk-3.0 issue:
```
sudo apt install build-essential libgtk-3-dev
```

## Installing Extras

Extras! Everyone loves extras. To take advantage of all Minerva's features, you'll need ZMQ bindings, the Gstreamer library, and a Redis server.

* **ZMQ bindings** provide an easy and reliable way to network your devices.
* **GStreamer** controls media playback directly within Minerva.
* **Redis** provides real-time crash recovery.

You'll need to install these tools on whichever computers you would like to **run** Minerva.

### ZMQ for Communication

Enable reliable messaging to other devices by compiling with the "zmq-comm" feature.
```
cargo build --features "zmq-comm"
```

You can install ZMQ bindings on a Debian-like system with
```
apt install libzmq3-dev
```

Currently, rust-zmq requires ZeroMQ 4.1. If your operating system does not provide packages of a new-enough libzmq, you will have to install it from source. See https://github.com/zeromq/libzmq/releases.

### GStreamer for Audio/Video

Enable audio and video playback by compiling with the "media-out" feature.
```
cargo build --features "media-out"
```

To meet the media playback dependancies, you will need to follow the playform-specific instructions for GStreamer-rs: https://gitlab.freedesktop.org/gstreamer/gstreamer-rs

This replaces the separate audio and video features in previous versions and syncronizes the options available to both.

Audio output supports Alsa and Pulse Audio. Each output has its advantages - documentation forthcoming.

### Redis for Instant Recovery

The most up-to-date instructions for installing Redis can be found here: https://redis.io/. You'll also need to copy the [redis server configuration](examples/redis.conf) into the Redis configuration folder.

### Make It Pretty

GTK can be easily re-themed. We recommend the Materia Dark theme for Minerva which will automatically load if you install the Materia theme package (See here: https://github.com/nana-4/materia-theme). On a GNU/Linux system, simply install the materia-gtk-theme package.

We are migrating to a web interface, so this will not be necessary in the long term.

## Contributing

Please join us in the pursuit of free and open source software for the arts! Email patton@DecodeDetroit.com to discuss how your skills might help the project.

## License

This project is licensed under the GNU GPL Version 3 - see the [LICENSE](LICENSE) file for details

Thanks to all the wonderful free and open source people out there who have made this project possible, especially Mozilla et al. for creating such a beautiful language, the folks at Arduino for creating the ubiquitous microcontroller platform, and the team at Adafruit for their tireless committment to open source hardware.
