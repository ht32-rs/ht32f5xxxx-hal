#![no_std]
#![no_main]

use cortex_m_rt::entry;
use ht32f5xxxx_hal::{pac, prelude::*, spi};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};
use nb::block;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Example: SPI");
    let dp = pac::Peripherals::take().unwrap();
    let ckcu = dp.CKCU.constrain(dp.RSTCU);

    let clocks = ckcu.configuration.ck_sys(8.mhz()).freeze();
    let gpioa = dp.GPIOA.split();
    let miso = gpioa.pa2.into_input_floating().into_alternate_af5();
    let sck = gpioa.pa0.into_output_push_pull().into_alternate_af5();
    let mosi = gpioa.pa1.into_output_push_pull().into_alternate_af5();

    let mut spi: spi::Spi<_, u8> = dp.SPI1.spi(
        sck, miso, mosi,
        spi::MODE_0,
        1.mhz(),
        &clocks,
    );

    rprintln!("Starting SPI write");
    spi.write(&[0x11, 0x22, 0x33]).unwrap();

    // Echo what is received on the SPI
    let mut received = 0;
    rprintln!("Starting SPI write loop");
    loop {
        spi.write(&[0x11, 0x22, 0x33]).unwrap();
        block!(spi.send(received)).ok();
        received = block!(spi.read()).unwrap();
        rprintln!("Received: {}", received);
    }
}

