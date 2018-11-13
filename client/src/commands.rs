use nxusb::prefixes::{CommandPrefix, ReadPrefix, WritePrefix};

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
pub trait ClientCommandState<T: CommandPrefix> {
    fn prefix(&self) -> T;

    fn needs_push(&self) -> bool;

    fn push_block(&mut self, block: &mut [u8]) -> Result<usize, String>;

    fn needs_pull(&self) -> bool;

    fn pull_block(&mut self, buffer: &[u8]) -> Result<usize, String>;
}

#[derive(Debug)]
pub struct ReadState<StoreType: FileContentStorer> {
    pub prefix: ReadPrefix,
    pub file_name: String,
    pub output_name: String,
    store: Option<StoreType>,
    push_idx: usize,
    pull_idx: usize,
    pub file_size: usize,
}


impl<StoreType: FileContentStorer> ReadState<StoreType> {
    pub fn new_read(
        prefix: ReadPrefix,
        file_name: &str,
        output_name: &str,
    ) -> Result<Self, String> {
        dprintln!("Now starting read of file {} to local storage {}.", file_name, output_name);
        if prefix.file_name_length != file_name.len() as u16 {
            Err(format!("Could not verify prefix matches this file: got name {:?} which doesn't have length {}", file_name, prefix.file_name_length))
        } else {
            Ok(Self {
                prefix,
                file_name: file_name.to_owned(),
                output_name: output_name.to_owned(),
                store: None,
                push_idx: 0,
                pull_idx: 0,
                file_size: 0,
            })
        }
    }
}

impl<StoreType: FileContentStorer> ClientCommandState<ReadPrefix> for ReadState<StoreType> {
    fn prefix(&self) -> ReadPrefix {
        self.prefix
    }

    fn needs_push(&self) -> bool {
        self.push_idx < self.prefix.file_name_length as usize - 1
    }

    fn push_block(&mut self, block: &mut [u8]) -> Result<usize, String> {
        let mut cur_pushed = 0;
        while cur_pushed < block.len()
            && self.push_idx + cur_pushed < self.prefix.file_name_length as usize
        {
            block[cur_pushed] = self.file_name.as_bytes()[self.push_idx + cur_pushed];
            cur_pushed += 1;
        }
        self.push_idx += cur_pushed;
        dprintln!("Now pushing block: {:?}", block);
        Ok(cur_pushed)
    }

    fn needs_pull(&self) -> bool {
        !self.needs_push() && self.pull_idx < self.file_size + 4
    }

    fn pull_block(&mut self, buffer: &[u8]) -> Result<usize, String> {
        let block_sz = buffer.len();
        let mut cur_pulled = 0;

        dprintln!("Now processing pulled block {:?}", buffer);

        //Extract the file length 
        while self.pull_idx + cur_pulled < 4 && cur_pulled < block_sz {
            let read_byte = buffer[cur_pulled];
            let byte_offset = 3 - (self.pull_idx + cur_pulled);
            let bit_offset = byte_offset * 8;
            self.file_size |= (read_byte as usize) << bit_offset;
            cur_pulled += 1;
        }
        if self.pull_idx + cur_pulled < 4 {
            self.pull_idx += cur_pulled;
            return Ok(cur_pulled);
        }

        dprintln!("Now have {}/{} bytes of the file {}.", self.pull_idx + cur_pulled - 4, self.file_size, self.file_name);

        if self.store.is_none() {
            let fl = StoreType::for_name(&self.output_name, self.file_size)?;
            self.store = Some(fl);
        }

        let bytes_remaining = self.file_size - self.pull_idx + 4 - cur_pulled; 
        let bytes_to_push = if bytes_remaining >= block_sz - cur_pulled {
            &buffer[cur_pulled ..]
        } else {
            &buffer[cur_pulled .. cur_pulled + bytes_remaining]
        };
        let fl = self
            .store
            .as_mut()
            .ok_or("Store is somehow none after creation!")?;
        let rval = fl.push_bytes(bytes_to_push)?;
        cur_pulled += rval;
        self.pull_idx += cur_pulled;
        Ok(cur_pulled)
    }
}

pub struct WriteState<FileType : FileRetriever> {
    pub prefix : WritePrefix, 
    pub file : FileType, 
    pub switch_name : String, 
    push_idx : usize, 
}
impl <FileType : FileRetriever>  WriteState<FileType> { 
    pub fn new_write(prefix : WritePrefix, switch_path : &str, computer_path : &str) -> Result<Self, String> {
        let file = FileType::open_file(computer_path)?;
        if switch_path.len() != prefix.file_name_length as usize {
            return Err(format!("Error verifying prefix: path {} does not have length {}.", switch_path, prefix.file_name_length));
        }
        Ok(WriteState {
            prefix, 
            file, 
            switch_name : switch_path.to_owned(), 
            push_idx : 0
        })
    }
}
impl <FileType : FileRetriever> ClientCommandState<WritePrefix> for WriteState<FileType> {
    fn prefix(&self) -> WritePrefix {
        self.prefix
    }

    fn needs_push(&self) -> bool {
        self.push_idx < self.prefix.file_length as usize + self.prefix().file_name_length as usize
    }

    fn push_block(&mut self, block: &mut [u8]) -> Result<usize, String> {
        dprintln!("Entered write push.");
        let mut cur_pushed = 0; 
        while self.push_idx + cur_pushed < self.prefix.file_name_length as usize && cur_pushed < block.len() {
            block[cur_pushed] = self.switch_name.as_bytes()[self.push_idx + cur_pushed];
            cur_pushed += 1;
        }
        dprintln!("Finished pushing name; starting file.");
        while self.push_idx + cur_pushed < self.prefix.file_length  as usize && cur_pushed < block.len() {
            let space_left = &mut block[cur_pushed ..];
            cur_pushed += self.file.read_bytes(space_left)?;
        }
        self.push_idx += cur_pushed;
        Ok(cur_pushed)
    }

    fn needs_pull(&self) -> bool {
        false
    }

    fn pull_block(&mut self, _buffer: &[u8]) -> Result<usize, String> {
        Ok(0)
    }
}

pub trait FileRetriever: Sized {
    fn open_file(&str) -> Result<Self, String> ;
    fn name(&self) -> &str;
    fn len(&self) -> usize;
    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, String>;
}

pub trait FileContentStorer: Sized {
    fn for_name(name: &str, size: usize) -> Result<Self, String>;
    fn push_bytes(&mut self, buffer: &[u8]) -> Result<usize, String>;
}
