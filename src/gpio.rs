//! General Purpose Input / Output

use core::marker::PhantomData;

/// Extension trait to split a GPIO peripheral in independent pins and registers
pub trait GpioExt {
    /// The to split the GPIO into
    type Parts;

    /// Splits the GPIO block into independent pins and registers
    fn split(self) -> Self::Parts;
}

/// Output mode (type state)
/// `MODE`: describes the output type
pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}

/// Open drain output (type state)
pub struct OpenDrain;

/// Push pull output (type state)
pub struct PushPull;

/// Input mode (type state)
/// `MODE`: describes the output type
pub struct Input<MODE> {
    _mode: PhantomData<MODE>,
}

/// Pulled up input (type state)
pub struct PullUp;

/// Pulled down input (type state)
pub struct PullDown;

/// FLoating input (type state)
pub struct Floating;

/// Disabled input (type state)
/// Holtek chips do allow a chip to be configured as input but not actually read
/// any data.
pub struct Disabled;

/// Alternate function 0 (type state)
pub struct AF0;
/// Alternate function 1 (type state)
pub struct AF1;
/// Alternate function 2 (type state)
pub struct AF2;
/// Alternate function 3 (type state)
pub struct AF3;
/// Alternate function 4 (type state)
pub struct AF4;
/// Alternate function 5 (type state)
pub struct AF5;
/// Alternate function 6 (type state)
pub struct AF6;
/// Alternate function 7 (type state)
pub struct AF7;
/// Alternate function 8 (type state)
pub struct AF8;
/// Alternate function 9 (type state)
pub struct AF9;
/// Alternate function 10 (type state)
pub struct AF10;
/// Alternate function 11 (type state)
pub struct AF11;
/// Alternate function 12 (type state)
pub struct AF12;
/// Alternate function 13 (type state)
pub struct AF13;
/// Alternate function 14 (type state)
pub struct AF14;
/// Alternate function 15 (type state)
pub struct AF15;

/// The 4 current values that can be used for output pins
/// TODO: Migrate these into the PAC and re-export them here in order to avoid
/// API breaking.
#[derive(Copy, Clone, Debug)]
pub enum GpioCurrent {
    MA4,
    MA8,
    MA12,
    MA16,
}

impl GpioCurrent {
    fn to_bits(&self) -> u8 {
        match self {
            Self::MA4 => 0b00,
            Self::MA8 => 0b01,
            Self::MA12 => 0b10,
            Self::MA16 => 0b11,
        }
    }
}

