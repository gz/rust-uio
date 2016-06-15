use std::io;
use std::io::prelude::*;
use std::os::unix::prelude::AsRawFd;
use std::fs;
use std::fs::{File, OpenOptions};
use std::num::{ParseIntError};
use std::error::Error;
use std::str::FromStr;
use libc::{MAP_SHARED};
use mmap;

const PAGESIZE: usize = 4096;

#[derive(Debug)]
pub enum UioError {
    Address,
    Io(io::Error),
    Map(mmap::MapError),
    Parse,
}

impl From<io::Error> for UioError {
    fn from(e: io::Error) -> Self {
        UioError::Io(e)
    }
}

impl From<ParseIntError> for UioError {
    fn from(_: ParseIntError) -> Self {
        UioError::Parse
    }
}

impl From<mmap::MapError> for UioError {
    fn from(e: mmap::MapError) -> Self {
        UioError::Map(e)
    }
}

pub struct UioDevice {
    uio_num: usize,
    //path: &'static str,
    devfile: File,
}

impl UioDevice {

    /// Creates a new UIO device for Linux.
    ///
    /// # Arguments
    ///  * uio_num - UIO index of device (i.e., 1 for /dev/uio1)
    pub fn new(uio_num: usize) -> io::Result<UioDevice> {
        let path = format!("/dev/uio{}", uio_num);
        let f = try!(File::open(path));
        Ok( UioDevice { uio_num: uio_num, devfile: f } )
    }

    /// Return a vector of mappable resources (i.e., PCI bars) including their size.
    pub fn get_resource_info(&mut self) -> Result<Vec<(String, u64)>, UioError> {
        let paths = try!(fs::read_dir(format!("/sys/class/uio/uio{}/device/", self.uio_num)));

        let mut bars = Vec::new();
        for p in paths {
            let path = try!(p);
            let file_name = path.file_name().into_string().expect("Is valid UTF-8 string.");

            if file_name.starts_with("resource") && file_name.len() > "resource".len() {
                let metadata = try!(fs::metadata(path.path()));
                bars.push( (file_name, metadata.len()) );
            }
        }

        Ok(bars)
    }

    /// Maps a given resource into the virtual address space of the process.
    ///
    /// # Arguments
    ///   * bar_nr: The index to the given resource (i.e., 1 for /sys/class/uio/uioX/device/resource1)
    pub fn map_resource(&self, bar_nr: usize) -> Result<mmap::MemoryMap, UioError> {
        let filename = format!("/sys/class/uio/uio{}/device/resource{}", self.uio_num, bar_nr);
        let f = try!(OpenOptions::new().read(true).write(true).open(filename.to_string()));
        let metadata = try!(fs::metadata(filename.clone()));
        let fd = f.as_raw_fd();

        let res = try!(mmap::MemoryMap::new(metadata.len() as usize,
                    &[ mmap::MapOption::MapFd(fd),
                       mmap::MapOption::MapOffset(0),
                       mmap::MapOption::MapNonStandardFlags(MAP_SHARED),
                       mmap::MapOption::MapReadable, mmap::MapOption::MapWritable
                       ]));

        Ok(res)
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

        match buffer.parse::<T>() {
            Err(_) => { Err(UioError::Parse) },
            Ok(addr) => Ok(addr)
        }
    }

    /// The amount of events.
    pub fn get_event_count(&self) -> Result<u32, UioError> {
        let filename = format!("/sys/class/uio/uio{}/event", self.uio_num);
        self.parse_from::<u32>(filename)
    }

    /// The name of the UIO device.
    pub fn get_name(&self) -> Result<String, UioError> {
        let filename = format!("/sys/class/uio/uio{}/name", self.uio_num);
        self.read_file(filename)
    }

    /// The version of the UIO driver.
    pub fn get_version(&self) -> Result<String, UioError> {
        let filename = format!("/sys/class/uio/uio{}/version", self.uio_num);
        self.read_file(filename)
    }

    /// The size of a given mapping.
    ///
    /// # Arguments
    ///  * mapping: The given index of the mapping (i.e., 1 for /sys/class/uio/uioX/maps/map1)
    pub fn map_size(&self, mapping: usize) -> Result<usize, UioError> {
        let filename = format!("/sys/class/uio/uio{}/maps/map{}/size", self.uio_num, mapping);
        self.parse_from::<usize>(filename)
    }

    /// The address of a given mapping.
    ///
    /// # Arguments
    ///  * mapping: The given index of the mapping (i.e., 1 for /sys/class/uio/uioX/maps/map1)
    pub fn map_addr(&self, mapping: usize) -> Result<usize, UioError> {
        let filename = format!("/sys/class/uio/uio{}/maps/map{}/addr", self.uio_num, mapping);
        self.parse_from::<usize>(filename)
    }

    /// Return a list of all possible memory mappings.
    pub fn get_map_info(&mut self) -> Result<Vec<String>, UioError> {
        let paths = try!(fs::read_dir(format!("/sys/class/uio/uio{}/maps/", self.uio_num)));

        let mut map = Vec::new();
        for p in paths {
            let path = try!(p);
            let file_name = path.file_name().into_string().expect("Is valid UTF-8 string.");

            if file_name.starts_with("map") && file_name.len() > "map".len() {
                map.push(file_name);
            }
        }

        Ok(map)
    }

    /// Map an available memory mapping.
    ///
    /// # Arguments
    ///  * mapping: The given index of the mapping (i.e., 1 for /sys/class/uio/uioX/maps/map1)
    pub fn map_mapping(&self, mapping: usize) -> Result<mmap::MemoryMap, UioError> {
        let offset = mapping * PAGESIZE;
        let fd = self.devfile.as_raw_fd();
        let map_size = self.map_size(mapping).unwrap(); // TODO

        let res = try!(mmap::MemoryMap::new(map_size,
                       &[ mmap::MapOption::MapFd(fd),
                          mmap::MapOption::MapOffset(offset),
                          mmap::MapOption::MapNonStandardFlags(MAP_SHARED),
                          mmap::MapOption::MapReadable, mmap::MapOption::MapWritable  ]));
        Ok(res)
    }

}

#[cfg(test)]
mod tests {

    #[test]
    fn open() {
        let res = ::linux::UioDevice::new(0);
        match res {
            Err(e) => { panic!("Can not open device /dev/uio0: {}", e); },
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

    #[test]
    fn map() {
        let mut res = ::linux::UioDevice::new(0).unwrap();
        let bars = res.map_resource(5);
        match bars {
            Err(e) => { panic!("Can not map PCI stuff: {:?}", e); },
            Ok(f) => (),
        }

    }

    #[test]
    fn bar_info() {
        let mut res = ::linux::UioDevice::new(0).unwrap();
        let bars = res.get_resource_info();
        match bars {
            Err(e) => { panic!("Can not map PCI stuff: {:?}", e); },
            Ok(f) => (),
        }

    }
}
