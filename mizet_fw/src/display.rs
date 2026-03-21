use core::sync::atomic::Ordering;

use crate::keymap::{KEYMAP, get_next_idx, get_prev_idx};
use crate::shared::{
    Button, CURRENT_INDEX, Direction, INPUT_CH, InputEvent, MODE_CH, MainMode, ModeChange,
    Modes, MovementAxis, PointerMode, load_modes,
};

use defmt::*;
use embassy_futures::select::{Either, select};
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
    fn set_state(&mut self, event: InputEvent) {
        match event {
            InputEvent::ButtonPress(button) | InputEvent::ButtonRelease(button) => {
                let pressed = matches!(event, InputEvent::ButtonPress(_));
                match button {
                    Button::A => self.button_a_pressed = pressed,
                    Button::B => self.button_b_pressed = pressed,
                    Button::C => self.button_c_pressed = pressed,
                    Button::D => self.button_d_pressed = pressed,
                    Button::Encoder => self.encoder_pressed = pressed,
                }
            }
            InputEvent::Rotary(direction) => {
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
    let has_middle = KEYMAP[index].middle_key.is_some();
    let y_top = if has_middle { 0 } else { 2 };
    let y_bottom = if has_middle { 20 } else { 18 };

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

fn draw_button_rect<D>(
    display: &mut D,
    top_left: Point,
    size: Size,
    is_pressed: bool,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    Rectangle::new(top_left, size)
        .into_styled(if is_pressed { FILLED } else { BORDERED })
        .draw(display)?;
    Ok(())
}

fn draw_button_label<D>(
    display: &mut D,
    text: &str,
    position: Point,
    is_pressed: bool,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    Text::with_baseline(
        text,
        position,
        if is_pressed { INV_TEXT_SM } else { TEXT_SM },
        Baseline::Top,
    )
    .draw(display)?;
    Ok(())
}

#[derive(Clone, Copy)]
enum ArrowDirection {
    Up,
    Down,
    Left,
    Right,
}

fn draw_arrow<D>(
    display: &mut D,
    point: Point,
    is_pressed: bool,
    direction: ArrowDirection,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    #[rustfmt::skip]
    #[allow(clippy::unusual_byte_groupings)]
    const ARROW_V_DATA: &[u8] = &[
        0b001000_00,
        0b011100_00,
        0b101010_00,
        0b001000_00,
        0b001000_00,
        0b001000_00,
    ];
    #[rustfmt::skip]
    #[allow(clippy::unusual_byte_groupings)]
    const ARROW_H_DATA: &[u8] = &[
        0b001000_00,
        0b010000_00,
        0b111111_00,
        0b010000_00,
        0b001000_00,
        0b000000_00,
    ];

    const IMAGE_DIM: usize = 6;
    fn flip_v_image_data(data: &[u8]) -> [u8; IMAGE_DIM] {
        let mut flipped_data = [0u8; IMAGE_DIM];
        for (i, byte) in data.iter().enumerate() {
            flipped_data[data.len() - 1 - i] = *byte;
        }
        flipped_data
    }
    fn flip_h_image_data(data: &[u8]) -> [u8; IMAGE_DIM] {
        let mut flipped_data = [0u8; IMAGE_DIM];
        for (i, byte) in data.iter().enumerate() {
            flipped_data[i] = byte.reverse_bits() << 2;
        }
        flipped_data
    }
    fn inv_image_data(data: &[u8]) -> [u8; IMAGE_DIM] {
        let mut inv_data = [0u8; IMAGE_DIM];
        for (i, byte) in data.iter().enumerate() {
            inv_data[i] = !byte;
        }
        inv_data
    }

    let arrow_data = match direction {
        ArrowDirection::Up => ARROW_V_DATA,
        ArrowDirection::Down => &flip_v_image_data(ARROW_V_DATA),
        ArrowDirection::Left => ARROW_H_DATA,
        ArrowDirection::Right => &flip_h_image_data(ARROW_H_DATA),
    };
    let inv_arrow_data = inv_image_data(arrow_data);
    let raw_image = if is_pressed {
        ImageRaw::<BinaryColor>::new(&inv_arrow_data, IMAGE_DIM as u32)
    } else {
        ImageRaw::<BinaryColor>::new(arrow_data, IMAGE_DIM as u32)
    };
    Image::new(&raw_image, point).draw(display)?;

    Ok(())
}

fn draw_base_ui<D>(display: &mut D, ui_state: &UiState) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    draw_button_rect(
        display,
        Point::new(-1, -1),
        Size::new(20, 12),
        ui_state.button_a_pressed,
    )?;
    draw_button_rect(
        display,
        Point::new(-1, 10),
        Size::new(20, 12),
        ui_state.button_b_pressed,
    )?;
    draw_button_rect(
        display,
        Point::new(-1, 21),
        Size::new(20, 12),
        ui_state.button_c_pressed,
    )?;
    draw_button_rect(
        display,
        Point::new(18, -1),
        Size::new(9, 34),
        ui_state.button_d_pressed,
    )?;

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
    draw_arrow(
        display,
        Point::new(20, 13),
        ui_state.button_d_pressed,
        ArrowDirection::Up,
    )?;

    draw_button_label(
        display,
        "Ctl",
        Point::new(button_offset + 1, 1),
        ui_state.button_a_pressed,
    )?;
    draw_button_label(
        display,
        "Alt",
        Point::new(button_offset + 1, 12),
        ui_state.button_b_pressed,
    )?;
    draw_button_label(
        display,
        "Gui",
        Point::new(button_offset + 1, 23),
        ui_state.button_c_pressed,
    )?;

    Line::new(Point::new(26, 0), Point::new(26, 31))
        .into_styled(BORDERED)
        .draw(display)?;
    Line::new(Point::new(46, 1), Point::new(46, 30))
        .into_styled(BORDERED)
        .draw(display)?;

    draw_button_rect(
        display,
        Point::new(66, 0),
        Size::new(23, 32),
        ui_state.encoder_pressed,
    )?;

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

fn draw_mouse_ui<D>(display: &mut D, ui_state: &UiState, modes: Modes) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    draw_base_ui(display, ui_state)?;
    draw_arrow(
        display,
        Point::new(20, 13),
        ui_state.button_d_pressed,
        if modes.pointer_mode == PointerMode::Move {
            ArrowDirection::Up
        } else {
            ArrowDirection::Down
        },
    )?;

    draw_button_label(display, "R", Point::new(6, 1), ui_state.button_a_pressed)?;
    draw_button_label(display, "M", Point::new(6, 12), ui_state.button_b_pressed)?;
    draw_button_label(display, "L", Point::new(6, 23), ui_state.button_c_pressed)?;

    let y_offset = if modes.pointer_mode == PointerMode::Move {
        15
    } else {
        0
    };

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
    draw_button_rect(
        display,
        Point::new(95, -1),
        Size::new(34, 34),
        ui_state.encoder_pressed,
    )?;

    Text::with_baseline("Scroll", Point::new(39, 2), TEXT_LG, Baseline::Top).draw(display)?;
    Text::with_baseline("Move", Point::new(46, 17), TEXT_LG, Baseline::Top).draw(display)?;

    const ARROW_X: i32 = 112;
    const ARROW_Y: i32 = 13;
    let (points, dirs) = if modes.movement_axis == MovementAxis::Y {
        (
            [
                Point::new(ARROW_X - 3, ARROW_Y - 3),
                Point::new(ARROW_X - 3, ARROW_Y + 3),
            ],
            [ArrowDirection::Up, ArrowDirection::Down],
        )
    } else {
        (
            [
                Point::new(ARROW_X - 6, ARROW_Y),
                Point::new(ARROW_X, ARROW_Y),
            ],
            [ArrowDirection::Left, ArrowDirection::Right],
        )
    };

    for i in 0..2 {
        draw_arrow(display, points[i], ui_state.encoder_pressed, dirs[i])?;
    }

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
    let modes = load_modes();
    match modes.main_mode {
        MainMode::Keyboard => match &ui_state.rotation_animate {
            Some(direction) => {
                handle_rotation(display, ui_state, direction).await?;
                ui_state.rotation_animate = None;
            }
            None => draw_keyboard_ui(display, ui_state, 0, 0)?,
        },
        MainMode::Mouse => draw_mouse_ui(display, ui_state, modes)?,
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

    let mut event_subscriber = INPUT_CH.subscriber().unwrap();
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
