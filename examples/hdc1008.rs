use ch341_i2c;
use ch341_i2c::embedded_hal as hal;
use std::error::Error;

const HDC1000_I2CADDR: u8 = 0x40;

const HDC1000_REG_MANUFID: u8 = 0xFE;   // Manufacturer ID register. ID of Texas Instruments
const HDC1000_REG_DEVICEID: u8 = 0xFF;  // Device ID register

/// Read a 16bit register from HDC1008 humidity and temperature sensor.
/// This is done by writing the register address `addr` followed by
/// a two byte read response, which is the register value
fn read16<D>(i2c_dev: &mut D, addr: u8) -> Result<u16, Box<dyn Error>>
where
    D: hal::blocking::i2c::Write<Error = Box<dyn Error>>
        + hal::blocking::i2c::Read<Error = Box<dyn Error>>,
{
    let w = [addr];
    let mut r: [u8; 2] = [0; 2];

    i2c_dev.write(HDC1000_I2CADDR, &w)?;

    i2c_dev.read(HDC1000_I2CADDR, &mut r)?;

    Ok(u16::from_be_bytes(r))
}

fn main() {
    // Create a new ch341 i2c host adapter device
    let mut i2c = ch341_i2c::new().unwrap();

    // Read HDC1008 i2c device manufacturer id register
    let v = read16(&mut i2c, HDC1000_REG_MANUFID).expect("read failed");
    println!("manuf id {:04x} expected {:04x}", v, 0x5449);

    // Read HDC1008 i2c device id register
    let v = read16(&mut i2c, HDC1000_REG_DEVICEID).expect("read failed");
    println!("device id {:04x} expected {:04x}", v, 0x1000);
}
