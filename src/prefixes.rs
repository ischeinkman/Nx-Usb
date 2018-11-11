#[inline]
fn extract_bytes_u16(inp: u16) -> (u8, u8) {
    let first = ((inp & 0xFF00) >> 8) as u8;
    let second = (inp & 0x00FF) as u8;
    (first, second)
}

#[inline]
fn extract_bytes_u32(inp: u32) -> (u8, u8, u8, u8) {
    let last = (inp & 0xFF) as u8;
    let third = ((inp & 0xFF00) >> 8) as u8;
    let second = ((inp & 0xFF0000) >> 16) as u8;
    let head = ((inp & 0xFF000000) >> 24) as u8;

    (head, second, third, last)
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
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

    fn serialize(&self) -> [u8; PREFIX_LENGTH];
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

    fn serialize(&self) -> [u8; PREFIX_LENGTH] {
        let flag_bytes = extract_bytes_u16(self.flags);
        let name_length_bytes = extract_bytes_u16(self.file_name_length);
        [
            flag_bytes.0,
            flag_bytes.1,
            name_length_bytes.0,
            name_length_bytes.1,
            0,
            0,
            0,
            0,
        ]
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
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

    fn serialize(&self) -> [u8; PREFIX_LENGTH] {
        let flag_bytes = extract_bytes_u16(self.flags);
        let name_length_bytes = extract_bytes_u16(self.file_name_length);
        let file_length_bytes = extract_bytes_u32(self.file_length);

        [
            flag_bytes.0,
            flag_bytes.1,
            name_length_bytes.0,
            name_length_bytes.1,
            file_length_bytes.0,
            file_length_bytes.1,
            file_length_bytes.2,
            file_length_bytes.3,
        ]
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
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

    fn serialize(&self) -> [u8; PREFIX_LENGTH] {
        match self {
            Prefixes::Write(w) => w.serialize(),
            Prefixes::Read(r) => r.serialize(),
        }
    }
}
