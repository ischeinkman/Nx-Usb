use nxusb::prefixes::{Prefixes};

pub trait ClientDevice {
    fn push_prefix(&mut self, prefix : Prefixes) -> Result<usize, String>;
    fn block_size(&self) -> usize;
    fn pull_block(&mut self, buffer: &mut [u8]) -> Result<usize, String>;
    fn push_block(&mut self, bytes: &[u8]) -> Result<usize, String>;
}