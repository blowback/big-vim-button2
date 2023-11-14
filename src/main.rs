use esp_idf_sys as _;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

static FLAG: AtomicBool = AtomicBool::new(false);

// ISR, runs in interrupt context. do not call std, libc, or FreeRTOS functions!
fn gpio_int_callback() {
    FLAG.store(true, Ordering::Relaxed);
}

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
    // MB now needs to be mut, so we can set interrupty stuff
    let mut button = PinDriver::input(peripherals.pins.gpio38).unwrap();
    // it won't let me do this, complains "the trait bound Gpio38::OutputPin is not satisfied"
    // this is probably because GPIOs 34-39 are *input only* on the ESP32 and don't have
    // pullups/pulldowns
    //button.set_pull(Pull:Up).unwrap(); // do we need internal pull-up? Yes!
    button.set_interrupt_type(InterruptType::PosEdge).unwrap();

    unsafe {
        // this automatically disables the interrupt...
        button.subscribe(gpio_int_callback).unwrap();
    }

    // ...so we enable it here
    button.enable_interrupt().unwrap();

    let delay = 500_u32;

    loop {
        led.set_high().unwrap();
        FreeRtos::delay_ms(delay);
        led.set_low().unwrap();
        FreeRtos::delay_ms(delay);

        if FLAG.load(Ordering::Relaxed) {
            FLAG.store(false, Ordering::Relaxed);
            println!("Button pressed!");
            button.enable_interrupt().unwrap();
        }
    }
}
