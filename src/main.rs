#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::PIO0;
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::pio_programs::rotary_encoder::{Direction, PioEncoder, PioEncoderProgram};
use embassy_rp::{
    Peri,
    gpio::{self, AnyPin},
};
use embassy_time::Timer;
use gpio::{Input, Level, Output, Pull};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
   PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn blink(pin: Peri<'static, AnyPin>) {
    let mut led = Output::new(pin, Level::Low);

    loop {
        info!("LED On");
        led.set_high();
        Timer::after_secs(2).await;

        info!("LED Off");
        led.set_low();
        Timer::after_secs(2).await;
    }
}

#[embassy_executor::task]
async fn handle_encoder(mut encoder: PioEncoder<'static, PIO0, 0>) {
    loop {
        let mut count = 0;
        loop {
            info!("Count: {}", count);
            count += match encoder.read().await {
                Direction::Clockwise => 1,
                Direction::CounterClockwise => -1,
            };
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO0, Irqs);

    let prg = PioEncoderProgram::new(&mut common);
    let encoder = PioEncoder::new(&mut common, sm0, p.PIN_13, p.PIN_14, &prg);

    spawner.spawn(blink(p.PIN_25.into())).unwrap();
    spawner.spawn(handle_encoder(encoder)).unwrap();

    let mut button = Input::new(p.PIN_23, Pull::Up);
    loop {
        button.wait_for_low().await;
        info!("Button Pressed");
        button.wait_for_high().await;
        info!("Button Released");
    }
}
