#[macro_use]
extern crate lazy_static;
extern crate libusb;

use libusb::{Direction, TransferType};
use std::error::Error;
use std::time::Duration;
use log::{debug, error};

pub use embedded_hal;

pub enum I2CSpeed {
    Low,      // 20kHz
    Standard, // 100kHz
    Fast,     // 400kHz
    High,     // 750kHz
}

const CH341_VID: u16 = 0x1a86;
const CH341_PID: u16 = 0x5512;

const CH341_I2C_LOW_SPEED: u8 = 0; // low speed - 20kHz
const CH341_I2C_STANDARD_SPEED: u8 = 1; // standard speed - 100kHz
const CH341_I2C_FAST_SPEED: u8 = 2; // fast speed - 400kHz
const CH341_I2C_HIGH_SPEED: u8 = 3; // high speed - 750kHz

const CH341_CMD_I2C_STREAM: u8 = 0xAA;

const CH341_CMD_I2C_STM_STA: u8 = 0x74;
const CH341_CMD_I2C_STM_STO: u8 = 0x75;
const CH341_CMD_I2C_STM_OUT: u8 = 0x80;
const CH341_CMD_I2C_STM_IN: u8 = 0xC0;
const CH341_CMD_I2C_STM_SET: u8 = 0x60;
const CH341_CMD_I2C_STM_END: u8 = 0x00;

const DEFAULT_USB_TIMEOUT: Duration = Duration::from_millis(1000);

lazy_static! {
    static ref CONTEXT: libusb::Result<libusb::Context> = libusb::Context::new();
}

pub struct Device<'a> {
    _dev: libusb::Device<'a>,
    _desc: libusb::DeviceDescriptor,
    handle: libusb::DeviceHandle<'a>,
    ep_in: u8,
    ep_out: u8,
}

fn open_device<F>(
    context: &libusb::Context,
    func: F,
) -> Option<(
    libusb::Device,
    libusb::DeviceDescriptor,
    libusb::DeviceHandle,
)>
where
    F: Fn(&libusb::Device, &libusb::DeviceDescriptor) -> bool,
{
    let devices = match context.devices() {
        Ok(d) => d,
        Err(_) => return None,
    };

    for device in devices.iter() {
        let desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };

        debug!("found {:#06x}:{:#06x}", desc.vendor_id(), desc.product_id());
        if func(&device, &desc) {
            match device.open() {
                Ok(handle) => return Some((device, desc, handle)),
                Err(_) => continue,
            }
        }
    }

    None
}

fn open_device_vid_pid(
    context: &libusb::Context,
    vid: u16,
    pid: u16,
) -> Option<(
    libusb::Device,
    libusb::DeviceDescriptor,
    libusb::DeviceHandle,
)> {
    open_device(context, |_, d| d.vendor_id() == vid && d.product_id() == pid)
}

fn open_device_sn(
    context: &libusb::Context,
    sn: String
) -> Option<(
    libusb::Device,
    libusb::DeviceDescriptor,
    libusb::DeviceHandle,
)> {
    open_device(context, |dev, desc| {
        match dev.open() {
            Err(_) => return false,
            Ok(dh) => {
                match dh.read_languages(DEFAULT_USB_TIMEOUT) {
                    Err(_) => return false,
                    Ok(langs) => {
                        for lang in langs {
                            match dh.read_serial_number_string(lang, desc, DEFAULT_USB_TIMEOUT) {
                                Err(e) => { error!("{}", e.strerror()) },
                                Ok(dev_sn) => {
                                    debug!("checking sn: {}", dev_sn.clone());
                                    if dev_sn == sn {
                                        return true;
                                    }
                                },
                            }
                        }
                        false
                    }
                }
            }
        }
    })
}


