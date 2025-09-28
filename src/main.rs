mod elevator;

use anyhow::Result;
use esp_idf_svc::hal::{
    gpio::{Input, PinDriver, Pull},
    prelude::Peripherals,
};
use smart_leds::{SmartLedsWrite, RGB8};
use std::time::Duration;
use ws2812_esp32_rmt_driver::{
    driver::color::{LedPixelColor, LedPixelColorGrb24, LedPixelColorImpl},
    LedPixelEsp32Rmt, Ws2812Esp32Rmt, Ws2812Esp32RmtDriver,
};

use esp32_led_animation::{
    led_animation::{
        basic_pixel_sequence_animation::Rgb8BasicPixelSequenceAnimation,
        basic_pixel_sequences::{FOURTH_OF_JULY_SEQUENCE, OFF_WHITE_SEQUENCE},
        elevator_number_animation::Rgb8SingleLedFadeAnimation,
    },
    Direction, RgbLedAnimation,
};

/// 8-bit RGB LED pixel color (total 32-bit pixel), Typical RGB LED (WS2811) pixel color
/// NOTE: this should be implemented in the library already.
pub type LedPixelColorRgb24 = LedPixelColorImpl<3, 0, 1, 2, 255>;

/// 8-bit RGB (total 24-bit pixel) LED driver wrapper providing smart-leds API,
/// Typical RGB LED (WS2811) driver wrapper providing smart-leds API
///
pub type Ws2811Esp32Rmt<'d> = LedPixelEsp32Rmt<'d, RGB8, LedPixelColorRgb24>;

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    // number of pixels on LED light strip
    const NUM_PIXELS: usize = 50;

    // driver for communicating with the onboard WS2812 LED
    let mut onboard_led_driver =
        Ws2812Esp32RmtDriver::new(peripherals.rmt.channel0, peripherals.pins.gpio48)?;
    // driver for our led strip
    let mut strip_led_driver =
        Ws2811Esp32Rmt::new(peripherals.rmt.channel1, peripherals.pins.gpio46)?;
    let mut strip2_led_driver =
        Ws2811Esp32Rmt::new(peripherals.rmt.channel2, peripherals.pins.gpio8)?;

    let mut up_button = PinDriver::input(peripherals.pins.gpio37)?;
    up_button.set_pull(Pull::Up)?;
    // start all pixels as yellow at first

    set_led_yellow(&mut onboard_led_driver)?;
    std::thread::sleep(Duration::from_secs(1));
    let pixels = std::iter::repeat(RGB8::new(255, 255, 0)).take(NUM_PIXELS);
    strip_led_driver.write(pixels)?;

    set_led_green(&mut onboard_led_driver)?;
    std::thread::sleep(Duration::from_millis(400));

    let mut floor_number_animation =
        Rgb8SingleLedFadeAnimation::new(NUM_PIXELS, RGB8::new(255, 255, 255), 30);

    let mut pixel_animation = Rgb8BasicPixelSequenceAnimation::new(
        NUM_PIXELS,
        OFF_WHITE_SEQUENCE.to_vec(),
        Direction::Forward,
    );

    let mut cur_floor_idx: usize = 0;

    let mut button_last_state;
    let mut button_current_state = false;

    set_led_blue(&mut onboard_led_driver)?;
    // Prevent program from exiting
    loop {
        // check the button state
        button_last_state = button_current_state;
        if up_button.is_low() {
            button_current_state = true;
        } else {
            button_current_state = false;
            set_led_blue(&mut onboard_led_driver)?;
        }

        // perform action based on button state
        if button_current_state {
            set_led_green(&mut onboard_led_driver)?;
        } else {
            set_led_blue(&mut onboard_led_driver)?;
        }

        // DEBUG: if the button was just pressed, increase the floor index
        if button_current_state && !button_last_state {
            // since we have only so many pixels to represent the floors, we want to make sure we
            // don't go out of bounds when we increase the floor index.

            log::warn!("Floor index changed to {cur_floor_idx}");
            // now we can set the floor LED since we just changed the data we are sending
            floor_number_animation
                .set_led_on(cur_floor_idx)
                .expect("Could not set floor number LED");

            // increment this index after we set the floor number, so we start at 0
            cur_floor_idx = (cur_floor_idx + 1) % NUM_PIXELS;
            log::error!("Floor animation colors = {:?}", floor_number_animation);
        }

        floor_number_animation.next_frame();
        let pixels = floor_number_animation.as_ref().clone().into_iter();

        #[cfg(false)]
        {
            pixel_animation.next_frame();
            let pixels = pixel_animation.as_ref().clone().into_iter();
        }

        strip2_led_driver.write(pixels.clone()).unwrap();
        strip_led_driver.write(pixels).unwrap();
        std::thread::sleep(Duration::from_millis(100));
    }
}

// TODO: the functions below are kinda dumb

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
