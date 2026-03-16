use core::sync::atomic::Ordering;

use crate::keymap::{KEYMAP, get_next_idx, get_prev_idx};
use crate::shared::{
    Button, CURRENT_INDEX, Direction, EVENT_CH, IS_KEYBOARD_MODE, IS_MOVE_MODE, IS_MOVEMENT_Y,
    ModeChange, MODE_CH, UiEvent,
};

use defmt::*;
use embassy_futures::select::{select, Either};
use embassy_rp::i2c;
use embassy_rp::peripherals::I2C0;
use embassy_time::{Duration, Ticker, Timer};
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::mono_font::{
    DecorationDimensions, MonoFont, MonoTextStyle, MonoTextStyleBuilder, ascii::FONT_5X8,
    mapping::ASCII,
};
use embedded_graphics::{
    image::{Image, ImageRaw},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
};
use ssd1306::mode::BufferedGraphicsModeAsync;
use ssd1306::{Ssd1306Async, prelude::*};
use {defmt_rtt as _, panic_probe as _};

const DEPARTURE_7X12: MonoFont = MonoFont {
    image: ImageRaw::new(include_bytes!("fonts/DepartureMono.raw"), 112),
    glyph_mapping: &ASCII,
    character_size: Size::new(7, 12),
    character_spacing: 1,
    baseline: 9,
    underline: DecorationDimensions::default_underline(10),
    strikethrough: DecorationDimensions::default_strikethrough(10),
};

static BORDERED: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_color(BinaryColor::On)
    .stroke_width(1)
    .build();
static FILLED: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .fill_color(BinaryColor::On)
    .build();
static TEXT_SM: MonoTextStyle<BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_5X8)
    .text_color(BinaryColor::On)
    .build();
static INV_TEXT_SM: MonoTextStyle<BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_5X8)
    .text_color(BinaryColor::Off)
    .build();
static TEXT_LG: MonoTextStyle<BinaryColor> = MonoTextStyleBuilder::new()
    .font(&DEPARTURE_7X12)
    .text_color(BinaryColor::On)
    .build();
static INV_TEXT_LG: MonoTextStyle<BinaryColor> = MonoTextStyleBuilder::new()
    .font(&DEPARTURE_7X12)
    .text_color(BinaryColor::Off)
    .build();

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
    rotation_animate: Option<Direction>,
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
            UiEvent::Rotary(direction) => {
                self.rotation_animate = Some(direction);
            }
        }
    }

    fn reset(&mut self) {
        self.button_a_pressed = false;
        self.button_b_pressed = false;
        self.button_c_pressed = false;
        self.button_d_pressed = false;
        self.encoder_pressed = false;
        self.rotation_animate = None;
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

fn draw_key<D>(
    display: &mut D,
    index: usize,
    x: i32,
    style: &MonoTextStyle<BinaryColor>,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let y_top = if KEYMAP[index].middle_key.is_some() {
        0
    } else {
        2
    };
    let y_bottom = if KEYMAP[index].middle_key.is_some() {
        20
    } else {
        18
    };

    Text::with_baseline(
        KEYMAP[index].shifted_key,
        Point::new(x, y_top),
        *style,
        Baseline::Top,
    )
    .draw(display)?;

    if let Some(middle_key) = KEYMAP[index].middle_key {
        Text::with_baseline(middle_key, Point::new(x, 10), *style, Baseline::Top).draw(display)?;
    }

    Text::with_baseline(
        KEYMAP[index].key,
        Point::new(x, y_bottom),
        *style,
        Baseline::Top,
    )
    .draw(display)?;

    Ok(())
}

fn draw_base_ui<D>(display: &mut D, ui_state: &UiState) -> Result<(), D::Error>
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

    Rectangle::new(Point::new(-1, -1), Size::new(20, 12))
        .into_styled(if ui_state.button_a_pressed {
            FILLED
        } else {
            BORDERED
        })
        .draw(display)?;
    Rectangle::new(Point::new(-1, 10), Size::new(20, 12))
        .into_styled(if ui_state.button_b_pressed {
            FILLED
        } else {
            BORDERED
        })
        .draw(display)?;
    Rectangle::new(Point::new(-1, 21), Size::new(20, 12))
        .into_styled(if ui_state.button_c_pressed {
            FILLED
        } else {
            BORDERED
        })
        .draw(display)?;
    Rectangle::new(Point::new(18, -1), Size::new(9, 34))
        .into_styled(if ui_state.button_d_pressed {
            FILLED
        } else {
            BORDERED
        })
        .draw(display)?;

    let inv_arrow_data = inv_image_data(ARROW_DATA);
    let raw_image = if ui_state.button_d_pressed {
        ImageRaw::<BinaryColor>::new(&inv_arrow_data, 5)
    } else {
        ImageRaw::<BinaryColor>::new(ARROW_DATA, 5)
    };
    Image::new(&raw_image, Point::new(20, 13)).draw(display)?;

    Ok(())
}

