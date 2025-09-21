# README

This is the repository for driving WS2812B LED strips on the ESP32.
It is currently tested to run on the ESP32-S3.

## TODO
- Implement support for WS2811 LEDs.
    - Seems like all we need to do to handle these LEDs is to swap the R and G values when we push the bits.
