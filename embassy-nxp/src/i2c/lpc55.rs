use embassy_hal_internal::{Peri, PeripheralType};
use embedded_io::{self, ErrorKind};
use nxp_pac::syscon::regs::Sdioclkctrl;

use crate::gpio::{match_iocon, AnyPin, Bank, SealedPin};
use crate::pac::flexcomm::Flexcomm as FlexcommReg;
use crate::pac::i2c::I2c as I2cReg;
use crate::pac::iocon::vals::PioFunc;
use crate::pac::*;
use crate::{Blocking, Mode};
use core::marker::PhantomData;

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

/// I2C driver.
#[derive(Debug)]
pub struct I2c<'d, M: Mode> {
    phantom: PhantomData<(&'d (), M)>,
}

impl<'d> I2c<'d, Blocking> {
    /// Create a new driver instance in blocking mode.
    pub fn new_blocking<T: Instance>(
        peri: Peri<'d, T>,
        sda: Peri<'d, impl SdaPin<T>>,
        scl: Peri<'d, impl SclPin<T>>,
        config: Config,
    ) -> Self {
        Self::new_inner(peri, scl.into(), sda.into(), config)
    }
}

impl<'d, M: Mode> I2c<'d, M> {
    fn new_inner<T: Instance>(
        _peri: Peri<'d, T>,
        scl: Peri<'d, AnyPin>,
        sda: Peri<'d, AnyPin>,
        config: Config,
    ) -> Self {
        Self { phantom: PhantomData }
    }

    fn init<T: Instance>(sda: Option<Peri<'_, AnyPin>>, scl: Option<Peri<'_, AnyPin>>, config: Config) {
        Self::configure_flexcomm(T::info().fc_reg, T::instance_number());
        Self::configure_clock::<T>(&config);
        Self::pin_config::<T>(sda, scl);
        Self::configure_i2c(T::info(), &config);
    }
    fn configure_flexcomm(flexcomm_register: crate::pac::flexcomm::Flexcomm, instance_number: usize) {
        critical_section::with(|_cs| {
            if !(SYSCON.ahbclkctrl0().read().iocon()) {
                SYSCON.ahbclkctrl0().modify(|w| w.set_iocon(true));
            }
        });
        critical_section::with(|_cs| {
            if !(SYSCON.ahbclkctrl1().read().fc(instance_number)) {
                SYSCON.ahbclkctrl1().modify(|w| w.set_fc(instance_number, true));
            }
        });
        SYSCON
            .presetctrl1()
            .modify(|w| w.set_fc_rst(instance_number, syscon::vals::FcRst::ASSERTED));
        SYSCON
            .presetctrl1()
            .modify(|w| w.set_fc_rst(instance_number, syscon::vals::FcRst::RELEASED));
        flexcomm_register
            .pselid()
            .modify(|w| w.set_persel(flexcomm::vals::Persel::I2C));
    }
    fn configure_clock<T: Instance>(config: Config) {}
}
struct Info {
    i2c_reg: I2cReg,
    fc_reg: FlexcommReg,
}

trait SealedInstance {
    fn info() -> &'static Info;
    fn instance_number() -> usize;
    fn sda_pin_func() -> PioFunc;
    fn scl_pin_func() -> PioFunc;
}

/// UART instance.
#[allow(private_bounds)]
pub trait Instance: SealedInstance + PeripheralType {}

fn configure_clock<T: Instance>(config: &Config) {}

/// Trait for SDA pins.
pub trait SdaPin<T: Instance>: crate::gpio::Pin {}
/// Trait for SCL pins.
pub trait SclPin<T: Instance>: crate::gpio::Pin {}