fn draw_keyboard_ui<D>(
    display: &mut D,
    ui_state: &UiState,
    button_offset: i32,
    key_offset: i32,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    draw_base_ui(display, ui_state)?;

    Text::with_baseline(
        "Ctl",
        Point::new(button_offset + 1, 1),
        if ui_state.button_a_pressed {
            INV_TEXT_SM
        } else {
            TEXT_SM
        },
        Baseline::Top,
    )
    .draw(display)?;
    Text::with_baseline(
        "Alt",
        Point::new(button_offset + 1, 12),
        if ui_state.button_b_pressed {
            INV_TEXT_SM
        } else {
            TEXT_SM
        },
        Baseline::Top,
    )
    .draw(display)?;
    Text::with_baseline(
        "Gui",
        Point::new(button_offset + 1, 23),
        if ui_state.button_c_pressed {
            INV_TEXT_SM
        } else {
            TEXT_SM
        },
        Baseline::Top,
    )
    .draw(display)?;

    Line::new(Point::new(26, 0), Point::new(26, 31))
        .into_styled(BORDERED)
        .draw(display)?;
    Line::new(Point::new(46, 1), Point::new(46, 30))
        .into_styled(BORDERED)
        .draw(display)?;

    Rectangle::new(Point::new(66, 0), Size::new(23, 32))
        .into_styled(if ui_state.encoder_pressed {
            FILLED
        } else {
            BORDERED
        })
        .draw(display)?;

    Line::new(Point::new(108, 1), Point::new(108, 30))
        .into_styled(BORDERED)
        .draw(display)?;

    let current_index = CURRENT_INDEX.load(Ordering::Relaxed);
    let prev1_index = get_prev_idx(current_index);
    let prev2_index = get_prev_idx(prev1_index);
    let next1_index = get_next_idx(current_index);
    let next2_index = get_next_idx(next1_index);

    draw_key(display, prev2_index, key_offset + 33, &TEXT_LG)?;
    draw_key(display, prev1_index, key_offset + 53, &TEXT_LG)?;
    draw_key(
        display,
        current_index,
        key_offset + 74,
        if ui_state.encoder_pressed {
            &INV_TEXT_LG
        } else {
            &TEXT_LG
        },
    )?;
    draw_key(display, next1_index, key_offset + 95, &TEXT_LG)?;
    draw_key(display, next2_index, key_offset + 115, &TEXT_LG)?;

    Ok(())
}

