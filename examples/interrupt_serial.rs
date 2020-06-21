#![no_std]
#![no_main]

use core::cell::RefCell;
use core::ops::DerefMut;
use cortex_m::interrupt::Mutex;
use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;
use ht32f5xxxx_hal::{pac, pac::interrupt, prelude::*, serial};
use nb::block;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

static RX: Mutex<RefCell<Option<serial::Rx<pac::USART1, u8>>>> = Mutex::new(RefCell::new(None));

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

    let mut serial: serial::Serial<_, u8> = dp
        .USART1
        .serial(tx, rx, serial::config::Config::default(), &clocks)
        .unwrap();

    serial.listen(serial::Event::ReceiveDataReady);

    let (mut tx, rx) = serial.split();
    cortex_m::interrupt::free(|cs| {
        *RX.borrow(cs).borrow_mut() = Some(rx);
    });

    unsafe { NVIC::unmask(pac::Interrupt::USART1) };

    let mut to_send = 0;
    loop {
        to_send += 1;
        rprintln!("Sending {}", to_send);
        block!(tx.write(to_send)).ok();
    }
}

#[interrupt]
fn USART1() {
    cortex_m::interrupt::free(|cs| {
        rprintln!("Interrupt");
        let mut received = 0;
        if let Some(ref mut rx) = RX.borrow(cs).borrow_mut().deref_mut() {
            received = block!(rx.read()).unwrap();
        }
        rprintln!("Received: {}", received);
    });
}
