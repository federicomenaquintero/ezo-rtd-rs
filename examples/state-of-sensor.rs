#![recursion_limit = "1024"]
//! An example that retrieves the current settings of the RTD EZO chip.
//!
extern crate ezo_rtd;
extern crate i2cdev;

use ezo_rtd::errors::*;
use ezo_rtd::{CommandBuilder, I2cCommand, TemperatureCommand};
use i2cdev::linux::LinuxI2CDevice;

const I2C_BUS_ID: u8 = 1;
const EZO_SENSOR_ADDR: u16 = 101; // could be specified as 0x65

fn run() -> Result<()> {
    let device_path = format!("/dev/i2c-{}", I2C_BUS_ID);
    let mut dev = LinuxI2CDevice::new(&device_path, EZO_SENSOR_ADDR)
        .chain_err(|| "Could not open I2C device")?;
    let mut response = String::new();
    let commands = [TemperatureCommand::Status,
                    TemperatureCommand::CalibrationState,
                    TemperatureCommand::DataloggerInterval,
                    TemperatureCommand::LedState,
                    TemperatureCommand::ExportInfo,
                    TemperatureCommand::Export,
                    TemperatureCommand::Export,
                    TemperatureCommand::Export,
                    TemperatureCommand::Sleep];
    for cmd in commands.iter() {
        let mut builder = cmd.build();
        builder.run(&mut dev)?;
        response += &builder.parse_response()?;
        response += &"\n";
    }
    println!("responses:");
    println!("{}", response);
    Ok(())
}

fn main() {
    if let Err(ref e) = run() {
        println!("error: {}", e);

        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }
        ::std::process::exit(1);
    }
}
