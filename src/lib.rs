#![cfg_attr(not(test), no_std)]

#[cfg(not(feature = "device-selected"))]
compile_error!(
    "This crate requires one of the following device features enabled:
     ht32f52342_52
"
);

pub use embedded_hal as hal;

pub use nb;
pub use nb::block;

#[cfg(any(feature = "ht32f52342_52"))]
pub use ht32f5xxxx::ht32f52342_52 as ht32;

// Enable use of interrupt macro
#[cfg(feature = "rt")]
pub use crate::ht32::interrupt;

#[cfg(feature = "device-selected")]
pub use ht32 as pac;

#[cfg(feature = "device-selected")]
pub mod prelude;

#[cfg(feature = "device-selected")]
pub mod time;

#[cfg(feature = "device-selected")]
pub mod ckcu;
