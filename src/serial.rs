//! Serial bus UART and USART
use crate::ckcu::Clocks;
use crate::gpio::{
    gpioa::{PA10, PA14, PA15, PA2, PA3, PA4, PA5, PA8},
    gpiob::{PB0, PB1, PB15, PB2, PB3, PB4, PB5, PB6, PB8},
    gpioc::{PC0, PC1, PC12, PC13, PC3, PC4, PC5, PC6, PC7},
    Floating, Input, Output, PushPull, AF6,
};
use crate::hal::blocking::serial as serial_block;
use crate::hal::serial;
use crate::hal::serial::Write;
use crate::ht32::{CKCU, RSTCU, UART0, UART1, USART0, USART1};
use core::convert::Infallible;
use core::marker::PhantomData;
use core::ptr;
use nb::block;

#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    Framing,
    Parity,
    Overrun,
}

#[derive(Debug)]
pub enum Event {
    FramingError,
    ParityError,
    OverrunError,
    TransmitComplete,
    TransmitRegisterEmpty,
    ReceiveDataReady,
}

pub trait PinTx<SERIAL> {}
pub trait PinRx<SERIAL> {}

#[derive(Debug)]
pub struct Serial<SERIAL, WORD = u8> {
    serial: SERIAL,
    _word: PhantomData<WORD>,
}

#[derive(Debug)]
pub struct Tx<SERIAL, WORD> {
    _serial: PhantomData<SERIAL>,
    _word: PhantomData<WORD>,
}

#[derive(Debug)]
pub struct Rx<SERIAL, WORD> {
    _serial: PhantomData<SERIAL>,
    _word: PhantomData<WORD>,
}

pub mod config {
    use crate::time::Bps;
    use crate::time::U32Ext;

    pub enum WordLength {
        /// 1 word = 7 bit
        DataBits7,
        /// 1 word = 8 bit
        DataBits8,
        /// 1 word = 9 bit
        DataBits9,
    }

    pub enum Parity {
        /// No parity bit
        ParityNone,
        /// Even parity bit
        ParityEven,
        /// Odd parity bit
        ParityOdd,
    }

    pub enum StopBits {
        /// 1 stop bit
        STOP1,
        /// 2 stop bits
        STOP2,
    }

    pub struct Config {
        pub baudrate: Bps,
        pub wordlength: WordLength,
        pub parity: Parity,
        pub stopbits: StopBits,
    }

    impl Config {
        pub fn baudrate(mut self, baudrate: Bps) -> Self {
            self.baudrate = baudrate;
            self
        }

        pub fn parity_none(mut self) -> Self {
            self.parity = Parity::ParityNone;
            self
        }

        pub fn parity_even(mut self) -> Self {
            self.parity = Parity::ParityEven;
            self
        }

        pub fn parity_odd(mut self) -> Self {
            self.parity = Parity::ParityOdd;
            self
        }

        pub fn wordlength_7(mut self) -> Self {
            self.wordlength = WordLength::DataBits7;
            self
        }

        pub fn wordlength_8(mut self) -> Self {
            self.wordlength = WordLength::DataBits8;
            self
        }

        pub fn wordlength_9(mut self) -> Self {
            self.wordlength = WordLength::DataBits9;
            self
        }

        pub fn stopbits(mut self, stopbits: StopBits) -> Self {
            self.stopbits = stopbits;
            self
        }
    }

    #[derive(Debug)]
    pub enum InvalidConfig {
        /// Thrown if the word length in the config does not match the word length
        /// in the type
        WordLengthMismatch,
    }

    impl Default for Config {
        fn default() -> Config {
            let baudrate = 9600u32.bps();
            Config {
                baudrate,
                wordlength: WordLength::DataBits8,
                parity: Parity::ParityNone,
                stopbits: StopBits::STOP1,
            }
        }
    }
}

pub trait SerialExt<SERIAL, WORD> {
    fn serial<TX, RX>(
        self,
        _tx: TX,
        _rx: RX,
        config: config::Config,
        clocks: &Clocks,
    ) -> Result<Serial<SERIAL, WORD>, config::InvalidConfig>
    where
        TX: PinTx<SERIAL>,
        RX: PinRx<SERIAL>;

