#![no_std]
#![no_main]

mod button;
mod display;
mod encoder;
mod shared;

use core::sync::atomic::AtomicBool;

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::{I2C0, PIO0};
use embassy_rp::pio_programs::rotary_encoder::{PioEncoder, PioEncoderProgram};
use embassy_rp::{gpio, i2c, pio};
use gpio::{Input, Pull};
use shared::Button;
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
   PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
   I2C0_IRQ => i2c::InterruptHandler<I2C0>;
});

static IS_KEYBOARD_MODE: AtomicBool = AtomicBool::new(true);

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");
    let p = embassy_rp::init(Default::default());

    // let button_a = Input::new(p.PIN_23, Pull::Up);
    let button_a = Input::new(p.PIN_29, Pull::Up);
    let button_b = Input::new(p.PIN_28, Pull::Up);
    let button_c = Input::new(p.PIN_27, Pull::Up);
    let button_d = Input::new(p.PIN_17, Pull::Up);
    let encoder_sw = Input::new(p.PIN_16, Pull::Up);

    let encoder_a = p.PIN_14;
    let encoder_b = p.PIN_15;

    let sda = p.PIN_12;
    let scl = p.PIN_13;

    // PIO and encoder init
    let pio::Pio {
        mut common, sm0, ..
    } = pio::Pio::new(p.PIO0, Irqs);
    let prg = PioEncoderProgram::new(&mut common);
    let encoder = PioEncoder::new(&mut common, sm0, encoder_a, encoder_b, &prg);
    info!("Configured PIO");

    // I2C init
    let mut i2c_config = i2c::Config::default();
    i2c_config.frequency = 400_000;
    i2c_config.sda_pullup = false;
    i2c_config.scl_pullup = false;
    let i2c_bus = i2c::I2c::new_async(p.I2C0, scl, sda, Irqs, i2c_config);
    info!("Configured I2C");

    // Display init
    let display_interface = I2CDisplayInterface::new(i2c_bus);
    let display = Ssd1306Async::new(
        display_interface,
        DisplaySize128x32,
        DisplayRotation::Rotate180,
    )
    .into_buffered_graphics_mode();

    spawner
        .spawn(button::button_task(button_a, Button::A))
        .unwrap();
    spawner
        .spawn(button::button_task(button_b, Button::B))
        .unwrap();
    spawner
        .spawn(button::button_task(button_c, Button::C))
        .unwrap();
    spawner
        .spawn(button::button_task(button_d, Button::D))
        .unwrap();
    spawner
        .spawn(button::button_task(encoder_sw, Button::Encoder))
        .unwrap();
    spawner.spawn(encoder::encoder_task(encoder)).unwrap();
    spawner.spawn(display::display_task(display)).unwrap();
}
