use embassy_hal_internal::{Peri, PeripheralType};
use embedded_io::{self, ErrorKind};

use crate::gpio::{match_iocon, AnyPin, Bank, SealedPin};
use crate::pac::flexcomm::Flexcomm as FlexcommReg;
use crate::pac::i2c::I2c;
use crate::pac::iocon::vals::PioFunc;
use crate::pac::*;
use crate::{Blocking, Mode};

/// I2C error abort reason
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AbortReason {
    /// A bus operation was not acknowledged, e.g. due to the addressed device
    /// not being available on the bus or the device not being ready to process
    /// requests at the moment
    NoAcknowledge,
    /// The arbitration was lost, e.g. electrical problems with the clock signal
    ArbitrationLoss,
    /// Transmit ended with data still in fifo
    TxNotEmpty(u16),
    /// Other reason.
    Other(u32),
}

/// I2C error
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// I2C abort with error
    Abort(AbortReason),
    /// User passed in a read buffer that was 0 length
    InvalidReadBufferLength,
    /// User passed in a write buffer that was 0 length
    InvalidWriteBufferLength,
    /// Target i2c address is out of range
    AddressOutOfRange(u16),
    /// Target i2c address is reserved
    #[deprecated = "embassy_rp no longer prevents accesses to reserved addresses."]
    AddressReserved(u16),
}

/// I2C Config error
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConfigError {
    /// Max i2c speed is 1MHz
    FrequencyTooHigh,
    /// The sys clock is too slow to support given frequency
    ClockTooSlow,
    /// The sys clock is too fast to support given frequency
    ClockTooFast,
}
/// I2C config.
#[non_exhaustive]
#[derive(Copy, Clone)]
pub struct Config {
    /// Frequency.
    pub frequency: u32,
    /// Enable internal pullup on SDA.
    ///
    /// Using external pullup resistors is recommended for I2C. If you do
    /// have external pullups you should not enable this.
    pub sda_pullup: bool,
    /// Enable internal pullup on SCL.
    ///
    /// Using external pullup resistors is recommended for I2C. If you do
    /// have external pullups you should not enable this.
    pub scl_pullup: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            frequency: 100_000,
            sda_pullup: true,
            scl_pullup: true,
        }
    }
}

impl<'d, T: Instance> I2c<'d, T, Blocking> {
    /// Create a new driver instance in blocking mode.
    pub fn new_blocking(
        peri: Peri<'d, T>,
        scl: Peri<'d, impl AnyPin<T>>,
        sda: Peri<'d, impl AnyPin<T>>,
        config: Config,
    ) -> Self {
        Self::new_inner(peri, scl.into(), sda.into(), config)
    }
}

/// I2C driver.
#[derive(Debug)]
pub struct I2c<'d, T: Instance, M: Mode> {
    phantom: PhantomData<(&'d mut T, M)>,
}

fn init<T: Instance>(sda: Option<Peri<'_, AnyPin>>, scl: Option<Peri<'_, AnyPin>>, config: Config) {
    Self::configure_flexcomm(T::info().fc_reg, T::instance_number());
    Self::configure_clock::<T>(&config);
    Self::pin_config::<T>(tx, rx);
    Self::configure_usart(T::info(), &config);
}
