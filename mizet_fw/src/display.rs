use crate::shared::EVENT_CH;

use defmt::*;
use embassy_rp::i2c;
use embassy_rp::peripherals::I2C0;
use embassy_time::Timer;
use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{MonoTextStyleBuilder, ascii::FONT_5X8},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
};
use ssd1306::mode::BufferedGraphicsModeAsync;
use ssd1306::{Ssd1306Async, prelude::*};
use {defmt_rtt as _, panic_probe as _};

fn draw_logo<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    #[rustfmt::skip]
    #[allow(clippy::unusual_byte_groupings)]
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
    startup_image.draw(display)?;

    Ok(())
}

fn draw_ui<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let border = PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .stroke_width(1)
        .build();
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_5X8)
        .text_color(BinaryColor::On)
        .build();

    Rectangle::new(Point::new(-1, -1), Size::new(25, 12))
        .into_styled(border)
        .draw(display)?;
    Rectangle::new(Point::new(-1, 10), Size::new(25, 12))
        .into_styled(border)
        .draw(display)?;
    Rectangle::new(Point::new(-1, 21), Size::new(25, 12))
        .into_styled(border)
        .draw(display)?;

    Text::with_baseline("Ctrl", Point::new(1, 1), text_style, Baseline::Top).draw(display)?;
    Text::with_baseline("Alt", Point::new(1, 12), text_style, Baseline::Top).draw(display)?;
    Text::with_baseline("Shft", Point::new(1, 23), text_style, Baseline::Top).draw(display)?;

    Ok(())
}

#[embassy_executor::task]
pub async fn display_task(
    mut display: Ssd1306Async<
        I2CInterface<i2c::I2c<'static, I2C0, i2c::Async>>,
        DisplaySize128x32,
        BufferedGraphicsModeAsync<DisplaySize128x32>,
    >,
) {
    display.init().await.unwrap();
    Timer::after_millis(100).await;
    info!("Configured Display");

    draw_logo(&mut display).unwrap();
    display.flush().await.unwrap();

    Timer::after_millis(1000).await;
    display.clear(BinaryColor::Off).unwrap();
    draw_ui(&mut display).unwrap();
    display.flush().await.unwrap();

    loop {
        let event = EVENT_CH.receive().await;
        info!("Received event");
    }
}
