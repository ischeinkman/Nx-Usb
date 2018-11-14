# NX-USB

## Description

Nx-USB allows you to transfer files between your Nintendo Switch and computer, no WiFi or rebooting required!

## Usage

Nx-USB is composed of 2 parts: a "server" component that runs on the switch, called `nxusb_server.nro`, and a "client" component that runs each command on the computer.

### Setup

1. Grab a copy of `nxusb_server.nro`. This can either be done by building it yourself, or just by downloading it from the Releases page.

2. Make sure that you have gotten `nxusb_server.nro` on to your Switch somewhere where it can be ran from.

3. Find a valid client, which at this point is only possible by downloading Rust and building it yourself via `cargo build -p client`. At the moment, the client executable is just called `client`, so that is what we will be using to refer to it here, but this may change in the future.

### Running

1. Connect the Switch to the computer via USB.

2. Run `nxusb_server.nro` on the Switch.

3. Use the client to transfer files between the computer and the switch:

   * To "push" a file TO the Switch FROM the computer, use `./client --push [NEW PATH ON SWITCH] [EXISTING FILE ON COMPUTER]`.

   * To "pull" a file FROM the Switch TO the computer, use `./client --pull [EXISTING FILE ON SWITCH] [NEW PATH ON COMPUTER`]`

## Development

This project was built in Rust with [libnx-rs](https://github.com/ischeinkman/libnx-rs). Docker is currently the prefered build evironment, but it is perfectly possible to build an `nro` without it as long as you have `devkitpro`, `xargo`, and nightly Rust installed. No matter which environment is being used, you can build an `nro` by calling `./makew`; this builds the correct crate via `xargo` and then converts the `nx_elf` to an `nro`. 