macro_rules! gpio {
    ($GPIOX:ident, $gpiox:ident, $PXx:ident, $pxrst:ident, $pxen:ident, $gpiox_doutr:ident, $gpiox_dinr:ident, $gpiox_drvr:ident, $gpiox_dircr:ident, $gpiox_pur:ident, $gpiox_pdr:ident, $gpiox_iner: ident, $gpiox_odr:ident, [
         $($PXi:ident: ($pxi:ident, $i:expr, $MODE:ty, $AF:ty, $doutx: ident, $dinx: ident, $dvx:ident, $dirx:ident, $pux: ident, $pdx:ident, $inenx:ident, $odx:ident, $cfgx:ident, $afio_gpxcfgr:ident ),)+
    ]) => {
        pub mod $gpiox {
            use core::convert::Infallible;
            use core::marker::PhantomData;

            use crate::hal::digital::v2::{InputPin, OutputPin, StatefulOutputPin, toggleable};
            use crate::ht32::{$GPIOX, RSTCU, AFIO, CKCU};

            use super::{
                Output, Input, OpenDrain, PushPull, PullDown, PullUp, Floating,
                AF0, AF1, AF2, AF3, AF4, AF5, AF6, AF7, AF8, AF9, AF10, AF11,
                AF12, AF13, AF14, AF15, GpioCurrent, GpioExt, Disabled
            };


            /// The to split the GPIO into
            pub struct Parts {
                $(
                    /// Pin
                    pub $pxi: $PXi<$MODE, $AF>,
                )+
            }

            impl GpioExt for $GPIOX {
                type Parts = Parts;

                fn split(self) -> Parts {
                    let rstcu = unsafe { &*RSTCU::ptr() };
                    let ckcu = unsafe { &*CKCU::ptr() };
                    // reset the GPIO port before using it
                    rstcu.rstcu_ahbprstr.modify(|_, w| w.$pxrst().set_bit());
                    // enable the AHB clock for the GPIO port
                    ckcu.ckcu_ahbccr.modify(|_, w| w.$pxen().set_bit());


                    Parts {
                        $(
                            $pxi: $PXi { _mode: PhantomData, _af: PhantomData },
                        )+
                    }
                }
            }

            /// A general struct that can describe all the pins in this GPIO block,
            /// in case one would have to iterate over them, store them in an array
            /// etc.
            pub struct $PXx<MODE> {
                i: u8,
                _mode: PhantomData<MODE>
            }

            impl<MODE> $PXx<MODE> {
                pub fn get_id(&self) -> u8 {
                    self.i
                }
            }

            // All PXx in any `Output` mode can do this
            impl<OUTPUT> OutputPin for $PXx<Output<OUTPUT>> {
                // There can be no (detectible) errors for GPIO on this chip
                type Error = Infallible;

                fn set_high(&mut self) -> Result<(), Self::Error> {
                    // Set the i-th bit of the corresponding GPIO data out register to 1
                    unsafe { (*$GPIOX::ptr()).$gpiox_doutr.modify(|_,w| w.bits(1 << self.i)) };
                    Ok(())
                }

                fn set_low(&mut self) -> Result<(), Self::Error> {
                    // Set the i-th bit of the corresponding GPIO data out register to 0
                    unsafe { (*$GPIOX::ptr()).$gpiox_doutr.modify(|_,w| w.bits(0 << self.i)) };
                    Ok(())
                }
            }

            // All PXx in any `Output` mode can do this
	    impl<MODE> StatefulOutputPin for $PXx<Output<MODE>> {
                fn is_set_high(&self) -> Result<bool, Self::Error> {
                    self.is_set_low().map(|v| !v)
                }

                fn is_set_low(&self) -> Result<bool, Self::Error> {
                    // Check whether the i-th bit of the corresponding GPIO data out register is 0
                    Ok(unsafe { (*$GPIOX::ptr()).$gpiox_doutr.read().bits() & (1 << self.i) == 0 })
                }
            }

            // All PXx in any `Output` mode can do this
            impl<MODE> toggleable::Default for $PXx<Output<MODE>> {}

            // All PXx in any `Input` mode can do this
            impl<MODE> InputPin for $PXx<Input<MODE>> {
                type Error = Infallible;

                fn is_high(&self) -> Result<bool, Self::Error> {
                    self.is_low().map(|v| !v)
                }

                fn is_low(&self) -> Result<bool, Self::Error> {
                    // Check whether the i-th bit of the corresponding GPIO data in register is 0
                    Ok(unsafe { (*$GPIOX::ptr()).$gpiox_dinr.read().bits() & (1 << self.i) == 0 })
                }
            }

            // This is where all pins of this GPIO block as well as the GPIO state
            // machine is actually implemented.
            $(
                /// Pin
                pub struct $PXi<MODE, AF> {
                    _mode: PhantomData<MODE>,
                    _af: PhantomData<AF>
                }

                // These state transitions should be possible for any pin
                impl<MODE, AF> $PXi<MODE, AF> {
                    /// Change the AF to AF0, leave the IO mode alone though
                    pub fn into_alternate_af0(self) -> $PXi<MODE, AF0> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b0000)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF1, leave the IO mode alone though
                    pub fn into_alternate_af1(self) -> $PXi<MODE, AF1> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b0001)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF2, leave the IO mode alone though
                    pub fn into_alternate_af2(self) -> $PXi<MODE, AF2> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b0010)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF3, leave the IO mode alone though
                    pub fn into_alternate_af3(self) -> $PXi<MODE, AF3> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b0011)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF4, leave the IO mode alone though
                    pub fn into_alternate_af4(self) -> $PXi<MODE, AF4> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b0100)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF5, leave the IO mode alone though
                    pub fn into_alternate_af5(self) -> $PXi<MODE, AF5> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b0101)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF6, leave the IO mode alone though
                    pub fn into_alternate_af6(self) -> $PXi<MODE, AF6> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b0110)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF7, leave the IO mode alone though
                    pub fn into_alternate_af7(self) -> $PXi<MODE, AF7> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b0111)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF8, leave the IO mode alone though
                    pub fn into_alternate_af8(self) -> $PXi<MODE, AF8> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b1000)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF9, leave the IO mode alone though
                    pub fn into_alternate_af9(self) -> $PXi<MODE, AF9> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b1001)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF10, leave the IO mode alone though
                    pub fn into_alternate_af10(self) -> $PXi<MODE, AF10> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b1010)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF11, leave the IO mode alone though
                    pub fn into_alternate_af11(self) -> $PXi<MODE, AF11> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b1011)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF12, leave the IO mode alone though
                    pub fn into_alternate_af12(self) -> $PXi<MODE, AF12> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b1100)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF13, leave the IO mode alone though
                    pub fn into_alternate_af13(self) -> $PXi<MODE, AF13> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b1101)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF14, leave the IO mode alone though
                    pub fn into_alternate_af14(self) -> $PXi<MODE, AF14> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b1110)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the AF to AF15, leave the IO mode alone though
                    pub fn into_alternate_af15(self) -> $PXi<MODE, AF15> {
                        // Enable the AFIO APB clock
                        (unsafe { &*CKCU::ptr() }).ckcu_apbccr0.modify(|_, w| w.afioen().set_bit());
                        // Set the AF
                        unsafe { (&*AFIO::ptr()).$afio_gpxcfgr.modify(|_, w| w.$cfgx().bits(0b1111)) };

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the pin to an output pin in push pull mode
                    pub fn into_output_push_pull(self) -> $PXi<Output<PushPull>, AF> {
                        // Set the direction to output
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_dircr.modify(|_, w| w.$dirx().set_bit());
                        // Disable open drain -> implcitly enable push pull
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_odr.modify(|_, w| w.$odx().clear_bit());

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the pin into an output pin in open drain mode
                    pub fn into_output_open_drain(self) -> $PXi<Output<OpenDrain>, AF> {
                        // Set the direction to output
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_dircr.modify(|_, w| w.$dirx().set_bit());
                        // Enable open drain
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_odr.modify(|_, w| w.$odx().set_bit());

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the pin into an input pin in pull up mode
                    pub fn into_input_pull_up(self) -> $PXi<Input<PullUp>, AF> {
                        // Set the direction to input
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_dircr.modify(|_, w| w.$dirx().clear_bit());
                        // Enable pull up
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_pur.modify(|_, w| w.$pux().set_bit());
                        // Enable the input function, this is what allows us to actually
                        // read values from the Schmitt trigger inside the GPIO circuit
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_iner.modify(|_, w| w.$inenx().set_bit());

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the pin into an input pin in pull down mode.
                    pub fn into_input_pull_down(self) -> $PXi<Input<PullDown>, AF> {
                        // Set the direction to input
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_dircr.modify(|_, w| w.$dirx().clear_bit());
                        // According to User Manual page 133 pull up takes priority over pull down,
                        // hence we have to disable it here explicitly
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_pur.modify(|_, w| w.$pux().clear_bit());
                        // Enable pull down
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_pdr.modify(|_, w| w.$pdx().set_bit());
                        // Enable the input function, this is what allows us to actually
                        // read values from the Schmitt trigger inside the GPIO circuit
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_iner.modify(|_, w| w.$inenx().set_bit());

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }

                    /// Change the pin into an input pin in floating mode
                    pub fn into_input_floating(self) -> $PXi<Input<Floating>, AF> {
                        // Set the direction to input
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_dircr.modify(|_, w| w.$dirx().clear_bit());
                        // Disable pull up
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_pur.modify(|_, w| w.$pux().clear_bit());
                        // Disable pull down
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_pdr.modify(|_, w| w.$pdx().clear_bit());
                        // Enable the input function, this is what allows us to actually
                        // read values from the Schmitt trigger inside the GPIO circuit
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_iner.modify(|_, w| w.$inenx().set_bit());

                        $PXi { _mode: PhantomData, _af: PhantomData }
                    }
                }

                impl<OUTPUT, AF> $PXi<Output<OUTPUT>, AF> {
                    pub fn set_output_drive_current(&mut self, current: GpioCurrent) {
                        unsafe { (*$GPIOX::ptr()).$gpiox_drvr.modify(|_, w| w.$dvx().bits(current.to_bits())) }
                    }
                }

                impl<MODE, AF> $PXi<MODE, AF> {
                    /// Erases the pin number from the type
                    ///
                    /// This is useful when you want to collect the pins into an array where you
                    /// need all the elements to have the same type
                    pub fn downgrade(self) -> $PXx<MODE> {
                        $PXx {
                            i: $i,
                            _mode: self._mode,
                        }
                    }
                }

                impl<OUTPUT, AF> OutputPin for $PXi<Output<OUTPUT>, AF> {
                    type Error = Infallible;

                    fn set_high(&mut self) -> Result<(), Self::Error> {
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_doutr.modify(|_,w| w.$doutx().set_bit());
                        Ok(())
                    }

                    fn set_low(&mut self) -> Result<(), Self::Error> {
                        (unsafe { &*$GPIOX::ptr() }).$gpiox_doutr.modify(|_,w| w.$doutx().clear_bit());
                        Ok(())
                    }
                }

                impl<OUTPUT, AF> StatefulOutputPin for $PXi<Output<OUTPUT>, AF> {
                    fn is_set_high(&self) -> Result<bool, Self::Error> {
                        self.is_set_low().map(|v| !v)
                    }

                    fn is_set_low(&self) -> Result<bool, Self::Error> {
                        Ok((unsafe { &*$GPIOX::ptr() }).$gpiox_doutr.read().$doutx().bit_is_clear())
                    }
                }

                impl<OUTPUT, AF> toggleable::Default for $PXi<Output<OUTPUT>, AF> {}

                impl<INPUT, AF> InputPin for $PXi<Input<INPUT>, AF> {
                    type Error = Infallible;

                    fn is_high(&self) -> Result<bool, Self::Error> {
                        self.is_low().map(|v| !v)
                    }

                    fn is_low(&self) -> Result<bool, Self::Error> {
                        Ok((unsafe { &*$GPIOX::ptr() }).$gpiox_dinr.read().$dinx().bit_is_clear())
                    }
                }
            )+
        }
    }
}

