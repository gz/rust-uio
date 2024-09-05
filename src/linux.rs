use fs2::FileExt;
use libc;
use nix::sys::mman::{MapFlags, ProtFlags};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::prelude::*;
use std::mem::transmute;
use std::num::{NonZeroUsize, ParseIntError};
use std::os::unix::prelude::AsRawFd;

const PAGESIZE: usize = 4096;

#[derive(Debug)]
pub enum UioError {
    Address,
    Size,
    Io(io::Error),
    Map(nix::Error),
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

impl From<nix::Error> for UioError {
    fn from(e: nix::Error) -> Self {
        UioError::Map(e)
    }
}

pub struct UioDevice {
    uio_num: usize,
    //path: &'static str,
    devfile: File,
}

impl Drop for UioDevice {
    fn drop(&mut self) {
        self.devfile
            .unlock()
            .expect("Failed to release lock on /dev/uio* device");
    }
}

impl UioDevice {
    #[deprecated(since = "0.3.0", note = "Use blocking_new or try_new instead")]
    pub fn new(uio_num: usize) -> io::Result<UioDevice> {
        Self::blocking_new(uio_num)
    }

    /// Creates a new UIO device for Linux.
    ///
    /// This variant will block until it can obtain an exclusive lock on the
    /// uio device.
    ///
    /// # Arguments
    ///  * uio_num - UIO index of device (i.e., 1 for /dev/uio1)
    pub fn blocking_new(uio_num: usize) -> io::Result<UioDevice> {
        let path = format!("/dev/uio{}", uio_num);
        let devfile = OpenOptions::new().read(true).write(true).open(path)?;
        devfile.lock_exclusive()?;
        Ok(UioDevice { uio_num, devfile })
    }

    /// Creates a new UIO device for Linux.
    ///
    /// This variant will return Err(`EWOULDBLOCK`) instead of blocking, if it
    /// can't obtain an exclusive lock on the uio device.
    ///
    /// # Arguments
    ///  * uio_num - UIO index of device (i.e., 1 for /dev/uio1)
    pub fn try_new(uio_num: usize) -> io::Result<UioDevice> {
        let path = format!("/dev/uio{}", uio_num);
        let devfile = OpenOptions::new().read(true).write(true).open(path)?;
        devfile.try_lock_exclusive()?;
        Ok(UioDevice { uio_num, devfile })
    }

    /// Return a vector of mappable resources (i.e., PCI bars) including their size.
    pub fn get_resource_info(&mut self) -> Result<Vec<(String, u64)>, UioError> {
        let paths = fs::read_dir(format!("/sys/class/uio/uio{}/device/", self.uio_num))?;

        let mut bars = Vec::new();
        for p in paths {
            let path = p?;
            let file_name = path
                .file_name()
                .into_string()
                .expect("Is valid UTF-8 string.");

            if file_name.starts_with("resource") && file_name.len() > "resource".len() {
                let metadata = fs::metadata(path.path())?;
                bars.push((file_name, metadata.len()));
            }
        }

        Ok(bars)
    }

    /// Maps a given resource into the virtual address space of the process.
    ///
    /// # Arguments
    ///   * bar_nr: The index to the given resource (i.e., 1 for /sys/class/uio/uioX/device/resource1)
    pub fn map_resource(&self, bar_nr: usize) -> Result<*mut libc::c_void, UioError> {
        let filename = format!(
            "/sys/class/uio/uio{}/device/resource{}",
            self.uio_num, bar_nr
        );
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .open(filename.to_string())?;
        let metadata = fs::metadata(filename.clone())?;
        let length = NonZeroUsize::new(metadata.len() as usize).ok_or(UioError::Size)?;
        let fd = f.as_raw_fd();

        let res = unsafe {
            nix::sys::mman::mmap(
                None,
                length,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                fd,
                0 as libc::off_t,
            )
        };
        match res {
            Ok(m) => Ok(m),
            Err(e) => Err(UioError::from(e)),
        }
    }

    fn read_file(&self, path: String) -> Result<String, UioError> {
        let mut file = File::open(path)?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;
        Ok(buffer.trim().to_string())
    }

    /// The amount of events.
    pub fn get_event_count(&self) -> Result<u32, UioError> {
        let filename = format!("/sys/class/uio/uio{}/event", self.uio_num);
        let buffer = self.read_file(filename)?;
        match u32::from_str_radix(&buffer, 10) {
            Ok(v) => Ok(v),
            Err(e) => Err(UioError::from(e)),
        }
    }

    /// UIO device number (e.g. 0 for /dev/uio0)
    pub fn get_num(&self) -> usize {
        self.uio_num
    }

    /// Path to UIO device file (e.g. "/dev/uio0")
    pub fn get_dev_path(&self) -> impl AsRef<std::path::Path> {
        format!("/dev/uio{}", self.uio_num)
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
        let filename = format!(
            "/sys/class/uio/uio{}/maps/map{}/size",
            self.uio_num, mapping
        );
        let buffer = self.read_file(filename)?;
        match usize::from_str_radix(&buffer[2..], 16) {
            Ok(v) => Ok(v),
            Err(e) => Err(UioError::from(e)),
        }
    }

    /// The address of a given mapping.
    ///
    /// # Arguments
    ///  * mapping: The given index of the mapping (i.e., 1 for /sys/class/uio/uioX/maps/map1)
    pub fn map_addr(&self, mapping: usize) -> Result<usize, UioError> {
        let filename = format!(
            "/sys/class/uio/uio{}/maps/map{}/addr",
            self.uio_num, mapping
        );
        let buffer = self.read_file(filename)?;
        match usize::from_str_radix(&buffer[2..], 16) {
            Ok(v) => Ok(v),
            Err(e) => Err(UioError::from(e)),
        }
    }

