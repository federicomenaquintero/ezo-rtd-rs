use errors::*;
use i2cdev::core::I2CDevice;
use i2cdev::linux::LinuxI2CDevice;
use std::thread;
use std::time::Duration;

/// Maximum ascii-character response size + 2
pub const MAX_RESPONSE_LENGTH: usize = 16;

/// Allowable baudrates used when changing the chip to UART mode.
#[derive(Debug)]
pub enum BpsRate {
    Bps300 = 300,
    Bps1200 = 1200,
    Bps2400 = 2400,
    Bps9600 = 9600,
    Bps19200 = 19200,
    Bps38400 = 38400,
    Bps57600 = 57600,
    Bps115200 = 115200,
}

/// Known response codes from EZO chip interactions.
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum ResponseCode {
    NoDataExpected = 0xFF,
    Pending = 0xFE,
    DeviceError = 0x02,
    Success = 0x01,
    UnknownError = 0x00, // This code is NOT implemented by the EZO chips
}

/// Command-related parameters used to build I2C write/read interactions.
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct CommandOptions {
    pub command: String,
    pub delay: Option<u64>,
    pub response: Option<CommandResponse>,
}

/// Allowed responses from I2C read interactions.
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum CommandResponse {
    Ack,
    CalibrationState,
    DataloggerInterval,
    DeviceInformation,
    ExportInfo,
    Export,
    LedState,
    MemoryRecall,
    MemoryRecallLastLocation,
    ProtocolLockState,
    Reading,
    ScaleState,
    Status,
}

/// Builds commands.
pub trait CommandBuilder {
    fn finish(&self) -> Self;
    fn run(&self, dev: &mut LinuxI2CDevice) -> Result<String>;
    fn set_command(&mut self, command_str: String) -> &mut Self;
    fn set_delay(&mut self, delay: u64) -> &mut Self;
    fn set_response(&mut self, response: CommandResponse) -> &mut Self;
}

impl CommandBuilder for CommandOptions {
    fn finish(&self) -> CommandOptions {
        self.clone()
    }
    fn run(&self, dev: &mut LinuxI2CDevice) -> Result<String> {
        if let Err(_) = dev.write(self.command.as_bytes()) {
            thread::sleep(Duration::from_millis(300));
            dev.write(self.command.as_bytes())
                .chain_err(|| "Command could not be sent")?;
        };
        if let Some(delay) = self.delay {
            thread::sleep(Duration::from_millis(delay));
        }
        if let Some(_) = self.response {
            let mut data_buffer = [0u8; MAX_RESPONSE_LENGTH];
            if let Err(_) = dev.read(&mut data_buffer) {
                thread::sleep(Duration::from_millis(300));
                dev.read(&mut data_buffer)
                    .chain_err(|| "Error reading from device")?;
            };
            match response_code(data_buffer[0]) {
                ResponseCode::NoDataExpected => println!("No data expected."),
                ResponseCode::Pending => println!("Pending"),
                ResponseCode::DeviceError => println!("Error"),
                ResponseCode::Success => {
                    return Ok(String::from_utf8(parse_data_ascii_bytes(&data_buffer[1..]))
                        .chain_err(|| "Data is not parsable")?)
                },
                ResponseCode::UnknownError => println!("NO RESPONSE"),
            };
        }
        Ok(String::new())
    }

    /// Sets the ASCII string for the command to be sent
    fn set_command(&mut self, command_str: String) -> &mut CommandOptions {
        self.command = command_str;
        self
    }
    fn set_delay(&mut self, delay: u64) -> &mut CommandOptions {
        self.delay = Some(delay);
        self
    }
    fn set_response(&mut self, response: CommandResponse) -> &mut CommandOptions {
        self.response = Some(response);
        self
    }
}

/// Useful for properly building I2C parameters from a command.
pub trait I2cCommand {
    fn build(&self) -> CommandOptions;
}

