use crate::pin::{self, Pin};
use serial::{unix::TTYPort, BaudRate, SerialPort};
use std::{
    io::{Read, Write},
    time::Duration,
};

pub trait Transport {
    fn send(&mut self, data: Vec<u8>) -> Result<(), String>;
    fn receive(&mut self, count: usize) -> Result<Vec<u8>, String>;
}

struct DummyTransport;
impl Transport for DummyTransport {
    fn send(&mut self, _data: Vec<u8>) -> Result<(), String> {
        Ok(())
    }

    fn receive(&mut self, _count: usize) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }
}

struct RS485SenderImpl {
    serial: TTYPort,
    pin: Box<dyn Pin>,
}

#[allow(dead_code)]
impl RS485SenderImpl {
    pub fn new(path: &str, pin: u16, baudrate: usize, timeout: Duration) -> Result<Self, String> {
        match serial::open(path) {
            Ok(mut port) => {
                if let Err(error) = port.set_timeout(timeout) {
                    return Err(error.to_string());
                }

                if let Err(err) = port.reconfigure(&|config| {
                    Ok({
                        config.set_baud_rate(BaudRate::from_speed(baudrate))?;
                    })
                }) {
                    return Err(err.to_string());
                }

                let pin = match pin::new(pin) {
                    Ok(it) => it,
                    Err(error) => return Err(error.to_string()),
                };

                Ok(Self { serial: port, pin })
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

impl Transport for RS485SenderImpl {
    fn send(&mut self, data: Vec<u8>) -> Result<(), String> {
        if let Err(error) = self.pin.set_value(true) {
            return Err(error.to_string());
        }
        std::thread::sleep(Duration::from_millis(2));

        let result = match self.serial.write(data.as_slice()) {
            Ok(_) => Ok(()),
            Err(err) => return Err(err.to_string()),
        };

        std::thread::sleep(Duration::from_millis(4));

        if let Err(error) = self.pin.set_value(false) {
            return Err(error.to_string());
        }

        result
    }

    fn receive(&mut self, max_bytes_to_read: usize) -> Result<Vec<u8>, String> {
        let mut buf = Vec::with_capacity(max_bytes_to_read);
        unsafe { buf.set_len(max_bytes_to_read) };

        match self.serial.read(&mut buf[..]) {
            Ok(size) => {
                if size < max_bytes_to_read {
                    unsafe { buf.set_len(size) };
                }
                Ok(buf)
            }
            Err(error) => Err(error.to_string()),
        }
    }
}

#[allow(unused_variables)]
pub fn new(
    path: &str,
    pin: u16,
    baudrate: usize,
    timeout: std::time::Duration,
) -> Result<Box<dyn Transport>, String> {
    #[cfg(target_os = "linux")]
    match RS485SenderImpl::new(path, pin, baudrate, timeout) {
        Ok(result) => Ok(Box::new(result)),
        Err(error) => Err(error.to_string()),
    }

    #[cfg(target_os = "macos")]
    Ok(Box::new(DummyTransport {}))
}
