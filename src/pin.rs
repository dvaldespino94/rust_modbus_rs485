use gpio::{sysfs::SysFsGpioOutput, GpioOut};

// Output only GPIO Abstraction
pub trait Pin {
    fn set_value(&mut self, value: bool) -> Result<(), String>;
}

// Raspberry Pi GPIO implementation
struct PinRpiGPIO {
    // The underlying gpio pin to set
    gpio: SysFsGpioOutput,
}

impl PinRpiGPIO {
    fn new(pin: u16) -> Result<Box<dyn Pin>, String> {
        match gpio::sysfs::SysFsGpioOutput::open(pin) {
            Ok(it) => Ok(Box::new(Self { gpio: it })),
            Err(error) => Err(error.to_string()),
        }
    }
}

impl Pin for PinRpiGPIO {
    fn set_value(&mut self, value: bool) -> Result<(), String> {
        match self.gpio.set_value(value) {
            Ok(_) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

// Dummy pin implementation
struct DummyPin;
impl Pin for DummyPin {
    fn set_value(&mut self, _value: bool) -> Result<(), String> {
        Ok(())
    }
}

// Return default pin implementation
pub fn new(pin: u16) -> Result<Box<dyn Pin>, String> {
    // #[cfg(target_os = "linux")]
    match PinRpiGPIO::new(pin) {
        Ok(it) => Ok(it),
        Err(error) => Err(error.to_string()),
    }

    // #[cfg(target_os = "macos")]
    // Ok(Box::new(DummyPin {}))
}
