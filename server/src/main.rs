extern crate libc;

pub mod commands;

pub mod interface;

extern crate nxusb;
pub use nxusb::prefixes;

pub mod libnx_impl;

pub mod test_impl;

use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::panic;
use std::result::Result;

extern crate libnx_rs;
use libnx_rs::{console, usbcomms};
pub fn main() {
    let mut stderr = match redirect_stderr("nxusb.stderr.txt") {
        Ok(f) => f,
        Err(_) => {
            return;
        }
    };
    let rval = panic::catch_unwind(runner);
    if let Err(_) = rval {
        eprintln!("Caught a panic in runner!");
    } else if let Ok(Err(e)) = rval {
        eprintln!("Runner got an error : {:?}", e);
    }

    let _f = stderr.flush();
}

pub fn runner() -> Result<(), String> {
    eprintln!("Initing console.");
    let mut debug = console::ConsoleHandle::default();
    let mut usb_interface = [usbcomms::UsbCommsInterface::default()];
    eprintln!("Initing interface array{:?}", usb_interface);
    eprintln!("Initing UsbCommsContext.");
    let _usb_context = usbcomms::UsbCommsContext::initialize(&mut usb_interface)
        .map_err(|e| format!("Libnx Error: {:?}", e))?;
    eprintln!("Creating empty echo_buffer.");
    let mut echo_buffer = [0u8; 100];
    while echo_buffer[0..4] != [b'q', b'u', b'i', b't'] {
        println!(
            "Writing bytes to interface {:?}: {:?}",
            usb_interface,
            echo_buffer.as_ref()
        );
        eprintln!(
            "Writing bytes to interface {:?}: {:?}",
            usb_interface,
            echo_buffer.as_ref()
        );
        usb_interface[0].write_bytes(&mut echo_buffer);
        eprintln!("Reading bytes from interface {:?}", usb_interface);
        usb_interface[0].read_bytes(&mut echo_buffer);
        debug.update();
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
