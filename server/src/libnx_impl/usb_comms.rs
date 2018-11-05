
use libnx_rs::usbcomms::{UsbCommsInterface};
use server::interface::Device;
use server::prefixes::{PREFIX_LENGTH, Prefixes, CommandPrefix};


const TEST_BLOCK_SIZE : usize = 4096;
impl Device for UsbCommsInterface {

    fn read_prefix(&mut self) -> Result<Prefixes, String> {
        let mut prefix_buff : [u8 ; PREFIX_LENGTH] = [0 ; PREFIX_LENGTH];
        let read_bytes = self.read_bytes(&mut prefix_buff);
        match read_bytes {
            PREFIX_LENGTH => Prefixes::parse_prefix(prefix_buff).ok_or(format!("Could not parse bytes {:?}", prefix_buff).to_owned()), 
            0 => Err("Read 0 bytes for prefix. Is this interface initialized?".to_owned()), 
            n => Err(format!("Bad read prefix result: expected {} but read {} bytes instead.", PREFIX_LENGTH, n).to_owned())
        }
    }

    fn block_size(&self) -> usize {
        TEST_BLOCK_SIZE
    }
    fn read_block(&mut self, buffer: &mut [u8]) -> Result<usize, String> {
        if buffer.len() != TEST_BLOCK_SIZE {
            Err(format!("Bad read block size: expected {} but got block of size {}", TEST_BLOCK_SIZE, buffer.len()).to_owned())
        }
        else {
            let bt_read = self.read_bytes(buffer);
            match bt_read {
                TEST_BLOCK_SIZE => Ok(TEST_BLOCK_SIZE), 
                0 => Err("Read 0 bytes. Is this interface initialized?".to_owned()), 
                n => Err(format!("Bad read block result: expected {} but read {} bytes instead.", TEST_BLOCK_SIZE, n).to_owned())
            }
        }
    }

    fn write_block(&mut self, bytes: &[u8]) -> Result<usize, String> {
        if bytes.len() != TEST_BLOCK_SIZE {
            return Err(format!("Bad write block size: expected {} but got block of size {}", TEST_BLOCK_SIZE, bytes.len()).to_owned());
        }
        else {
            let bt_written = self.write_bytes(bytes);
            match bt_written {
                TEST_BLOCK_SIZE => Ok(TEST_BLOCK_SIZE), 
                0 => Err("Wrote 0 bytes. Is this interface initialized?".to_owned()), 
                n => Err(format!("Bad write block result: expected {} but wrote {} bytes instead.", TEST_BLOCK_SIZE, n).to_owned())
            }
        }
    }
}