fn find_bulk_endpoints(dev: &libusb::Device) -> Result<(u8, u8), Box<dyn Error>> {
    let config_desc = dev.active_config_descriptor()?;
    let not_found = std::io::Error::from(std::io::ErrorKind::NotFound);
    let interface0 = config_desc.interfaces().nth(0).ok_or(not_found)?;

    let mut ep_in: Option<u8> = None;
    let mut ep_out: Option<u8> = None;
    for id in interface0.descriptors() {
        for ed in id.endpoint_descriptors() {
            if ed.transfer_type() != TransferType::Bulk {
                continue;
            }
            if ep_in.is_none() && ed.direction() == Direction::In {
                ep_in = Some(ed.address());
            }
            if ep_out.is_none() && ed.direction() == Direction::Out {
                ep_out = Some(ed.address());
            }
        }
    }

    Ok((ep_out.unwrap_or_default(), ep_in.unwrap_or_default()))
}

pub fn new<'a>() -> Result<Device<'a>, Box<dyn Error>> {
    match CONTEXT.as_ref() {
        Ok(context) => match open_device_vid_pid(context, CH341_VID, CH341_PID) {
            Some((device, device_desc, handle)) => {
                let (ep_out, ep_in) = find_bulk_endpoints(&device)?;
                let mut d = Device {
                    _dev: device,
                    _desc: device_desc,
                    handle: handle,
                    ep_in: ep_in,
                    ep_out: ep_out,
                };
                d.set_speed(I2CSpeed::Standard)?;
                Ok(d)
            }
            None => {
                let not_found = std::io::Error::from(std::io::ErrorKind::NotFound);
                error!("could not find device {:04x}:{:04x}", CH341_VID, CH341_PID);
                Err(Box::new(not_found))
            }
        },
        Err(err) => Err(Box::new(err)),
    }
}

pub fn new_with_sn<'a>(sn: String) -> Result<Device<'a>, Box<dyn Error>> {
    match CONTEXT.as_ref() {
        Ok(context) => match open_device_sn(context, sn) {
            Some((device, device_desc, handle)) => {
                let (ep_out, ep_in) = find_bulk_endpoints(&device)?;
                let mut d = Device {
                    _dev: device,
                    _desc: device_desc,
                    handle: handle,
                    ep_in: ep_in,
                    ep_out: ep_out,
                };
                d.set_speed(I2CSpeed::Standard)?;
                Ok(d)
            }
            None => {
                let not_found = std::io::Error::from(std::io::ErrorKind::NotFound);
                error!("could not find device {:04x}:{:04x}", CH341_VID, CH341_PID);
                Err(Box::new(not_found))
            }
        },
        Err(err) => Err(Box::new(err)),
    }
}

impl Device<'_> {
    fn bulk_xfer(&mut self, out: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
        self.handle
            .write_bulk(self.ep_out, out.as_slice(), DEFAULT_USB_TIMEOUT)?;

        let mut array: [u8; 32] = [0; 32];
        match self
            .handle
            .read_bulk(self.ep_in, &mut array, DEFAULT_USB_TIMEOUT)
        {
            Ok(len) => Ok(array[0..len].to_vec()),
            Err(err) => Err(Box::new(err)),
        }
    }

    fn bulk_write(&mut self, out: Vec<u8>) -> Result<usize, Box<dyn Error>> {
        match self
            .handle
            .write_bulk(self.ep_out, out.as_slice(), DEFAULT_USB_TIMEOUT)
        {
            Ok(len) => Ok(len),
            Err(err) => Err(Box::new(err)),
        }
    }

    fn set_speed(&mut self, speed: I2CSpeed) -> Result<(), Box<dyn Error>> {
        let s = match speed {
            I2CSpeed::Low => CH341_I2C_LOW_SPEED,
            I2CSpeed::Standard => CH341_I2C_STANDARD_SPEED,
            I2CSpeed::Fast => CH341_I2C_FAST_SPEED,
            I2CSpeed::High => CH341_I2C_HIGH_SPEED,
        };

        let cmd = vec![
            CH341_CMD_I2C_STREAM,
            CH341_CMD_I2C_STM_SET | s,
            CH341_CMD_I2C_STM_END,
        ];

        self.bulk_write(cmd).map(|_| ())
    }

    fn check_dev(&mut self, addr: u8) -> Result<(), Box<dyn Error>> {
        let cmd = vec![
            CH341_CMD_I2C_STREAM,
            CH341_CMD_I2C_STM_STA,
            CH341_CMD_I2C_STM_OUT, /* NOTE: must be zero length otherwise it messes up the device */
            (addr << 1) | 0x1,
            CH341_CMD_I2C_STM_IN, /* NOTE: zero length here as well */
            CH341_CMD_I2C_STM_STO,
            CH341_CMD_I2C_STM_END,
        ];

        match self.bulk_xfer(cmd) {
            Ok(inp) => {
                let v = inp.get(0).ok_or("err")?;
                if v & 0x80 != 0 {
                    let timeout = std::io::Error::from(std::io::ErrorKind::TimedOut);
                    Err(Box::new(timeout))
                } else {
                    Ok(())
                }
            }
            Err(err) => Err(err),
        }
    }
}

