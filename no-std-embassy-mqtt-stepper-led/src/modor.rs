use esp_hal::delay::Delay;
use esp_hal::gpio::{AnyPin, Level, Output};

pub struct SuperSimpleModor<'a> {
    output1: Output<'a>,
    output2: Output<'a>,
    output3: Output<'a>,
    output4: Output<'a>,
}

impl<'a> SuperSimpleModor<'a> {
    pub fn new(gpio1: AnyPin, gpio2: AnyPin, gpio3: AnyPin, gpio4: AnyPin) -> Self {
        let output1 = Output::new(gpio1, Level::Low);
        let output2 = Output::new(gpio2, Level::Low);
        let output3 = Output::new(gpio3, Level::Low);
        let output4 = Output::new(gpio4, Level::Low);

        Self {
            output1,
            output2,
            output3,
            output4,
        }
    }

    pub fn steps(&mut self, count: u16) {
        let delay = Delay::new();
        let delay_time = 3000;
        // 512 full round
        for _i in 0..count {
            self.output1.set_level(Level::High);
            self.output2.set_level(Level::Low);
            self.output3.set_level(Level::Low);
            self.output4.set_level(Level::Low);
            delay.delay_micros(delay_time);
            self.output1.set_level(Level::Low);
            self.output2.set_level(Level::High);
            self.output3.set_level(Level::Low);
            self.output4.set_level(Level::Low);
            delay.delay_micros(delay_time);
            self.output1.set_level(Level::Low);
            self.output2.set_level(Level::Low);
            self.output3.set_level(Level::High);
            self.output4.set_level(Level::Low);
            delay.delay_micros(delay_time);
            self.output1.set_level(Level::Low);
            self.output2.set_level(Level::Low);
            self.output3.set_level(Level::Low);
            self.output4.set_level(Level::High);
            delay.delay_micros(delay_time);
        }
    }
}
