#![no_std]
#![no_main]

use cortex_m_rt::entry;
use ht32f5xxxx_hal::{pac, prelude::*};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Example: I2C");
    let dp = pac::Peripherals::take().unwrap();
    let ckcu = dp.CKCU.constrain(dp.RSTCU);

    let clocks = ckcu.configuration.ck_sys(8.mhz()).freeze();
    let gpioa = dp.GPIOA.split();
    let scl = gpioa.pa4.into_output_open_drain().into_alternate_af7();
    let sda = gpioa.pa5.into_output_open_drain().into_alternate_af7();

    let mut i2c = dp.I2C0.i2c(scl, sda, 100.khz(), &clocks);
    let mut buf = [0x60];
    loop {
        buf[0] = 0x11;
        i2c.write_read(0x76, &buf.clone(), &mut buf).unwrap();
        rprintln!("{:?}", buf);
    }
}
