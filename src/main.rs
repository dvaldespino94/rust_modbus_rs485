use std::{
    io::{Read, Write},
    time::Duration,
};

// #[macro_use]
// extern crate nickel;

// use nickel::Nickel;
// use rustorm::{
//     DbError,
//     FromDao,
//     Pool,
//     TableName,
//     ToColumnNames,
//     ToDao,
//     ToTableName,
// };

// mod for_insert {
//     #[derive(Debug, PartialEq, ToDao, ToColumnNames, ToTableName)]
//     pub struct Sensor {
//         pub reg: u16,
//         pub value: u16,
//     }
// }

// mod for_retrieve {
//     #[derive(Debug, FromDao, ToColumnNames, ToTableName)]
//     pub struct Sensor {
//         pub id: i32,
//         pub reg: u16,
//         pub value: u16,
//     }
// }

// fn main() {
//     let mut server = Nickel::new();
//     let pool = Pool::new();
//     pool.em("sqlite://test.sqlite").unwrap();
//     let s0=for_insrt::Sensor{

//     };

//     server.utilize(router! {
//         get "**" => |_req, _res| {
//             "Hello world!"
//         }
//     });

//     server.listen("127.0.0.1:6767").unwrap();
// }

use gpio::GpioOut;
use rmodbus::client::ModbusRequest;
use serial::{unix::TTYPort, BaudRate, SerialPort};

trait RS485Sender {
    fn send(&mut self, data: Vec<u8>) -> Result<(), String>;
    fn receive(&mut self, count: usize) -> Result<Vec<u8>, String>;
}

struct RS485SenderImpl {
    serial: TTYPort,
    pin: gpio::sysfs::SysFsGpioOutput,
}

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

                let pin = match gpio::sysfs::SysFsGpioOutput::open(pin) {
                    Ok(it) => it,
                    Err(error) => return Err(error.to_string()),
                };

                Ok(Self { serial: port, pin })
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

impl RS485Sender for RS485SenderImpl {
    fn send(&mut self, data: Vec<u8>) -> Result<(), String> {
        if let Err(error) = self.pin.set_high() {
            return Err(error.to_string());
        }
        std::thread::sleep(Duration::from_millis(2));

        let result = match self.serial.write(data.as_slice()) {
            Ok(_) => Ok(()),
            Err(err) => return Err(err.to_string()),
        };

	std::thread::sleep(Duration::from_millis(10));

        if let Err(error) = self.pin.set_low() {
            return Err(error.to_string());
        }
	

        result
    }

    fn receive(&mut self, count: usize) -> Result<Vec<u8>, String> {
        let mut buf = [0; 256];
        match self.serial.read(&mut buf[..]){
            Ok(_) => Ok(buf),
            Err(error) => Err(error.to_string()),
        }
    }
}

fn main() {
    match RS485SenderImpl::new("/dev/ttyS1", 6, 19200, Duration::from_secs(1)) {
        Ok(mut port) => {
            let mut request = ModbusRequest::new(1, rmodbus::ModbusProto::Rtu);
            let mut request_buffer = Vec::new();
            request
                .generate_get_holdings(1, 1, &mut request_buffer)
                .unwrap();
//		port.send("This is a test".as_bytes().to_vec()).unwrap();
            port.send(request_buffer).expect("Error sending data");
            println!("Got: {:?}", port.receive(255));
        }
        Err(error) => {
            println!("Error creating port: {error:?}");
        }
    }
}
