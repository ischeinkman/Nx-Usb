extern crate nx;
use nx::console;
use nx::usbcomms;
use nx::hid;

extern crate fakefat;
use fakefat::{FakeFat, StdFileSystem};

extern crate scsi;
use scsi::scsi::commands::*;
use scsi::scsi::ScsiResponder;
use scsi::{ErrorCause, ScsiError};
use scsi::CommunicationChannel;

use std::io::{Read, Seek, SeekFrom, Write};
pub fn main() {
    nx::no_crash_panic();
    let output = console::Handle::new().unwrap();
    println!("Now creating USB interfaces.");
    output.flush();
    let interfaces = [usbcomms::UsbCommsInterfaceInfo {
        bInterfaceClass : 0x08,
        bInterfaceSubClass : 0x06,
        bInterfaceProtocol : 0x80,
    }];

    let usb_handle = usbcomms::Handle::new_ex(&interfaces).unwrap().unwrap();
    let mut usb_channel = UsbCommunicationChannel { handle : usb_handle, interface : 0};

    println!("Created USB interfaces. Now setting up fake File Allocation Table.");
    output.flush();

    let base = get_base_dir();
    let mut faker = ResponseStruct::new(FakeFat::new(StdFileSystem{}, &base));
    
    println!("FAT created successfully at directory {}. Now running. ", base);
    output.flush();

    while !should_finish() {
        faker.process_command(&mut usb_channel).unwrap();
    }
}

pub fn get_base_dir() -> String {
    "/".to_owned()
}

pub fn should_finish() -> bool {
    let handheld = hid::input_down(hid::Controller::Handheld) | hid::input_held(hid::Controller::Handheld);
    let p1 = hid::input_down(hid::Controller::Player(1)) | hid::input_held(hid::Controller::Player(1));
    let p2 = hid::input_down(hid::Controller::Player(2)) | hid::input_held(hid::Controller::Player(2));
    
    let all = handheld | p1 | p2;
    all & (hid::Key::Plus as u64) != 0
}

pub struct UsbCommunicationChannel {
    handle : usbcomms::Handle, 
    interface : u32, 
}

impl CommunicationChannel for UsbCommunicationChannel {
    fn out_transfer<B: AsRef<[u8]>>(&mut self, bytes: B) -> Result<usize, ScsiError> {
        let bytes = bytes.as_ref();
        let out = self.handle.write_ex(bytes, self.interface);
        Ok(out)
    }
    fn in_transfer<B: AsMut<[u8]>>(&mut self, mut bytes: B) -> Result<usize, ScsiError> {
        let bytes = bytes.as_mut();
        let out = self.handle.read_ex(bytes, self.interface);
        Ok(out)
    }
}

pub struct ResponseStruct {
    faker: FakeFat<StdFileSystem>,
    rw_idx: usize,
    bytes_waiting: usize,
}

impl ResponseStruct {
    pub fn new(faker : FakeFat<StdFileSystem>) -> Self {
        ResponseStruct {
            faker, 
            rw_idx : 0,
            bytes_waiting : 0,
        }
    }
}

impl ScsiResponder for ResponseStruct {
    type BlockType = ResponseBlock;
    fn memory_buffer(&mut self) -> Self::BlockType {
        ResponseBlock::default()
    }
    fn read_capacity(
        &mut self,
        _command: ReadCapacityCommand,
    ) -> Result<(ReadCapacityResponse, CommandStatusWrapper), ScsiError> {
        let retval = ReadCapacityResponse {
            logical_block_address: 0,
            block_length: 512,
        };
        Ok((retval, CommandStatusWrapper::default()))
    }
    fn inquiry(
        &mut self,
        _command: InquiryCommand,
    ) -> Result<(InquiryResponse, CommandStatusWrapper), ScsiError> {
        Ok(Default::default())
    }

    fn request_sense(
        &mut self,
        _command: RequestSenseCommand,
    ) -> Result<CommandStatusWrapper, ScsiError> {
        Ok(Default::default())
    }
    fn test_unit_ready(
        &mut self,
        _command: TestUnitReady,
    ) -> Result<CommandStatusWrapper, ScsiError> {
        Ok(Default::default())
    }
    fn read10_start(&mut self, command: Read10Command) -> Result<(), ScsiError> {
        //TODO: Verify we arent in the middle of another transfer.
        let block_start = command.block_address as usize;
        let bytes_per_block = command.block_size as usize;
        self.rw_idx = block_start * bytes_per_block;
        self.faker
            .seek(SeekFrom::Start(self.rw_idx as u64))
            .map_err(|_| ScsiError::from_cause(ErrorCause::UnsupportedOperationError))?;
        self.bytes_waiting = command.transfer_blocks as usize * bytes_per_block;
        Ok(())
    }

    fn read_block(&mut self, buffer: &mut [u8]) -> Result<Option<CommandStatusWrapper>, ScsiError> {
        let bytes_to_read = 512.min(self.bytes_waiting);
        let buf_slice = &mut buffer[0..bytes_to_read];
        let _red = self
            .faker
            .read(buf_slice)
            .map_err(|_| ScsiError::from_cause(ErrorCause::UnsupportedOperationError))?;
        self.rw_idx += bytes_to_read;
        self.bytes_waiting -= bytes_to_read;
        if self.bytes_waiting == 0 {
            Ok(Some(Default::default()))
        } else {
            Ok(None)
        }
    }
    fn write10_start(&mut self, command: Write10Command) -> Result<(), ScsiError> {
        //TODO: Verify we arent in the middle of another transfer.
        let block_start = command.block_address as usize;
        let bytes_per_block = command.block_size as usize;
        self.rw_idx = block_start * bytes_per_block;
        self.faker
            .seek(SeekFrom::Start(self.rw_idx as u64))
            .map_err(|_| ScsiError::from_cause(ErrorCause::UnsupportedOperationError))?;
        self.bytes_waiting = command.transfer_blocks as usize * bytes_per_block;
        Ok(())
    }
    fn write_block(&mut self, buffer: &[u8]) -> Result<Option<CommandStatusWrapper>, ScsiError> {
        let bytes_to_read = 512.min(self.bytes_waiting);
        let buf_slice = &buffer[0..bytes_to_read];
        let _red = self
            .faker
            .write(buf_slice)
            .map_err(|_| ScsiError::from_cause(ErrorCause::UnsupportedOperationError))?;
        self.rw_idx += bytes_to_read;
        self.bytes_waiting -= bytes_to_read;
        if self.bytes_waiting == 0 {
            Ok(Some(Default::default()))
        } else {
            Ok(None)
        }
    }
}

pub struct ResponseBlock([u8; 512]);
impl Default for ResponseBlock {
    fn default() -> ResponseBlock {
        ResponseBlock([0; 512])
    }
}

impl AsRef<[u8]> for ResponseBlock {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for ResponseBlock {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}
