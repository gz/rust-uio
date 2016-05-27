use std::io;
use std::io::prelude::*;
use std::os::unix::prelude::AsRawFd;
use std::fs::{File};
use mmap;

pub struct UioDevice {
    uio_num: usize,
    //path: &'static str,
    devfile: File,
    mappings: Vec<[u8; 4096]>,
}

/*
int uio_get_mem_size(struct uio_info_t* info, int map_num)
{
	int ret;
	char filename[64];
	info->maps[map_num].size = UIO_INVALID_SIZE;
	sprintf(filename, "/sys/class/uio/uio%d/maps/map%d/size",
		info->uio_num, map_num);
	FILE* file = fopen(filename,"r");
	if (!file) return -1;
	ret = fscanf(file,"0x%lx",&info->maps[map_num].size);
	fclose(file);
	if (ret<0) return -2;
	return 0;
}

int uio_get_mem_addr(struct uio_info_t* info, int map_num)
{
	int ret;
	char filename[64];
	info->maps[map_num].addr = UIO_INVALID_ADDR;
	sprintf(filename, "/sys/class/uio/uio%d/maps/map%d/addr",
		info->uio_num, map_num);
	FILE* file = fopen(filename,"r");
	if (!file) return -1;
	ret = fscanf(file,"0x%lx",&info->maps[map_num].addr);
	fclose(file);
	if (ret<0) return -2;
	return 0;
}
*/

enum UioError {
    Address,
}

impl UioDevice {

    pub fn new(uio_num: usize) -> io::Result<UioDevice> {
        let path = format!("/dev/{}", uio_num);
        let mut f = try!(File::open(path));
        Ok( UioDevice { uio_num: uio_num, devfile: f, mappings: Vec::new() } )
    }

    fn get_mem_addr(&self, mapping: usize) -> Result<usize, UioError> {
        let filename = format!("/sys/class/uio/uio{}/maps/map{}/addr", self.uio_num, mapping);
        let mut file = try!(File::open(filename));
        let mut buffer = String::new();
        try!(file.read_to_string(&mut buffer));

        let addr = try!(buffer.parse::<usize>());
        Ok(addr)
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
