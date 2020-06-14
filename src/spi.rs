//! Serial Peripheral Interface (SPI) bus
use crate::time::Hertz;
pub use crate::hal::spi::{
    Mode, Phase, Polarity, MODE_0, MODE_1, MODE_2, MODE_3,
};
use crate::hal;
use crate::ht32::{SPI0, SPI1, CKCU, RSTCU};
use crate::ckcu::Clocks;
use crate::gpio::{
    Output, Input, AF5, PushPull, Floating,
    gpioa::{PA0, PA1, PA2, PA4, PA5, PA6, PA9, PA11, PA15},
    gpiob::{PB0, PB1, PB3, PB4, PB5, PB6},
    gpioc::{PC0, PC2, PC3, PC5, PC8, PC9, PC11, PC12, PC13},
};
use core::marker::PhantomData;
use core::convert::TryInto;
use core::ptr;

#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    /// Overrun occurred
    Overrun,
    /// Write Collision occured
    WriteCollision,
}

pub trait PinSck<SPI> {}
pub trait PinMiso<SPI> {}
pub trait PinMosi<SPI> {}

#[derive(Debug)]
pub struct Spi<SPI, WORD = u8> {
    spi: SPI,
    _word: PhantomData<WORD>,
}

pub trait SpiExt<SPI, WORD>: Sized {
    fn spi<SCK, MISO, MOSI, F>(
        self,
	sck: SCK,
	miso: MISO,
	mosi: MOSI,
        mode: Mode,
        freq: F,
        clocks: &Clocks,
    ) -> Spi<SPI, WORD>
    where
	SCK: PinSck<SPI>,
	MISO: PinMiso<SPI>,
	MOSI: PinMosi<SPI>,
        F: Into<Hertz>;

    fn spi_unchecked<F>(
        self,
        mode: Mode,
        freq: F,
        clocks: &Clocks,
    ) -> Spi<SPI, WORD>
    where
        F: Into<Hertz>;
}

macro_rules! spi {
    ($($SPIX:ident: ($spiX:ident, $spiXen:ident, $spiXrst:ident) => ($($WORD:ident),+),)+) => {
        $(
            $(
                impl Spi<$SPIX, $WORD> {
                    fn $spiX<F>(
                        spi: $SPIX,
                        mode: Mode,
                        freq: F,
                        clocks: &Clocks,
                    ) -> Spi<$SPIX, $WORD>
                    where
                        F: Into<Hertz>
                    {
                        let rstcu = unsafe { &*RSTCU::ptr() };
                        let ckcu = unsafe { &*CKCU::ptr() };
                        // reset the SPI port before using it
                        rstcu.rstcu_apbprstr0.modify(|_, w| w.$spiXrst().set_bit());
                        // enable the AHB clock for the SPI port
                        ckcu.ckcu_apbccr0.modify(|_, w| w.$spiXen().set_bit());

                        // The values for the format register can be found at
                        // User Manual page 489, they follow this pattern
                        // from left to right:
                        // 1st bit = CPOL
                        // 2nd bit = CPOL ^ CPHA
                        // 3rd bit = !(CPOL ^ CPHA)
                        let cpol = (mode.polarity == Polarity::IdleHigh) as u8;
                        let cpha = (mode.phase == Phase::CaptureOnSecondTransition) as u8;
                        let mode =
                            (cpol << 2) |
                            ((cpol ^ cpha) << 1) |
                            (!(cpol ^ cpha));

                        spi.spi_cr1.modify(|_, w| unsafe {
                            w.mode().
                                // master mode
                                set_bit()
                                .selm().
                                // software SS
                                clear_bit().
                                firstbit().
                                // MSB first
                                clear_bit().
                                format().
                                bits(mode).
                                dfl().
                                // data frame length
                                bits((core::mem::size_of::<$WORD>()*8).try_into().unwrap())
                        });

                        // f_sck = f_pclk / (2 *  (CP + 1)) according to User Manual page 491
                        // -> CP = (f_pclk / (2 * f_sck)) - 1
                        // for pclk = hclk
                        let freq = freq.into();
                        let spi_div = (clocks.hclk.0 / (2 * freq.0)) - 1;
                        assert!(spi_div <= 65535);

                        spi.spi_cpr.write(|w| unsafe { w.cp().bits(spi_div as u16) });

                        // Select pin output enable
                        // This causes the chip to not mode fault all the time
                        // when it's not in a multi master setup.
                        spi.spi_cr0.modify(|_, w| w.seloen().set_bit());

                        spi.spi_cr0.modify(|_, w| w.spien().set_bit());
                        Spi { spi, _word: PhantomData }
                    }

                    pub fn free(self) -> $SPIX {
                        self.spi
                    }
                }

                impl SpiExt<$SPIX, $WORD> for $SPIX {
	            fn spi<SCK, MISO, MOSI, F>(
                        self,
                	_sck: SCK,
                	_miso: MISO,
                	_mosi: MOSI,
                        mode: Mode,
                        freq: F,
                        clocks: &Clocks,
                    ) -> Spi<$SPIX, $WORD>
                    where
                	SCK: PinSck<$SPIX>,
                	MISO: PinMiso<$SPIX>,
                	MOSI: PinMosi<$SPIX>,
                        F: Into<Hertz>
                    {
	                Spi::<$SPIX, $WORD>::$spiX(self, mode, freq, clocks)
	            }

	            fn spi_unchecked<F>(
                        self,
                        mode: Mode,
                        freq: F,
                        clocks: &Clocks,
                    ) -> Spi<$SPIX, $WORD>
                    where
                        F: Into<Hertz>
                    {
	                Spi::<$SPIX, $WORD>::$spiX(self, mode, freq, clocks)
	            }
	        }

                impl hal::spi::FullDuplex<$WORD> for Spi<$SPIX, $WORD> {
                    type Error = Error;

                    fn read(&mut self) -> nb::Result<$WORD, Error> {
                        let sr = self.spi.spi_sr.read();

                        Err(if sr.ro().bit_is_set() {
                            nb::Error::Other(Error::Overrun)
                        }
                        else if sr.wc().bit_is_set() {
                            nb::Error::Other(Error::WriteCollision)
                        }
                        else if sr.rxbne().bit_is_set() {
                            return Ok(unsafe {
                                    ptr::read_volatile(
                                        &self.spi.spi_dr as *const _ as *const $WORD,
                                    )
                                }
                            )
                        }
                        else {
                            nb::Error::WouldBlock
                        })
                    }

                    fn send(&mut self, byte: $WORD) -> nb::Result<(), Error> {
                        let sr = self.spi.spi_sr.read();

                        Err(if sr.ro().bit_is_set() {
                            nb::Error::Other(Error::Overrun)
                        }
                        else if sr.wc().bit_is_set() {
                            nb::Error::Other(Error::WriteCollision)
                        }
                        else if sr.busy().bit_is_set() {
                            nb::Error::WouldBlock
                        }
                        else {
                            unsafe {
                                ptr::write_volatile(
                                    &self.spi.spi_dr as *const _ as *mut $WORD,
                                    byte,
                                )
                            }
                            return Ok(());
                        })
                    }
                }

                impl hal::blocking::spi::transfer::Default<$WORD>
                    for Spi<$SPIX, $WORD> {}

                impl hal::blocking::spi::write::Default<$WORD>
                    for Spi<$SPIX, $WORD> {}
            )+
        )+
    }
}

