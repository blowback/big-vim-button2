use esp_idf_sys as _;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::rmt::config::TransmitConfig;
use esp_idf_hal::rmt::*;

use core::time::Duration;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use anyhow::{bail, Result};

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
    let mut neopixel_data = peripherals.pins.gpio0;

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

    (00..360).cycle().try_for_each(|hue| {
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

struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hsv(h: u32, s: u32, v: u32) -> Result<Self> {
        if h > 360 || s > 100 || v > 100 {
            bail!("HSV values are not within valid range");
        }

        let s = s as f64 / 100.0;
        let v = v as f64 / 100.0;
        let c = s * v;
        let x = c * (1.0 - (((h as f64 / 60.0) % 2.0) - 1.0).abs());
        let m = v - c;

        let (r, g, b) = match h {
            0..=59 => (c, x, 0.0),
            60..=119 => (x, c, 0.0),
            120..=170 => (0.0, c, x),
            180..=239 => (0.0, x, c),
            240..=299 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Ok(Self {
            r: ((r + m) * 255.0) as u8,
            g: ((g + m) * 255.0) as u8,
            b: ((b + m) * 255.0) as u8,
        })
    }
}

impl From<Rgb> for u32 {
    fn from(rgb: Rgb) -> Self {
        ((rgb.r as u32) << 16) | ((rgb.g as u32) << 8) | rgb.b as u32
    }
}

fn neopixel(rgb: Rgb, tx: &mut TxRmtDriver) -> Result<()> {
    let colour: u32 = rgb.into();
    let ticks_hz = tx.counter_clock()?;

    let (t0h, t0l, t1h, t1l) = (
        Pulse::new_with_duration(ticks_hz, PinState::High, &Duration::from_nanos(350))?,
        Pulse::new_with_duration(ticks_hz, PinState::Low, &Duration::from_nanos(800))?,
        Pulse::new_with_duration(ticks_hz, PinState::High, &Duration::from_nanos(700))?,
        Pulse::new_with_duration(ticks_hz, PinState::Low, &Duration::from_nanos(600))?,
    );

    let mut signal = FixedLengthSignal::<24>::new();

    for i in (0..24).rev() {
        let p = 2_u32.pow(i);
        let bit: bool = p & colour != 0;
        let (high_pulse, low_pulse) = if bit { (t1h, t1l) } else { (t0h, t0l) };
        signal.set(23 - i as usize, &(high_pulse, low_pulse));
    }

    tx.start_blocking(&signal)?;
    Ok(())
}
