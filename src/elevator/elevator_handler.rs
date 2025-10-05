use esp32_led_animation::{
    led_animation::{
        elevator_animation::{ElevatorDirection, ElevatorStartingPosition, Rgb8ElevatorAnimation},
        single_led_fade_animation::Rgb8SingleLedFadeAnimation,
    },
    RgbLedAnimation,
};
use esp_idf_svc::{hal::task::queue::Queue, sys::random};
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_esp32_rmt_driver::{
    driver::color::LedPixelColor, LedPixelEsp32Rmt, Ws2812Esp32RmtDriverError,
};

const FADE_STEP_VALUE: u8 = 70;

const NORMAL_FLOOR_NUMBER_COLOR: RGB8 = RGB8::new(255, 255, 255);

//defines for the elevator
const NORMAL_ELEVATOR_COLOR: RGB8 = RGB8::new(255, 255, 255);
const RED_ELEVATOR_COLOR: RGB8 = RGB8::new(255, 0, 0);
const ELEVATOR_SPEED: usize = 1;

const ELEVATOR_RANDOM_NEW_FLOOR_CHANCE: f64 = 0.1;
/// Struct to hold information about an elevator.
pub struct ElevatorHandler<'d, CDev>
where
    CDev: LedPixelColor + From<RGB8>,
{
    /// The floor index the elevator is on.
    floor_idx: usize,
    /// The number of floors that the elevator can go to. Also corresponds to the number of LEDs
    /// that are going to be written to.
    num_floors: usize,
    /// LED driver for writing to the LED strip. Just specifying RGB8 since the animations use that
    /// already.
    floor_number_led_driver: LedPixelEsp32Rmt<'d, RGB8, CDev>,
    /// LED driver for the elevator car.
    elevator_car_led_driver: LedPixelEsp32Rmt<'d, RGB8, CDev>,
    /// The animation to play
    floor_number_animation: Rgb8SingleLedFadeAnimation,
    /// The animation for the elevator car.
    /// NOTE: Timing for where the elevator is should be based on this animation.
    elevator_car_animation: Rgb8ElevatorAnimation,
    /// The next floor we are going to.
    /// NOTE: Real elevators work differently, but you can't press the buttons inside the
    /// elevator, so nobody will notice.
    /// FUTURE: If we implemented a real elevator, we'd need some sort of
    /// queue system so it'd go to floors in the order they're pressed, while picking up people
    /// that are going in the same direction, if the floor goes past a floor with a button
    /// pressed.
    next_floor_idx: Option<usize>,
    /// The floor that the elevator is coming from.
    /// TODO: if someone comes from the 13th floor, light up the elevator red.
    from_floor_idx: usize,
    /// If the first floor button is pressed. The elevator will go to this floor after the current floor has
    /// been handled already.
    first_floor_button_pressed: bool,
}