macro_rules! pins {
    ($($SPIX:ty: SCK: [$($SCK:ty),*] MISO: [$($MISO:ty),*] MOSI: [$($MOSI:ty),*])+) => {
        $(
            $(
                impl PinSck<$SPIX> for $SCK {}
            )*
            $(
                impl PinMiso<$SPIX> for $MISO {}
            )*
            $(
                impl PinMosi<$SPIX> for $MOSI {}
            )*
        )+
    }
}

spi! {
    SPI0: (spi0, spi0en, spi0rst) => (u8, u16),
    SPI1: (spi1, spi1en, spi1rst) => (u8, u16),
}

pins! {
    SPI0:
        SCK: [
            PA4<Output<PushPull>, AF5>,
            PC0<Output<PushPull>, AF5>,
            PB3<Output<PushPull>, AF5>
        ]
        MISO: [
            PA6<Input<Floating>, AF5>,
            PA11<Input<Floating>, AF5>,
            PB5<Input<Floating>, AF5>
        ]
        MOSI: [
            PA5<Output<PushPull>, AF5>,
            PA9<Output<PushPull>, AF5>,
            PB4<Output<PushPull>, AF5>
        ]
    SPI1:
        SCK: [
            PA0<Output<PushPull>, AF5>,
            PC5<Output<PushPull>, AF5>,
            PC11<Output<PushPull>, AF5>,
            PA15<Output<PushPull>, AF5>,
            PC2<Output<PushPull>, AF5>
        ]
        MISO: [
            PA2<Input<Floating>, AF5>,
            PC9<Input<Floating>, AF5>,
            PC13<Input<Floating>, AF5>,
            PB1<Input<Floating>, AF5>,
            PB6<Input<Floating>, AF5>
        ]
        MOSI: [
            PA1<Output<PushPull>, AF5>,
            PC8<Output<PushPull>, AF5>,
            PC12<Output<PushPull>, AF5>,
            PB0<Output<PushPull>, AF5>,
            PC3<Output<PushPull>, AF5>
        ]

}
