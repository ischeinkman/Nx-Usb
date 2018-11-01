use super::prefixes::{CommandPrefix, ReadPrefix, WritePrefix};

use std::marker::PhantomData;

pub trait CommandRunner<T: CommandPrefix> {
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
pub struct ReadCommand<FileReaderType: FileReader> {
    prefix: ReadPrefix,
    file_name: String,
    file: Option<FileReaderType>,
    finished: bool,
    _phantoms: PhantomData<FileReaderType>,
}

/// A trait to abstract over a cursor-based approach for reading an object from a
/// name.
pub trait FileReader: Sized {
    /// Creates a handle to the object to be read
    fn new(file_name: &str) -> Result<Self, String>;

    /// Reads the next bytes to the given buffer, returning the number of bytes read.
    /// This function either fills up the buffer if it can or short-circuits if it reaches
    /// the end of the file's content before the buffer is filled.
    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, String>;
}

impl<FileReaderType: FileReader> CommandRunner<ReadPrefix> for ReadCommand<FileReaderType> {
    fn from_prefix(prefix: ReadPrefix) -> Self {
        let ln = prefix.file_name_length as usize;
        ReadCommand {
            prefix,
            file_name: String::with_capacity(ln),
            file: None,
            finished: false,
            _phantoms: PhantomData,
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
            let fl = FileReaderType::new(&self.file_name)?;
            self.file = Some(fl);
            Ok(need_bytes)
        }
    }

    fn needs_output(&self) -> bool {
        !self.finished && !self.needs_input()
    }

    fn output_block(&mut self, buffer: &mut [u8]) -> Result<usize, String> {
        let fl = if let Some(f) = &mut self.file {
            f
        } else {
            return Ok(0);
        };
        let buflen = buffer.len();
        let read_bytes = fl.read_bytes(buffer)?;
        if read_bytes < buflen {
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

pub struct WriteCommand<FileWriterType: FileWriter> {
    prefix: WritePrefix,
    file_name: String,
    file: Option<FileWriterType>,
    finished: bool,
    write_idx: usize,
    _phantoms: PhantomData<FileWriterType>,
}

impl<WriterType: FileWriter> CommandRunner<WritePrefix> for WriteCommand<WriterType> {
    fn from_prefix(prefix: WritePrefix) -> Self {
        let ln = prefix.file_name_length as usize;
        WriteCommand {
            prefix,
            file_name: String::with_capacity(ln),
            file: None,
            finished: false,
            write_idx: 0,
            _phantoms: PhantomData,
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
