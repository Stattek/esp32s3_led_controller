use std::slice::SliceIndex;

use smart_leds::RGB8;
use ws2812_esp32_rmt_driver::Ws2812Esp32RmtDriver;

pub trait RgbLedAnimation {
    fn next_frame(&mut self);
}

pub struct Rgb8RainbowAnimation {
    pixels: Vec<RGB8>,
    cur_color_increment: Vec<PixelColor>, // should be the same length as pixels
}

#[derive(Clone, Copy)]
enum PixelColor {
    Red,
    Blue,
    Green,
}

impl AsRef<Vec<RGB8>> for Rgb8RainbowAnimation {
    fn as_ref(&self) -> &Vec<RGB8> {
        &self.pixels
    }
}

impl Rgb8RainbowAnimation {
    /// Creates a new animation, with all lights off at the start
    pub fn new(num_pixels: usize) -> Self {
        // let's start off with all off
        Self {
            pixels: std::iter::repeat(RGB8::new(0, 0, 0))
                .take(num_pixels)
                .collect(),
            cur_color_increment: std::iter::repeat(PixelColor::Red)
                .take(num_pixels)
                .collect(),
        }
    }
}

fn pixel_increment(val: &mut u8) -> Result<(), ()> {
    let mut output = Ok(());
    let result = val.checked_add(1);
    match result {
        Some(ok_val) => {
            *val = ok_val;
        }
        None => {
            output = Err(());
        }
    }

    output
}

fn pixel_decrement(val: &mut u8) -> Result<(), ()> {
    let mut output = Ok(());
    let result = val.checked_sub(1);
    match result {
        Some(ok_val) => {
            *val = ok_val;
        }
        None => {
            output = Err(());
        }
    }

    output
}

impl RgbLedAnimation for Rgb8RainbowAnimation {
    fn next_frame(&mut self) {
        for i in 0..self.pixels.len() {
            let cur_increment = &mut self.cur_color_increment[i];
            match *cur_increment {
                PixelColor::Red => {
                    let inc_result = pixel_increment(&mut self.pixels[i].r);
                    if inc_result.is_err() {
                        // reached highest value, increment next color
                        self.cur_color_increment[i] = PixelColor::Green;
                    }

                    // subtract last color
                    let _ = pixel_decrement(&mut self.pixels[i].b);
                }
                PixelColor::Green => {
                    let inc_result = pixel_increment(&mut self.pixels[i].g);
                    if inc_result.is_err() {
                        // reached highest value, increment next color
                        self.cur_color_increment[i] = PixelColor::Blue;
                    }

                    // subtract last color
                    let _ = pixel_decrement(&mut self.pixels[i].r);
                }
                PixelColor::Blue => {
                    let inc_result = pixel_increment(&mut self.pixels[i].b);
                    if inc_result.is_err() {
                        // reached highest value, increment next color
                        self.cur_color_increment[i] = PixelColor::Red;
                    }

                    // subtract last color
                    let _ = pixel_decrement(&mut self.pixels[i].g);
                }
            }
        }
    }
}
