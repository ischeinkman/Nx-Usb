pub struct ReadPrefix {
    pub flags: u16,
    pub file_name_length: u16,
}

pub const PREFIX_LENGTH: usize = 8; //Bytes

pub trait CommandPrefix
where
    Self: Sized,
{
    fn parse_prefix(prefix: [u8; PREFIX_LENGTH]) -> Option<Self>;
    fn arg_bytes_to_read(&self) -> usize;
}

impl CommandPrefix for ReadPrefix {
    fn parse_prefix(prefix: [u8; PREFIX_LENGTH]) -> Option<ReadPrefix> {
        if prefix[0] & 128 != 0 {
            return None;
        }
        let flags: u16 = (prefix[0] as u16) << 8 | (prefix[1] as u16);
        let file_name_length: u16 = (prefix[2] as u16) << 8 | (prefix[3] as u16);
        Some(ReadPrefix {
            flags,
            file_name_length,
        })
    }
    fn arg_bytes_to_read(&self) -> usize {
        self.file_name_length as usize
    }
}

pub struct WritePrefix {
    pub flags: u16,
    pub file_name_length: u16,
    pub file_length: u32,
}

impl CommandPrefix for WritePrefix {
    fn parse_prefix(prefix: [u8; PREFIX_LENGTH]) -> Option<WritePrefix> {
        if prefix[0] & 128 == 0 {
            return None;
        }
        let flags: u16 = (prefix[0] as u16) << 8 | (prefix[1] as u16);
        let file_name_length: u16 = (prefix[2] as u16) << 8 | (prefix[3] as u16);
        let file_length: u32 = (prefix[4] as u32) << 24
            | (prefix[5] as u32) << 16
            | (prefix[6] as u32) << 8
            | (prefix[7] as u32);
        Some(WritePrefix {
            flags,
            file_name_length,
            file_length,
        })
    }

    fn arg_bytes_to_read(&self) -> usize {
        self.file_name_length as usize + self.file_length as usize
    }
}

pub enum Prefixes {
    Write(WritePrefix),
    Read(ReadPrefix),
}

impl CommandPrefix for Prefixes {
    fn parse_prefix(prefix: [u8; PREFIX_LENGTH]) -> Option<Prefixes> {
        WritePrefix::parse_prefix(prefix)
            .map(|w| Prefixes::Write(w))
            .or(ReadPrefix::parse_prefix(prefix).map(|r| Prefixes::Read(r)))
    }

    fn arg_bytes_to_read(&self) -> usize {
        match self {
            Prefixes::Read(ref inner) => inner.arg_bytes_to_read(),
            Prefixes::Write(ref inner) => inner.arg_bytes_to_read(),
        }
    }
}
