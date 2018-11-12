use interface::ServerDevice;
use libnx_rs::usbcomms::UsbCommsInterface;
use nxusb::prefixes::{CommandPrefix, Prefixes, PREFIX_LENGTH};

const TEST_BLOCK_SIZE: usize = 1024;
impl ServerDevice for UsbCommsInterface {
    fn read_prefix(&mut self) -> Result<Prefixes, String> {
        let mut prefix_buff: [u8; PREFIX_LENGTH] = [0; PREFIX_LENGTH];
        let mut read_idx = 0;
        let mut read_count = 0;
        while read_idx < PREFIX_LENGTH && read_count < 4{
            let read_bytes = self.read_bytes(&mut prefix_buff[read_idx..]);
            println!("ReadPrefix attempt {}: got {} bytes.", read_count, read_bytes);
            read_idx += read_bytes;
            read_count += 1;
        }
        if read_idx == PREFIX_LENGTH {
            Prefixes::parse_prefix(prefix_buff)
                .ok_or(format!("Could not parse bytes {:?}", prefix_buff).to_owned())
        }
        else {
            Err(format!("Only retrieved {} bytes out of {}. Buffer {:?}", read_idx, PREFIX_LENGTH, prefix_buff))
        }
    }

    fn block_size(&self) -> usize {
        TEST_BLOCK_SIZE
    }
    fn read_block(&mut self, buffer: &mut [u8]) -> Result<usize, String> {
        if buffer.len() != TEST_BLOCK_SIZE {
            Err(format!(
                "Bad read block size: expected {} but got block of size {}",
                TEST_BLOCK_SIZE,
                buffer.len()
            ).to_owned())
        } else {
            let bt_read = self.read_bytes(buffer);
            match bt_read {
                TEST_BLOCK_SIZE => Ok(TEST_BLOCK_SIZE),
                0 => Err("Read 0 bytes. Is this interface initialized?".to_owned()),
                n => Err(format!(
                    "Bad read block result: expected {} but read {} bytes instead.",
                    TEST_BLOCK_SIZE, n
                ).to_owned()),
            }
        }
    }

    fn write_block(&mut self, bytes: &[u8]) -> Result<usize, String> {
        if bytes.len() != TEST_BLOCK_SIZE {
            return Err(format!(
                "Bad write block size: expected {} but got block of size {}",
                TEST_BLOCK_SIZE,
                bytes.len()
            ).to_owned());
        } else {
            let bt_written = self.write_bytes(bytes);
            match bt_written {
                TEST_BLOCK_SIZE => Ok(TEST_BLOCK_SIZE),
                0 => Err("Wrote 0 bytes. Is this interface initialized?".to_owned()),
                n => Err(format!(
                    "Bad write block result: expected {} but wrote {} bytes instead.",
                    TEST_BLOCK_SIZE, n
                ).to_owned()),
            }
        }
    }
}
