
use commands::{FileContentStorer, FileRetriever};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub struct StdFile {
    path : String,
    file : File,
}
impl FileContentStorer for StdFile {
    fn for_name(name : &str, _size : usize) -> Result<Self, String>  {

        let file = File::create(name).map_err(|e| format!("Error creating file: {:?}", e))?;

        Ok(StdFile {
            path : name.to_owned(),
            file
        })

    }
    fn push_bytes(&mut self, buffer : &[u8]) -> Result<usize, String>  {
        self.file.write(buffer).map_err(|e| format!("File write err: {:?}", e))
    }

}

impl FileRetriever for StdFile {
    fn name(&self) -> &str {
        &self.path
    }
    fn read_bytes(&mut self, buffer : &mut [u8]) -> Result<usize, String> {
        self.file.read(buffer).map_err(|e| format!("File read err: {:?}", e))
    }
}