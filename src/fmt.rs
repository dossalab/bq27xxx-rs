//! This file ensures that defmt is optional, providing stubs if it's not available

#![macro_use]

#[cfg(feature = "defmt")]
pub use defmt::{bitflags, info};

#[cfg(not(feature = "defmt"))]
pub use bitflags::bitflags;

#[cfg(not(feature = "defmt"))]
macro_rules! info {
    ($($item:expr),*) => {};
}
