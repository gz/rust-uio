Linux UIO library for Rust
--------------------

A thin abstraction library for writing user-space drivers in Linux by using the UIO facility (https://www.kernel.org/doc/html/latest/driver-api/uio-howto.html).

In order to use this library, you'll need to make sure your device uses the Linux UIO driver module. For example, the following
sample commands unload the ahci driver in Linux and use the `uio_pci_generic` module for the SSD disk for the PCI
device with vendor `0x8086` and device id `0x1d02`. (Note: Dangerous, don't do this if you don't know what you're doing).

```
$ modprobe -r ahci
$ sudo modprobe uio
$ sudo modprobe uio_pci_generic
$ echo "0x8086 0x1d02" > /sys/bus/pci/drivers/uio_pci_generic/new_id
$ lspci -v -d :0x1d02 | grep "Kernel driver in use"
```

Afterwards you should have one or more uio devices available in `/dev/uio*` which you can use to instantiate the
UioDevice struct:

```rust
extern crate uio;
use uio::*;

pub fn main() {
    let uio_num = 1; // /dev/uio1
    let dev = UioDevice::new(uio_num).unwrap();
    let bar = dev.map_resource(5).unwrap();
}
```


Resources
--------------------

For more information about UIO check the following links:
  * https://lwn.net/Articles/232575/
  * http://alvarom.com/2014/12/17/linux-user-space-drivers-with-interrupts/
  * http://lxr.free-electrons.com/source/drivers/uio/uio_cif.c
  * https://www.kernel.org/doc/html/latest/driver-api/uio-howto.html
  * http://lxr.free-electrons.com/source/drivers/uio/uio_dmem_genirq.c
  * http://www.osadl.org/projects/downloads/UIO/user/
  * http://dpdk.org/browse/dpdk/tree/tools/dpdk_nic_bind.py
