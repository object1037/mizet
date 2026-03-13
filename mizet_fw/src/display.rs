use crate::shared::{Button, Direction, EVENT_CH, Mode, UiEvent};

use defmt::*;
use embassy_rp::i2c;
use embassy_rp::peripherals::I2C0;
use embassy_time::{Duration, Ticker, Timer};
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{MonoTextStyleBuilder, ascii::FONT_5X8},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
};
use ssd1306::mode::BufferedGraphicsModeAsync;
use ssd1306::{Ssd1306Async, prelude::*};
use {defmt_rtt as _, panic_probe as _};

type MyDisplay = Ssd1306Async<
    I2CInterface<i2c::I2c<'static, I2C0, i2c::Async>>,
    DisplaySize128x32,
    BufferedGraphicsModeAsync<DisplaySize128x32>,
>;

struct UiState {
    button_a_pressed: bool,
    button_b_pressed: bool,
    button_c_pressed: bool,
    button_d_pressed: bool,
    encoder_pressed: bool,
    current_idx: i32,
    mode: Mode,
}

impl UiState {
    fn set_state(&mut self, event: UiEvent) {
        match event {
            UiEvent::ButtonPress(button) | UiEvent::ButtonRelease(button) => {
                let pressed = matches!(event, UiEvent::ButtonPress(_));
                match button {
                    Button::A => self.button_a_pressed = pressed,
                    Button::B => self.button_b_pressed = pressed,
                    Button::C => self.button_c_pressed = pressed,
                    Button::D => self.button_d_pressed = pressed,
                    Button::Encoder => self.encoder_pressed = pressed,
                }
            }
            UiEvent::Rotary(direction) => match direction {
                Direction::Clockwise => self.current_idx += 1,
                Direction::CounterClockwise => self.current_idx -= 1,
            },
            UiEvent::ModeToggle => {
                self.mode = match self.mode {
                    Mode::Keyboard => Mode::Mouse,
                    Mode::Mouse => Mode::Keyboard,
                };
                self.button_a_pressed = false;
                self.button_b_pressed = false;
                self.button_c_pressed = false;
                self.button_d_pressed = false;
                self.encoder_pressed = false;
            }
        }
    }
}

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

async fn refresh_ui(
    display: &mut MyDisplay,
    ui_state: &UiState,
) -> Result<(), <MyDisplay as DrawTarget>::Error> {
    display.clear(BinaryColor::Off)?;
    match ui_state.mode {
        Mode::Keyboard => draw_keyboard_ui(display, ui_state, 0)?,
        Mode::Mouse => draw_mouse_ui(display, ui_state)?,
    }
    display.flush().await?;

    Ok(())
}

fn draw_keyboard_ui<D>(
    display: &mut D,
    ui_state: &UiState,
    button_offset: i32,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    #[rustfmt::skip]
    #[allow(clippy::unusual_byte_groupings)]
    const ARROW_DATA: &[u8] = &[
        0b00100_000,
        0b01110_000,
        0b10101_000,
        0b00100_000,
        0b00100_000,
        0b00100_000,
    ];

    const TOTAL_BYTES: usize = 6;
    fn inv_image_data(data: &[u8]) -> [u8; TOTAL_BYTES] {
        let mut inv_data = [0u8; TOTAL_BYTES];
        for (i, byte) in data.iter().enumerate() {
            inv_data[i] = !byte;
        }
        inv_data
    }

    let border = PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .stroke_width(1)
        .build();
    let fill = PrimitiveStyleBuilder::new()
        .fill_color(BinaryColor::On)
        .build();
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_5X8)
        .text_color(BinaryColor::On)
        .build();
    let inv_text_style = MonoTextStyleBuilder::new()
        .font(&FONT_5X8)
        .text_color(BinaryColor::Off)
        .build();

    Rectangle::new(Point::new(-1, -1), Size::new(20, 12))
        .into_styled(if ui_state.button_a_pressed {
            fill
        } else {
            border
        })
        .draw(display)?;
    Rectangle::new(Point::new(-1, 10), Size::new(20, 12))
        .into_styled(if ui_state.button_b_pressed {
            fill
        } else {
            border
        })
        .draw(display)?;
    Rectangle::new(Point::new(-1, 21), Size::new(20, 12))
        .into_styled(if ui_state.button_c_pressed {
            fill
        } else {
            border
        })
        .draw(display)?;
    Rectangle::new(Point::new(18, -1), Size::new(9, 34))
        .into_styled(if ui_state.button_d_pressed {
            fill
        } else {
            border
        })
        .draw(display)?;

    Text::with_baseline(
        "Ctl",
        Point::new(button_offset + 1, 1),
        if ui_state.button_a_pressed {
            inv_text_style
        } else {
            text_style
        },
        Baseline::Top,
    )
    .draw(display)?;
    Text::with_baseline(
        "Alt",
        Point::new(button_offset + 1, 12),
        if ui_state.button_b_pressed {
            inv_text_style
        } else {
            text_style
        },
        Baseline::Top,
    )
    .draw(display)?;
    Text::with_baseline(
        "Gui",
        Point::new(button_offset + 1, 23),
        if ui_state.button_c_pressed {
            inv_text_style
        } else {
            text_style
        },
        Baseline::Top,
    )
    .draw(display)?;

    let inv_arrow_data = inv_image_data(ARROW_DATA);
    let raw_image = if ui_state.button_d_pressed {
        ImageRaw::<BinaryColor>::new(&inv_arrow_data, 5)
    } else {
        ImageRaw::<BinaryColor>::new(ARROW_DATA, 5)
    };
    Image::new(&raw_image, Point::new(20, 13)).draw(display)?;

    Line::new(Point::new(26, 0), Point::new(26, 31))
        .into_styled(border)
        .draw(display)?;
    Line::new(Point::new(46, 1), Point::new(46, 30))
        .into_styled(border)
        .draw(display)?;

    Rectangle::new(Point::new(66, 0), Size::new(23, 32))
        .into_styled(border)
        .draw(display)?;

    Line::new(Point::new(108, 1), Point::new(108, 30))
        .into_styled(border)
        .draw(display)?;

    Ok(())
}

fn draw_mouse_ui<D>(_display: &mut D, ui_state: &UiState) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    // Placeholder for mouse mode UI
    Ok(())
}

async fn refresh_initial_ui(
    display: &mut MyDisplay,
    ui_state: &UiState,
) -> Result<(), <MyDisplay as DrawTarget>::Error> {
    let mut ticker = Ticker::every(Duration::from_millis(33));
    for i in 0..6 {
        let button_offset = -20 + i * 4;
        display.clear(BinaryColor::Off)?;
        draw_keyboard_ui(display, ui_state, button_offset)?;
        display.flush().await?;
        ticker.next().await;
    }

    Ok(())
}

#[embassy_executor::task]
pub async fn display_task(mut display: MyDisplay) {
    display.init().await.unwrap();
    info!("Configured Display");

    draw_logo(&mut display).unwrap();
    display.flush().await.unwrap();

    Timer::after_millis(1500).await;

    let mut ui_state = UiState {
        button_a_pressed: false,
        button_b_pressed: false,
        button_c_pressed: false,
        button_d_pressed: false,
        encoder_pressed: false,
        current_idx: 0,
        mode: Mode::Keyboard,
    };

    refresh_initial_ui(&mut display, &ui_state).await.unwrap();

    let mut subscriber = EVENT_CH.subscriber().unwrap();

    loop {
        let event = subscriber.next_message_pure().await;
        ui_state.set_state(event);

        refresh_ui(&mut display, &ui_state).await.unwrap();
    }
}
