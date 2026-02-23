#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    Peri,
    gpio::{self, AnyPin},
};
use embassy_time::Timer;
use gpio::{Input, Level, Output, Pull};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task]
async fn blink(pin: Peri<'static, AnyPin>) {
    let mut led = Output::new(pin, Level::Low);

    loop {
        info!("LED On");
        led.set_high();
        Timer::after_secs(1).await;

        info!("LED Off");
        led.set_low();
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    spawner.spawn(blink(p.PIN_25.into())).unwrap();

    let mut button = Input::new(p.PIN_23, Pull::Up);
    loop {
        button.wait_for_low().await;
        info!("Button Pressed");
        button.wait_for_high().await;
        info!("Button Released");
    }
}
