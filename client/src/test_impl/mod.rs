#![cfg(test)]
use commands::{ClientCommandState, FileContentStorer, FileRetriever, ReadState};
use interface::ClientDevice;
use nxusb::prefixes::{CommandPrefix, Prefixes, ReadPrefix, WritePrefix, PREFIX_LENGTH};
use std::collections::HashMap;
use std::sync::{Once, ONCE_INIT};
use std::vec::Vec;

struct TestFileContext {
    files: HashMap<String, Vec<u8>>,
}

static mut CONTEXT: Option<TestFileContext> = None;
static INIT: Once = ONCE_INIT;

impl TestFileContext {
    unsafe fn get_context() -> &'static mut TestFileContext {
        INIT.call_once(|| {
            CONTEXT = Some(TestFileContext {
                files: HashMap::new(),
            })
        });
        CONTEXT.as_mut().unwrap()
    }
}
pub struct TestFile {
    name: String,
    read_idx: usize,
}
impl FileRetriever for TestFile {
    fn name(&self) -> &str {
        &self.name
    }
    fn open_file(nm : &str) -> Result<Self, String> {
        Ok(TestFile {
            name : nm.to_owned(),
            read_idx :0
        })
    }
    fn len(&self) -> usize {
        let bts: Vec<u8> = unsafe {
            TestFileContext::get_context()
                .files
                .get(&self.name)
                .unwrap_or(&Vec::new())
                .to_vec()
        };
        bts.len()

    }
    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, String> {
        let bts: Vec<u8> = unsafe {
            TestFileContext::get_context()
                .files
                .get(&self.name)
                .unwrap_or(&Vec::new())
                .to_vec()
        };

        let mut bts_read = 0;
        while bts_read < buffer.len() && self.read_idx + bts_read < bts.len() {
            buffer[bts_read] = bts[self.read_idx + bts_read];
            bts_read += 1;
        }
        self.read_idx += bts_read;
        Ok(bts_read)
    }
}

#[derive(Debug)]
struct TestFileStorer {
    name: String,
}

impl FileContentStorer for TestFileStorer {
    fn for_name(name: &str, _size: usize) -> Result<Self, String> {
        let name = name.to_owned();
        unsafe {
            TestFileContext::get_context()
                .files
                .insert(name.clone(), Vec::new());
        }
        Ok(TestFileStorer { name: name })
    }

    fn push_bytes(&mut self, buffer: &[u8]) -> Result<usize, String> {
        let fl: &mut Vec<u8> = unsafe {
            TestFileContext::get_context()
                .files
                .get_mut(&self.name)
                .ok_or(format!(
                    "Err: could not find buffer for file named {}.",
                    self.name
                ))?
        };
        fl.extend_from_slice(buffer);
        Ok(buffer.len())
    }
}

const TEST_BLOCK_SIZE: usize = 100;
pub struct TestUsbDevice {
    input_buf: Vec<u8>,
    output_buf: Vec<u8>,
}

impl TestUsbDevice {
    pub fn empty() -> TestUsbDevice {
        TestUsbDevice {
            input_buf: Vec::new(),
            output_buf: Vec::new(),
        }
    }

    pub fn push_input(&mut self, input_bytes: &[u8]) {
        self.input_buf.extend_from_slice(input_bytes);
    }

    pub fn pull_input(&mut self, count: usize) -> Vec<u8> {
        self.input_buf.drain(0..count).collect()
    }

    pub fn push_output(&mut self, output_bytes: &[u8]) {
        self.output_buf.extend_from_slice(output_bytes);
    }

    pub fn pull_output(&mut self, count: usize) -> Vec<u8> {
        self.output_buf.drain(0..count).collect()
    }
}

impl ClientDevice for TestUsbDevice {
    fn block_size(&self) -> usize {
        TEST_BLOCK_SIZE
    }

    fn pull_block(&mut self, buffer: &mut [u8]) -> Result<usize, String> {
        if buffer.len() < TEST_BLOCK_SIZE {
            Err(format!(
                "Dont have large enough output buf to read block! Need {} but was passed {}.",
                TEST_BLOCK_SIZE,
                buffer.len()
            ))
        } else if self.input_buf.len() < TEST_BLOCK_SIZE {
            Err(format!(
                "Dont have large enough input buf to read block! Need {} but have {}.",
                TEST_BLOCK_SIZE,
                self.input_buf.len()
            ))
        } else {
            let bts = self.pull_input(TEST_BLOCK_SIZE);
            for idx in 0..TEST_BLOCK_SIZE {
                buffer[idx] = bts[idx];
            }
            Ok(TEST_BLOCK_SIZE)
        }
    }

    fn push_block(&mut self, bytes: &[u8]) -> Result<usize, String> {
        if bytes.len() != self.block_size() {
            Err(format!(
                "Got less then a block size of bytes to output: {} but expected {}",
                bytes.len(),
                TEST_BLOCK_SIZE
            ))
        } else {
            self.push_output(bytes);
            Ok(bytes.len())
        }
    }

