// NOTE: WS2811 support isn't already implemented in ws2812_esp32_rmt_driver

use smart_leds::RGB8;
use ws2812_esp32_rmt_driver::{driver::color::LedPixelColorImpl, LedPixelEsp32Rmt};

/// 8-bit RGB LED pixel color (total 32-bit pixel), Typical RGB LED (WS2811) pixel color
pub type LedPixelColorRgb24 = LedPixelColorImpl<3, 0, 1, 2, 255>;

/// 8-bit RGB (total 24-bit pixel) LED driver wrapper providing smart-leds API,
/// Typical RGB LED (WS2811) driver wrapper providing smart-leds API
pub type Ws2811Esp32Rmt<'d> = LedPixelEsp32Rmt<'d, RGB8, LedPixelColorRgb24>;
