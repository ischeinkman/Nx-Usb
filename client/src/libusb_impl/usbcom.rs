use interface::ClientDevice;
use libusb::{Context, Device, DeviceDescriptor, DeviceHandle, TransferType};
use nxusb::prefixes::{CommandPrefix, Prefixes};
use std::time::Duration;

#[derive(Debug, Copy, Clone)]
struct Endpoint {
    config: u8,
    iface: u8,
    setting: u8,
    address: u8,
}

#[derive(Debug, Copy, Clone)]
struct ReadEndpoint(Endpoint);

#[derive(Debug, Copy, Clone)]
struct WriteEndpoint(Endpoint);
pub struct UsbClient<'a> {
    device_handle: DeviceHandle<'a>,
    read_endpoint: ReadEndpoint,
    write_endpoint: WriteEndpoint,
}

impl<'a> UsbClient<'a> {
    pub fn from_vendor_product(ctx: &'a mut Context, vid: u16, pid: u16) -> Result<Self, String> {
        let (mut device, device_desc, mut device_handle) = open_device(ctx, vid, pid)?;
        let (read_endpoint, write_endpoint) = find_bulk_endpoints(&mut device, &device_desc)?;
        device_handle.reset().map_err(|e| format!("Found reset err: {:?}", e))?;
        device_handle.claim_interface(read_endpoint.0.iface).map_err(|e| format!("Could not claim iface {}: {:?}", read_endpoint.0.iface, e))?;
        //device_handle.unconfigure().map_err(|e| format!("Unconfigure err: {:?}", e))?;
        //device_handle.set_active_configuration(read_endpoint.0.config).map_err(|e| format!("Could not set active config: {:?}", e))?;
        Ok(UsbClient {
            device_handle, 
            read_endpoint, 
            write_endpoint
        })
    }
}

const CLIENT_BLOCK_SIZE : usize = 256;
impl<'a> ClientDevice for UsbClient<'a> {
    fn push_prefix(&mut self, prefix: Prefixes) -> Result<usize, String> {
        let bts = prefix.serialize();
        push_bytes(&mut self.device_handle, &self.write_endpoint, &bts)
    }
    fn block_size(&self) -> usize {
        CLIENT_BLOCK_SIZE
    }
    fn pull_block(&mut self, buffer: &mut [u8]) -> Result<usize, String> {
        pull_bytes(&mut self.device_handle, &self.read_endpoint, buffer)
    }
    fn push_block(&mut self, bytes: &[u8]) -> Result<usize, String> {
        push_bytes(&mut self.device_handle, &self.write_endpoint, &bytes)
    }
}

fn open_device(
    context: &mut Context,
    vid: u16,
    pid: u16,
) -> Result<(Device, DeviceDescriptor, DeviceHandle), String> {
    let devices = context
        .devices()
        .map_err(|e| format!("Device list open err: {:?}", e))?;
    for device in devices.iter() {
        println!("Looking for device with pid {}, vid{}", pid, vid);
        let device_desc: DeviceDescriptor = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };
        println!("Now checking device with descriptor {:?}", device_desc);

        if device_desc.vendor_id() == vid && device_desc.product_id() == pid {
            println!("Device matches!");
            return device
                .open()
                .map(|handle| (device, device_desc, handle))
                .map_err(|e| format!("Handle open err: {:?}", e));
        }
    }
    Err(format!(
        "Could not find USB device with Vendor ID {} and Product ID {}.",
        vid, pid
    ))
}


fn find_bulk_endpoints(
    device: &mut Device,
    desc: &DeviceDescriptor,
) -> Result<(ReadEndpoint, WriteEndpoint), String> {
    println!("Now checking for endpoints.");
    let mut read_endpoint: Option<ReadEndpoint> = None;
    let mut write_endpoint: Option<WriteEndpoint> = None;
    for config_idx in 0..desc.num_configurations() {
        let config_desc = match device.config_descriptor(config_idx) {
            Ok(c) => c,
            Err(_) => continue,
        };
        println!("Checking config {} -> {:?}", config_idx, config_desc);

        for interface in config_desc.interfaces() {
            for interface_desc in interface.descriptors() {
                println!("Checking interface descriptor {:?}",  interface_desc);
                let (mut read_endpoints, mut write_endpoints): (
                    Vec<libusb::EndpointDescriptor>,
                    Vec<libusb::EndpointDescriptor>,
                ) = interface_desc
                    .endpoint_descriptors()
                    .filter(|endpoint_desc| endpoint_desc.transfer_type() == TransferType::Bulk)
                    .partition(|endpoint_desc| endpoint_desc.direction() == libusb::Direction::In);

                let read_ifaces = read_endpoints.len() + read_endpoint.map_or(0, |_| 1);
                if read_ifaces > 1 {
                    return Err("Found too many read_endpoints!".to_owned());
                }
                let write_ifaces = write_endpoints.len() + write_endpoint.map_or(0, |_| 1);
                if write_ifaces > 1 {
                    return Err("Found too many write_endpoints!".to_owned());
                }
                read_endpoint = read_endpoints.pop().map(|endpoint_desc|ReadEndpoint(Endpoint {
                    config : config_desc.number(), 
                    iface : interface_desc.interface_number(), 
                    setting : interface_desc.setting_number(), 
                    address : endpoint_desc.address(), 
                }));
                write_endpoint = write_endpoints.pop().map(|endpoint_desc|WriteEndpoint(Endpoint {
                    config : config_desc.number(), 
                    iface : interface_desc.interface_number(), 
                    setting : interface_desc.setting_number(), 
                    address : endpoint_desc.address(), 
                }));
                println!("Have endpoints {:?}, {:?}", read_endpoint, write_endpoint);
                if read_endpoint.is_some() && write_endpoint.is_some() {
                    return Ok((read_endpoint.unwrap(), write_endpoint.unwrap()));
                }
            }
        }
    }
    Err("Could not find bulk read/write endpoints!".to_owned())
}

fn pull_bytes(handle : &mut DeviceHandle, endpoint : &ReadEndpoint, buffer : &mut [u8]) -> Result<usize, String> {
    let endpoint = endpoint.0;
    let timeout = Duration::from_secs(30);
    let rval = handle.read_bulk(endpoint.address, buffer, timeout).map_err(|e| format!("Read Error: {:?}", e));
    rval
}

fn push_bytes(handle : &mut DeviceHandle, endpoint : &WriteEndpoint, buffer : &[u8]) -> Result<usize, String> {
    let endpoint = endpoint.0;
    let timeout = Duration::from_secs(30);
    let rval = handle.write_bulk(endpoint.address, buffer, timeout).map_err(|e| format!("Write Error: {:?}", e));
    rval
}