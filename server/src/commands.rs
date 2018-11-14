use nxusb::prefixes::{CommandPrefix, ReadPrefix, WritePrefix, Prefixes};

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
pub trait ServerCommandState<T: CommandPrefix> {
    /// Initialized a new, unstarted command environment from a command prefix.
    fn from_prefix(prefix: T) -> Self;

    /// Checks or not the command being run needs to get more input from the
    /// communication line.
    fn needs_input(&self) -> bool;

    /// Passes a block from the communication line to the command, and returns
    /// either the number of bytes read from the block or an error message.
    fn input_block(&mut self, block: &[u8]) -> Result<usize, String>;

    /// Checks or not the command being run needs to pass output to the
    /// communication line.
    fn needs_output(&self) -> bool;

    /// Passes a block to the communication line from the command, and returns
    /// either the number of bytes writen to the block or an error message.
    fn output_block(&mut self, buffer: &mut [u8]) -> Result<usize, String>;
}

/// A command to read a file from the device and return its contents to the
/// communication line.
///
/// The parameter `FileReaderType` is the type to be used to find the files and
/// read their content.
#[derive(Debug)]
pub struct ReadCommandState<FileReaderType: FileReader> {
    prefix: ReadPrefix,
    file_name: String,
    file: Option<FileReaderType>,
    finished: bool,
}

/// A trait to abstract over a cursor-based approach for reading an object from a
/// name.
pub trait FileReader: Sized {
    /// Creates a handle to the object to be read
    fn new(file_name: &str) -> Result<Self, String>;

    /// Gets the number of bytes in this File.
    fn len(&self) -> usize;

    /// Reads the next bytes to the given buffer, returning the number of bytes read.
    /// This function either fills up the buffer if it can or short-circuits if it reaches
    /// the end of the file's content before the buffer is filled.
    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, String>;
}

impl<FileReaderType: FileReader> ServerCommandState<ReadPrefix>
    for ReadCommandState<FileReaderType>
{
    fn from_prefix(prefix: ReadPrefix) -> Self {
        let ln = prefix.file_name_length as usize;
        ReadCommandState {
            prefix,
            file_name: String::with_capacity(ln),
            file: None,
            finished: false,
        }
    }

    fn needs_input(&self) -> bool {
        self.file_name.len() < self.prefix.file_name_length as usize
    }

    fn input_block(&mut self, block: &[u8]) -> Result<usize, String> {
        let start_len = self.file_name.len();
        let need_bytes = self.prefix.file_name_length as usize - start_len;
        let block_size = block.len();
        if need_bytes > block_size {
            let part = String::from_utf8(block.to_vec())
                .map_err(|e| format!("UTF8 Error: {:?}", e).to_owned())?;
            self.file_name.push_str(&part);
            Ok(block_size)
        } else {
            let bytes = block[0..need_bytes].to_vec();
            let part =
                String::from_utf8(bytes).map_err(|e| format!("UTF8 Error: {:?}", e).to_owned())?;
            self.file_name.push_str(&part);
            Ok(need_bytes)
        }
    }

    fn needs_output(&self) -> bool {
        !self.finished && !self.needs_input()
    }

    fn output_block(&mut self, buffer: &mut [u8]) -> Result<usize, String> {
        let buflen = buffer.len();
        let buffer_idx_begin = if self.file.is_none() {
            let fl = FileReaderType::new(&self.file_name)?;
            let fl_len = fl.len();
            dprintln!("Now starting output of file {} with size {}.", self.file_name, fl_len);
            buffer[0] = ((fl_len & 0xFF000000) >> 24) as u8;
            buffer[1] = ((fl_len & 0xFF0000) >> 16) as u8;
            buffer[2] = ((fl_len & 0xFF00) >> 8) as u8;
            buffer[3] = (fl_len & 0xFF) as u8;
            self.file = Some(fl);
            4
        } else {
            0
        };
        let fl = if let Some(f) = &mut self.file {
            f
        } else {
            return Ok(0);
        };
        let read_bytes = fl.read_bytes(&mut buffer[buffer_idx_begin..])?;
        if read_bytes < buflen - buffer_idx_begin {
            self.finished = true;
        }
        Ok(read_bytes)
    }
}

/// A trait to abstract over a cursor-based approach for writing files to a given
/// file name.
pub trait FileWriter: Sized {
    /// Creates a handle to the object to be written to
    fn new(file_name: &str) -> Result<Self, String>;

    /// Writes to the file using bytes from the given buffer, returning the number of bytes written.
    fn write_bytes(&mut self, buffer: &[u8]) -> Result<usize, String>;
}