/// Crude parser for the data string sent by the EZO chip.
pub fn parse_data_ascii_bytes(data_buffer: &[u8]) -> Vec<u8> {
    match data_buffer.iter().position(|&x| x == 0) {
        Some(len) => read_hardware_buffer(&data_buffer[..len], true),
        _ => read_hardware_buffer(&data_buffer[..], true),
    }
}

/// Read buffer from the hardware. Accepts a `flipping` flag for glitchy hardware.
pub fn read_hardware_buffer(buf: &[u8], flipping: bool) -> Vec<u8> {
    if flipping {
        buf.iter().map(|buf| (*buf & !0x80)).collect()
    } else {
        Vec::from(&buf[..])
    }
}

/// Determines the response code sent by the EZO chip.
pub fn response_code(code_byte: u8) -> ResponseCode {
    use self::ResponseCode::*;
    match code_byte {
        x if x == NoDataExpected as u8 => NoDataExpected,
        x if x == Pending as u8 => Pending,
        x if x == DeviceError as u8 => DeviceError,
        x if x == Success as u8 => Success,
        _ => UnknownError,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_no_data_response_code() {
        assert_eq!(response_code(255), ResponseCode::NoDataExpected);
    }

    #[test]
    fn process_pending_response_code() {
        assert_eq!(response_code(254), ResponseCode::Pending);
    }

    #[test]
    fn process_error_response_code() {
        assert_eq!(response_code(2), ResponseCode::DeviceError);
    }

    #[test]
    fn process_success_response_code() {
        assert_eq!(response_code(1), ResponseCode::Success);
    }

    #[test]
    fn process_unknown_response_code() {
        assert_eq!(response_code(0), ResponseCode::UnknownError);
        assert_eq!(response_code(16), ResponseCode::UnknownError);
        assert_eq!(response_code(156), ResponseCode::UnknownError);
        assert_eq!(response_code(256), ResponseCode::UnknownError);
    }

    #[test]
    fn parsing_nonzeros_response() {
        let data: [u8; 0] = [];
        let parsed = parse_data_ascii_bytes(&data);
        assert_eq!(parsed.len(), 0);
        let data: [u8; 6] = [0, 98, 99, 65, 66, 67];
        let parsed = parse_data_ascii_bytes(&data);
        assert_eq!(parsed.len(), 0);
        let data: [u8; 6] = [97, 98, 0, 65, 66, 67];
        let parsed = parse_data_ascii_bytes(&data);
        assert_eq!(parsed.len(), 2);
        let data: [u8; 6] = [97, 98, 99, 65, 66, 67];
        let parsed = parse_data_ascii_bytes(&data);
        assert_eq!(parsed.len(), 6);
    }

    #[test]
    fn parsing_abc_response() {
        let data: [u8; 6] = [97, 98, 99, 65, 66, 67];
        let parsed = String::from_utf8(parse_data_ascii_bytes(&data)).unwrap();
        assert_eq!(&parsed, "abcABC");
    }

    #[test]
    fn parsing_empty_response() {
        let data: [u8; 3] = [0, 0, 0];
        let parsed = String::from_utf8(parse_data_ascii_bytes(&data)).unwrap();
        assert_eq!(&parsed, "");
    }

    #[test]
    fn parsing_non_flipped_data_response() {
        let data: [u8; 11] = [63, 73, 44, 112, 72, 44, 49, 46, 57, 56, 0];
        let parsed = String::from_utf8(parse_data_ascii_bytes(&data)).unwrap();
        assert_eq!(&parsed, "?I,pH,1.98");
    }

    #[test]
    fn parsing_flipped_data_response() {
        let data: [u8; 11] = [63, 73, 172, 112, 200, 172, 49, 46, 57, 56, 0];
        let parsed = String::from_utf8(parse_data_ascii_bytes(&data)).unwrap();
        assert_eq!(&parsed, "?I,pH,1.98");
    }
}