impl<'a> embedded_hal::blocking::i2c::Read for Device<'a> {
    type Error = Box<dyn Error>;

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Box<dyn Error>> {
        self.check_dev(address)?;
        let mut cmd = vec![
            CH341_CMD_I2C_STREAM,
            CH341_CMD_I2C_STM_STA,
            CH341_CMD_I2C_STM_OUT | 1,
            (address << 1) | 0x1,
        ];

        if buffer.len() > 0 {
            for _ in 1..buffer.len() {
                cmd.push(CH341_CMD_I2C_STM_IN | 1);
            }
            cmd.push(CH341_CMD_I2C_STM_IN);
        }
        cmd.push(CH341_CMD_I2C_STM_STO);
        cmd.push(CH341_CMD_I2C_STM_END);

        let rcv = self.bulk_xfer(cmd)?;
        buffer.copy_from_slice(&rcv);
        Ok(())
    }
}

impl<'a> embedded_hal::blocking::i2c::Write for Device<'a> {
    type Error = Box<dyn Error>;

    fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
        self.check_dev(address)?;
        let mut cmd = vec![
            CH341_CMD_I2C_STREAM,
            CH341_CMD_I2C_STM_STA,
            CH341_CMD_I2C_STM_OUT | (1 + bytes.len() as u8),
            address << 1,
        ];

        cmd.extend(bytes.iter().cloned());

        cmd.push(CH341_CMD_I2C_STM_STO);
        cmd.push(CH341_CMD_I2C_STM_END);

        self.bulk_write(cmd)?;
        Ok(())
    }
}

impl<'a> embedded_hal::blocking::i2c::WriteRead for Device<'a> {
    type Error = Box<dyn Error>;

    fn write_read(
        &mut self,
        address: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Box<dyn Error>> {
        self.check_dev(address)?;

        // write data phase
        let mut cmd = vec![
            CH341_CMD_I2C_STREAM,
            CH341_CMD_I2C_STM_STA,
            CH341_CMD_I2C_STM_OUT | (1 + bytes.len() as u8),
            address << 1,
        ];

        cmd.extend(bytes.iter().cloned());
        self.bulk_write(cmd)?;

        // read data phase
        let mut cmd = vec![
            CH341_CMD_I2C_STREAM,
            CH341_CMD_I2C_STM_STA,
            CH341_CMD_I2C_STM_OUT | 1,
            (address << 1) | 0x1,
        ];

        if buffer.len() > 0 {
            for _ in 1..buffer.len() {
                cmd.push(CH341_CMD_I2C_STM_IN | 1);
            }
            cmd.push(CH341_CMD_I2C_STM_IN);
        }
        cmd.push(CH341_CMD_I2C_STM_STO);
        cmd.push(CH341_CMD_I2C_STM_END);

        let rcv = self.bulk_xfer(cmd)?;
        buffer.copy_from_slice(&rcv);

        Ok(())
    }
}