#[cfg(any(feature = "ht32f52342_52"))]
gpio!(GPIOA, gpioa, PA, parst, paen, gpioa_doutr, gpioa_dinr, gpioa_drvr, gpioa_dircr, gpioa_pur, gpioa_pdr, gpioa_iner, gpioa_odr, [
    PA0: (pa0, 0, Input<Disabled>, AF0, dout0, din0, dv0, dir0, pu0, pd0, inen0, od0, cfg0, afio_gpacfglr),
    PA1: (pa1, 1, Input<Disabled>, AF0, dout1, din1, dv1, dir1, pu1, pd1, inen1, od1, cfg1, afio_gpacfglr),
    PA2: (pa2, 2, Input<Disabled>, AF0, dout2, din2, dv2, dir2, pu2, pd2, inen2, od2, cfg2, afio_gpacfglr),
    PA3: (pa3, 3, Input<Disabled>, AF0, dout3, din3, dv3, dir3, pu3, pd3, inen3, od3, cfg3, afio_gpacfglr),
    PA4: (pa4, 4, Input<Disabled>, AF0, dout4, din4, dv4, dir4, pu4, pd4, inen4, od4, cfg4, afio_gpacfglr),
    PA5: (pa5, 5, Input<Disabled>, AF0, dout5, din5, dv5, dir5, pu5, pd5, inen5, od5, cfg5, afio_gpacfglr),
    PA6: (pa6, 6, Input<Disabled>, AF0, dout6, din6, dv6, dir6, pu6, pd6, inen6, od6, cfg6, afio_gpacfglr),
    PA7: (pa7, 7, Input<Disabled>, AF0, dout7, din7, dv7, dir7, pu7, pd7, inen7, od7, cfg7, afio_gpacfglr),
    // Refer to User Manual page128 for the default state
    PA8: (pa8, 8, Input<PullUp>, AF0, dout8, din8, dv8, dir8, pu8, pd8, inen8, od8, cfg8, afio_gpacfghr),
    // Refer to User Manual page128 for the default state
    PA9: (pa9, 9, Input<PullUp>, AF0, dout9, din9, dv9, dir9, pu9, pd9, inen9, od9, cfg9, afio_gpacfghr),
    PA10: (pa10, 10, Input<Disabled>, AF0, dout10, din10, dv10, dir10, pu10, pd10, inen10, od10, cfg10, afio_gpacfghr),
    PA11: (pa11, 11, Input<Disabled>, AF0, dout11, din11, dv11, dir11, pu11, pd11, inen11, od11, cfg11, afio_gpacfghr),
    // SWCLK
    PA12: (pa12, 12, Input<PullUp>, AF0, dout12, din12, dv12, dir12, pu12, pd12, inen12, od12, cfg12, afio_gpacfghr),
    // SWDIO
    PA13: (pa13, 13, Input<PullUp>, AF0, dout13, din13, dv13, dir13, pu13, pd13, inen13, od13, cfg13, afio_gpacfghr),
    PA14: (pa14, 14, Input<Disabled>, AF0, dout14, din14, dv14, dir14, pu14, pd14, inen14, od14, cfg14, afio_gpacfghr),
    PA15: (pa15, 15, Input<Disabled>, AF0, dout15, din15, dv15, dir15, pu15, pd15, inen15, od15, cfg15, afio_gpacfghr),
]);