fn draw_mouse_ui<D>(display: &mut D, ui_state: &UiState) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    draw_base_ui(display, ui_state)?;

    Text::with_baseline(
        "R",
        Point::new(6, 1),
        if ui_state.button_a_pressed {
            INV_TEXT_SM
        } else {
            TEXT_SM
        },
        Baseline::Top,
    )
    .draw(display)?;
    Text::with_baseline(
        "M",
        Point::new(6, 12),
        if ui_state.button_b_pressed {
            INV_TEXT_SM
        } else {
            TEXT_SM
        },
        Baseline::Top,
    )
    .draw(display)?;
    Text::with_baseline(
        "L",
        Point::new(6, 23),
        if ui_state.button_c_pressed {
            INV_TEXT_SM
        } else {
            TEXT_SM
        },
        Baseline::Top,
    )
    .draw(display)?;

    let is_move_mode = IS_MOVE_MODE.load(Ordering::Relaxed);
    let y_offset = if is_move_mode { 15 } else { 0 };

    const CORNER_L: i32 = 28;
    const CORNER_R: i32 = 93;
    const CORNER_T: i32 = 1;
    const CORNER_B: i32 = 15;
    const CORNER_LEN: i32 = 4;

    let corners = [
        (CORNER_L, CORNER_T, 1, 1),   // top-left
        (CORNER_R, CORNER_T, -1, 1),  // top-right
        (CORNER_L, CORNER_B, 1, -1),  // bottom-left
        (CORNER_R, CORNER_B, -1, -1), // bottom-right
    ];

    for &(x, y, sx, sy) in &corners {
        Line::new(
            Point::new(x, y + y_offset),
            Point::new(x + sx * CORNER_LEN, y + y_offset),
        )
        .into_styled(BORDERED)
        .draw(display)?;
        Line::new(
            Point::new(x, y + y_offset),
            Point::new(x, y + y_offset + sy * CORNER_LEN),
        )
        .into_styled(BORDERED)
        .draw(display)?;
    }
    Rectangle::new(Point::new(95, -1), Size::new(34, 34))
        .into_styled(if ui_state.encoder_pressed {
            FILLED
        } else {
            BORDERED
        })
        .draw(display)?;

    let is_movement_y = IS_MOVEMENT_Y.load(Ordering::Relaxed);

    let axis_label = if is_movement_y { "Y" } else { "X" };
    Text::with_baseline("Scroll", Point::new(39, 2), TEXT_LG, Baseline::Top).draw(display)?;
    Text::with_baseline("Move", Point::new(46, 17), TEXT_LG, Baseline::Top).draw(display)?;
    Text::with_baseline(
        axis_label,
        Point::new(108, 10),
        if ui_state.encoder_pressed {
            INV_TEXT_LG
        } else {
            TEXT_LG
        },
        Baseline::Top,
    )
    .draw(display)?;

    Ok(())
}

async fn handle_rotation(
    display: &mut MyDisplay,
    ui_state: &UiState,
    direction: &Direction,
) -> Result<(), <MyDisplay as DrawTarget>::Error> {
    // let mut ticker = Ticker::every(Duration::from_millis(17));
    const OFFSET_STEP: i32 = 7;
    let mut offset = 0;
    for i in 0..=2 {
        // ticker.next().await;
        if i == 1 {
            // Update index in the middle of the animation for smoother effect
            let current_index = CURRENT_INDEX.load(Ordering::Relaxed);
            let new_index = match direction {
                Direction::Clockwise => get_prev_idx(current_index),
                Direction::CounterClockwise => get_next_idx(current_index),
            };
            CURRENT_INDEX.store(new_index, Ordering::Relaxed);
            offset = match direction {
                Direction::Clockwise => -OFFSET_STEP,
                Direction::CounterClockwise => OFFSET_STEP,
            };
        } else {
            offset += match direction {
                Direction::Clockwise => OFFSET_STEP,
                Direction::CounterClockwise => -OFFSET_STEP,
            };
        }

        display.clear(BinaryColor::Off)?;
        draw_keyboard_ui(display, ui_state, 0, offset)?;
        display.flush().await?;
    }

    Ok(())
}

async fn refresh_ui(
    display: &mut MyDisplay,
    ui_state: &mut UiState,
) -> Result<(), <MyDisplay as DrawTarget>::Error> {
    display.clear(BinaryColor::Off)?;
    match IS_KEYBOARD_MODE.load(Ordering::Relaxed) {
        true => match &ui_state.rotation_animate {
            Some(direction) => {
                handle_rotation(display, ui_state, direction).await?;
                ui_state.rotation_animate = None;
            }
            None => draw_keyboard_ui(display, ui_state, 0, 0)?,
        },
        false => draw_mouse_ui(display, ui_state)?,
    }
    display.flush().await?;

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
        draw_keyboard_ui(display, ui_state, button_offset, 0)?;
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
        rotation_animate: None,
    };

    refresh_initial_ui(&mut display, &ui_state).await.unwrap();

    let mut event_subscriber = EVENT_CH.subscriber().unwrap();
    let mut mode_subscriber = MODE_CH.subscriber().unwrap();

    loop {
        match select(
            event_subscriber.next_message_pure(),
            mode_subscriber.next_message_pure(),
        )
        .await
        {
            Either::First(event) => {
                ui_state.set_state(event);
            }
            Either::Second(mode_change) => {
                if let ModeChange::MainMode = mode_change {
                    ui_state.reset();
                }
            }
        }

        refresh_ui(&mut display, &mut ui_state).await.unwrap();
    }
}
