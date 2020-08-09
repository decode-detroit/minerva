# Minerva: Dynamic Show Control Software

Minerva allows you to quickly program and control an interactive show. Connection modules allow real-time control of video, audio, DMX, and other common show elements.

In addition, Minerva supports multiple protocols for interacting with microcontroller devices (Arduino, ESP, and others) and includes a simple framework for new protocols. Submit an issue to request their addition! 

Minerva is written in pure Rust with bindings into several C libaries to support audio and video. The software is extensively tested (in the real world) to be robust, and includes an optional live-backup feature to resume the show if power is lost.

Minerva is in active development, and we are currently migrating to a web interface. You can find a stable copy of the GTK interface under the gtk-interface branch.

## IN ACTIVE DEVELOPMENT

Minerva is under very active development (several breaking changes per year). If you intend to use Minerva in a live deployment, we recommend you use the gtk-interface branch which is locked at version 0.8.3.

### Prerequisites

You'll need Rust and GTK+ to compile and run Minerva. Binaries will be available soon!

* Installation of Rust: https://www.rust-lang.org/
* Installation of GTK+: https://www.gtk.org/ (This is usually installed already on GNU/Linux systems. Search for package libgtk-3-0.)

To take advantage of all Minerva's features, you'll need ZMQ bindings, an audio library (platform specific) and a Redis server.
* ZMQ bindings provide an easy and reliable way to network your devices. On GNU/Linux systems, you can install the package libzmq3-dev.
* Audio support allows you to trigger and control audio playback directly within Minerva.
* Redis provides real-time crash recovery to make sure your systems run reliably.

Installation instructions for these are below in "Installing Extras". If you're just looking to experiment, you can safely get by without these features. :)


### Issues Installing
If you run into issues with glib-2.0 or gdk-3.0, you can run these commands on a Debian-like system:

glib2.0 issue: 

```
sudo apt install libgtk2.0
```

gdk-3.0 issue:

```
sudo apt install build-essential libgtk-3-dev
```


### Installing

Clone or download this repository. Then compile and run the program using Cargo (included with Rust):

```
cargo run
```

This will take several minutes to download all the components. You'll be left with a running Minerva instance with an example configuration loaded. You can use

```
cargo run
```

to run Minerva again (it will not recompile this time), or run the binary directly (located in the generated "target" folder)

Play around! The example configuration has an example of most features.

Note: This procedure should work on all systems. If you run into an error or bug, let us know!

### Installing Extras

Install ZMQ bindings on a Debian-like system with

```
apt install libzmq3-dev
```

Currently, rust-zmq requires ZeroMQ 4.1. If your OS of choice does not provide packages of a new-enough libzmq, you will have to install it from source. See https://github.com/zeromq/libzmq/releases.

The most up-to-date instructions for installing Redis can be found here: https://redis.io/. You'll also need to copy the [redis server configuration](examples/redis.conf) into the Redis configuration folder.

Enable audio support by compiling with the "audio" feature.

```
cargo build --features audio
```

To meet the audio dependencies, you will need to follow the platform-specific instructions for Rust CPAL: https://github.com/RustAudio/cpal.

Enable video support by compiling with the "video" feature.

```
cargo build --features video
```

To meet the video dependancies, you will need to follow the playform-specific instructions for GStreamer-rs: https://gitlab.freedesktop.org/gstreamer/gstreamer-rs

The video playback is still in beta status, so please report any bugs/crashes you encounter!

### Make It Pretty!

GTK can be easily re-themed. We recommend the Materia Dark theme for Minerva which will automatically load if you install the Materia theme package (See here: https://github.com/nana-4/materia-theme). On GNU/Linux system, simply install the materia-gtk-theme package.

## Contributing

Please join us in the pursuit of easier Internet of Things! Email patton@DecodeDetroit.com to see how your skills might help the project.

## License

This project is licensed under the GNU GPL Version 3 - see the [LICENSE](LICENSE) file for details

Thanks to all the wonderful free and open source people out there who have made this project possible, especially the folks at Rust for creating such a beautiful language for Minerva and the folks at Arduino for creating a ubiquitous microcontroller platform.
