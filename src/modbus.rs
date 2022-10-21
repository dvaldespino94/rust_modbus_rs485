use rmodbus::{client::ModbusRequest, guess_response_frame_len};

use crate::transport::Transport;

// Modbus abstraction layer
pub trait Modbus {
    fn request_register(&mut self, addr: u8, register: u16, count: u16)
        -> Result<Vec<u16>, String>;
}

// Modbus over RS485 implementation
struct ModbusRtu {
    transport: Box<dyn Transport>,
}

impl Modbus for ModbusRtu {
    // Query registers
    fn request_register(
        &mut self,
        addr: u8,
        register: u16,
        count: u16,
    ) -> Result<Vec<u16>, String> {
        // Create the request
        let mut request = ModbusRequest::new(addr, rmodbus::ModbusProto::Rtu);

        // Buffer to hold the request
        let mut request_buffer = Vec::new();

        // Generate the register request into the request buffer
        if let Err(error) = request.generate_get_holdings(register, count, &mut request_buffer) {
            return Err(error.to_string());
        }

        // Send the request buffer
        if let Err(error) = self.transport.send(request_buffer) {
            return Err(error.to_string());
        }

        // Minimal length to be able to guess the package length
        const MESSAGE_HEADER_LENGTH: u8 = 6;

        // Receive the response
        match self.transport.receive(MESSAGE_HEADER_LENGTH as usize) {
            Ok(mut data) => {
                // Return error if not enough data was received
                // TODO: Maybe implement this with a buffer inside a loop
                if data.len() < MESSAGE_HEADER_LENGTH as usize {
                    return Err("Not enough data received".to_string());
                }

                // Guess how long the whole message is based on the first bytes
                let guessed_length =
                    match guess_response_frame_len(&data, rmodbus::ModbusProto::Rtu) {
                        Ok(it) => it,
                        Err(error) => return Err(error.to_string()),
                    };

                // If there was some more data to be received append it to the buffer
                if guessed_length > MESSAGE_HEADER_LENGTH {
                    match self
                        .transport
                        .receive((guessed_length - MESSAGE_HEADER_LENGTH) as usize)
                    {
                        Ok(more_data) => data.extend(more_data),
                        Err(error) => return Err(error.to_string()),
                    };
                }

                // Parse the modbus response
                match request.parse_ok(&data) {
                    Ok(it) => it,
                    Err(error) => return Err(error.to_string()),
                };

                // Get the values from the response
                let mut values: Vec<u16> = Vec::new();
                match request.parse_u16(&data, &mut values) {
                    Ok(_) => return Ok(values),
                    Err(error) => return Err(error.to_string()),
                }
            }
            Err(error) => Err(error),
        }
    }
}

impl ModbusRtu {
    fn new(transport: Box<dyn Transport>) -> Self {
        Self { transport }
    }
}

pub fn create(port: Box<dyn Transport>) -> Result<Box<dyn Modbus>, String> {
    Ok(Box::new(ModbusRtu::new(port)))
}
