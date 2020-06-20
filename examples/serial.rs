#![no_std]
#![no_main]

use cortex_m_rt::entry;
use ht32f5xxxx_hal::{pac, prelude::*, serial};
use nb::block;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Example: Serial");
    let dp = pac::Peripherals::take().unwrap();
    let ckcu = dp.CKCU.constrain(dp.RSTCU);

    let clocks = ckcu.configuration.ck_sys(8.mhz()).freeze();
    let gpioa = dp.GPIOA.split();
    let tx = gpioa.pa4.into_output_push_pull().into_alternate_af6();
    let rx = gpioa.pa5.into_input_floating().into_alternate_af6();

    let serial: serial::Serial<_, u8> = dp
        .USART1
        .serial(tx, rx, serial::config::Config::default(), &clocks)
        .unwrap();

    let (mut tx, mut rx) = serial.split();

    let mut received = 97; // ASCII a
    loop {
        block!(tx.write(received)).ok();
        // Echo what is received on the serial link.
        received = block!(rx.read()).unwrap();
        rprintln!("Received: {}", received);
    }
}