    fn serial_unchecked(
        self,
        config: config::Config,
        clocks: &Clocks,
    ) -> Result<Serial<SERIAL, WORD>, config::InvalidConfig>;
}

macro_rules! serial {
    ($($SERIALX:ident: ($serialX:ident, $serialXen:ident, $serialXrst:ident, $serial_cr:ident, $serial_dlr:ident, $serial_sifr:ident, $serial_dr:ident, $serial_ier:ident) => ($($WORD:ident),+),)+) => {
        $(
            $(
                impl Serial<$SERIALX, $WORD> {
                    fn $serialX<>(
                        serial: $SERIALX,
                        config: config::Config,
                        clocks: &Clocks,
                    ) -> Result<Serial<$SERIALX, $WORD>, config::InvalidConfig>
                    {
                        let rstcu = unsafe { &*RSTCU::ptr() };
                        let ckcu = unsafe { &*CKCU::ptr() };

                        // reset the serial port before using it
                        rstcu.rstcu_apbprstr0.modify(|_, w| w.$serialXrst().set_bit());
                        // enable the APB clock for the serial port
                        ckcu.ckcu_apbccr0.modify(|_, w| w.$serialXen().set_bit());

                        // According to User Manual page 528
                        // baud rate = ck_uart / brd
                        // -> brd = ck_uart / baud rate
                        let baud_div: u16 = (clocks.hclk.0 / config.baudrate.0) as u16;
                        assert!(baud_div >= 16);


                        // 1st element is whether to enable even parity
                        // 2nd element is whether to enable parity at all
                        // refer to User Manual page 531
                        let parity = match config.parity {
                            config::Parity::ParityNone => (false,false),
                            config::Parity::ParityEven => (true,true),
                            config::Parity::ParityOdd => (false,true)
                        };

                        // value for the number of stop bits register
                        // Refer to User Manual page 531
                        let stop_bits = match config.stopbits {
                            config::StopBits::STOP1 => false,
                            config::StopBits::STOP2 => true,
                        };

                        let word_size = core::mem::size_of::<$WORD>();
                        // value for the world length select register
                        // Refer to User Manual page 532
                        let word_length = match config.wordlength {
                            config::WordLength::DataBits7 => {
                                if word_size != 1 {
                                    return Err(config::InvalidConfig::WordLengthMismatch)
                                }
                                0b00
                            },
                            config::WordLength::DataBits8 => {
                                if word_size != 1 {
                                    return Err(config::InvalidConfig::WordLengthMismatch)
                                }
                                0b01
                            },
                            config::WordLength::DataBits9 => {
                                if word_size != 2 {
                                    return Err(config::InvalidConfig::WordLengthMismatch)
                                }
                                0b10
                            }
                        };

                        // setup the baud rate clock
                        serial.$serial_dlr.write(|w| unsafe {w.brd().bits(baud_div)});

                        // configure the peripheral
                        serial.$serial_cr.modify(|_, w| unsafe {
                            w.epe().
                                // enable even parity if required
                                bit(parity.0).
                                pbe().
                                // enable parity if required
                                bit(parity.1).
                                nsb().
                                // set number of stop bits
                                bit(stop_bits).
                                wls().
                                // set word length
                                bits(word_length)
                        });

                        // enable TX and RX
                        serial.$serial_cr.modify(|_, w| w.urrxen().set_bit().urtxen().set_bit());

                        Ok(Serial { serial, _word: PhantomData })
                    }

                    pub fn split(self) -> (Tx<$SERIALX, $WORD>, Rx<$SERIALX, $WORD>) {
                        (
                            Tx {
                                _serial: PhantomData,
                                _word: PhantomData
                            },
                            Rx {
                                _serial: PhantomData,
                                _word: PhantomData
                            },
                        )
                    }

                    pub fn free(self) -> $SERIALX {
                        // Wait until the data register is empty to release the peripheral
                        while self.serial.$serial_sifr.read().txde().bit_is_clear() {}

                        self.serial
                    }

                    /// Starts listening for an interrupt event
                    pub fn listen(&mut self, event: Event) {
                        match event {
                            Event::FramingError => self.serial.$serial_ier.modify(|_, w| w.feie().set_bit()),
                            Event::ParityError => self.serial.$serial_ier.modify(|_, w| w.peie().set_bit()),
                            Event::OverrunError => self.serial.$serial_ier.modify(|_, w| w.oeie().set_bit()),
                            Event::TransmitComplete => self.serial.$serial_ier.modify(|_, w| w.txcie().set_bit()),
                            Event::TransmitRegisterEmpty => self.serial.$serial_ier.modify(|_, w| w.txdeie().set_bit()),
                            Event::ReceiveDataReady => self.serial.$serial_ier.modify(|_, w| w.rxdrie().set_bit()),
                        }
                    }

                    /// Starts listening for an interrupt event
                    pub fn unlisten(&mut self, event: Event) {
                        match event {
                            Event::FramingError => self.serial.$serial_ier.modify(|_, w| w.feie().clear_bit()),
                            Event::ParityError => self.serial.$serial_ier.modify(|_, w| w.peie().clear_bit()),
                            Event::OverrunError => self.serial.$serial_ier.modify(|_, w| w.oeie().clear_bit()),
                            Event::TransmitComplete => self.serial.$serial_ier.modify(|_, w| w.txcie().clear_bit()),
                            Event::TransmitRegisterEmpty => self.serial.$serial_ier.modify(|_, w| w.txdeie().clear_bit()),
                            Event::ReceiveDataReady => self.serial.$serial_ier.modify(|_, w| w.rxdrie().clear_bit()),
                        }
                    }
                }

                impl SerialExt<$SERIALX, $WORD> for $SERIALX {
                    fn serial<TX, RX>(
                        self,
                        _tx: TX,
                        _rx: RX,
                        config: config::Config,
                        clocks: &Clocks,
                    ) -> Result<Serial<$SERIALX, $WORD>, config::InvalidConfig>
                    where
                        TX: PinTx<$SERIALX>,
                        RX: PinRx<$SERIALX>
                    {
	                    Serial::<$SERIALX, $WORD>::$serialX(self, config, clocks)
                    }

                    fn serial_unchecked(
                        self,
                        config: config::Config,
                        clocks: &Clocks,
                    ) -> Result<Serial<$SERIALX, $WORD>, config::InvalidConfig>
                    {
	                    Serial::<$SERIALX, $WORD>::$serialX(self, config, clocks)
                    }
                }

                impl serial::Read<$WORD> for Serial<$SERIALX, $WORD> {
                    type Error = Error;

                    fn read(&mut self) -> nb::Result<$WORD, Error> {
                        let mut rx: Rx<$SERIALX, $WORD> = Rx {
                            _serial: PhantomData,
                            _word: PhantomData
                        };
                        rx.read()
                    }
                }

                impl serial::Read<$WORD> for Rx<$SERIALX, $WORD> {
                    type Error = Error;

                    fn read(&mut self) -> nb::Result<$WORD, Error> {
                        let sifr = unsafe { (*$SERIALX::ptr()).$serial_sifr.read() };

                        Err(if sifr.pei().bit_is_set() {
                            nb::Error::Other(Error::Parity)
                        }
                        else if sifr.fei().bit_is_set() {
                            nb::Error::Other(Error::Framing)
                        }
                        else if sifr.oei().bit_is_set() {
                            nb::Error::Other(Error::Overrun)
                        }
                        else if sifr.rxdr().bit_is_set() {
                            return Ok(unsafe {
                                    ptr::read_volatile(
                                        &(*$SERIALX::ptr()).$serial_dr as *const _ as *const $WORD,
                                    )
                                }
                            )
                        }
                        else {
                            nb::Error::WouldBlock
                        })
                    }
                }

				impl serial::Write<$WORD> for Serial<$SERIALX, $WORD> {
                	type Error = Infallible;

                	fn flush(&mut self) -> nb::Result<(), Infallible> {
                	    let mut tx: Tx<$SERIALX, $WORD> = Tx {
                	        _serial: PhantomData,
                            _word: PhantomData
                	    };
                	    tx.flush()
                	}

                	fn write(&mut self, byte: $WORD) -> nb::Result<(), Infallible> {
                	    let mut tx: Tx<$SERIALX, $WORD> = Tx {
                	        _serial: PhantomData,
                            _word: PhantomData
                	    };
                	    tx.write(byte)
                	}
            	}

            	impl serial_block::write::Default<$WORD> for Serial<$SERIALX, $WORD> {}

                impl serial::Write<$WORD> for Tx<$SERIALX, $WORD> {
                    type Error = Infallible;

                	fn flush(&mut self) -> nb::Result<(), Infallible> {
                        let sifr = unsafe { (*$SERIALX::ptr()).$serial_sifr.read() };

                        if sifr.txc().bit_is_set() {
                            Ok(())
                        }
                        else {
                            Err(nb::Error::WouldBlock)
                        }
                	}

                	fn write(&mut self, byte: $WORD) -> nb::Result<(), Infallible> {
                        let sifr = unsafe { (*$SERIALX::ptr()).$serial_sifr.read() };

                        if sifr.txde().bit_is_set() {
                            unsafe {
                                ptr::write_volatile(
                                    &(*$SERIALX::ptr()).$serial_dr as *const _ as *mut $WORD,
                                    byte
                                )
                            }
                            Ok(())
                        }
                        else {
                            Err(nb::Error::WouldBlock)
                        }

                	}
                }
             )+
         )+
    }
}

