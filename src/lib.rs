
extern crate libusb;

use std::error::Error;
use std::slice;
use std::time::Duration;

const CH341_VID: u16 = 0x1a86;
const CH341_PID: u16 = 0x5512;

const CH341_I2C_LOW_SPEED: u8 = 0;      // low speed - 20kHz
const CH341_I2C_STANDARD_SPEED: u8 = 1; // standard speed - 100kHz
const CH341_I2C_FAST_SPEED: u8 = 2;     // fast speed - 400kHz
const CH341_I2C_HIGH_SPEED: u8 = 3;     // high speed - 750kHz

const CH341_CMD_I2C_STREAM: u8 = 0xAA;

const CH341_CMD_I2C_STM_STA: u8 = 0x74;
const CH341_CMD_I2C_STM_STO: u8 = 0x75;
const CH341_CMD_I2C_STM_OUT: u8 = 0x80;
const CH341_CMD_I2C_STM_IN: u8 = 0xC0;
const CH341_CMD_I2C_STM_SET: u8 = 0x60;
const CH341_CMD_I2C_STM_END: u8 = 0x00;

pub struct Device {
    dev: libusb::Device,
    desc: libusb::DeviceDescriptor,
    ctx: libusb::Context,
    //handle: libusb::DeviceHandle<'a>,
}

fn open_device(context: &mut libusb::Context, vid: u16, pid: u16) -> Option<(libusb::Device, libusb::DeviceDescriptor, libusb::DeviceHandle)> {
    let devices = match context.devices() {
        Ok(d) => d,
        Err(_) => return None
    };

    for device in devices.iter() {
        let device_desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue
        };

        if device_desc.vendor_id() == vid && device_desc.product_id() == pid {
            match device.open() {
                Ok(handle) => return Some((device, device_desc, handle)),
                Err(_) => continue
            }
        }
    }

    None
}

/* 
pub fn new<'a>() -> Result<Device<'a>, Box<dyn Error>> {
    match libusb::Context::new() {
        Ok(mut context) => {
            match open_device(&mut context, CH341_VID, CH341_PID) {
                Some((device, device_desc, handle)) => Ok(
                    Device {
                        dev: device,
                        desc: device_desc,
                        //handle: handle,
                    }
                ),
                None => panic!("could not find device {:04x}:{:04x}", CH341_VID, CH341_PID)
            }
        },
        Err(e) => panic!("could not initialize libusb: {}", e)
    }
}
*/

/*
impl Device<'_> {
    fn bulk_xfer(&mut self, out: Vec<u8>) -> Option<Vec<u8>> {
        
        match self.handle.write_bulk(0x00, out.as_slice(), Duration::from_secs(1)) {
            Ok(len) => {
                
            },
            Err(err) => println!("could not write from endpoint: {}", err)
        }

        let mut array: [u8; 32] = [0; 32];
        match self.handle.read_bulk(0x00, &mut array, Duration::from_secs(1)) {
            Ok(len) => {
                Some(array[0..len].to_vec())
            },
            Err(err) => { println!("could not read from endpoint: {}", err); None }
        }
    }
}
*/
/*
#[derive(Debug)]
struct Endpoint {
    config: u8,
    iface: u8,
    setting: u8,
    address: u8
}


fn read_device(device: &mut libusb::Device, device_desc: &libusb::DeviceDescriptor, handle: &mut libusb::DeviceHandle) -> libusb::Result<()> {
    try!(handle.reset());

    let timeout = Duration::from_secs(1);
    let languages = try!(handle.read_languages(timeout));

    println!("Active configuration: {}", try!(handle.active_configuration()));
    println!("Languages: {:?}", languages);

    if languages.len() > 0 {
        let language = languages[0];

        println!("Manufacturer: {:?}", handle.read_manufacturer_string(language, device_desc, timeout).ok());
        println!("Product: {:?}", handle.read_product_string(language, device_desc, timeout).ok());
        println!("Serial Number: {:?}", handle.read_serial_number_string(language, device_desc, timeout).ok());
    }

    match find_readable_endpoint(device, device_desc, libusb::TransferType::Interrupt) {
        Some(endpoint) => read_endpoint(handle, endpoint, libusb::TransferType::Interrupt),
        None => println!("No readable interrupt endpoint")
    }

    match find_readable_endpoint(device, device_desc, libusb::TransferType::Bulk) {
        Some(endpoint) => read_endpoint(handle, endpoint, libusb::TransferType::Bulk),
        None => println!("No readable bulk endpoint")
    }

    Ok(())
}

fn find_readable_endpoint(device: &mut libusb::Device, device_desc: &libusb::DeviceDescriptor, transfer_type: libusb::TransferType) -> Option<Endpoint> {
    for n in 0..device_desc.num_configurations() {
        let config_desc = match device.config_descriptor(n) {
            Ok(c) => c,
            Err(_) => continue
        };

        for interface in config_desc.interfaces() {
            for interface_desc in interface.descriptors() {
                for endpoint_desc in interface_desc.endpoint_descriptors() {
                    if endpoint_desc.direction() == libusb::Direction::In && endpoint_desc.transfer_type() == transfer_type {
                        return Some(Endpoint {
                            config: config_desc.number(),
                            iface: interface_desc.interface_number(),
                            setting: interface_desc.setting_number(),
                            address: endpoint_desc.address()
                        });
                    }
                }
            }
        }
    }

    None
}

fn read_endpoint(handle: &mut libusb::DeviceHandle, endpoint: Endpoint, transfer_type: libusb::TransferType) {
    println!("Reading from endpoint: {:?}", endpoint);

    let has_kernel_driver = match handle.kernel_driver_active(endpoint.iface) {
        Ok(true) => {
            handle.detach_kernel_driver(endpoint.iface).ok();
            true
        },
        _ => false
    };

    println!(" - kernel driver? {}", has_kernel_driver);

    match configure_endpoint(handle, &endpoint) {
        Ok(_) => {
            let mut vec = Vec::<u8>::with_capacity(256);
            let mut buf = unsafe { slice::from_raw_parts_mut((&mut vec[..]).as_mut_ptr(), vec.capacity()) };

            let timeout = Duration::from_secs(1);

            match transfer_type {
                libusb::TransferType::Interrupt => {
                    match handle.read_interrupt(endpoint.address, buf, timeout) {
                        Ok(len) => {
                            unsafe { vec.set_len(len) };
                            println!(" - read: {:?}", vec);
                        },
                        Err(err) => println!("could not read from endpoint: {}", err)
                    }
                },
                libusb::TransferType::Bulk => {
                    match handle.read_bulk(endpoint.address, buf, timeout) {
                        Ok(len) => {
                            unsafe { vec.set_len(len) };
                            println!(" - read: {:?}", vec);
                        },
                        Err(err) => println!("could not read from endpoint: {}", err)
                    }
                },
                _ => ()
            }
        },
        Err(err) => println!("could not configure endpoint: {}", err)
    }

    if has_kernel_driver {
        handle.attach_kernel_driver(endpoint.iface).ok();
    }
}

fn configure_endpoint<'a>(handle: &'a mut libusb::DeviceHandle, endpoint: &Endpoint) -> libusb::Result<()> {
    try!(handle.set_active_configuration(endpoint.config));
    try!(handle.claim_interface(endpoint.iface));
    try!(handle.set_alternate_setting(endpoint.iface, endpoint.setting));
    Ok(())
}
*/
pub fn really_complicated_code(a: u8, b: u8) -> Result<u8, Box<dyn Error>> {
    Ok(a + b)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
