use std::io;
use std::io::prelude::*;
use std::os::unix::prelude::AsRawFd;
use std::fs::{File};
use std::num::{ParseIntError};
use std::error::Error;
use std::str::FromStr;
use std::fmt;
use libc::{MAP_SHARED};
use mmap;

pub struct UioDevice {
    uio_num: usize,
    //path: &'static str,
    devfile: File,
    mappings: Vec<[u8; 4096]>,
}

#[derive(Debug)]
pub enum UioError {
    Address,
    Io(io::Error),
    Parse,
}

impl From<io::Error> for UioError {
    fn from(e: io::Error) -> Self {
        UioError::Io(e)
    }
}

impl From<ParseIntError> for UioError {
    fn from(e: ParseIntError) -> Self {
        UioError::Parse
    }
}

/*

int size = 8192;
struct cif *fi;
pthread_mutexattr_t attr;

cif_status_thread_stop = 0;

fi = (struct cif *)calloc (1, sizeof (struct cif));
if (!fi)
    goto out;

fi->size = size;

fi->fd = open (name, O_RDWR);
if (fi->fd == -1)
    goto out_free;

fi->io_plx = mmap (NULL, 128, PROT_READ | PROT_WRITE,
           MAP_SHARED, fi->fd, 0 );
if (fi->io_plx == MAP_FAILED)
    goto out_close;

fi->io = mmap (NULL, size, PROT_READ | PROT_WRITE,
           MAP_SHARED, fi->fd, getpagesize() );
if (fi->io == MAP_FAILED)
    goto out_close;

 */

impl UioDevice {

    pub fn new(uio_num: usize) -> io::Result<UioDevice> {
        let path = format!("/dev/uio{}", uio_num);
        let mut f = try!(File::open(path));

        let fd = f.as_raw_fd();
        let io_plx = mmap::MemoryMap::new(128,
                    &[ mmap::MapOption::MapFd(fd),
                       mmap::MapOption::MapOffset(0),
                       mmap::MapOption::MapNonStandardFlags(MAP_SHARED),
                       mmap::MapOption::MapReadable,
                       //mmap::MapOption::MapWritable
                       ]);
        match io_plx {
            Err(e) => { panic!("Can not mmap device region: {}", e); },
            Ok(m) => (),
        }


        Ok( UioDevice { uio_num: uio_num, devfile: f, mappings: Vec::new() } )
    }

    fn read_file(&self, path: String) -> Result<String, UioError> {
        let mut file = try!(File::open(path));
        let mut buffer = String::new();
        try!(file.read_to_string(&mut buffer));
        Ok(buffer.trim().to_string())
    }

    fn parse_from<T>(&self, path: String) -> Result<T, UioError>
       where T: FromStr {
        let buffer = try!(self.read_file(path));
        println!("buffer: {:?}", buffer);

        match buffer.parse::<T>() {
            Err(e) => { Err(UioError::Parse) },
            Ok(addr) => Ok(addr)
        }
    }

    fn get_mem_size(&self, mapping: usize) -> Result<usize, UioError> {
        let filename = format!("/sys/class/uio/uio{}/maps/map{}/size", self.uio_num, mapping);
        self.parse_from::<usize>(filename)
    }

    fn get_mem_addr(&self, mapping: usize) -> Result<usize, UioError> {
        let filename = format!("/sys/class/uio/uio{}/maps/map{}/addr", self.uio_num, mapping);
        self.parse_from::<usize>(filename)
    }

    pub fn get_event_count(&self) -> Result<u32, UioError> {
        let filename = format!("/sys/class/uio/uio{}/event", self.uio_num);
        self.parse_from::<u32>(filename)
    }

    pub fn get_name(&self) -> Result<String, UioError> {
        let filename = format!("/sys/class/uio/uio{}/name", self.uio_num);
        self.read_file(filename)
    }

    pub fn get_version(&self) -> Result<String, UioError> {
        let filename = format!("/sys/class/uio/uio{}/version", self.uio_num);
        self.read_file(filename)
    }


    pub fn map(&self, mapping: usize) -> Result<mmap::MemoryMap, mmap::MapError> {
        let offset = mapping * 4096; // TODO: use getpagesize()
        let fd = self.devfile.as_raw_fd();
        let map_size = self.get_mem_size(mapping).unwrap(); // TODO

        let res = mmap::MemoryMap::new(map_size,
            &[ mmap::MapOption::MapFd(fd),
               mmap::MapOption::MapOffset(offset),
               mmap::MapOption::MapReadable  ]); // TODO: Add MAP_SHARED
        res
    }

}

#[cfg(test)]
mod tests {

    #[test]
    fn open() {
        let res = ::linux::UioDevice::new(0);
        match res {
            Err(e) => { println!("Can not open device /dev/uio0: {}", e); assert!(false); },
            Ok(f) => (),
        }
    }


    #[test]
    fn print_info() {
        let res = ::linux::UioDevice::new(0).unwrap();
        let name = res.get_name().expect("Can't get name");
        let version = res.get_version().expect("Can't get version");
        let event_count = res.get_event_count().expect("Can't get event count");
        assert_eq!(name, "uio_pci_generic");
        assert_eq!(version, "0.01.0");
        assert_eq!(event_count, 0);
    }

    fn map() {
        let res = ::linux::UioDevice::new(0).unwrap();
        let mapping = res.map(0);
        match mapping {
            Err(e) => { println!("Can not mmap device region: {}", e); assert!(false); },
            Ok(m) => (),
        }
    }

}
