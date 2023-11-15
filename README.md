# Big Vim Button #2

Many years ago, I mounted a giant industrial stop button in a wooden box, hooked it up to an Arduino, and used it as a single-key keyboard. The idea was that you could then hit the "Escape" key with considerable gusto (it being a mainstay of your typical vim session).

This seems like a good candidate for learning some embedded rust, so I'm going to re-implement it wirelessly (Bluetooth Low Energy) using an ESP32.

## Hardware

Board is an [Adafruit Huzzah32 Feather V2](https://learn.adafruit.com/adafruit-esp32-feather-v2/overview), with 8MB flash and 2MB RAM, It's got LiPoly battery management onboard and an RGB LED.

We've got a red LED on GPIO13, an RGB LED (NeoPixel, WS2712B compatible) on GPIO0 (power from GPIO2).

We've got a button on GPIO38.


## Project setup 

Scaffolded the project using steps in [The Rust on ESP Book](https://esp-rs.github.io/book/).

Remember to source `~/.export-esp.sh`

Board set to ESP32 (the original, as that's what I had plugged in).

Generated project with `cargo generate` and the template `esp-idf-template` (i.e. we're going to use std).

## References

[ESP32 Embeeded Rust at the HAL: GPIO Button controlled blinking](https://apollolabsblog.hashnode.dev/esp32-embedded-rust-at-the-hal-gpio-button-controlled-blinking) has some useful background, although it targets the esp32c3 device and is **no_std** !!

[ESP32 Standard Library Embedded Rust: GPIO Control](https://apollolabsblog.hashnode.dev/esp32-standard-library-embedded-rust-gpio-control) would have been a much better starting point, in retrospect.

[esp-idf-hal NeoPixel example using RMT peripheral](https://github.com/esp-rs/esp-idf-hal/blob/master/examples/rmt_neopixel.rs)

## Things we have learned

### Peripherals
The whole `Peripherals` business seems to be device-dependent. `Peripherals::take()` seems fairly uniquitous, as does `Peripherals::steal()`, but some PACs have a `split()` method to get at the structures within the Peripherals object, whereas ESP32 favours a constructor-based approach (e.g. `IO::new`).

Oh wait, this `Peripherals` is from `pac::Peripherals` in the `esp32_hal` (i.e. device specific) crate. And we want to use the esp-idf-hal multi-platform crate. Which seems to have something different:

```
use esp_idf_hal::peripherals::Peripherals;

let peripherals = Peripherals::take().unwrap(); // as before
let mut led = peripherals.pins.gpio2.into_output()?;  // NB 'pins' not 'GPIO'

led.set_high()?;
led.set_low()?;
```

### no-std vs std, hal-esp32 vs esp-idf-hal

*BUT* GPIO access in esp-idf-hal has recently changed: [Completely revamped GPIO metaphor](https://github.com/esp-rs/esp-idf-hal/blob/master/CHANGELOG.md#completely-revamped-gpio-metaphor)

Now we convert a concrete pin like Gpio0 into a more generic pin `Any*Pin` using `downgrade_*`, e.g.:

```
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::gpio::OutputPin;  // marker trait
use esp_idf_hal::gpio::PinDriver; // pin driver sets pin modes

let peripherals = Peripherals::take().unwrap();

let mut led = PinDriver::output(peripherals.pins.gpio2.downgrade_output())?;

led.set_high()?;
led.setlow()?;
```

### Individual pins are represented by TYPES

For example, GPIO0 is represented by Gpio0 which is a *type*, not an instance. This means that you can't put them in arrays. But handily, you can use those `downgrade_*` methods mentioned above to get an `Any*Pin` type that you *can* put in an array.