impl<'d, CDev> ElevatorHandler<'d, CDev>
where
    CDev: LedPixelColor + From<RGB8>,
{
    pub fn new(
        floor_idx: usize,
        floor_number_led_driver: LedPixelEsp32Rmt<'d, RGB8, CDev>,
        num_floors: usize,
        elevator_car_led_driver: LedPixelEsp32Rmt<'d, RGB8, CDev>,
        elevator_car_num_leds: usize,
    ) -> Result<Self, ()> {
        if floor_idx >= num_floors {
            // invalid state for this elevator
            log::error!(
                "Floor index ({}) is greater than number of floors ({})",
                floor_idx,
                num_floors
            );
            Err(())
        } else {
            // create the animations now
            let floor_number_animation =
                Rgb8SingleLedFadeAnimation::new(num_floors, NORMAL_ELEVATOR_COLOR, FADE_STEP_VALUE);
            let elevator_car_animation = Rgb8ElevatorAnimation::new(
                NORMAL_ELEVATOR_COLOR,
                elevator_car_num_leds,
                elevator_car_num_leds,
                FADE_STEP_VALUE,
                true,
                ElevatorDirection::Up,
                ELEVATOR_SPEED,
                ElevatorStartingPosition::Center,
            );
            Ok(Self {
                floor_idx,
                num_floors,
                floor_number_led_driver,
                elevator_car_led_driver,
                floor_number_animation,
                elevator_car_animation,
                next_floor_idx: None,
                from_floor_idx: floor_idx,
                first_floor_button_pressed: false,
            })
        }
    }

    /// Calculates the current floor number based on the animation.
    fn get_floor_idx(&self) -> usize {
        let elevator_top_idx = std::cmp::max(
            self.elevator_car_animation.head_idx(),
            self.elevator_car_animation.tail_idx(),
        );
        if elevator_top_idx <= 0 {
            // if it's at or below 0, just treat it as the bottom floor
            // NOTE: prevents errors when we divide later, when we cast types
            return 0;
        }

        // approximate distance between floors
        let distance_between_floors = self.elevator_car_animation.elevator_length()
            + (self.elevator_car_animation.elevator_length() / 2);

        // divide top of elevator by the distance between floors
        elevator_top_idx as usize / distance_between_floors
    }

    /// Checks if the elevator should go to a next floor, and changes direction if it is not going
    /// in that direction already.
    ///
    /// * `new_floor_idx`: The new floor index to go to.
    fn elevator_check_next_floor(&mut self, new_floor_idx: usize) {
        if let Some(next_floor_idx) = self.next_floor_idx {
            // we have some next floor to go to
            if next_floor_idx == new_floor_idx {
                // the last floor we stopped at is the one we are at now
                self.from_floor_idx = next_floor_idx;
                self.next_floor_idx = None;
            }
        } else {
            // no floor to go to, have a random chance to go to a new floor every frame
            let should_go_to_next_floor = rand::random_bool(ELEVATOR_RANDOM_NEW_FLOOR_CHANCE);

            if should_go_to_next_floor {
                // only go to a floor that's not the current one
                let mut random_floor = rand::random::<u32>() as usize % self.num_floors;
                while random_floor != self.from_floor_idx {
                    random_floor = rand::random::<u32>() as usize % self.num_floors;
                }

                // save this random floor
                self.next_floor_idx = Some(random_floor);
                log::debug!("Go to floor {random_floor}");

                // change directions if the floor is a different direction than the elevator is already going
                if (random_floor < self.from_floor_idx
                    && self.elevator_car_animation.elevator_direction() != ElevatorDirection::Down)
                    || (random_floor > self.from_floor_idx
                        && self.elevator_car_animation.elevator_direction()
                            != ElevatorDirection::Up)
                {
                    self.elevator_car_animation.change_direction();
                    log::debug!("Elevator car changed directions!")
                }
            }
        }
    }

    /// Sets the floor number.
    ///
    /// * `new_floor_idx`: The new floor index.
    fn set_floor_number(&mut self, new_floor_idx: usize) {
        self.floor_idx = new_floor_idx;
        let should_flicker = rand::random_bool(0.5);
        self.floor_number_animation
            .set_led_on(self.floor_idx, should_flicker)
            .expect("Could not write floor number animation");
    }

    /// Simulates the next frame of the elevator and writes to LEDs.
    fn elevator_next_frame(&mut self) -> Result<(), Ws2812Esp32RmtDriverError> {
        self.elevator_car_animation.next_frame();
        let elevator_car_pixels = self.elevator_car_animation.as_ref().clone();
        self.elevator_car_led_driver.write(elevator_car_pixels)
    }

    /// Simulates the next frame of the floor number buttons and writes to LEDs.
    fn floor_number_next_frame(&mut self) -> Result<(), Ws2812Esp32RmtDriverError> {
        self.floor_number_animation.next_frame();
        let floor_number_pixels = self.floor_number_animation.as_ref().clone();
        self.floor_number_led_driver.write(floor_number_pixels)
    }

    /// Calculate the next frame and write it to the LEDs.
    pub fn next_frame(&mut self) -> Result<(), Ws2812Esp32RmtDriverError> {
        // move the elevator car first before we set the elevator number
        self.elevator_next_frame()?;

        // now let's see what floor the elevator is on.
        let cur_floor_idx = self.get_floor_idx();
        log::debug!("current elevator floor index = {}", cur_floor_idx);

        // perform checks for next floor
        self.elevator_check_next_floor(cur_floor_idx);

        // write again only if the new floor index is different
        if cur_floor_idx != self.floor_idx {
            self.set_floor_number(cur_floor_idx);
        }

        // handle the floor number animation
        // NOTE: we need to do this every frame, in case it is flickering
        self.floor_number_next_frame()?;
        Ok(())
    }

    /// Moves the elevator down a floor.
    pub fn move_down(&mut self, flicker: bool) -> Result<(), ()> {
        if self.floor_idx == 0 {
            log::error!("Tried to move down a floor when at the bottom floor");
            Err(())
        } else {
            // go down a floor
            self.floor_idx -= 1;
            self.floor_number_animation
                .set_led_on(self.floor_idx, flicker)
        }
    }

    /// Moves the elevator up a floor.
    pub fn move_up(&mut self, flicker: bool) -> Result<(), ()> {
        if self.floor_idx >= self.num_floors {
            log::error!("Tried to move up a floor when already at the top floor");
            Err(())
        } else {
            // go up a floor
            self.floor_idx += 1;
            self.floor_number_animation
                .set_led_on(self.floor_idx, flicker)
        }
    }
}
