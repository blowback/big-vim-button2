use esp_idf_sys as _;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::config::TransmitConfig;
use esp_idf_hal::rmt::*;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use anyhow::Result;

mod neopixel;
use crate::neopixel::*;

static FLAG: AtomicBool = AtomicBool::new(false);

// ISR, runs in interrupt context. do not call std, libc, or FreeRTOS functions!
fn gpio_int_callback() {
    FLAG.store(true, Ordering::Relaxed);
}

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    // take the device peripherals
    let peripherals = Peripherals::take()?;

    // obtain a handle to the LED pin and configure it as an output
    // MB *mutable*
    let mut led = PinDriver::output(peripherals.pins.gpio13)?;

    let mut neopixel_power = PinDriver::output(peripherals.pins.gpio2)?;
    let neopixel_data = peripherals.pins.gpio0;

    let channel = peripherals.rmt.channel0;
    let config = TransmitConfig::new().clock_divider(1);
    let mut tx = TxRmtDriver::new(channel, neopixel_data, &config)?;

    // turn on the NeoPixel
    neopixel_power.set_high()?;

    // 3 seconds white at 10% brightness
    neopixel(Rgb::new(25, 25, 25), &mut tx)?;

    // obtain a handle to the button pin and configure it as an input
    // MB now needs to be mut, so we can set interrupty stuff
    let mut button = PinDriver::input(peripherals.pins.gpio38)?;
    // it won't let me do this, complains "the trait bound Gpio38::OutputPin is not satisfied"
    // this is probably because GPIOs 34-39 are *input only* on the ESP32 and don't have
    // pullups/pulldowns
    //button.set_pull(Pull:Up).unwrap(); // do we need internal pull-up? Yes!
    button.set_interrupt_type(InterruptType::PosEdge)?;

    unsafe {
        // this automatically disables the interrupt...
        button.subscribe(gpio_int_callback)?;
    }

    // ...so we enable it here
    button.enable_interrupt()?;

    let _ = (00..360).cycle().try_for_each(|hue| {
        if FLAG.load(Ordering::Relaxed) {
            FLAG.store(false, Ordering::Relaxed);
            println!("Button pressed!");
            button.enable_interrupt()?;
        }

        if hue % 60 == 0 {
            led.toggle()?;
        }

        FreeRtos::delay_ms(20);
        let rgb = Rgb::from_hsv(hue, 100, 20)?; // 20% brightness
        neopixel(rgb, &mut tx)
    });

    Ok(())
}
