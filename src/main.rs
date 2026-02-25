#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::{I2C0, PIO0};
use embassy_rp::pio_programs::rotary_encoder::{Direction, PioEncoder, PioEncoderProgram};
use embassy_rp::{gpio, i2c, pio};
use embassy_time::Timer;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X13},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use gpio::{Input, Pull};
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
   PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
   I2C0_IRQ => i2c::InterruptHandler<I2C0>;
});

#[embassy_executor::task]
async fn handle_button(mut button: Input<'static>) {
    loop {
        button.wait_for_low().await;
        info!("Button Pressed");

        button.wait_for_high().await;
        info!("Button Released");
    }
}

#[embassy_executor::task]
async fn handle_encoder(mut encoder: PioEncoder<'static, PIO0, 0>) {
    let mut count = 0;
    loop {
        info!("Count: {}", count);
        count += match encoder.read().await {
            Direction::Clockwise => 1,
            Direction::CounterClockwise => -1,
        };
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");
    let p = embassy_rp::init(Default::default());

    let button = Input::new(p.PIN_23, Pull::Up);

    // PIO and encoder init
    let pio::Pio {
        mut common, sm0, ..
    } = pio::Pio::new(p.PIO0, Irqs);
    let prg = PioEncoderProgram::new(&mut common);
    let encoder = PioEncoder::new(&mut common, sm0, p.PIN_13, p.PIN_14, &prg);
    info!("Configured PIO");

    // I2C init
    let sda = p.PIN_16;
    let scl = p.PIN_17;
    let mut i2c_config = i2c::Config::default();
    i2c_config.frequency = 400_000;
    i2c_config.sda_pullup = false;
    i2c_config.scl_pullup = false;
    let i2c_bus = i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, i2c_config);
    info!("Configured I2C");

    // Display init
    let display_interface = I2CDisplayInterface::new(i2c_bus);
    let mut display = Ssd1306Async::new(
        display_interface,
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();
    display.init().await.unwrap();
    display.set_display_on(true).await.unwrap();
    Timer::after_millis(100).await;
    info!("Configured Display");

    spawner.spawn(handle_button(button)).unwrap();
    spawner.spawn(handle_encoder(encoder)).unwrap();
}