    /// The name of a given mapping.
    ///
    /// # Arguments
    ///  * mapping: The given index of the mapping (i.e., 1 for /sys/class/uio/uioX/maps/map1)
    pub fn map_name(&self, mapping: usize) -> Result<String, UioError> {
        let filename = format!(
            "/sys/class/uio/uio{}/maps/map{}/name",
            self.uio_num, mapping
        );
        self.read_file(filename)
    }

    /// Return a list of all possible memory mappings.
    #[deprecated(since = "0.3.0", note = "Use get_mapping_info() instead")]
    pub fn get_map_info(&mut self) -> Result<Vec<String>, UioError> {
        let paths = fs::read_dir(format!("/sys/class/uio/uio{}/maps/", self.uio_num))?;

        let mut map = Vec::new();
        for p in paths {
            let path = p?;
            let file_name = path
                .file_name()
                .into_string()
                .expect("Is valid UTF-8 string.");

            if file_name.starts_with("map") && file_name.len() > "map".len() {
                map.push(file_name);
            }
        }

        Ok(map)
    }

    /// Complete information about all Mappings available
    ///
    /// This reads all files under `/sys/class/uio/uioN/maps/*`, where N ==
    /// `self.uio_num`. If any of the files are missing or otherwise unreadable,
    /// that Mapping will be skipped.
    pub fn get_mapping_info(&mut self) -> Result<Vec<MappingInfo>, UioError> {
        let paths = fs::read_dir(format!("/sys/class/uio/uio{}/maps/", self.uio_num))?;

        let mut map = Vec::new();
        'each_map_dir: for p in paths {
            let entry = p?;
            let dir_name = entry.file_name();
            let Some(dir_name) = dir_name.to_str() else {
                break 'each_map_dir;
            };
            if !(entry.file_type()?.is_dir() && dir_name.starts_with("map")) {
                break 'each_map_dir;
            }

            let Ok(index) = dir_name.trim_start_matches("map").parse() else {
                break 'each_map_dir;
            };

            let addr = self.map_addr(index)?;
            let name = self.map_name(index)?;
            let len = self.map_size(index)?;

            map.push(MappingInfo {
                index,
                addr,
                len,
                name,
            });
        }

        Ok(map)
    }

    /// Map an available memory mapping.
    ///
    /// # Arguments
    ///  * mapping: The given index of the mapping (i.e., 1 for /sys/class/uio/uioX/maps/map1)
    pub fn map_mapping(&self, mapping: usize) -> Result<*mut libc::c_void, UioError> {
        let offset = mapping * PAGESIZE;
        let fd = self.devfile.as_raw_fd();
        let map_size = self.map_size(mapping)?;
        let map_size = NonZeroUsize::new(map_size).ok_or(UioError::Size)?;

        let res = unsafe {
            nix::sys::mman::mmap(
                None,
                map_size,
                ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
                MapFlags::MAP_SHARED,
                fd,
                offset as libc::off_t,
            )
        };
        match res {
            Ok(m) => Ok(m),
            Err(e) => Err(UioError::from(e)),
        }
    }

    /// Enable interrupt
    pub fn irq_enable(&mut self) -> io::Result<()> {
        let bytes: [u8; 4] = unsafe { transmute(1u32) };
        self.devfile.write(&bytes)?;
        Ok(())
    }

    /// Disable interrupt
    pub fn irq_disable(&mut self) -> io::Result<()> {
        let bytes: [u8; 4] = unsafe { transmute(0u32) };
        self.devfile.write(&bytes)?;
        Ok(())
    }

    /// Wait for interrupt
    pub fn irq_wait(&mut self) -> io::Result<u32> {
        let mut bytes: [u8; 4] = [0, 0, 0, 0];
        self.devfile.read(&mut bytes)?;
        Ok(unsafe { transmute(bytes) })
    }
}

/// All information about one of a UioDevice's Mapping
/// This is a dump of everything contained in `/sys/class/uio/uio{n}/maps/map*/*`
pub struct MappingInfo {
    /// Index of the Mapping
    ///
    /// E.g. the `0` in `.../maps/map0`
    pub index: usize,

    /// Physical address of the Mapping
    pub addr: usize,

    /// Length in bytes of the Mapping region
    pub len: usize,

    /// Name supplied by the UIO device
    ///
    /// Typically this would be set in a device-tree entry
    pub name: String,
}

#[cfg(test)]
mod tests {

    #[test]
    fn open() {
        let res = ::linux::UioDevice::try_new(0);
        match res {
            Err(e) => {
                panic!("Can not open device /dev/uio0: {}", e);
            }
            Ok(_f) => (),
        }
    }

    #[test]
    fn print_info() {
        let res = ::linux::UioDevice::try_new(0).unwrap();
        let name = res.get_name().expect("Can't get name");
        let version = res.get_version().expect("Can't get version");
        let event_count = res.get_event_count().expect("Can't get event count");
        assert_eq!(name, "uio_pci_generic");
        assert_eq!(version, "0.01.0");
        assert_eq!(event_count, 0);
    }

    #[test]
    fn map() {
        let res = ::linux::UioDevice::try_new(0).unwrap();
        let bars = res.map_resource(5);
        match bars {
            Err(e) => {
                panic!("Can not map PCI stuff: {:?}", e);
            }
            Ok(_f) => (),
        }
    }

    #[test]
    fn bar_info() {
        let mut res = ::linux::UioDevice::try_new(0).unwrap();
        let bars = res.get_resource_info();
        match bars {
            Err(e) => {
                panic!("Can not map PCI stuff: {:?}", e);
            }
            Ok(_f) => (),
        }
    }
}