#[cfg(any(feature = "ht32f52342_52"))]
gpio!(GPIOB, gpiob, PB, pbrst, pben, gpiob_doutr, gpiob_dinr, gpiob_drvr, gpiob_dircr, gpiob_pur, gpiob_pdr, gpiob_iner, gpiob_odr, [
    PB0: (pb0, 0, Input<Disabled>, AF0, dout0, din0, dv0, dir0, pu0, pd0, inen0, od0, cfg0, afio_gpbcfglr),
    PB1: (pb1, 1, Input<Disabled>, AF0, dout1, din1, dv1, dir1, pu1, pd1, inen1, od1, cfg1, afio_gpbcfglr),
    PB2: (pb2, 2, Input<Disabled>, AF0, dout2, din2, dv2, dir2, pu2, pd2, inen2, od2, cfg2, afio_gpbcfglr),
    PB3: (pb3, 3, Input<Disabled>, AF0, dout3, din3, dv3, dir3, pu3, pd3, inen3, od3, cfg3, afio_gpbcfglr),
    PB4: (pb4, 4, Input<Disabled>, AF0, dout4, din4, dv4, dir4, pu4, pd4, inen4, od4, cfg4, afio_gpbcfglr),
    PB5: (pb5, 5, Input<Disabled>, AF0, dout5, din5, dv5, dir5, pu5, pd5, inen5, od5, cfg5, afio_gpbcfglr),
    PB6: (pb6, 6, Input<Disabled>, AF0, dout6, din6, dv6, dir6, pu6, pd6, inen6, od6, cfg6, afio_gpbcfglr),
    PB7: (pb7, 7, Input<Disabled>, AF0, dout7, din7, dv7, dir7, pu7, pd7, inen7, od7, cfg7, afio_gpbcfglr),
    PB8: (pb8, 8, Input<Disabled>, AF0, dout8, din8, dv8, dir8, pu8, pd8, inen8, od8, cfg8, afio_gpbcfghr),
    PB9: (pb9, 9, Input<Disabled>, AF0, dout9, din9, dv9, dir9, pu9, pd9, inen9, od9, cfg9, afio_gpbcfghr),
    PB10: (pb10, 10, Input<Disabled>, AF0, dout10, din10, dv10, dir10, pu10, pd10, inen10, od10, cfg10, afio_gpbcfghr),
    PB11: (pb11, 11, Input<Disabled>, AF0, dout11, din11, dv11, dir11, pu11, pd11, inen11, od11, cfg11, afio_gpbcfghr),
    PB12: (pb12, 12, Input<Disabled>, AF0, dout12, din12, dv12, dir12, pu12, pd12, inen12, od12, cfg12, afio_gpbcfghr),
    PB13: (pb13, 13, Input<Disabled>, AF0, dout13, din13, dv13, dir13, pu13, pd13, inen13, od13, cfg13, afio_gpbcfghr),
    PB14: (pb14, 14, Input<Disabled>, AF0, dout14, din14, dv14, dir14, pu14, pd14, inen14, od14, cfg14, afio_gpbcfghr),
    PB15: (pb15, 15, Input<Disabled>, AF0, dout15, din15, dv15, dir15, pu15, pd15, inen15, od15, cfg15, afio_gpbcfghr),
]);

