# Minerva
#### Interactive Show Control

Quickly configure and control an interactive show, no programming experience necessary.

* **Plug-And-Play**: Control video, audio, DMX, LEDs, microcontrollers, and other common show elements with the connection modules. You can run everthing on a single computer or seamlessly connect multiple computers for large installations.

* **Interactive**: Trigger linked events from the user interface, connected devices, or the web endpoint. Minerva supports multiple protocols for bidirectional communication with microcontrollers (Arduino, RaspberryPi, ESP, and others) and includes a simple framework for new protocols.

* **Reliable**: Minerva is written in pure Rust, a threadsafe language. The software has been extensively tested (in real-world installations) and includes an optional live-backup feature to resume instantly if power is lost.

## Getting Started

If you're on a 64-bit GNU/Linux system or 32-bit Raspberry Pi, you can use the the [binary release here](https://github.com/decode-detroit/minerva/releases) and skip down to [Installing Extras](#Installing-Extras) below.

If you're on Windows or Mac, binaries are still a work in progress.

## Compile From Source (Cross-Platform)

If you would like to contribute to Minerva, or if you are on Windows or Mac, you'll need to compile from source. Start with these prerequisites.

### Prerequisites

You'll need Rust to compile and run Minerva.

* Installation of Rust: https://www.rust-lang.org/

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

## Installing Extras

Extras! Everyone loves extras. To take advantage of all Minerva's features, you'll need ZMQ bindings, the Gstreamer library, and a Redis server.

* **ZMQ bindings** provide an easy and reliable way to network your devices.
* Sister program **Apollo** controls media playback directly from Minerva.
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

### Apollo for Audio/Video

Audio and video playback support is built in to Minerva by default.

Minerva uses an external program, [Apollo](https://github.com/decode-detroit/apollo), for all media playback. The two projects are developed concurrently and are separate to improve reliability and reusability.

### Redis for Instant Recovery

The most up-to-date instructions for installing Redis can be found here: https://redis.io/.

The default configuration should work just fine for most purposes. For super high reliabilty, you'll want to make sure every change is written to the disk (add to redis.conf):
```
save 60 1
```

### DMX For Lighting/Effects Control

The DMX connection doesn't require any additional software or libraries to run and is included by default.

On Debian-like systems, you may need to add your user to the dialout group:
```
sudo adduser $USER dialout
```
You'll need to log out and log back in for this to take effect.

All DMX channels default to 0. This can cause confusion when the channel isn't explicitly set by the user, but is nonetheless necessary for the device to function. For example, the main dimmer channel on a light fixture needs to be manually set to 255.

## Raspberry Pi-like Systems (ARM)

It's possible to run Minerva on less-capible systems! For example, a Raspberry Pi 4 can manage most of the tasks of a full computer (video is a bit touchy).

Take careful notes of the steps to
* cross-compile Minerva, and
* setup your Raspberry Pi host to run Minerva

Note: These instructions are written for *compiling* the software on Ubuntu 20.04.

### Cross-Compiling To Raspbian (armhf, 32bit)

Note: These settings are largely analogous for arm64, but the 64-bit version hasn't been tested.

To cross-compile, install the correct rust target and install the linker.
```
rustup target add armv7-unknown-linux-gnueabihf
sudo apt install gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf
```
You'll also need to add the armhf architecture to dpkg.
```
sudo dpkg --add-architecture armhf
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

Install the dev packages for the new architecture.
```
sudo apt update
sudo apt install libssl-dev:armhf
```

Compile the program using the special armhf build target:
```
env PKG_CONFIG_ALLOW_CROSS=1 PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabihf/pkgconfig/ cargo build_armhf
```

#### Prepare Your Raspberry Pi

In addition to any packages above (e.g. ZMQ), you need to cross compile [Apollo](https://github.com/decode-detroit/apollo) and enable the corresponding changes to the Raspberry Pi listed there for video playback.

Hardware decoding works well for videos up to 1080p at 30 fps. There is a short delay when switching between playing videos, but there is no delay when playing a new video after the first has stopped.

## Contributing

Please join us in the pursuit of free and open source software for the arts! Email patton@DecodeDetroit.com to discuss how your skills might help the project.

## License

This project is licensed under the GNU GPL Version 3 - see the [LICENSE](LICENSE) file for details

Thanks to all the wonderful free and open source people out there who have made this project possible, especially Mozilla et al. for a beautiful language, the folks at Arduino for the ubiquitous microcontroller platform, and the team at Adafruit for their tireless committment to open source hardware.
