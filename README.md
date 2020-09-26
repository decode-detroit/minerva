# Minerva: Dynamic Show Control Software

Minerva allows you to quickly program and control an interactive show. Connection modules allow real-time control of video, audio, DMX, and other common show elements. Show events can be triggered from the UI or a RESTful API.

In addition, Minerva supports multiple protocols for interacting with microcontroller devicesas input or output (Arduino, ESP, and others) and includes a simple framework for new protocols. Submit an issue to request their addition!

Minerva is written in pure Rust with bindings into several C libaries to support communication and audio and video. The software has been extensively tested (in the real world), and includes an optional live-backup feature to resume the show if power is lost.

Minerva is in active development, and we are currently migrating to a web interface. You can find a stable copy of the GTK interface under the gtk-interface branch.

## In Active Development

Minerva is under very active development (one or two breaking changes each year). If you intend to use Minerva in a live deployment, we recommend you use the gtk-interface branch which is locked at version 0.8.3.

### Prerequisites

You'll need Rust and GTK+ to compile and run Minerva. Binaries will be available soon!

* Installation of Rust: https://www.rust-lang.org/
* Installation of GTK+: https://www.gtk.org/ (This is usually installed already on GNU/Linux systems. Search for package libgtk-3-0.)

To take advantage of all Minerva's features, you'll need ZMQ bindings, the gstreamer library, and a Redis server.
* ZMQ bindings provide an easy and reliable way to network your devices. On GNU/Linux systems, you can install the package libzmq3-dev.
* Media support allows you to trigger and control media playback directly within Minerva.
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

Extras! Everyone loves extras. You'll need to install whichever computers you would like to *run* the software (i.e. you can compile without them, but the program won't load until you have them).

Enable reliable messaging to other devices by compiling with the "zmq-comm" feature.

```
cargo build --features zmq-comm
```

You can install ZMQ bindings on a Debian-like system with

```
apt install libzmq3-dev
```

Currently, rust-zmq requires ZeroMQ 4.1. If your OS of choice does not provide packages of a new-enough libzmq, you will have to install it from source. See https://github.com/zeromq/libzmq/releases.

The most up-to-date instructions for installing Redis can be found here: https://redis.io/. You'll also need to copy the [redis server configuration](examples/redis.conf) into the Redis configuration folder.

Enable audio and video playback by compiling with the "media-out" feature.

```
cargo build --features video
```

To meet the media playback dependancies, you will need to follow the playform-specific instructions for GStreamer-rs: https://gitlab.freedesktop.org/gstreamer/gstreamer-rs

This replaces the separate audio and video features in previous versions and syncronizes the options available to both.

### Make It Pretty!

GTK can be easily re-themed. We recommend the Materia Dark theme for Minerva which will automatically load if you install the Materia theme package (See here: https://github.com/nana-4/materia-theme). On GNU/Linux system, simply install the materia-gtk-theme package.

## Contributing

Please join us in the pursuit of easier Internet of Things! Email patton@DecodeDetroit.com to see how your skills might help the project.

## License

This project is licensed under the GNU GPL Version 3 - see the [LICENSE](LICENSE) file for details

Thanks to all the wonderful free and open source people out there who have made this project possible, especially the folks at Mozilla et al. for creating such a beautiful language and the folks at Arduino for creating a ubiquitous microcontroller platform.