    fn push_prefix(&mut self, prefix: Prefixes) -> Result<usize, String> {
        let bts = prefix.serialize();
        self.output_buf.extend_from_slice(&bts);
        Ok(PREFIX_LENGTH)
    }
}

#[test]
fn test_read_prefix_pushing() {
    let expected: [u8; PREFIX_LENGTH] = [0x0, 0x0, 0x0, 0x10, 0x0, 0x0, 0x0, 0x0];
    let prefix = ReadPrefix {
        flags: 0,
        file_name_length: 16,
    };
    let mut usb_ctx = TestUsbDevice::empty();
    usb_ctx.push_prefix(Prefixes::Read(prefix)).unwrap();
    let actual = &usb_ctx.output_buf[0..PREFIX_LENGTH];
    assert_eq!(&expected, &actual);
}

#[test]
fn test_write_prefix_pushing() {
    let prefix = WritePrefix {
        flags: 0b1010101010101010,
        file_name_length: 16,
        file_length: 4096,
    };
    let expected: [u8; PREFIX_LENGTH] =
        [0b10101010, 0b10101010, 0x00, 0x10, 0x00, 0x00, 0x10, 0x00];
    let mut usb_ctx = TestUsbDevice::empty();
    usb_ctx.push_prefix(Prefixes::Write(prefix)).unwrap();
    let actual = &usb_ctx.output_buf[0..PREFIX_LENGTH];
    assert_eq!(&expected, actual);
}

#[test]
fn test_read_file() {
    let mut test_read_buffer = [0; TEST_BLOCK_SIZE];
    let mut test_write_buffer = [0; TEST_BLOCK_SIZE];
    let mut usb_ctx = TestUsbDevice::empty();
    let read_prefix = ReadPrefix {
        flags: 0,
        file_name_length: 3,
    };
    usb_ctx.push_prefix(Prefixes::Read(read_prefix)).unwrap();
    let mut read_state =
        ReadState::<TestFileStorer>::new_read(read_prefix, "fla", "fla_out").unwrap();
    usb_ctx
        .input_buf
        .extend_from_slice(&[0, 0, 0, 5, b'H', b'e', b'l', b'l', b'o']);
    assert_eq!(9, usb_ctx.input_buf.len());
    usb_ctx.input_buf.resize(10000, 0);
    assert_eq!(10000, usb_ctx.input_buf.len());
    while read_state.needs_push() || read_state.needs_pull() {
        if read_state.needs_pull() {
            let _blk = usb_ctx.pull_block(&mut test_read_buffer).unwrap();
            let _write = read_state.pull_block(&test_read_buffer).unwrap();
        }
        if read_state.needs_push() {
            let _written = read_state.push_block(&mut test_write_buffer).unwrap();
            let _blk = usb_ctx.push_block(&test_write_buffer).unwrap();
        }
    }
    let read_content = unsafe { TestFileContext::get_context().files.get("fla_out").unwrap() };
    assert_eq!(read_content, &vec![b'H', b'e', b'l', b'l', b'o']);
}

#[test]
fn test_write_file() {
    //TODO: This
    /*
    let name = vec![b'f', b'l', b'a'];
    let file = vec![b'H', b'e', b'l', b'l', b'o'];

    let mut usb_ctx = TestUsbDevice::empty();
    usb_ctx.push_input(&name);
    usb_ctx.push_input(&file);
    usb_ctx.push_input(&[0; 100]);
    usb_ctx.push_input(&[0; 100]);
    usb_ctx.push_input(&[0; 100]);

    let mut test_read_buffer: [u8; TEST_BLOCK_SIZE] = [0; TEST_BLOCK_SIZE];
    let mut test_write_buffer: [u8; TEST_BLOCK_SIZE] = [0; TEST_BLOCK_SIZE];
    let write_prefix = WritePrefix {
        flags: 0b1000000000000000,
        file_name_length: 3,
        file_length: 5,
    };
    let mut write_command = WriteCommandState::<TestFileWriter>::from_prefix(write_prefix);

    while write_command.needs_input() || write_command.needs_output() {
        if write_command.needs_input() {
            let _blk = usb_ctx.read_block(&mut test_read_buffer).unwrap();
            let _write = write_command.input_block(&test_read_buffer).unwrap();
        }
        if write_command.needs_output() {
            let _written = write_command.output_block(&mut test_write_buffer).unwrap();
            let _blk = usb_ctx.write_block(&test_write_buffer).unwrap();
        }
    }

    let fl_ctx = unsafe { TestFileContext::get_context() };
    let written_fl = if let Some(f) = fl_ctx.files.get("fla") {
        f
    } else {
        panic!(
            "Couldn't find the file \"fla\" in the context map {:?}",
            fl_ctx.files
        );
    };
    assert_eq!(written_fl, &file);
    assert!(usb_ctx.input_buf.into_iter().all(|a| a == 0));
    assert!(usb_ctx.output_buf.is_empty());
    */
}
