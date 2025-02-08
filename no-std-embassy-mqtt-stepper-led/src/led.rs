use esp_hal::gpio::AnyPin;
use esp_hal::peripherals::RMT;
use esp_hal::rmt::Rmt;
use esp_hal::time::RateExtU32;
use esp_hal_smartled::{smartLedBuffer, SmartLedsAdapter};
use smart_leds::{brightness, gamma, SmartLedsWrite, RGB8};

pub struct SuperEzLed {
    led: SmartLedsAdapter<esp_hal::rmt::Channel<esp_hal::Blocking, 0>, 25>,
}

impl SuperEzLed {
    pub fn new(led_pin: AnyPin, rmt: RMT) -> Self {
        let freq = 80.MHz();
        let rmt = Rmt::new(rmt, freq).unwrap();
        let rmt_buffer = smartLedBuffer!(1);
        let led = SmartLedsAdapter::new(rmt.channel0, led_pin, rmt_buffer);

        Self { led }
    }

    pub fn set_color(&mut self, color: Color) {
        let color = match color {
            Color::Red => RGB8::new(255, 0, 0),
            Color::Green => RGB8::new(0, 255, 0),
            Color::Blue => RGB8::new(0, 0, 255),
            Color::Yellow => RGB8::new(255, 255, 0),
            Color::Orange => RGB8::new(255, 125, 255),
            Color::Purple => RGB8::new(128, 0, 128),
            Color::StrongOrange => RGB8::new(184, 134, 11),
        };
        let data = [color];
        self.led
            .write(brightness(gamma(data.iter().cloned()), 10))
            .unwrap();
    }
}

pub enum Color {
    Red,
    Green,
    Blue,
    Yellow,
    Orange,
    Purple,
    StrongOrange,
}
