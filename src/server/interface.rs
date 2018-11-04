use server::prefixes::Prefixes;

pub trait ServerDevice {
    fn read_prefix(&mut self) -> Result<Prefixes, String>;
    fn block_size(&self) -> usize;
    fn read_block(&mut self, buffer: &mut [u8]) -> Result<usize, String>;
    fn write_block(&mut self, bytes: &[u8]) -> Result<usize, String>;
}