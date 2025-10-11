use esp32_led_animation::{
    led_animation::{
        elevator_animation::{ElevatorDirection, ElevatorStartingPosition, Rgb8ElevatorAnimation},
        single_led_fade_animation::Rgb8SingleLedFadeAnimation,
    },
    RgbLedAnimation,
};
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_esp32_rmt_driver::{
    driver::color::LedPixelColor, LedPixelEsp32Rmt, Ws2812Esp32RmtDriverError,
};

/// amount to fade in/out LEDs
const ELEVATOR_CAR_FADE_STEP_VALUE: u8 = 50;
const ELEVATOR_CAR_OVER_FADE_VALUE: u8 = 40;
const FLOOR_NUMBER_FADE_STEP_VALUE: u8 = 70;
const FLOOR_NUMBER_OVER_FADE_VALUE: u8 = 0;

/// The normal floor number color to use
const NORMAL_FLOOR_NUMBER_COLOR: RGB8 = RGB8::new(214, 210, 173);

//defines for the elevator
const NORMAL_ELEVATOR_COLOR: RGB8 = RGB8::new(255, 255, 225);
const RED_ELEVATOR_COLOR: RGB8 = RGB8::new(235, 0, 0);
const ELEVATOR_SPEED: usize = 4;
const ELEVATOR_STOP_FOR_NUM_FRAMES: u32 = 120;

///Chance every frame to get a new purely random floor to go to.
const ELEVATOR_RANDOM_NEW_FLOOR_CHANCE: f64 = 0.02;

// buttons
const ELEVATOR_NUM_BUTTONS: usize = 2;

/// Holds the button that is pressed. The value held inside is the index for the button LED to
/// light up.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonPressed {
    None = -1,
    Up = 0,
    Down = 1,
}

/// Struct to hold information about an elevator.
pub struct ElevatorHandler<'d, CDevFloor, CDevElevator, CDevButton>
where
    CDevFloor: LedPixelColor + From<RGB8>,
    CDevElevator: LedPixelColor + From<RGB8>,
    CDevButton: LedPixelColor + From<RGB8>,
{
    /// The floor index the elevator is on.
    floor_idx: usize,
    /// The floor that the elevator car should play an animation when arriving to.
    base_floor_idx: usize,
    /// The number of floors that the elevator can go to. Also corresponds to the number of LEDs
    /// that are going to be written to.
    num_floors: usize,
    /// LED driver for writing to the LED strip. Just specifying RGB8 since the animations use that
    /// already.
    floor_number_led_driver: LedPixelEsp32Rmt<'d, RGB8, CDevFloor>,
    /// LED driver for the elevator car.
    elevator_car_led_driver: LedPixelEsp32Rmt<'d, RGB8, CDevElevator>,
    /// LED driver for the buttons.
    button_led_driver: LedPixelEsp32Rmt<'d, RGB8, CDevButton>,
    /// The animation to play for the floor number.
    floor_number_animation: Rgb8SingleLedFadeAnimation,
    /// The animation for the elevator car.
    /// NOTE: Timing for where the elevator is should be based on this animation.
    elevator_car_animation: Rgb8ElevatorAnimation,
    /// The animation to play for the button activated.
    button_animation: Rgb8SingleLedFadeAnimation,
    /// The next floor we are going to.
    /// NOTE: Real elevators work differently, but you can't press the buttons inside the
    /// elevator, so nobody will notice.
    /// FUTURE: If we implemented a real elevator, we'd need some sort of
    /// queue system so it'd go to floors in the order they're pressed, while picking up people
    /// that are going in the same direction, if the floor goes past a floor with a button
    /// pressed.
    next_floor_idx: Option<usize>,
    /// The floor that the elevator is coming from.
    from_floor_idx: usize,
    /// If the first floor button is pressed. The elevator will go to this floor after the current floor has
    /// been handled already.
    button_pressed: ButtonPressed,
    /// The frames remaining for the elevator to be stopped.
    frames_stopped_remaining: u32,
    /// The index of floor 13.
    floor_13_idx: Option<usize>,
}

impl<'d, CDevFloor, CDevElevator, CDevButton>
    ElevatorHandler<'d, CDevFloor, CDevElevator, CDevButton>
