//! A platform agnostic driver to interface with the HX711 (load cell amplifier and ADC)
//!
//! This driver was built using [`embedded-hal`] traits.
//!
//! [`embedded-hal`]: https://docs.rs/embedded-hal/0.2

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]

extern crate embedded_hal as hal;

#[macro_use(block)]
extern crate nb;

use hal::digital::v2::InputPin;
use hal::digital::v2::OutputPin;

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
    IN: InputPin<Error=PINERR>,
    OUT: OutputPin<Error=PINERR>,
{
    /// Creates a new driver from Input and Outut pins
    pub fn new(dout: IN, pd_sck: OUT) -> Self {
        Hx711 {
            dout,
            pd_sck,
            mode: Mode::ChAGain128,
        }
    }

    /// Set the mode (channel and gain).
    pub fn set_mode(&mut self, mode: Mode) -> Result<(), PINERR>{
        self.mode = mode;
        block!(self.retrieve())?;
        Ok(())
    }

    /// Reset the chip. Mode is Channel A Gain 128 after reset.
    pub fn reset(&mut self) -> Result<(), PINERR> {
        self.pd_sck.set_high()?;
        for _ in 1..3 {
            self.dout.is_high()?;
        }
        self.pd_sck.set_low()?;
        Ok(())
    }

    /// Retrieve the latest conversion value if available
    pub fn retrieve(&mut self) -> nb::Result<i32, PINERR> {
        self.pd_sck.set_low()?;
        if self.dout.is_high()? {
            // Conversion not ready yet
            return Err(nb::Error::WouldBlock);
        }

        let mut count: i32 = 0;
        for _ in 0..24 {
            // Read 24 bits
            count <<= 1;
            self.pd_sck.set_high()?;
            self.pd_sck.set_low()?;
            if self.dout.is_high()? {
                count += 1;
            }
        }

        // Continue to set mode for next conversion
        let n_reads = self.mode as u16;
        for _ in 0..n_reads {
            self.pd_sck.set_high()?;
            self.pd_sck.set_low()?;
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
    if x >= 0x800000 {
        x | !0xFFFFFF
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