macro_rules! serial_pins {
    ($($SERIALX:ty: TX: [$($TX:ty),*] RX: [$($RX:ty),*])+) => {
        $(
            $(
                impl PinTx<$SERIALX> for $TX {}
            )*
            $(
                impl PinRx<$SERIALX> for $RX {}
            )*
        )+
    }
}

serial_pins! {
    UART0:
        TX: [
            PC4<Output<PushPull>, AF6>,
            PB2<Output<PushPull>, AF6>,
            PB6<Output<PushPull>, AF6>
        ]
        RX: [
            PC5<Input<Floating>, AF6>,
            PB3<Input<Floating>, AF6>,
            PB8<Input<Floating>, AF6>
        ]
    UART1:
        TX: [
            PC12<Output<PushPull>, AF6>,
            PB4<Output<PushPull>, AF6>,
            PC1<Output<PushPull>, AF6>
        ]
        RX: [
            PC13<Input<Floating>, AF6>,
            PB5<Input<Floating>, AF6>,
            PC3<Input<Floating>, AF6>
        ]
    USART0:
        TX: [
            PA2<Output<PushPull>, AF6>,
            PC6<Output<PushPull>, AF6>,
            PA8<Output<PushPull>, AF6>,
            PB0<Output<PushPull>, AF6>
        ]
        RX: [
            PA3<Input<Floating>, AF6>,
            PC7<Input<Floating>, AF6>,
            PA10<Input<Floating>, AF6>,
            PB1<Input<Floating>, AF6>
        ]
    USART1:
        TX: [
            PA4<Output<PushPull>, AF6>,
            PB15<Output<PushPull>, AF6>,
            PA14<Output<PushPull>, AF6>
        ]
        RX: [
            PA5<Input<Floating>, AF6>,
            PC0<Input<Floating>, AF6>,
            PA15<Input<Floating>, AF6>
        ]
}

serial! {
    UART0: (uart0, ur0en, ur0rst, uart_urcr, uart_urdlr, uart_ursifr, uart_urdr, uart_urier) => (u8, u16),
    UART1: (uart1, ur1en, ur0rst, uart_urcr, uart_urdlr, uart_ursifr, uart_urdr, uart_urier) => (u8, u16),
    USART0: (usart0, usr0en, usr0rst, usart_usrcr, usart_usrdlr, usart_usrsifr, usart_usrdr, usart_usrier) => (u8, u16),
    USART1: (usart1, usr1en, usr1rst, usart_usrcr, usart_usrdlr, usart_usrsifr, usart_usrdr, usart_usrier) => (u8, u16),
}

impl<SERIAL> core::fmt::Write for Tx<SERIAL, u8>
where
    Tx<SERIAL, u8>: serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let _ = s.as_bytes().iter().map(|c| block!(self.write(*c))).last();
        Ok(())
    }
}
