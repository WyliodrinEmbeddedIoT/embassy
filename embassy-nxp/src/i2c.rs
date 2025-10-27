//! Inter-Integrated Circuit (I2C) driver.

#[cfg_attr(feature = "lpc55-core0", path = "./i2c/lpc55.rs")]
mod inner;
pub use inner::*;
