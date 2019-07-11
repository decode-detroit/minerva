# Minerva: Internet Of Things Made Simple

Program and Synchronize your internet of things in One Step.

Minerva is a programming suite for networks of microcontrollers. Write one configuration for your whole system and Minerva will program the relevant code onto each device.

In addition, Minerva can be used as a master device on the network for operator input.

Minerva is in active development, and although these are our aspirations, at the moment Minerva only serves as a master operations device. Please join in the development to solve the headache of programming the internet of things!

## IN ACTIVE DEVELOPMENT

Minerva (and her corresponding hardware components) are under very active development (several breaking changes per month). If you intend to use Minerva in a live deployment, please contact us so that we can make sure to get you a stable copy.

### Prerequisites

You'll need Rust and GTK+ to compile and run Minerva. Binaries will be available soon!

* Installation of Rust: https://www.rust-lang.org/
* Installation of GTK+: https://www.gtk.org/ (This is usually installed already on GNU/Linux systems. Search for package libgtk-3-0.)

To take advantage of all Minerva's features, you'll need ZMQ bindings and a Redis server.
* ZMQ bindings provide an easy and reliable way to network your devices. On GNU/Linux systems, you can install the package libzmq3-dev.
* Redis provides real-time crash recovery to make sure your systems run reliably.

Installation instructions for these are below in "Installing Extras". If you're just looking to experiment, you can safely get by without these features. :)

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

## Contributing

Please join us in the pursuit of easier Internet of Things! Email patton@DecodeDetroit.com to see how your skills might help the project.

## License

This project is licensed under the GNU GPL Version 3 - see the [LICENSE](LICENSE) file for details

Thanks to all the wonderful free and open source people out there who have made this project possible, especially the folks at Rust for creating such a beautiful language for Minerva and the folks at Arduino for creating a ubiquitous microcontroller platform.
