use crate::led_animation::ws2812_led_animation::{PixelColor, RgbLedAnimation};
use smart_leds::RGB8;

/// Direction the snake is moving in.
#[derive(Clone, Copy)]
enum Direction {
    Forward,
    Backward,
}

/// Struct to handle performing a simple rainbow animation on many LEDs.
pub struct Rgb8RainbowSnakeAnimation {
    main_pixel_color: RGB8,
    cur_color_increment: PixelColor, // should be the same length as pixels
    pixels: Vec<RGB8>,
    head_location: usize,
    snake_length: usize,
    snake_direction: Direction, // direction snake is moving in
    color_step_amount: u8,
}

impl AsRef<Vec<RGB8>> for Rgb8RainbowSnakeAnimation {
    fn as_ref(&self) -> &Vec<RGB8> {
        &self.pixels
    }
}

impl Rgb8RainbowSnakeAnimation {
    /// Creates a new animation, with all lights off at the start.
    pub fn new(num_pixels: usize, snake_length: usize, color_step_amount: u8) -> Self {
        // let's start off with all off
        Self {
            main_pixel_color: RGB8::new(0, 0, 0),
            pixels: std::iter::repeat(RGB8::new(0, 0, 0))
                .take(num_pixels)
                .collect(),
            cur_color_increment: PixelColor::Red,
            head_location: 0,
            snake_length: snake_length,
            snake_direction: Direction::Forward,
            color_step_amount: color_step_amount,
        }
    }
}

/// Increments a pixel's value.
///
/// # Returns
/// - `Err` when it would have overflowed (does not set `val`),
/// `Ok` when the value was increment and there was no overflow.
fn pixel_increment(val: &mut u8, amount: u8) -> Result<(), ()> {
    let mut output = Ok(());
    let result = val.checked_add(amount);
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

/// Decrements a pixel's value.
///
/// # Returns
/// - `Err` when it would have overflowed (does not set `val`),
/// `Ok` when the value was increment and there was no overflow.
fn pixel_decrement(val: &mut u8, amount: u8) -> Result<(), ()> {
    let mut output = Ok(());
    let result = val.checked_sub(amount);
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

fn pixel_reset(val: &mut RGB8) {
    *val = RGB8::new(0, 0, 0);
}

impl RgbLedAnimation for Rgb8RainbowSnakeAnimation {
    fn next_frame(&mut self) {
        let cur_increment = &mut self.cur_color_increment;
        match *cur_increment {
            PixelColor::Red => {
                let inc_result =
                    pixel_increment(&mut self.main_pixel_color.r, self.color_step_amount);
                if inc_result.is_err() {
                    // reached highest value, increment next color
                    *cur_increment = PixelColor::Green;
                }

                // subtract last color
                let _ = pixel_decrement(&mut self.main_pixel_color.b, self.color_step_amount);
            }
            PixelColor::Green => {
                let inc_result =
                    pixel_increment(&mut self.main_pixel_color.g, self.color_step_amount);
                if inc_result.is_err() {
                    // reached highest value, increment next color
                    *cur_increment = PixelColor::Blue;
                }

                // subtract last color
                let _ = pixel_decrement(&mut self.main_pixel_color.r, self.color_step_amount);
            }
            PixelColor::Blue => {
                let inc_result =
                    pixel_increment(&mut self.main_pixel_color.b, self.color_step_amount);
                if inc_result.is_err() {
                    // reached highest value, increment next color
                    *cur_increment = PixelColor::Red;
                }

                // subtract last color
                let _ = pixel_decrement(&mut self.main_pixel_color.g, self.color_step_amount);
            }
        }

        // TODO: check this loop, I think it's very wrong for making a rainbow snake
        for (i, pixel) in self.pixels.iter_mut().enumerate() {
            match self.snake_direction {
                Direction::Forward => {
                    // NOTE: having this as a usize means we do not need to worry about
                    // the position being negative. It will wrap back around
                    // (to be a number much larger than the snake length) and we have
                    // up to `usize` LEDs, so we should not run into overflow issues here
                    let segment_position = self.head_location - i;
                    if segment_position < self.snake_length {
                        *pixel = self.main_pixel_color;
                    } else {
                        pixel_reset(pixel);
                    }
                }
                Direction::Backward => {
                    // TODO: this is probably wrong
                    let segment_position: i32 = (i - self.head_location) as i32;
                    if segment_position >= 0 && segment_position < self.snake_length as i32 {
                        *pixel = self.main_pixel_color;
                    } else {
                        pixel_reset(pixel);
                    }
                }
            }
        }

        match self.snake_direction {
            Direction::Backward => {
                let result = self.head_location.checked_sub(1);
                match result {
                    None => {
                        // overflow in negative direction, go forward
                        self.snake_direction = Direction::Forward;
                    }
                    Some(new_val) => {
                        self.head_location = new_val;
                    }
                }
            }
            Direction::Forward => {
                let result = self.head_location.checked_add(1);
                match result {
                    None => {
                        // overflow in positive direction, go backward
                        self.snake_direction = Direction::Backward;
                    }
                    Some(new_val) => {
                        if new_val > self.pixels.len() {
                            // no more pixels, go backward
                            self.snake_direction = Direction::Backward;
                        } else {
                            self.head_location = new_val;
                        }
                    }
                }
            }
        }
    }
}
