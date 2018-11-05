use commands::{FileReader, FileWriter, ReadCommandState, WriteCommandState, ServerCommandState};
use prefixes::{PREFIX_LENGTH, Prefixes, CommandPrefix, ReadPrefix, WritePrefix};
use interface::ServerDevice;
use std::collections::HashMap;
use std::vec::Vec;
use std::sync::{Once, ONCE_INIT};

struct TestFileContext {
    files : HashMap<String, Vec<u8>>,
}

static mut CONTEXT : Option<TestFileContext> = None;
static INIT : Once = ONCE_INIT;

impl TestFileContext {
    unsafe fn get_context() -> &'static mut  TestFileContext {
        INIT.call_once(|| {
            CONTEXT = Some(TestFileContext{
                files : HashMap::new()
            })
        });
        CONTEXT.as_mut().unwrap()
    }
}

#[derive(Debug)]
pub struct TestFileReader {
    bytes : Vec<u8>, 
}

impl FileReader for TestFileReader {
    fn new(name: &str) -> Result<Self, String> {
        let bts : Vec<u8> = unsafe {
            TestFileContext::get_context().files.get(name).unwrap_or(&Vec::new()).to_vec()
        };

        Ok(TestFileReader {
            bytes : bts
        })
    }
    fn len(&self) -> usize {
        self.bytes.len()
    }
    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, String> {
        let buflen = buffer.len();
        let num_bytes = buflen.min(self.bytes.len());
        for (idx, bt) in self.bytes.drain(0 .. num_bytes).enumerate() {
            buffer[idx] = bt;
        }
        Ok(num_bytes)
    }
}

#[derive(Debug)]
pub struct TestFileWriter {
    name : String
}

impl FileWriter for TestFileWriter {
    fn new(file_name: &str) -> Result<Self, String> {
        unsafe {
            TestFileContext::get_context().files.insert(file_name.to_string(), Vec::new());
        }
        Ok(TestFileWriter {
            name : file_name.to_owned()
        })
    }

    fn write_bytes(&mut self, buffer: &[u8]) -> Result<usize, String> {
        let fl : &mut Vec<u8> = unsafe {
            TestFileContext::get_context().files.get_mut(&self.name).ok_or(format!("Err: could not find buffer for file named {}.", self.name))?
        };
        fl.extend_from_slice(buffer);
        Ok(buffer.len())
    }
}


const TEST_BLOCK_SIZE : usize = 100;
pub struct TestUsbDevice {
    input_buf : Vec<u8>, 
    output_buf : Vec<u8>,
}

impl TestUsbDevice {
    pub fn empty() -> TestUsbDevice {
        TestUsbDevice {
            input_buf : Vec::new(), 
            output_buf : Vec::new(),
        }
    }

    pub fn push_input(&mut self, input_bytes : &[u8]) {
        self.input_buf.extend_from_slice(input_bytes);
    }
    
    pub fn pull_input(&mut self, count : usize) -> Vec<u8> {
        self.input_buf.drain(0 .. count).collect()
    }

    pub fn push_output(&mut self, output_bytes : &[u8]) {
        self.output_buf.extend_from_slice(output_bytes);
    }
    
    pub fn pull_output(&mut self, count : usize) -> Vec<u8> {
        self.output_buf.drain(0 .. count).collect()
    }
}


impl ServerDevice for TestUsbDevice {
    fn block_size(&self) -> usize {
        TEST_BLOCK_SIZE
    }

    fn read_block(&mut self, buffer: &mut [u8]) -> Result<usize, String> {
        if buffer.len() < TEST_BLOCK_SIZE {
            Err(format!("Dont have large enough output buf to read block! Need {} but was passed {}.", TEST_BLOCK_SIZE, buffer.len()))
        }
        else if self.input_buf.len() < TEST_BLOCK_SIZE {
            Err(format!("Dont have large enough input buf to read block! Need {} but have {}.", TEST_BLOCK_SIZE, self.input_buf.len()))
        }
        else {
            let bts = self.pull_input(TEST_BLOCK_SIZE);
            for idx in 0 .. TEST_BLOCK_SIZE {
                buffer[idx] = bts[idx];
            }
            Ok(TEST_BLOCK_SIZE)

        }

    }

    fn write_block(&mut self, bytes : &[u8]) -> Result<usize, String> {
        if bytes.len() != self.block_size() {
            Err(format!("Got less then a block size of bytes to output: {} but expected {}", bytes.len(), TEST_BLOCK_SIZE))
        }
        else {
            self.push_output(bytes);
            Ok(bytes.len())
        }
    }

