//! Inter Integrated Circuit implementation
use crate::ckcu::Clocks;
use crate::time::Hertz;
use crate::hal::blocking::i2c::{Read, Write, WriteRead};
use crate::ht32::{I2C0, I2C1, CKCU, RSTCU};
use core::convert::TryInto;
use crate::time::U32Ext;
use crate::gpio::{
    Output, OpenDrain, AF7,
    gpioa::{PA0, PA1, PA4, PA5, PA14, PA15},
    gpiob::{PB0, PB1, PB7, PB8, PB15},
    gpioc::{PC0, PC4, PC5, PC6, PC7, PC12, PC13, PC14, PC15},
    gpiod::PD0
};

#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    /// Arbitration error
    Arbitration,
    /// Bus error
    Bus,
    /// The slave didn't send ACK
    NotAcknowledge,
}

pub trait PinScl<I2C> {}

pub trait PinSda<I2C> {}

#[derive(Debug)]
pub struct I2c<I2C> {
    i2c: I2C
}

pub trait I2cExt<I2C>: Sized {
    fn i2c<SCL, SDA, F>(
        self,
        scl: SCL,
        sda: SDA,
        freq: F,
        clocks: &Clocks
    ) -> I2c<I2C>
    where
        SCL: PinScl<I2C>,
        SDA: PinSda<I2C>,
        F: Into<Hertz>;

    fn i2c_unchecked<F>(
        self,
        freq: F,
        clocks: &Clocks
    ) -> I2c<I2C>
    where
        F: Into<Hertz>;
}

macro_rules! busy_wait {
    ($i2c:expr, $field:ident, $variant:ident) => {
        loop {
            let status = $i2c.i2c_sr.read();

            if status.$field().$variant() {
                break;
            }
            else if status.arblos().bit_is_set() {
                return Err(Error::Arbitration)
            }
            else if status.rxnack().bit_is_set() {
                return Err(Error::NotAcknowledge)
            }
            else if status.buserr().bit_is_set() {
                return Err(Error::Bus)
            }
            else {
                // no error
            }
        }
    }
}

