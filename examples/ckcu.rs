#![no_std]
#![no_main]

use cortex_m_rt::entry;
use ht32f5xxxx_hal::{pac, prelude::*};
use panic_rtt_target;
use rtt_target::{rprintln, rtt_init_print};

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Example: CKCU");
    let dp = pac::Peripherals::take().unwrap();
    let ckcu = dp.CKCU.constrain();

    let clocks = ckcu.configuration.ck_sys(32.mhz()).ckout(CkoutSrc::CkSys).freeze();

    rprintln!("Calculating a fibonacci number");
    // This takes around 2 seconds with sys_ck = 40 Mhz and around three with sys_ck = 32 Mhz.
    // Hence the clock speed is actually changing
    rprintln!("Fibonnaci: {}", fibonacci_reccursive(32));
    rprintln!("Example: CKCU, done");
    loop {}
}

fn fibonacci_reccursive(n: i32) -> u64 {
    match n {
        0 => unreachable!(),
        1 | 2 => 1,
        3 => 2,
        _ => fibonacci_reccursive(n - 1) + fibonacci_reccursive(n - 2),
    }
}
