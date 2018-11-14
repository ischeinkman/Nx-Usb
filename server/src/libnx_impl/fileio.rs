use commands::FileReader;
use commands::FileWriter;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::fs::OpenOptions;
use std::os::unix::fs::MetadataExt;
use std::io::Seek;
use libnx_rs::fs::{FileSystem};
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
pub struct StdFileWriter {
    file: File,
}

impl FileWriter for StdFileWriter {
    fn new(file_name: &str) -> Result<Self, String> {
        let pt = Path::new(file_name);
        //For now we will error on file update b/c we're still not production-grade.
        if pt.exists() {
            Err(format!("File with name {} already exists!", file_name).to_owned())
        } else {
            let fl =
                File::create(pt).map_err(|e| format!("File create err: {:?}", e).to_owned())?;
            Ok(StdFileWriter { file: fl })
        }
    }

    fn write_bytes(&mut self, buffer: &[u8]) -> Result<usize, String> {
        self.file
            .write(buffer)
            .map_err(|e| format!("File write error: {:?}", e).to_owned())
    }
}

pub struct StdFileReader {
    path: String,
    file: Option<File>,
    idx: usize,
    file_len : usize, 
    finished: bool,
}

const LEN_BUFFER_SIZE : usize = 4 * 1024 * 1024;
impl FileReader for StdFileReader {
    fn new(file_name: &str) -> Result<Self, String> {
        let pt = Path::new(file_name);
        dprintln!("Creating StdFileReader for file {}.", file_name);
        let (file, ln): (Option<File>, usize) = if !file_name.ends_with('/') {
            dprintln!("It's a file; now opening.");
            let mut fl = File::open(pt).map_err(|e| format!("File open error: {:?}", e).to_owned())?;

            let mut ln : usize = 0; 
            let mut garbage : Vec<u8> = Vec::with_capacity(LEN_BUFFER_SIZE);
            garbage.resize(LEN_BUFFER_SIZE, 0);
            let mut rd;
            loop {
                rd = fl.read(&mut garbage).map_err(|e| format!("Fl.read error when calcing size: {:?}",e))?;
                ln += rd;
                if rd == 0 {
                    break;
                }
            }
            fl.seek(std::io::SeekFrom::Start(0)).map_err(|e| format!("Seek err: {:?}", e))?;
            (Some(fl), ln)
        } else {
            dprintln!("Not a file.");
            (None, 0)
        };

        Ok(StdFileReader {
            path: file_name.to_owned(),
            file,
            idx: 0,
            file_len : ln,  
            finished: !pt.exists(),
        })
    }

    fn len(&self) -> usize {
        self.file_len
    }

    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, String> {
        let buflen = buffer.len();
        if self.finished {
            Ok(0)
        } else if let Some(fl) = &mut self.file {
            let rd = fl
                .read(buffer)
                .map_err(|e| format!("File read error: {:?}", e).to_owned())?;
            self.idx += rd;
            if rd < buflen {
                self.finished = true;
            }
            Ok(rd)
        } else {
            let dirpath = Path::new(&self.path);
            let ents = dirpath
                .read_dir()
                .map_err(|e| format!("Read dir error: {:?}", e).to_owned())?;
            let mut cur_offset = 0usize;
            for ent in ents.into_iter() {
                let ent = ent.map_err(|e| format!("Read entry error: {:?}", e).to_owned())?;
                let raw_name = ent
                    .file_name()
                    .into_string()
                    .map_err(|_| format!("Could not convert OsString.").to_owned())?;
                let name_str = format!("{}\0", raw_name);
                let name_bytes = name_str.as_bytes();

                let bytes_to_skip = self.idx - cur_offset;
                if bytes_to_skip > name_bytes.len() {
                    cur_offset += name_bytes.len();
                    continue;
                }
                let bytes_need_write = &name_bytes[bytes_to_skip..];
                if buflen - cur_offset < bytes_need_write.len() {
                    buffer[cur_offset..]
                        .copy_from_slice(&bytes_need_write[0..(buflen - cur_offset)]);
                    self.idx += buflen;
                    return Ok(buflen);
                }
                buffer[cur_offset..bytes_need_write.len()].copy_from_slice(bytes_need_write);
                cur_offset += bytes_need_write.len();
            }
            self.idx += cur_offset;
            self.finished = true;
            Ok(cur_offset)
        }
    }
}