macro_rules! i2c {
    ($($I2CX:ident: ($i2cX:ident, $i2cXen:ident, $i2cXrst:ident),)+) => {
        $(
            impl I2c<$I2CX> {
                /// Creates a new I2C peripheral
                pub fn $i2cX<F>(
                    i2c: $I2CX,
                    freq: F,
                    clocks: &Clocks,
                ) -> Self where
                    F: Into<Hertz>
                {
                    let freq = freq.into();

                    assert!(freq <= 1.mhz().into());

                    // SCL_low = 1/pclk * (SLPG + d)
                    // SCL_high = 1/pclk * (SHPG + d)
                    // where d = 6
                    // T_SCL = SCL_low + SCL_high
                    //
                    // Refer to User Manual page 470 and 471
                    // Note that PCLK = HCLK in this case as we didn't choose as
                    // PCLK divider
                    let (shpg, slpg) = if freq > 100.khz().into() {
                        // We are in Fast-mode or Fast-mode Plus, this means
                        // SCL_low = 2 * SCL_high, refer to I2C spec page 48
                        // -> SCL_low = 2/3 SCL
                        // -> SLPG = (2 * PCLK ) / (3 * SCL) - 6
                        let slpg = ((2 * clocks.hclk.0) / (3 * freq.0)) - 6;

                        // 1/pclk * ( SLPG + d ) = 2/pclk * (SHPG + d)
                        // -> SHPG = (SLPG - d)/2
                        // + 1 serves as a correction factor so SCL gets slower
                        // rather than larger as freq
                        let shpg = ((slpg - 6) / 2) + 1;
                        (shpg, slpg)
                    } else {
                        // We are in Standard mode, this means
                        // SCL_low = SCL_high, refer to I2C spec page 48
                        // -> SLPG = SHPG = pclk / (2*SCL) - 6
                        let scl_div = ((clocks.hclk.0) / (2 * freq.0)) - 6;
                        (scl_div, scl_div)
                    };

                    let rstcu = unsafe { &*RSTCU::ptr() };
                    let ckcu = unsafe { &*CKCU::ptr() };
                    // reset the I2C port before using it
                    rstcu.rstcu_apbprstr0.modify(|_, w| w.$i2cXrst().set_bit());
                    // enable the AHB clock for the I2C port
                    ckcu.ckcu_apbccr0.modify(|_, w| w.$i2cXen().set_bit());

                    // Configure the SCL clock values
                    i2c.i2c_shpgr.modify(|_, w| unsafe { w.shpg().bits(shpg.try_into().unwrap()) });
                    i2c.i2c_slpgr.modify(|_, w| unsafe { w.slpg().bits(slpg.try_into().unwrap()) });
                    // Enable the I2C port
                    i2c.i2c_cr.modify(|_, w| w.i2cen().set_bit());
                    I2c { i2c }
                }

                pub fn free(self) -> $I2CX {
                    self.i2c
                }
            }

            impl I2cExt<$I2CX> for $I2CX {
	    		fn i2c<SCL, SDA, F>(
                    self,
                    _scl: SCL,
                    _sda: SDA,
                    freq: F,
                    clocks: &Clocks
                ) -> I2c<$I2CX>
                where
                    SCL: PinScl<$I2CX>,
                    SDA: PinSda<$I2CX>,
                    F: Into<Hertz>
                {
                    I2c::$i2cX(self, freq, clocks)
                }

                fn i2c_unchecked<F>(
                    self,
                    freq: F,
                    clocks: &Clocks
                ) -> I2c<$I2CX>
                where
                    F: Into<Hertz>
                {
                    I2c::$i2cX(self, freq, clocks)
                }
            }

            impl Write for I2c<$I2CX> {
                type Error = Error;
                fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Error> {
                    // Refer to User Manual page 454 for details regarding this
                    // function
                    self.i2c.i2c_tar.modify(|_, w| unsafe {
                        w.rwd()
                            // Direction, write to slave
                            .clear_bit()
                            // Set slave address
                            .tar()
                            .bits((addr << 1) as u16)
                    });

                    // wait for the start to be sent
                    busy_wait!(self.i2c, sta, bit_is_set);
                    // wait for the address frame to be sent and ACKed
                    busy_wait!(self.i2c, adrs, bit_is_set);

                    for byte in bytes {
                        // wait for the byte to be sent and acked
                        busy_wait!(self.i2c, txde, bit_is_clear);
                        // send the byte
                        self.i2c.i2c_dr.write(|w| unsafe { w.data().bits(*byte) });
                    }

                    // send the STOP
                    self.i2c.i2c_cr.modify(|_, w| w.stop().set_bit());

                    Ok(())
                }
            }

            impl Read for I2c<$I2CX> {
                type Error = Error;
                fn read(&mut self, addr: u8, buffer: &mut [u8],) -> Result<(), Error> {
                    // Refer to User Manual page 455 for details regarding this
                    // function
                    self.i2c.i2c_tar.modify(|_, w| unsafe {
                        w.rwd()
                            // Direction, read from slave
                            .set_bit()
                            // Set slave address with read bit
                            .tar()
                            .bits(((addr << 1) | 1) as u16)
                    });

                    // wait for the start to be sent
                    busy_wait!(self.i2c, sta, bit_is_set);
                    // wait for the address frame to be sent and ACKed
                    busy_wait!(self.i2c, adrs, bit_is_set);

                    for byte in buffer {
                        // wait until we received data
                        busy_wait!(self.i2c, rxdne, bit_is_set);

                        *byte = self.i2c.i2c_dr.read().data().bits();
                    }

                    // send the STOP
                    self.i2c.i2c_cr.modify(|_, w| w.stop().set_bit());

                    Ok(())
                }
            }

            impl WriteRead for I2c<$I2CX> {
                type Error = Error;
		fn write_read(
                    &mut self,
                    addr: u8,
                    bytes: &[u8],
                    buffer: &mut [u8],
                ) -> Result<(), Error> {
                    // Refer to User Manual page 454 for details regarding this
                    // part of the function
                    self.i2c.i2c_tar.modify(|_, w| unsafe {
                        w.rwd()
                            // Direction, write to slave
                            .clear_bit()
                            // Set slave address
                            .tar()
                            .bits((addr << 1) as u16)
                    });

                    // wait for the start to be sent
                    busy_wait!(self.i2c, sta, bit_is_set);
                    // wait for the address frame to be sent and ACKed
                    busy_wait!(self.i2c, adrs, bit_is_set);

                    for byte in bytes {
                        // wait for the byte to be sent and acked
                        busy_wait!(self.i2c, txde, bit_is_clear);
                        // send the byte
                        self.i2c.i2c_dr.write(|w| unsafe { w.data().bits(*byte) });
                    }

                    // unlike write we explicitly don't send a stop here as
                    // this function is only a single I2C transaction

                    // Refer to User Manual page 455 for details regarding this
                    // part function
                    self.i2c.i2c_tar.modify(|_, w| unsafe {
                        w.rwd()
                            // Direction, read from slave
                            .set_bit()
                            // Set slave address with read bit
                            .tar()
                            .bits(((addr << 1) | 1) as u16)
                    });

                    // wait for the start to be sent
                    busy_wait!(self.i2c, sta, bit_is_set);
                    // wait for the address frame to be sent and ACKed
                    busy_wait!(self.i2c, adrs, bit_is_set);

                    for byte in buffer {
                        // wait until we received data
                        busy_wait!(self.i2c, rxdne, bit_is_set);

                        *byte = self.i2c.i2c_dr.read().data().bits();
                    }

                    // send the STOP
                    self.i2c.i2c_cr.modify(|_, w| w.stop().set_bit());

                    Ok(())
                }
            }
        )+
    }
}