#[cfg(any(feature = "ht32f52342_52"))]
gpio!(GPIOC, gpioc, PC, pcrst, pcen, gpioc_doutr, gpioc_dinr, gpioc_drvr, gpioc_dircr, gpioc_pur, gpioc_pdr, gpioc_iner, gpioc_odr, [
    PC0: (pc0, 0, Input<Disabled>, AF0, dout0, din0, dv0, dir0, pu0, pd0, inen0, od0, cfg0, afio_gpccfglr),
    PC1: (pc1, 1, Input<Disabled>, AF0, dout1, din1, dv1, dir1, pu1, pd1, inen1, od1, cfg1, afio_gpccfglr),
    PC2: (pc2, 2, Input<Disabled>, AF0, dout2, din2, dv2, dir2, pu2, pd2, inen2, od2, cfg2, afio_gpccfglr),
    PC3: (pc3, 3, Input<Disabled>, AF0, dout3, din3, dv3, dir3, pu3, pd3, inen3, od3, cfg3, afio_gpccfglr),
    PC4: (pc4, 4, Input<Disabled>, AF0, dout4, din4, dv4, dir4, pu4, pd4, inen4, od4, cfg4, afio_gpccfglr),
    PC5: (pc5, 5, Input<Disabled>, AF0, dout5, din5, dv5, dir5, pu5, pd5, inen5, od5, cfg5, afio_gpccfglr),
    PC6: (pc6, 6, Input<Disabled>, AF0, dout6, din6, dv6, dir6, pu6, pd6, inen6, od6, cfg6, afio_gpccfglr),
    PC7: (pc7, 7, Input<Disabled>, AF0, dout7, din7, dv7, dir7, pu7, pd7, inen7, od7, cfg7, afio_gpccfglr),
    PC8: (pc8, 8, Input<Disabled>, AF0, dout8, din8, dv8, dir8, pu8, pd8, inen8, od8, cfg8, afio_gpccfghr),
    PC9: (pc9, 9, Input<Disabled>, AF0, dout9, din9, dv9, dir9, pu9, pd9, inen9, od9, cfg9, afio_gpccfghr),
    PC10: (pc10, 10, Input<Disabled>, AF0, dout10, din10, dv10, dir10, pu10, pd10, inen10, od10, cfg10, afio_gpccfghr),
    PC11: (pc11, 11, Input<Disabled>, AF0, dout11, din11, dv11, dir11, pu11, pd11, inen11, od11, cfg11, afio_gpccfghr),
    PC12: (pc12, 12, Input<Disabled>, AF0, dout12, din12, dv12, dir12, pu12, pd12, inen12, od12, cfg12, afio_gpccfghr),
    PC13: (pc13, 13, Input<Disabled>, AF0, dout13, din13, dv13, dir13, pu13, pd13, inen13, od13, cfg13, afio_gpccfghr),
    PC14: (pc14, 14, Input<Disabled>, AF0, dout14, din14, dv14, dir14, pu14, pd14, inen14, od14, cfg14, afio_gpccfghr),
    PC15: (pc15, 15, Input<Disabled>, AF0, dout15, din15, dv15, dir15, pu15, pd15, inen15, od15, cfg15, afio_gpccfghr),
]);

// Block D only has 4 pins
#[cfg(any(feature = "ht32f52342_52"))]
gpio!(GPIOD, gpiod, PD, pdrst, pden, gpiod_doutr, gpiod_dinr, gpiod_drvr, gpiod_dircr, gpiod_pur, gpiod_pdr, gpiod_iner, gpiod_odr, [
    PD0: (pd0, 0, Input<Disabled>, AF0, dout0, din0, dv0, dir0, pu0, pd0, inen0, od0, cfg0, afio_gpdcfglr),
    PD1: (pd1, 1, Input<Disabled>, AF0, dout1, din1, dv1, dir1, pu1, pd1, inen1, od1, cfg1, afio_gpdcfglr),
    PD2: (pd2, 2, Input<Disabled>, AF0, dout2, din2, dv2, dir2, pu2, pd2, inen2, od2, cfg2, afio_gpdcfglr),
    PD3: (pd3, 3, Input<Disabled>, AF0, dout3, din3, dv3, dir3, pu3, pd3, inen3, od3, cfg3, afio_gpdcfglr),
]);