    fn read_prefix(&mut self) -> Result<Prefixes, String> {
        if self.input_buf.len() < PREFIX_LENGTH {
            Err(format!("Don't have large enough input buf to read prefix! Need {} but have {}.", PREFIX_LENGTH, self.input_buf.len()))
        }
        else {
            let mut prefix_buffer : [u8 ; PREFIX_LENGTH] = [0 ; PREFIX_LENGTH];
            let prefix_bytes = self.pull_input(PREFIX_LENGTH);
            for idx in 0 .. PREFIX_LENGTH {
                prefix_buffer[idx] = prefix_bytes[idx];
            }
            Prefixes::parse_prefix(prefix_buffer).ok_or("Err: found None when parsing prefix.".to_string())
        }
    }
}

#[test]
fn test_read_prefix_parsing() {
    let expected = ReadPrefix {
        flags : 0, 
        file_name_length : 16
    };
    let bts : [u8 ; PREFIX_LENGTH] = [0x0, 0x0, 0x0, 0x10, 0x0, 0x0, 0x0, 0x0];
    let mut usb_ctx = TestUsbDevice::empty();
    usb_ctx.push_input(&bts);
    let wrapped_actual = usb_ctx.read_prefix().unwrap();
    match wrapped_actual {
        Prefixes::Read(a) => {
            assert_eq!(expected, a);
        }
        Prefixes::Write(a) => {
            panic!("Got write prefix in read test: {:?} instead of expected {:?}.", a, expected);
        }
    }
}

#[test]
fn test_write_prefix_parsing() {
    let expected = WritePrefix {
        flags : 0b1010101010101010, 
        file_name_length : 16,
        file_length : 4096
    };
    let bts : [u8 ; PREFIX_LENGTH] = [
        0b10101010, 0b10101010, 
        0x00, 0x10, 
        0x00, 0x00, 
        0x10, 0x00
    ];
    let mut usb_ctx = TestUsbDevice::empty();
    usb_ctx.push_input(&bts);
    let wrapped_actual = usb_ctx.read_prefix().unwrap();
    match wrapped_actual {
        Prefixes::Write(a) => {
            assert_eq!(expected, a);
        }
        Prefixes::Read(a) => {
            panic!("Got read prefix in write test: {:?} instead of expected {:?}.", a, expected);
        }
    }
}

#[test]
fn test_read_file() {

    let file = vec![b'H', b'e', b'l', b'l', b'o'];
    let fl_ctx = unsafe { TestFileContext::get_context() };
    fl_ctx.files.insert("fla".to_string(), file.clone());

    let mut usb_ctx = TestUsbDevice::empty();
    usb_ctx.push_input(&[b'f', b'l', b'a']);
    usb_ctx.push_input(&[0 ; 100]);

    let mut test_read_buffer : [u8 ; TEST_BLOCK_SIZE] = [0 ; TEST_BLOCK_SIZE];
    let mut test_write_buffer : [u8 ; TEST_BLOCK_SIZE] = [0 ; TEST_BLOCK_SIZE];
    let read_prefix = ReadPrefix {
        flags : 0,
        file_name_length : 3
    };
    let mut read_command = ReadCommandState::<TestFileReader>::from_prefix(read_prefix);

    while read_command.needs_input() || read_command.needs_output() {
        if read_command.needs_input() {
            let _blk = usb_ctx.read_block(&mut test_read_buffer).unwrap();
            let _read = read_command.input_block(&test_read_buffer).unwrap();
        }
        if read_command.needs_output() {
            let _written = read_command.output_block(&mut test_write_buffer).unwrap();
            let _blk = usb_ctx.write_block(&test_write_buffer).unwrap();
        }
    }
    assert!(usb_ctx.input_buf.clone().into_iter().all(|a| a == 0));
    assert_eq!(usb_ctx.pull_output(4), vec![0, 0, 0, 5]);
    assert_eq!(usb_ctx.pull_output(5), file);
}

#[test]
fn test_write_file() {

    let name = vec![b'f', b'l', b'a'];
    let file = vec![b'H', b'e', b'l', b'l', b'o'];

    let mut usb_ctx = TestUsbDevice::empty();
    usb_ctx.push_input(&name);
    usb_ctx.push_input(&file);
    usb_ctx.push_input(&[0 ; 100]);
    usb_ctx.push_input(&[0 ; 100]);
    usb_ctx.push_input(&[0 ; 100]);

    let mut test_read_buffer : [u8 ; TEST_BLOCK_SIZE] = [0 ; TEST_BLOCK_SIZE];
    let mut test_write_buffer : [u8 ; TEST_BLOCK_SIZE] = [0 ; TEST_BLOCK_SIZE];
    let write_prefix = WritePrefix {
        flags : 0b1000000000000000,
        file_name_length : 3,
        file_length : 5
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
    
    let fl_ctx = unsafe { TestFileContext::get_context()};
    let written_fl = if let Some(f) = fl_ctx.files.get("fla") {
        f
    } else {
        panic!("Couldn't find the file \"fla\" in the context map {:?}", fl_ctx.files);
    };
    assert_eq!(written_fl, &file);
    assert!(usb_ctx.input_buf.into_iter().all(|a| a == 0));
    assert!(usb_ctx.output_buf.is_empty());
}