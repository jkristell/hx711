//! A platform agnostic driver to interface with the HX711 (load cell amplifier and ADC)
//!
//! This driver was built using [`embedded-hal`] traits.
//!
//! [`embedded-hal`]: https://docs.rs/embedded-hal/0.2

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]

use embedded_hal as hal;
use nb::block;

use hal::{
    digital::v2::{
        InputPin,
        OutputPin
    },
    blocking::delay::DelayUs,
};


/// Maximum ADC value
pub const MAX_VALUE: i32 = (1 << 23) - 1;

/// Minimum ADC value
pub const MIN_VALUE: i32 = (1 << 23);

/// HX711 driver
pub struct Hx711<IN, OUT> {
    dout: IN,
    pd_sck: OUT,
    mode: Mode,
}


impl<IN, OUT, PINERR> Hx711<IN, OUT>
where
    IN: InputPin<Error = PINERR>,
    OUT: OutputPin<Error = PINERR>,
{
    /// Creates a new driver from Input and Outut pins
    pub fn new(dout: IN, pd_sck: OUT) -> Self {
        let hx711 = Hx711 {
            dout,
            pd_sck,
            mode: Mode::ChAGain128,
        };
        hx711
    }

    /// Destruct the device and hand back the pins
    pub fn destroy(self) -> (IN, OUT) {
        (self.dout, self.pd_sck)
    }

    /// Enable 
    pub fn enable(&mut self) -> Result<(), PINERR> {
        self.pd_sck.set_low()
    }

    /// Disable - after 60 us of clk high the chip will go to sleep
    pub fn disable<DELAY>(&mut self, delay: &mut DELAY) -> Result<(), PINERR>
    where
        DELAY: DelayUs<u16>,
    {
        self.pd_sck.set_high()?;
        delay.delay_us(60);
        Ok(())
    }

    /// Set the mode (channel and gain).
    pub fn set_mode<DELAY>(&mut self, mode: Mode, delay: &mut DELAY) -> Result<(), PINERR> 
    where
        DELAY: DelayUs<u16>,
    {
        self.mode = mode;
        block!(self.retrieve(delay)).map(|_| ())
    }

    /// Retrieve the latest conversion value if available
    pub fn retrieve<DELAY>(&mut self, delay: &mut DELAY) -> nb::Result<i32, PINERR>
    where
        DELAY: DelayUs<u16>,
    {
        self.pd_sck.set_low()?;
        if self.dout.is_high()? {
            // Conversion not ready yet
            return Err(nb::Error::WouldBlock);
        }

        // Dout falling -> clock high > 0.1 us 
        delay.delay_us(1);

        let mut count: i32 = 0;
        for _ in 0..24 {
            // Read 24 bits
            count <<= 1;

            // Clock high
            self.pd_sck.set_high()?;

            delay.delay_us(1);

            // Read out data
            if self.dout.is_high()? {
                count += 1;
            }

            // Clock low
            self.pd_sck.set_low()?;
            delay.delay_us(1);
        }

        // Continue to set mode for next conversion
        let n_reads = self.mode as u16;
        for _ in 0..n_reads {
            self.pd_sck.set_high()?;
            delay.delay_us(1);
            self.pd_sck.set_low()?;
            delay.delay_us(1);
        }

        Ok(i24_to_i32(count))
    }
}

/// The HX711 can run in three modes:
#[derive(Copy, Clone)]
pub enum Mode {
    /// Chanel A with factor 128 gain
    ChAGain128 = 1,
    /// Chanel B with factor 64 gain
    ChBGain32 = 2,
    /// Chanel B with factor 32 gain
    ChBGain64 = 3,
}

/// Convert 24 bit signed integer to i32
fn i24_to_i32(x: i32) -> i32 {
    if x >= 0x80_0000 {
        x | !0x00FF_FFFF
    } else {
        x
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn convert() {
        assert_eq!(i24_to_i32(0x000001), 1);
        assert_eq!(i24_to_i32(0x000002), 2);
        assert_eq!(i24_to_i32(0xFFFFFF), -1);
        assert_eq!(i24_to_i32(0xFFFFF3), -13);
    }
}
