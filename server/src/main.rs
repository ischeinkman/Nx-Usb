extern crate libc;

pub mod commands;
use commands::{ServerCommandState, CommandStates};

pub mod interface;
use interface::ServerDevice;

extern crate nxusb;
pub use nxusb::prefixes;

pub mod libnx_impl;
use libnx_impl::{StdFileReader, StdFileWriter};

pub mod test_impl;

use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::panic;
use std::result::Result;

extern crate libnx_rs;
use libnx_rs::{console, usbcomms};

macro_rules! dprintln {
    () => ({
        println!();
        eprintln!();
    });
    ($($arg:tt)*) => ({
        println!($($arg)*);
        eprintln!($($arg)*);
    })
}
pub fn main() {
    let mut stderr = match redirect_stderr("nxusb_server.stderr.txt") {
        Ok(f) => f,
        Err(_) => {
            return;
        }
    };
    let rval = panic::catch_unwind(server_runner);
    if let Err(_) = rval {
        dprintln!("Caught a panic in runner!");
    } else if let Ok(Err(e)) = rval {
        dprintln!("Runner got an error : {:?}", e);
    }

    let _f = stderr.flush();
}

pub fn test_runner() -> Result<(), String> {
    dprintln!("Initing console.");
    let mut debug = console::ConsoleHandle::default();
    let mut usb_interface = [usbcomms::UsbCommsInterface::default()];
    dprintln!("Initing interface array{:?}", usb_interface);
    dprintln!("Initing UsbCommsContext.");
    let _usb_context = usbcomms::UsbCommsContext::initialize(&mut usb_interface)
        .map_err(|e| format!("Libnx Error: {:?}", e))?;
    dprintln!("Creating empty echo_buffer.");
    let mut echo_buffer = [0u8; 100];
    while echo_buffer[0..4] != [b'q', b'u', b'i', b't'] {
        dprintln!(
            "Writing bytes to interface {:?}: {:?}",
            usb_interface,
            echo_buffer.as_ref()
        );
        dprintln!(
            "Writing bytes to interface {:?}: {:?}",
            usb_interface,
            echo_buffer.as_ref()
        );
        usb_interface[0].write_bytes(&mut echo_buffer);
        dprintln!("Reading bytes from interface {:?}", usb_interface);
        usb_interface[0].read_bytes(&mut echo_buffer);
        debug.update();
    }
    Ok(())
}

pub fn server_runner() -> Result<(), String> {
    dprintln!("Initing console.");
    let mut debug = console::ConsoleHandle::default();
    let mut usb_interfaces = [usbcomms::UsbCommsInterface::default()];
    dprintln!("Initing interface array{:?}", usb_interfaces);
    dprintln!("Initing UsbCommsContext.");
    let _usb_context = usbcomms::UsbCommsContext::initialize(&mut usb_interfaces)
        .map_err(|e| format!("Libnx Error: {:?}", e))?;

    let usb_interface = &mut usb_interfaces[0];

    let mut hid_handle = libnx_rs::hid::HidContext {};
    let controller_handle = hid_handle.get_controller(libnx_rs::hid::HidControllerID::CONTROLLER_P1_AUTO);
    let mut current_command : Option<CommandStates<StdFileReader, StdFileWriter>> = None; 
    loop {
        hid_handle.scan_input();
        if controller_handle.keys_down_raw() & 1024 != 0 {
            break;
        }
        
        debug.update();
        
        if current_command.is_none() {
            dprintln!("Waiting for command prefix.");
            debug.update();
            let prefix = usb_interface.read_prefix()?;
            dprintln!("Found command prefix {:?}", prefix);
            debug.update();
            let command = CommandStates::from_prefix(prefix);
            current_command = Some(command);
        }

        let finished = {
            let command : &mut CommandStates<StdFileReader, StdFileWriter> = current_command.as_mut().ok_or("Error: current command shouldn't be None.")?;
            if command.needs_input() {
                dprintln!("Passing block of input to the current command.");
                let mut buffer : Vec<u8> = Vec::with_capacity(usb_interface.block_size());
                buffer.resize(usb_interface.block_size(), 0);
                usb_interface.read_block(&mut buffer)?;
                dprintln!("Read block.");
                command.input_block(&buffer)?;
                dprintln!("Passed input block.");
                false
            }
            else if command.needs_output() {
                dprintln!("Retrieving block of output from the current command.");
                let mut buffer : Vec<u8> = Vec::with_capacity(usb_interface.block_size());
                buffer.resize(usb_interface.block_size(), 0);
                command.output_block(&mut buffer)?;
                dprintln!("Got bytes to output.");
                usb_interface.write_block(&mut buffer)?;
                dprintln!("Retrieved output block.");
                false
            }
            else {
                dprintln!("Finished command.");
                true
            }
        };

        if finished {
            current_command = None;
        }

    }
    Ok(())
}

pub fn redirect_stderr(filename: &str) -> Result<File, String> {
    let mut outfile = OpenOptions::new()
        .write(true)
        .create(true)
        .open(filename)
        .map_err(|e| format!("io::Error: {}", e).to_owned())?;
    outfile
        .write_fmt(format_args!(
            "Redirecting standard error to {}.\n\n",
            filename
        )).map_err(|e| format!("io::Error: {}", e).to_owned())?;
    let raw_fd = outfile.as_raw_fd();
    let new_fd = unsafe {
        libc::fflush(0 as *mut libc::FILE);
        libc::dup2(raw_fd, libc::STDERR_FILENO)
    };
    if new_fd != libc::STDERR_FILENO {
        Err(format!(
            "Could not call dup2. Ended up redirecting fd {} to {} instead of {}.",
            raw_fd,
            new_fd,
            libc::STDERR_FILENO
        ))
    } else {
        Ok(outfile)
    }
}


