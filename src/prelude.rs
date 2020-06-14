//! Should be imported into every library consuming this one
pub use embedded_hal::digital::v2::InputPin as _embedded_hal_digital_v2_InputPin;
pub use embedded_hal::digital::v2::OutputPin as _embedded_hal_digital_v2_OutputPin;
pub use embedded_hal::digital::v2::StatefulOutputPin as _embedded_hal_digital_v2_StatefulOutputPin;
pub use embedded_hal::digital::v2::ToggleableOutputPin as _embedded_hal_digital_v2_ToggleableOutputPin;
pub use embedded_hal::prelude::*;

pub use crate::ckcu::CkcuExt as _ht32f5xxxx_ckcu_CkcuExt;
pub use crate::gpio::GpioExt as _ht32f5xxxx_gpio_GpioExt;
pub use crate::time::U32Ext as _ht32f5xxxx_hal_time_U32Ext;
pub use crate::spi::SpiExt as _ht32f5xxxx_hal_spi_SpiExt;
