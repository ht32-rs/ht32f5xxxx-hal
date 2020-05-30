#![no_std]
#![no_main]

use cortex_m;
use cortex_m_rt::entry;
use panic_rtt_target;
use rtt_target::{rprintln, rtt_init_print};
use ht32f5xxxx_hal::{pac, prelude::*};

#[entry]
fn main() -> ! {
    rtt_init_print!();
    loop {
        rprintln!("Hello, world!");
    }
}
