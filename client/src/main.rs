extern crate libusb;
extern crate nxusb;

use nxusb::prefixes::{Prefixes, ReadPrefix, WritePrefix};

pub mod interface;
use interface::ClientDevice;

pub mod commands;
use commands::{ClientCommandState, ReadState, WriteState, FileContentStorer, FileRetriever};

pub mod libusb_impl;
use libusb_impl::fileio::StdFile;
use libusb_impl::usbcom::UsbClient;

pub mod test_impl;

const SWITCH_VENDOR_ID: u16 = 1406;
const SWITCH_PRODUCT_ID: u16 = 12288;

const USAGE: &str = "Usage: nxusb_client [--push | --pull] [PATH ON SWITCH] [PATH ON COMPUTER]";

fn main() -> Result<(), String> {
    let mut args = std::env::args();
    if args.len() != 4 {
        println!("{}", USAGE);
        return Err(format!("Could not parse args: {:?}", args));
    }

    let _ = args.next();
    let push_string = parse_arg(&mut args)?;

    let should_push = if push_string == "--pull" {
        false
    } else if push_string == "--push" {
        true
    } else {
        println!("{}", USAGE);
        return Err(format!("Could not parse args: {:?}", args));
    };
    let switch_path = parse_arg(&mut args)?;
    let computer_path = parse_arg(&mut args)?;

    let mut usb_ctx: libusb::Context =
        libusb::Context::new().map_err(|e| format!("Usb context create err: {:?}", e))?;

    let mut nx_device =
        UsbClient::from_vendor_product(&mut usb_ctx, SWITCH_VENDOR_ID, SWITCH_PRODUCT_ID)?;
    if should_push {
        copy_to_switch(&mut nx_device, &switch_path, &computer_path).map(|_| ())
    } else {
        copy_from_switch(&mut nx_device, &switch_path, &computer_path).map(|_| ())
    }
}

#[inline]
fn parse_arg(args: &mut std::env::Args) -> Result<String, String> {
    match args.next() {
        Some(c) => Ok(c),
        _ => {
            println!("{}", USAGE);
            return Err(format!("Could not parse args: {:?}", args));
        }
    }
}

fn copy_from_switch(
    client: &mut UsbClient,
    switch_path: &str,
    computer_path: &str,
) -> Result<usize, String> {
    let prefix = ReadPrefix {
        flags: 0,
        file_name_length: switch_path.len() as u16,
    };
    client.push_prefix(Prefixes::Read(prefix))?;
    let mut command_state = ReadState::<StdFile>::new_read(prefix, switch_path, computer_path)?;
    let mut buffer: Vec<u8> = Vec::with_capacity(client.block_size());
    buffer.resize(client.block_size(), 0);
    loop {
        if command_state.needs_pull() {
            client.pull_block(&mut buffer)?;
            command_state.pull_block(&buffer)?;
        } else if command_state.needs_push() {
            command_state.push_block(&mut buffer)?;
            client.push_block(&buffer)?;
        } else {
            break;
        }
    }
    Ok(command_state.file_size)
}

fn copy_to_switch(
    client: &mut UsbClient,
    switch_path: &str,
    computer_path: &str,
) -> Result<usize, String> { 
    let fl = StdFile::open_file(computer_path)?;
    let prefix = WritePrefix {
        flags: u16::max_value(),
        file_name_length: switch_path.len() as u16,
        file_length : fl.len() as u32
    };
    client.push_prefix(Prefixes::Write(prefix))?;
    let mut command_state = WriteState::<StdFile>::new_write(prefix, switch_path, computer_path)?;
    let mut buffer: Vec<u8> = Vec::with_capacity(client.block_size());
    buffer.resize(client.block_size(), 0);
    loop {
        if command_state.needs_pull() {
            client.pull_block(&mut buffer)?;
            command_state.pull_block(&buffer)?;
        } else if command_state.needs_push() {
            command_state.push_block(&mut buffer)?;
            client.push_block(&buffer)?;
        } else {
            break;
        }
    }
    Ok(command_state.prefix().file_length as usize)
}
