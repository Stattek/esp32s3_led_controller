use std::time::Duration;

use anyhow::Result;
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::prelude::Peripherals};
use smart_leds::{SmartLedsWrite, White, RGB8};
use ws2812_esp32_rmt_driver::{
    driver::color::{LedPixelColor, LedPixelColorGrb24, LedPixelColorGrbw32},
    LedPixelEsp32Rmt, Ws2812Esp32Rmt, Ws2812Esp32RmtDriver, RGBW8,
};

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    // driver for communicating with the onboard WS2812 LED
    let mut onboard_led_driver =
        Ws2812Esp32RmtDriver::new(peripherals.rmt.channel0, peripherals.pins.gpio48).unwrap();
    let mut strip_led_driver =
        Ws2812Esp32Rmt::new(peripherals.rmt.channel1, peripherals.pins.gpio45).unwrap();

    let pixels = std::iter::repeat(RGB8::new(30, 0, 0)).take(25);
    strip_led_driver.write(pixels).unwrap();

    set_led_yellow(&mut onboard_led_driver)?;
    std::thread::sleep(Duration::from_secs(1));

    // This is not true until you actually create one
    log::info!("Server awaiting connection");
    set_led_green(&mut onboard_led_driver)?;

    // Prevent program from exiting
    loop {
        std::thread::sleep(Duration::from_millis(400));
        set_led_blue(&mut onboard_led_driver)?;

        let pixels = std::iter::repeat(RGBW8::new_alpha(6, 0, 0, White(0))).take(25);
        strip_led_driver.write_nocopy(pixels).unwrap();
        std::thread::sleep(Duration::from_millis(1000));
    }
}

/// Sets the onboard ESP32-S3 WS2812 LED to green.
fn set_led_green(led_driver: &mut Ws2812Esp32RmtDriver) -> anyhow::Result<()> {
    let green = LedPixelColorGrb24::new_with_rgb(0, 30, 0);
    let green_pixel: [u8; 3] = green.as_ref().try_into().unwrap();

    led_driver.write_blocking(green_pixel.into_iter())?;

    Ok(())
}

/// Sets the onboard ESP32-S3 WS2812 LED to yellow.
fn set_led_yellow(led_driver: &mut Ws2812Esp32RmtDriver) -> anyhow::Result<()> {
    let yellow = LedPixelColorGrb24::new_with_rgb(30, 30, 0);
    let yellow_pixel: [u8; 3] = yellow.as_ref().try_into().unwrap();

    led_driver.write_blocking(yellow_pixel.into_iter())?;

    Ok(())
}

/// Sets the onboard ESP32-S3 WS2812 LED to blue.
fn set_led_blue(led_driver: &mut Ws2812Esp32RmtDriver) -> anyhow::Result<()> {
    let blue = LedPixelColorGrb24::new_with_rgb(0, 0, 30);
    let blue_pixel: [u8; 3] = blue.as_ref().try_into().unwrap();

    led_driver.write_blocking(blue_pixel.into_iter())?;

    Ok(())
}
