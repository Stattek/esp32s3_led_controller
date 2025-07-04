use std::collections::VecDeque;

use crate::led_animation::ws2812_led_animation::{Direction, RgbLedAnimation};
use smart_leds::RGB8;

// predefined pixel sequences
#[allow(dead_code)]
pub const FOURTH_OF_JULY_SEQUENCE: [RGB8; 9] = [
    RGB8::new(255, 0, 0),
    RGB8::new(255, 0, 0),
    RGB8::new(255, 0, 0),
    RGB8::new(255, 255, 255),
    RGB8::new(255, 255, 255),
    RGB8::new(255, 255, 255),
    RGB8::new(0, 0, 255),
    RGB8::new(0, 0, 255),
    RGB8::new(0, 0, 255),
];

/// Struct to handle moving a basic pixel sequence on many LEDs.
pub struct Rgb8BasicPixelSequenceAnimation {
    repeated_color_sequence: VecDeque<RGB8>,
    pixels: Vec<RGB8>,
    num_pixels: usize,
    direction: Direction,
}

impl AsRef<Vec<RGB8>> for Rgb8BasicPixelSequenceAnimation {
    fn as_ref(&self) -> &Vec<RGB8> {
        &self.pixels
    }
}

impl Rgb8BasicPixelSequenceAnimation {
    /// Creates a new animation, with a basic pixel sequence.
    #[allow(dead_code)]
    pub fn new(num_pixels: usize, pixel_sequence: Vec<RGB8>, direction: Direction) -> Self {
        Self {
            repeated_color_sequence: VecDeque::from(pixel_sequence.clone()),
            pixels: pixel_sequence
                .into_iter()
                .cycle()
                .take(num_pixels)
                .collect(),
            num_pixels,
            direction,
        }
    }
}

impl RgbLedAnimation for Rgb8BasicPixelSequenceAnimation {
    fn next_frame(&mut self) {
        // we will move the pattern one pixel in our current direction.
        if let Some(popped) = match self.direction {
            Direction::Forward => self.repeated_color_sequence.pop_back(),
            Direction::Backward => self.repeated_color_sequence.pop_front(),
        } {
            match self.direction {
                Direction::Forward => self.repeated_color_sequence.push_front(popped),
                Direction::Backward => self.repeated_color_sequence.push_back(popped),
            }
        }

        // set the pixels to the new repeated pattern
        self.pixels = self
            .repeated_color_sequence
            .clone()
            .into_iter()
            .cycle()
            .take(self.num_pixels)
            .collect();
    }
}

impl Rgb8BasicPixelSequenceAnimation {
    /// Sets the direction of the animation.
    #[allow(dead_code)]
    pub fn set_direction(&mut self, direction: Direction) {
        self.direction = direction;
    }
}
