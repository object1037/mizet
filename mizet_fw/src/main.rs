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
    image::{Image, ImageRaw},
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X13},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use gpio::{Input, Pull};
use ssd1306::mode::BufferedGraphicsModeAsync;
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
   PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
   I2C0_IRQ => i2c::InterruptHandler<I2C0>;
});

enum Button {
    A,
    B,
    C,
    Mode,
}

enum UiEvent {
    ButtonPress(Button),
    Rotary(Direction),
    EncoderPush,
}

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

#[embassy_executor::task]
async fn handle_display(
    mut display: Ssd1306Async<
        I2CInterface<i2c::I2c<'static, I2C0, i2c::Async>>,
        DisplaySize128x32,
        BufferedGraphicsModeAsync<DisplaySize128x32>,
    >,
) {
    #[rustfmt::skip]
    const LOGO_DATA: &[u8] = &[
        0b11111010, 0b11100011, 0b10010_000,
        0b10101000, 0b00100011, 0b00111_000,
        0b10101010, 0b00111011, 0b10010_000
    ];

    const SCALE: usize = 5;
    const SRC_WIDTH: usize = 21;
    const SRC_HEIGHT: usize = 3;

    // Calculate dimensions
    const DST_WIDTH: usize = SRC_WIDTH * SCALE; // 105
    const DST_HEIGHT: usize = SRC_HEIGHT * SCALE; // 15
    const BYTES_PER_ROW: usize = DST_WIDTH.div_ceil(8); // 14 bytes
    const TOTAL_BYTES: usize = BYTES_PER_ROW * DST_HEIGHT; // 210 bytes

    fn scale_logo(src: &[u8]) -> [u8; TOTAL_BYTES] {
        // Initialize a stack array filled with zeros
        let mut dst = [0u8; TOTAL_BYTES];

        for y in 0..SRC_HEIGHT {
            for x in 0..SRC_WIDTH {
                // Locate bit in source
                let src_byte_idx = y * 3 + (x / 8);
                let src_bit_mask = 0x80 >> (x % 8);

                if (src[src_byte_idx] & src_bit_mask) != 0 {
                    // Map to SCALE x SCALE block in destination
                    for dy in 0..SCALE {
                        for dx in 0..SCALE {
                            let out_x = x * SCALE + dx;
                            let out_y = y * SCALE + dy;

                            let out_byte_idx = out_y * BYTES_PER_ROW + (out_x / 8);
                            let out_bit_mask = 0x80 >> (out_x % 8);

                            dst[out_byte_idx] |= out_bit_mask;
                        }
                    }
                }
            }
        }
        dst
    }

    let logo_data_scaled = scale_logo(LOGO_DATA);

    let raw_image = ImageRaw::<BinaryColor>::new(&logo_data_scaled, 105);

    let startup_image = Image::new(&raw_image, Point::new(11, 8));
    startup_image.draw(&mut display).unwrap();

    display.flush().await.unwrap();
    // loop {}
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");
    let p = embassy_rp::init(Default::default());

    let button = Input::new(p.PIN_23, Pull::Up);
    let encoder_a = p.PIN_10;
    let encoder_b = p.PIN_11;
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
    let mut display = Ssd1306Async::new(
        display_interface,
        DisplaySize128x32,
        DisplayRotation::Rotate180,
    )
    .into_buffered_graphics_mode();
    display.init().await.unwrap();
    Timer::after_millis(100).await;
    info!("Configured Display");

    spawner.spawn(handle_button(button)).unwrap();
    spawner.spawn(handle_encoder(encoder)).unwrap();
    spawner.spawn(handle_display(display)).unwrap();
}