macro_rules! pins {
    ($($I2CX:ty: SCL: [$($SCL:ty),*] SDA: [$($SDA:ty),*])+) => {
        $(
            $(
                impl PinScl<$I2CX> for $SCL {}
            )*
            $(
                impl PinSda<$I2CX> for $SDA {}
            )*
        )+
    }
}

i2c! {
    I2C0: (i2c0, i2c0en, i2c0rst),
    I2C1: (i2c1, i2c0en, i2c1rst),
}

pins! {
    I2C0:
        SCL: [
            PA4<Output<OpenDrain>, AF7>,
            PC6<Output<OpenDrain>, AF7>,
            PC12<Output<OpenDrain>, AF7>,
            PB0<Output<OpenDrain>, AF7>,
            PC14<Output<OpenDrain>, AF7>
        ]

        SDA: [
            PA5<Output<OpenDrain>, AF7>,
            PC7<Output<OpenDrain>, AF7>,
            PD0<Output<OpenDrain>, AF7>,
            PC13<Output<OpenDrain>, AF7>,
            PB1<Output<OpenDrain>, AF7>,
            PC15<Output<OpenDrain>, AF7>
        ]

    I2C1:
        SCL: [
            PA0<Output<OpenDrain>, AF7>,
            PC4<Output<OpenDrain>, AF7>,
            PB15<Output<OpenDrain>, AF7>,
            PA14<Output<OpenDrain>, AF7>,
            PB7<Output<OpenDrain>, AF7>
        ]

        SDA: [
            PA1<Output<OpenDrain>, AF7>,
            PC5<Output<OpenDrain>, AF7>,
            PC0<Output<OpenDrain>, AF7>,
            PA15<Output<OpenDrain>, AF7>,
            PB8<Output<OpenDrain>, AF7>
        ]
}