where
    CDevFloor: LedPixelColor + From<RGB8>,
    CDevElevator: LedPixelColor + From<RGB8>,
    CDevButton: LedPixelColor + From<RGB8>,
{
    /// Creates a new elevator handler to simulate the elevator.
    ///
    /// * `base_floor_idx`: The floor for the elevator to start on and appear as arriving to.
    /// * `floor_number_led_driver`: The floor number LED driver.
    /// * `num_floors`: The number of floors in the building. Also the number of LEDs that
    /// represent the floor number.
    /// * `elevator_car_led_driver`: The elevator car LED driver.
    /// * `elevator_car_num_leds`: The number of LEDs to represent the elevator car.
    /// * `button_led_driver`: The elevator button LED driver.
    /// * `floor_13_idx`: The index of the 13th floor.
    pub fn new(
        base_floor_idx: usize,
        floor_number_led_driver: LedPixelEsp32Rmt<'d, RGB8, CDevFloor>,
        num_floors: usize,
        elevator_car_led_driver: LedPixelEsp32Rmt<'d, RGB8, CDevElevator>,
        elevator_car_num_leds: usize,
        button_led_driver: LedPixelEsp32Rmt<'d, RGB8, CDevButton>,
        floor_13_idx: Option<usize>,
    ) -> Option<Self> {
        // bounds check base floor
        if base_floor_idx >= num_floors {
            // invalid state for this elevator
            log::error!(
                "Base floor index ({}) is greater than number of floors ({})",
                base_floor_idx,
                num_floors
            );

            return None;
        }

        // bounds check floor 13
        if let Some(floor_13_idx) = floor_13_idx {
            if floor_13_idx >= num_floors {
                return None;
            }
        }

        // create the animations now
        let floor_number_animation = Rgb8SingleLedFadeAnimation::new(
            num_floors,
            NORMAL_FLOOR_NUMBER_COLOR,
            FLOOR_NUMBER_FADE_STEP_VALUE,
            FLOOR_NUMBER_OVER_FADE_VALUE,
        );
        let elevator_car_animation = Rgb8ElevatorAnimation::new(
            NORMAL_ELEVATOR_COLOR,
            elevator_car_num_leds,
            elevator_car_num_leds,
            ELEVATOR_CAR_FADE_STEP_VALUE,
            ELEVATOR_CAR_OVER_FADE_VALUE,
            true,
            ElevatorDirection::Up, // just a default, can be anything
            ELEVATOR_SPEED,
            ElevatorStartingPosition::Center,
        );
        // NOTE: buttons share the same values as the elevator floor number
        let button_animation = Rgb8SingleLedFadeAnimation::new(
            ELEVATOR_NUM_BUTTONS,
            NORMAL_FLOOR_NUMBER_COLOR,
            FLOOR_NUMBER_FADE_STEP_VALUE,
            0,
        );

        Some(Self {
            floor_idx: base_floor_idx, // we need to always start at the base floor
            base_floor_idx,
            num_floors,
            floor_number_led_driver,
            elevator_car_led_driver,
            floor_number_animation,
            elevator_car_animation,
            next_floor_idx: None,
            from_floor_idx: base_floor_idx,
            button_led_driver,
            button_animation,
            button_pressed: ButtonPressed::None,
            frames_stopped_remaining: 0,
            floor_13_idx,
        })
    }

    /// Calculates the current floor number based on the animation.
    ///
    /// # Returns
    /// Tuple containing (The floor index, At stopping point).
    fn get_floor_idx(&self) -> (usize, bool) {
        // approximate distance between floors
        let distance_between_floors = self.elevator_car_animation.elevator_length()
            + (self.elevator_car_animation.elevator_length() / 2);

        // push the bottom index up so the first floor index is 0. We can save this as an unsigned
        // integer due to this.
        let elevator_bot_idx = (std::cmp::min(
            self.elevator_car_animation.head_idx(),
            self.elevator_car_animation.tail_idx(),
        ) + (self.base_floor_idx * distance_between_floors) as isize)
            as usize;
        log::debug!("elevator_bot_idx = {}", elevator_bot_idx);
        log::debug!(
            "is at mulitple of {}: {}",
            distance_between_floors,
            elevator_bot_idx & distance_between_floors == 0
        );

        (
            // divide top of elevator by the distance between floors
            elevator_bot_idx / distance_between_floors,
            // only stop if at a multiple of distance_between_floors
            elevator_bot_idx % distance_between_floors == 0,
        )
    }

    /// Checks if the elevator is coming from floor 13 and sets it to red, normal otherwise.
    fn elevator_check_elevator_color(&mut self) {
        let mut color_set = false;
        if let Some(floor_13_idx) = self.floor_13_idx {
            if self.from_floor_idx == floor_13_idx {
                // set to red if coming from floor 13
                log::debug!("Elevator is now red");
                self.elevator_car_animation.set_color(RED_ELEVATOR_COLOR);
                color_set = true;
            }
        }

        // just set back to normal if not set and we aren't coming from the base floor.
        // NOTE: Why? It's weird to see the color change right in front of your eyes,
        // so delay it until it's at another floor to make it less jarring.
        if !color_set && self.base_floor_idx != self.from_floor_idx {
            log::debug!("Elevator is now normal");
            self.elevator_car_animation.set_color(NORMAL_ELEVATOR_COLOR);
        }
    }

    /// Changes the elevator direction if the new floor is in a different direction.
    ///
    /// * `new_floor_idx`: The new floor index.
    fn check_elevator_change_direction(&mut self, new_floor_idx: usize) {
        if (new_floor_idx < self.from_floor_idx
            && self.elevator_car_animation.elevator_direction() != ElevatorDirection::Down)
            || (new_floor_idx > self.from_floor_idx
                && self.elevator_car_animation.elevator_direction() != ElevatorDirection::Up)
        {
            self.elevator_car_animation.change_direction();
            log::debug!("Elevator car changed directions!")
        }
    }

    fn elevator_check_stop(
        &mut self,
        next_floor_idx: usize,
        new_floor_idx: usize,
        at_stopping_point: bool,
    ) {
        if next_floor_idx == new_floor_idx && at_stopping_point {
            // we are at the floor to stop at, and we have reached the stopping point.
            self.from_floor_idx = next_floor_idx;
            self.next_floor_idx = None;

            log::debug!("arrived at destination floor {}", next_floor_idx);
            // stop the elevator
            self.elevator_car_animation.set_elevator_stopped(true);
            self.frames_stopped_remaining = ELEVATOR_STOP_FOR_NUM_FRAMES;
        }
    }

    /// Moves elevator to a particular floor.
    ///
    /// * `next_floor_idx`: The next floor to go to.
    ///
    /// # Returns
    /// Ok upon success, Err otherwise.
    fn elevator_move_to_floor(&mut self, next_floor_idx: usize) -> Result<(), ()> {
        if self.from_floor_idx == next_floor_idx || next_floor_idx >= self.num_floors {
            log::error!(
                "Tried to move to floor {} when coming from floor {}",
                next_floor_idx,
                self.from_floor_idx
            );
            return Err(());
        }
        // save this random floor
        self.next_floor_idx = Some(next_floor_idx);
        log::info!("Go to floor {next_floor_idx}");

        // change directions if the floor is a different direction than the elevator is already going
        self.check_elevator_change_direction(next_floor_idx);
        // Check for floor 13 to set the elevator color
        self.elevator_check_elevator_color();

        self.elevator_car_animation.set_elevator_stopped(false);
        log::debug!(
            "Begin moving to floor {}, num frames remaining should be 0 and is ({})",
            next_floor_idx,
            self.frames_stopped_remaining
        );
        Ok(())
    }

    /// Gets a random floor index.
    ///
    /// * `begin_floor_idx`: Beginning floor index. Inclusive.
    /// * `final_floor_idx`: Final floor index. Inclusive.
    fn get_random_floor_number(begin_floor_idx: usize, final_floor_idx: usize) -> usize {
        (rand::random::<u32>() as usize % (final_floor_idx + 1)) + begin_floor_idx
    }

    /// Tries to make the elevator go to a random floor.
    ///
    /// * `begin_floor_idx`: The beginning floor index to randomly choose. Inclusive.
    /// * `final_floor_idx`: The final floor index to randomly choose. Inclusive.
    /// * `guarantee_random_floor`: Whether we to guarantee that a random floor is chosen.
    fn try_random_floor(
        &mut self,
        begin_floor_idx: usize,
        final_floor_idx: usize,
        guarantee_random_floor: bool,
    ) -> Result<(), ()> {
        // prevent an infinite loop if the only floor to go to is the one that we're coming from.
        // Also avoid generating invalid numbers.
        if (begin_floor_idx >= self.num_floors || final_floor_idx >= self.num_floors)
            || (begin_floor_idx == self.from_floor_idx && final_floor_idx == self.from_floor_idx)
            || (begin_floor_idx > final_floor_idx || final_floor_idx < begin_floor_idx)
        {
            log::error!(
                "Cannot generate a number between [{},{}]",
                begin_floor_idx,
                final_floor_idx
            );
            return Err(());
        }

        let should_go_to_next_floor = rand::random_bool(ELEVATOR_RANDOM_NEW_FLOOR_CHANCE);

        if should_go_to_next_floor || guarantee_random_floor {
            // only go to a floor that's not the current one
            let mut random_floor = Self::get_random_floor_number(begin_floor_idx, final_floor_idx);
            while random_floor == self.from_floor_idx {
                random_floor = Self::get_random_floor_number(begin_floor_idx, final_floor_idx);
            }

            self.elevator_move_to_floor(random_floor)?;
        } else {
            log::debug!("Not going to a new floor");
        }

        Ok(())
    }

    fn handle_button_press(
        &mut self,
        begin_floor_idx: usize,
        final_floor_idx: usize,
    ) -> Result<(), ()> {
        if self.from_floor_idx == self.base_floor_idx {
            // if at the base floor already, go to a floor specified
            log::debug!("handle button press");
            let err = self.try_random_floor(begin_floor_idx, final_floor_idx, true);
            if err.is_err() {
                log::error!(
                    "Could not move to a floor between [{}, {}]",
                    begin_floor_idx,
                    final_floor_idx
                );
            }

            // NOTE: this button press is unset once it leaves from the base floor
            self.turn_off_buttons();
        } else {
            // not at the base floor, go to the base floor
            let err = self.elevator_move_to_floor(self.base_floor_idx);
            if err.is_err() {
                log::error!("Could not move to the base floor");
            }
        }

        Ok(())
    }

    /// Checks if the elevator should go to a next floor, and changes direction if it is not going
    /// in that direction already. Stops the elevator if it reaches the floor it wants.
    ///
    /// * `new_floor_idx`: The new floor index to go to.
    fn elevator_check_next_floor(&mut self, new_floor_idx: usize, at_stopping_point: bool) {
        if let Some(next_floor_idx) = self.next_floor_idx {
            log::debug!("checking stop");
            // we have some next floor to go to
            self.elevator_check_stop(next_floor_idx, new_floor_idx, at_stopping_point);
        } else if self.frames_stopped_remaining == 0 {
            log::debug!("checking next move");
            // elevator can now move
            if self.button_pressed == ButtonPressed::Down {
                log::debug!("Handle down press");
                // down button was pressed
                let err = self.handle_button_press(0, self.base_floor_idx - 1);
                if err.is_err() {
                    log::error!("Could not handle down button press");
                }
            } else if self.button_pressed == ButtonPressed::Up {
                log::debug!("Handle up press");
                // up button was pressed
                let err = self.handle_button_press(self.base_floor_idx + 1, self.num_floors - 1);
                if err.is_err() {
                    log::error!("Could not handle up button press");
                }
            } else {
                // no button pressed, purely random floor
                let err = self.try_random_floor(0, self.num_floors - 1, false);
                if err.is_err() {
                    log::error!("Could not try random floor");
                }
            }
        } else {
            // waiting...
            self.frames_stopped_remaining -= 1;
            log::debug!("waiting..")
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
        log::debug!("elevator_car_pixels = {:?}", elevator_car_pixels);
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
        // now let's see what floor the elevator is on.
        let (cur_floor_idx, at_stopping_point) = self.get_floor_idx();
        log::debug!("current elevator floor index = {}", cur_floor_idx);

        // perform checks for next floor
        self.elevator_check_next_floor(cur_floor_idx, at_stopping_point);

        // move the elevator car first before we set the elevator number
        self.elevator_next_frame()?;

        // write again only if the new floor index is different
        if cur_floor_idx != self.floor_idx {
            self.set_floor_number(cur_floor_idx);
        }

        // handle the floor number animation
        // NOTE: we need to do this every frame, in case it is flickering
        self.floor_number_next_frame()?;
        Ok(())
    }

    /// Simulate pressing down button.
    pub fn press_down_button(&mut self) {
        self.button_pressed = ButtonPressed::Down;
        self.button_animation
            .set_led_on(self.button_pressed as usize, true);
    }

    /// Simulate pressing up button.
    pub fn press_up_button(&mut self) {
        self.button_pressed = ButtonPressed::Up;
        self.button_animation
            .set_led_on(self.button_pressed as usize, true);
    }

    /// Turns off buttons.
    pub fn turn_off_buttons(&mut self) {
        self.button_pressed = ButtonPressed::None;
        self.button_animation.turn_led_off();
    }
}
