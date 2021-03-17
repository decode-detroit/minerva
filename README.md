# Minerva
#### Interactive Show Control

Quickly configure and control an interactive show, no programming experience necessary.

* **Plug-And-Play**: Control video, audio, DMX, LEDs, microcontrollers, and other common show elements with the connection modules. You can run everthing on a single computer or seamlessly connect multiple computers for large installations.

* **Interactive**: Trigger linked events from the user interface, connected devices, or the web endpoint. Minerva supports multiple protocols for bidirectional communication with microcontrollers (Arduino, RaspberryPi, ESP, and others) and includes a simple framework for new protocols.

* **Reliable**: Minerva is written in pure Rust, a threadsafe language. The software has been extensively tested (in real-world installations) and includes an optional live-backup feature to resume instantly if power is lost.

### In Active Development

Minerva is in active development and we are migrating to a web interface. You can find a stable copy of the GTK interface under the [gtk-interface branch](https://github.com/decode-detroit/minerva/tree/gtk-interface) which is locked at version 0.9.0.

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

### Issues Compiling

If you run into issues with glib-2.0 or gdk-3.0, you can run these commands on a Debian-like system:

glib2.0 issue: 
```
sudo apt install libgtk2.0-dev
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
sudo apt install libzmq3-dev
```

Currently, rust-zmq requires ZeroMQ 4.1. If your operating system does not provide packages of a new-enough libzmq, you will have to install it from source. See https://github.com/zeromq/libzmq/releases.

### GStreamer for Audio/Video

Enable audio and video playback by compiling with the "media-out" feature.
```
cargo build --features "media-out"
```

To meet the media playback dependancies, you will need to follow the platform-specific instructions for GStreamer-rs: https://gitlab.freedesktop.org/gstreamer/gstreamer-rs

On a Debian-like system, install gstreamer dependencies with
```
sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav libgstrtspserver-1.0-dev libges-1.0-dev
```

This replaces the separate audio and video features in previous versions and syncronizes the options available to both.

Audio output supports Alsa and Pulse Audio. Each output has its advantages - documentation forthcoming.

### Redis for Instant Recovery

The most up-to-date instructions for installing Redis can be found here: https://redis.io/. You'll also need to copy the [redis server configuration](examples/redis.conf) into the Redis configuration folder.

### DMX For Lighting/Effects Control

The DMX connection doesn't require any additional software or libraries to run and is included by default.

All DMX channels default to 0. This can cause confusion when the channel isn't explicitly set by the user, but is nonetheless necessary for the device to function. For example, the main dimmer channel on a light fixture needs to be manually set to 255.

### Make It Pretty

GTK can be easily re-themed. We recommend the Materia Dark theme for Minerva which will automatically load if you install the Materia theme package (See here: https://github.com/nana-4/materia-theme). On a GNU/Linux system, simply install the materia-gtk-theme package.

We are migrating to a web interface, so this will not be necessary in the long term.

## Raspberry Pi-like Systems (ARM)

It's possible to run Minerva on less-capible systems! For example, a Raspberry Pi 4 can manage most of the tasks of a full computer (video is a bit touchy - working on it).

Take careful notes of the steps to
* cross-compile Minerva, and
* setup your Raspberry Pi host to run Minerva

Note: These instructions are written for *compiling* the software on Ubuntu 20.04.

### Cross-Compiling To Ubuntu (arm64, 64bit)

To cross-compile, install the correct rust target and install the linker.
```
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu
```
You'll also need to add the arm64 architecture to dpkg.
```
sudo dpkg add-architecture arm64
```
And add these sources to the end of /etc/apt/sources.list.
```
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports/ focal main restricted
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports/ focal-updates main restricted+
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports/ focal universe
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports/ focal-updates universe
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports/ focal multiverse
deb [arch=arm64] http://ports.ubuntu.com/ubuntu-ports/ focal-updates multiverse
```
Make sure to add `[arch=amd64]` to the other sources while you're at it.

Install the GTK, ZMQ and GStreamer dev packages for the new architecture.
```
sudo apt update
sudo apt install libgtk-3-dev:arm64 libzmq3-dev:arm64 libgstreamer1.0-dev:arm64 libgstreamer-plugins-base1.0-dev:arm64 gstreamer1.0-plugins-base:arm64 gstreamer1.0-plugins-good:arm64 gstreamer1.0-plugins-bad:arm64 gstreamer1.0-plugins-ugly:arm64 gstreamer1.0-libav:arm64 libgstrtspserver-1.0-dev:arm64 libges-1.0-dev:arm64 libges-1.0-0:arm64
```

When you compile, pass several environment variables to the compilation.
```
env PKG_CONFIG_ALLOW_CROSS=1 PKG_CONFIG_PATH=/usr/lib/aarch-linux-gnu/pkgconfig/ cargo build_arm64
```

#### Prepare Your Raspberry Pi

In addition to all the packages above (e.g. ZMQ, GStreamer), you need to load the correct device tree overlay to enable video playback on a Raspberry Pi 4,

Add this to /boot/firmware/config.txt (it may tell you to put it in usercfg.txt instead)
```
dtoverlay=vc4-fkms-v3d
max_framebuffers=2
gpu_mem=512
```

Reboot, and voila! Still working out the bugs, but hardware decoding works well for smaller videos (>720p).

### Cross-Compiling To Raspbian (armhf, 32bit)

Note: Several settings here will conflict with the instructions for arm64 - you likely can't have both on the same system.

To cross-compile, install the correct rust target and install the linker.
```
rustup target add armv7-unknown-linux-gnueabihf
sudo apt install gcc-arm-linux-gnueabihf
```
You'll also need to add the armhf architecture to dpkg.
```
sudo dpkg add-architecture armhf
```
And add these sources to the end of /etc/apt/sources.list.
```
deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ focal main restricted
deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ focal-updates main restricted+
deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ focal universe
deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ focal-updates universe
deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ focal multiverse
deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ focal-updates multiverse
```
Make sure to add `[arch=amd64]` to the other sources while you're at it.

Install the gtk dev packages for the new architecture.
```
sudo apt update
sudo apt install libgtk-3-dev:armhf libzmq3-dev:armhf libgstreamer1.0-dev:armhf libgstreamer-plugins-base1.0-dev:armhf gstreamer1.0-plugins-base:armhf gstreamer1.0-plugins-good:armhf gstreamer1.0-plugins-bad:armhf gstreamer1.0-plugins-ugly:armhf gstreamer1.0-libav:armhf libgstrtspserver-1.0-dev:armhf libges-1.0-dev:armhf libges-1.0-0:armhf
```

When you compile, pass several environment variables to the compilation.
```
env PKG_CONFIG_ALLOW_CROSS=1 PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabihf/pkgconfig/ cargo build_armhf
```

Unfortunately, video doesn't seem to work out of the box. If you have success with armhf and video playback, let us know how you pulled it off!

## Contributing

Please join us in the pursuit of free and open source software for the arts! Email patton@DecodeDetroit.com to discuss how your skills might help the project.

## License

This project is licensed under the GNU GPL Version 3 - see the [LICENSE](LICENSE) file for details

Thanks to all the wonderful free and open source people out there who have made this project possible, especially Mozilla et al. for creating such a beautiful language, the folks at Arduino for creating the ubiquitous microcontroller platform, and the team at Adafruit for their tireless committment to open source hardware.
