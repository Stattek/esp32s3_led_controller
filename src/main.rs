mod elevator;
mod ws2811;

use anyhow::Result;
use esp_idf_svc::hal::{
    gpio::{Input, PinDriver, Pull},
    prelude::Peripherals,
};
use log::LevelFilter;
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
        elevator_animation::{self, Rgb8ElevatorAnimation},
        single_led_fade_animation::Rgb8SingleLedFadeAnimation,
    },
    Direction, RgbLedAnimation,
};

use crate::{
    elevator::elevator_handler::ElevatorHandler, ws2811::ws2811_rmt_types::Ws2811Esp32Rmt,
};

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    // number of pixels on LED light strip
    const NUM_FLOOR_BUTTON_PIXELS: usize = 14;
    const NUM_ELEVATOR_PIXELS: usize = 10;
    const YELLOW_PIXEL: RGB8 = RGB8::new(255, 255, 0);

    // driver for communicating with the onboard WS2812 LED
    let mut onboard_led_driver =
        Ws2812Esp32RmtDriver::new(peripherals.rmt.channel0, peripherals.pins.gpio48)?;
    // drivers for our led strips
    let mut elevator_led_driver =
        Ws2811Esp32Rmt::new(peripherals.rmt.channel1, peripherals.pins.gpio16)?;
    let mut floor_number_led_driver =
        Ws2811Esp32Rmt::new(peripherals.rmt.channel2, peripherals.pins.gpio8)?;

    // NOTE: just a test of the LEDs
    set_led_yellow(&mut onboard_led_driver)?;
    std::thread::sleep(Duration::from_secs(1));
    let yellow_elevator_pixels = std::iter::repeat_n(YELLOW_PIXEL, NUM_ELEVATOR_PIXELS);
    elevator_led_driver.write(yellow_elevator_pixels)?;
    let yellow_floor_pixels = std::iter::repeat_n(YELLOW_PIXEL, NUM_FLOOR_BUTTON_PIXELS);
    floor_number_led_driver.write(yellow_floor_pixels)?;
    std::thread::sleep(Duration::from_secs(1));

    // create our elevator
    let mut elevator = ElevatorHandler::new(
        1,
        floor_number_led_driver,
        NUM_FLOOR_BUTTON_PIXELS,
        elevator_led_driver,
        NUM_ELEVATOR_PIXELS,
    )
    .expect("Could not create elevator object");

    // button for going up
    let mut up_button = PinDriver::input(peripherals.pins.gpio37)?;
    up_button.set_pull(Pull::Up)?;

    set_led_green(&mut onboard_led_driver)?;
    std::thread::sleep(Duration::from_millis(400));

    // TODO: implement going to the first floor when buttons are pressed
    let mut button_last_state;
    let mut button_current_state = false;

    // READY
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

        elevator.next_frame()?;
        std::thread::sleep(Duration::from_millis(100));
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
