extern crate libusb;
extern crate nxusb;

use nxusb::prefixes::{Prefixes, ReadPrefix, WritePrefix, PREFIX_LENGTH};

pub mod interface;
use interface::ClientDevice;

pub mod commands;
use commands::{ClientCommandState, ReadState};

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
        Err("Push not yet implemented!".to_owned())
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
            println!("Trying to pull for a read.");
            client.pull_block(&mut buffer)?;
            command_state.pull_block(&buffer)?;
        } else if command_state.needs_push() {
            println!("Trying to push for a read.");
            client.push_block(&buffer)?;
            command_state.push_block(&mut buffer)?;
        } else {
            break;
        }
    }
    Ok(command_state.file_size)
}

fn usb_basic_test(usb_ctx: &mut libusb::Context) -> Result<(), String> {
    let devices = usb_ctx
        .devices()
        .map_err(|e| format!("Device iter create err: {:?}", e))?;
    for device in devices.iter() {
        println!("\n\n");
        match test_device(device) {
            Ok(_) => {}
            Err(e) => eprintln!("Found err testing device: {:?}", e),
        }
    }
    Ok(())
}
fn test_device(device: libusb::Device) -> Result<(), String> {
    let timeout = std::time::Duration::from_millis(100);
    let desc = device
        .device_descriptor()
        .map_err(|e| format!("Error getting desc: {:?}", e))?;
    let conf = device
        .active_config_descriptor()
        .map_err(|e| format!("Error getting conf: {:?}", e))?;
    let mut handle: libusb::DeviceHandle = match device.open() {
        Ok(d) => d,
        Err(e) => {
            return Err(format!("Found error reading device {:?}: {:?}", desc, e));
        }
    };
    let _ = handle.reset().map_err(|e| format!("Reset err: {:?}", e))?;

    let langs: Vec<libusb::Language> = handle.read_languages(timeout).unwrap();
    let lang = langs
        .into_iter()
        .find(|l| l.primary_language() == libusb::PrimaryLanguage::English)
        .unwrap();
    println!(
        "Found device {:?} (Serial : {:?}) by {:?}",
        handle
            .read_product_string(lang, &desc, timeout)
            .map_err(|e| format!("Device prod read err: {:?}", e))?,
        handle
            .read_manufacturer_string(lang, &desc, timeout)
            .map_err(|e| format!("Device manu read err: {:?}", e))?,
        handle
            .read_serial_number_string(lang, &desc, timeout)
            .map_err(|e| format!("Device serial read err: {:?}", e))?
    );
    println!("Vid: {}, Pid: {}", desc.vendor_id(), desc.product_id());

    for iface in conf.interfaces() {
        println!("Found iface number {}.", iface.number());
        for iface_desc in iface.descriptors() {
            println!("Found iface_desc: {:?}", iface_desc);
            for endpt_desc in iface_desc.endpoint_descriptors() {
                println!("Found endpoint_desc: {:?}", endpt_desc);
            }
        }
    }

    Ok(())
}
