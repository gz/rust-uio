use std::io;
use std::os::unix::prelude::AsRawFd;
use std::fs::{File};
use mmap;

pub struct UioDevice {
    path: &'static str,
    devfile: File,
    mappings: Vec<[u8; 4096]>,
}

impl UioDevice {

    pub fn new(path: &'static str) -> io::Result<UioDevice> {
        let mut f = try!(File::open(path));
        Ok( UioDevice { path: path, devfile: f, mappings: Vec::new() } )
    }

    pub fn map(&self, mapping: usize) -> Result<mmap::MemoryMap, mmap::MapError> {
        let offset = mapping * 4096; // TODO: use getpagesize()
        let fd = self.devfile.as_raw_fd();

        let res = mmap::MemoryMap::new(4096,
            &[ mmap::MapOption::MapFd(fd),
               mmap::MapOption::MapOffset(offset) //mmap::MapOption::MapReadable, mmap::MapOption::MapWritable
               ]);
        res
    }

}


#[cfg(test)]
mod tests {

    #[test]
    fn open() {
        let res = ::linux::UioDevice::new("/dev/uio0");
        match res {
            Err(e) => { println!("Can not open device /dev/uio0: {}", e); assert!(false); },
            Ok(f) => (),
        }
    }

    #[test]
    fn map() {
        let res = ::linux::UioDevice::new("/dev/uio0").unwrap();
        let mapping = res.map(0);
        match mapping {
            Err(e) => { println!("Can not mmap device region: {}", e); assert!(false); },
            Ok(m) => (),
        }
    }

}
