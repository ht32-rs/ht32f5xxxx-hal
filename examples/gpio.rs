#![no_std]
#![no_main]

use cortex_m_rt::entry;
use ht32f5xxxx_hal::{pac, prelude::*};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Example: GPIO");
    let dp = pac::Peripherals::take().unwrap();
    let ckcu = dp.CKCU.constrain(dp.RSTCU);

    ckcu.configuration.ck_sys(8.mhz()).freeze();
    let gpioa = dp.GPIOA.split();
    let mut led0 = gpioa.pa0.into_output_push_pull();
    let input0 = gpioa.pa1.into_input_pull_down();
    led0.set_high().unwrap();

    loop {
        // If you short PA0 to PA1 you should see true here
        rprintln!("input0, reading: {}", input0.is_high().unwrap());
    }

}
