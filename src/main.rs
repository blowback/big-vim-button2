use esp_idf_sys as _;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    // take the device peripherals
    let peripherals = Peripherals::take().unwrap();

    // obtain a handle to the LED pin and configure it as an output
    // MB *mutable*
    let mut led = PinDriver::output(peripherals.pins.gpio13).unwrap();

    // obtain a handle to the button pin and configure it as an input
    let button = PinDriver::input(peripherals.pins.gpio38).unwrap();
    //button.set_pull(Pull:Up).unwrap(); // do we need internal pull-up?

    let mut delay = 500_u32;

    loop {
        led.set_high().unwrap();
        FreeRtos::delay_ms(delay);
        led.set_low().unwrap();
        FreeRtos::delay_ms(delay);
    }
}