#[derive(Debug)]
pub struct WriteCommandState<FileWriterType: FileWriter> {
    prefix: WritePrefix,
    file_name: String,
    file: Option<FileWriterType>,
    finished: bool,
    write_idx: usize,
}

impl<WriterType: FileWriter> ServerCommandState<WritePrefix> for WriteCommandState<WriterType> {
    fn from_prefix(prefix: WritePrefix) -> Self {
        let ln = prefix.file_name_length as usize;
        WriteCommandState {
            prefix,
            file_name: String::with_capacity(ln),
            file: None,
            finished: false,
            write_idx: 0,
        }
    }
    fn needs_input(&self) -> bool {
        !self.finished
    }

    fn input_block(&mut self, block: &[u8]) -> Result<usize, String> {
        let block_size = block.len();
        let name_bytes_to_get = self.prefix.file_name_length as usize - self.file_name.len();
        let file_bytes_to_get = self.prefix.file_length as usize - self.write_idx;

        //Already finished: do nothing.
        if self.finished || block_size == 0 {
            Ok(0)
        }
        //Already finished but don't know it: set the flag.
        else if name_bytes_to_get == 0 && file_bytes_to_get == 0 {
            self.finished = true;
            Ok(0)
        }
        //The entire block is for the file name.
        else if name_bytes_to_get > block_size {
            let part = String::from_utf8(block.to_vec())
                .map_err(|e| format!("UTF8 Error: {:?}", e).to_owned())?;
            self.file_name.push_str(&part);
            Ok(block_size)
        }
        //The entire block is for the file content.
        else if name_bytes_to_get == 0 && file_bytes_to_get > 0 {
            //TODO: This is just a series of borrow checker manipulations to
            // short-circuit set self.file and then use it.
            if self.file.is_none() {
                let fl = WriterType::new(&self.file_name)?;
                self.file = Some(fl);
            }
            let bytes_to_write = &block[0..file_bytes_to_get.min(block_size)];
            if let Some(fl) = &mut self.file {
                let rval = fl.write_bytes(bytes_to_write);
                if let Ok(n) = rval {
                    self.write_idx += n;
                }
                rval
            }
            // Since we previously set self.file in the None case and short-circuited
            // if it didn't work, this branch should be impossible to reach.
            else {
                Err("File is somehow None after the branch!".to_owned())
            }
        }
        //Need to both finish the name and start the file
        else {
            let name_bytes = &block[0..name_bytes_to_get];
            let name_part = String::from_utf8(name_bytes.to_vec())
                .map_err(|e| format!("UTF8 Error: {:?}", e).to_owned())?;
            self.file_name.push_str(&name_part);
            let mut fl = WriterType::new(&self.file_name)?;
            let not_name = &block[name_bytes_to_get..];
            let file_bytes = &not_name[0..file_bytes_to_get.min(block_size - name_bytes_to_get)];
            let rval = fl.write_bytes(file_bytes);
            if let Ok(n) = rval {
                self.write_idx = n;
            }
            self.file = Some(fl);
            rval
        }
    }

    fn needs_output(&self) -> bool {
        false
    }

    fn output_block(&mut self, _buffer: &mut [u8]) -> Result<usize, String> {
        Ok(0)
    }
}

pub enum CommandStates<T : FileReader, U : FileWriter> {
    Read(ReadCommandState<T>), 
    Write(WriteCommandState<U>),
}

impl <T : FileReader, U : FileWriter> ServerCommandState<Prefixes> for CommandStates<T, U> {
    fn from_prefix(prefix: Prefixes) -> Self {
        match prefix {
            Prefixes::Read(r) => CommandStates::Read(ReadCommandState::from_prefix(r)), 
            Prefixes::Write(w) => CommandStates::Write(WriteCommandState::from_prefix(w))
        }
    }

    fn needs_input(&self) -> bool {
        match self {
            &CommandStates::Read(ref r) => r.needs_input(), 
            &CommandStates::Write(ref w) => w.needs_input()
        }
    }

    fn input_block(&mut self, block: &[u8]) -> Result<usize, String> {
        match self {
            &mut CommandStates::Read(ref mut r) => r.input_block( block), 
            &mut CommandStates::Write(ref mut w) => w.input_block(block)
        }
    }

    fn needs_output(&self) -> bool {
        match self {
            &CommandStates::Read(ref r) => r.needs_output(), 
            &CommandStates::Write(ref w) => w.needs_output()
        }
    }

    fn output_block(&mut self, buffer: &mut [u8]) -> Result<usize, String> {
        match self {
            &mut CommandStates::Read(ref mut r) => r.output_block(buffer), 
            &mut CommandStates::Write(ref mut w) => w.output_block(buffer)
        }

    }

}