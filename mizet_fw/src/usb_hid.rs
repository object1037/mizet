use core::sync::atomic::Ordering;

use crate::{
    keymap::KEYMAP,
    shared::{
        Button, CURRENT_INDEX, Direction, INPUT_CH, InputEvent, MODE_CH, MainMode, ModeChange,
        MovementAxis, PointerMode, load_modes,
    },
};

use defmt::*;
use embassy_futures::join::join;
use embassy_futures::select::{Either, select};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_time::Instant;
use embassy_usb::class::hid::{HidWriter, State};
use embassy_usb::{Builder, Config};
use usbd_hid::descriptor::{KeyboardReport, MouseReport, SerializedDescriptor};
use {defmt_rtt as _, panic_probe as _};

/// Number of recent inter-detent intervals averaged for speed estimate.
const ROTARY_DT_HISTORY_LEN: usize = 2;
/// Average dt at or below this (ms) maps to maximum move step.
const ROTARY_DT_FAST_MS: u32 = 70;
/// Average dt at or above this (ms) maps to minimum move step.
const ROTARY_DT_SLOW_MS: u32 = 240;
/// Gap since last detent above this (ms) clears history (stall).
const ROTARY_STALL_MS: u64 = 240;
const MIN_MOVE_STEP: i8 = 5;
const MAX_MOVE_STEP: i8 = 30;
const SCROLL_STEP: i8 = 1;

struct RotarySpeedEstimator {
    last_at: Option<Instant>,
    last_dir: Option<Direction>,
    dts: [u32; ROTARY_DT_HISTORY_LEN],
    len: u8,
}

impl Default for RotarySpeedEstimator {
    fn default() -> Self {
        Self {
            last_at: None,
            last_dir: None,
            dts: [0; ROTARY_DT_HISTORY_LEN],
            len: 0,
        }
    }
}

impl RotarySpeedEstimator {
    fn reset(&mut self) {
        self.last_at = None;
        self.last_dir = None;
        self.len = 0;
    }

    /// Returns pixel move step magnitude (5..=30) for this detent from recent rotation timing.
    fn move_step_for_tick(&mut self, now: Instant, direction: &Direction) -> i8 {
        let Some(last) = self.last_at else {
            self.last_at = Some(now);
            self.last_dir = Some(direction.clone());
            return MIN_MOVE_STEP;
        };

        let dt_ms = (now - last).as_millis();

        if dt_ms > ROTARY_STALL_MS {
            self.len = 0;
            self.last_at = Some(now);
            self.last_dir = Some(direction.clone());
            return MIN_MOVE_STEP;
        }

        let direction_flipped = matches!(
            (self.last_dir.as_ref(), direction),
            (Some(Direction::Clockwise), Direction::CounterClockwise)
                | (Some(Direction::CounterClockwise), Direction::Clockwise)
        );
        if direction_flipped {
            self.len = 0;
        }

        let dt_u32 = dt_ms as u32;
        let n = self.len as usize;
        if n < ROTARY_DT_HISTORY_LEN {
            self.dts[n] = dt_u32;
            self.len += 1;
        } else {
            for i in 1..ROTARY_DT_HISTORY_LEN {
                self.dts[i - 1] = self.dts[i];
            }
            self.dts[ROTARY_DT_HISTORY_LEN - 1] = dt_u32;
        }

        let count = self.len as u64;
        let sum: u64 = self.dts[..self.len as usize]
            .iter()
            .map(|&x| x as u64)
            .sum();
        let avg_dt = (sum / count) as u32;

        let step = map_avg_dt_to_move_step(avg_dt);

        self.last_at = Some(now);
        self.last_dir = Some(direction.clone());
        step
    }
}

fn map_avg_dt_to_move_step(avg_dt: u32) -> i8 {
    if avg_dt <= ROTARY_DT_FAST_MS {
        return MAX_MOVE_STEP;
    }
    if avg_dt >= ROTARY_DT_SLOW_MS {
        return MIN_MOVE_STEP;
    }
    let span = (ROTARY_DT_SLOW_MS - ROTARY_DT_FAST_MS) as u64;
    let num = (avg_dt - ROTARY_DT_FAST_MS) as u64;
    let delta = (MAX_MOVE_STEP - MIN_MOVE_STEP) as u64;
    let sub = (num * delta / span) as i8;
    MAX_MOVE_STEP - sub
}

#[derive(Default)]
struct UsbInputState {
    button_a: bool,
    button_b: bool,
    button_c: bool,
    button_d: bool,
}

impl UsbInputState {
    fn clear(&mut self) {
        self.button_a = false;
        self.button_b = false;
        self.button_c = false;
        self.button_d = false;
    }

    fn set_pressed(&mut self, button: Button, pressed: bool) {
        match button {
            Button::A => self.button_a = pressed,
            Button::B => self.button_b = pressed,
            Button::C => self.button_c = pressed,
            Button::D => self.button_d = pressed,
            Button::Encoder => {}
        }
    }

    fn keyboard_modifier(&self) -> u8 {
        let mut modifier = 0u8;
        if self.button_a {
            modifier |= 0x01;
        }
        if self.button_d {
            modifier |= 0x02;
        }
        if self.button_b {
            modifier |= 0x04;
        }
        if self.button_c {
            modifier |= 0x08;
        }
        modifier
    }

    fn mouse_buttons(&self) -> u8 {
        let mut buttons = 0u8;
        if self.button_c {
            buttons |= 0x01;
        }
        if self.button_a {
            buttons |= 0x02;
        }
        if self.button_b {
            buttons |= 0x04;
        }
        buttons
    }
}

fn keyboard_release_report(modifier: u8) -> KeyboardReport {
    KeyboardReport {
        modifier,
        reserved: 0,
        leds: 0,
        keycodes: [0, 0, 0, 0, 0, 0],
    }
}

fn keyboard_report(modifier: u8, keycode: Option<u8>) -> KeyboardReport {
    let mut report = keyboard_release_report(modifier);
    if let Some(code) = keycode {
        report.keycodes[0] = code;
    }
    report
}

fn mouse_report(buttons: u8, x: i8, y: i8, wheel: i8, pan: i8) -> MouseReport {
    MouseReport {
        buttons,
        x,
        y,
        wheel,
        pan,
    }
}

async fn send_keyboard<'d>(
    writer: &mut HidWriter<'d, Driver<'d, USB>, 8>,
    report: &KeyboardReport,
    context: &'static str,
) {
    if let Err(err) = writer.write_serialize(report).await {
        warn!("USB keyboard write failed ({}): {:?}", context, err);
    }
}

async fn send_mouse<'d>(
    writer: &mut HidWriter<'d, Driver<'d, USB>, 8>,
    report: &MouseReport,
    context: &'static str,
) {
    if let Err(err) = writer.write_serialize(report).await {
        warn!("USB mouse write failed ({}): {:?}", context, err);
    }
}

#[embassy_executor::task]
pub async fn usb_task(driver: Driver<'static, USB>) {
    let mut config = Config::new(0x16c0, 0x27d9); // Generic HID https://github.com/obdev/v-usb/blob/master/usbdrv/USB-IDs-for-free.txt
    config.manufacturer = Some("Pentronic Lab.");
    config.product = Some("mizet");
    config.serial_number = Some("0001");
    config.max_power = 100;
    config.max_packet_size_0 = 64;
    config.composite_with_iads = false;
    config.device_class = 0;
    config.device_sub_class = 0;
    config.device_protocol = 0;

    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut msos_descriptor = [0; 256];
    let mut control_buf = [0; 64];
    let mut keyboard_state = State::new();
    let mut mouse_state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut msos_descriptor,
        &mut control_buf,
    );

    let keyboard_config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 1,
        max_packet_size: 8,
    };
    let mouse_config = embassy_usb::class::hid::Config {
        report_descriptor: MouseReport::desc(),
        request_handler: None,
        poll_ms: 1,
        max_packet_size: 8,
    };

    let mut keyboard_writer =
        HidWriter::<_, 8>::new(&mut builder, &mut keyboard_state, keyboard_config);
    let mut mouse_writer = HidWriter::<_, 8>::new(&mut builder, &mut mouse_state, mouse_config);

    let mut usb = builder.build();

    let mut event_subscriber = INPUT_CH.subscriber().unwrap();
    let mut mode_subscriber = MODE_CH.subscriber().unwrap();
    let mut state = UsbInputState::default();
    let mut encoder_keycode: Option<u8> = None;
    let mut rotary_speed = RotarySpeedEstimator::default();

    let usb_fut = usb.run();
    let hid_fut = async {
        loop {
            match select(
                event_subscriber.next_message_pure(),
                mode_subscriber.next_message_pure(),
            )
            .await
            {
                Either::Second(mode) => {
                    if let ModeChange::MainMode = mode {
                        state.clear();
                        encoder_keycode = None;
                        rotary_speed.reset();
                        send_keyboard(
                            &mut keyboard_writer,
                            &keyboard_release_report(0),
                            "main mode switch",
                        )
                        .await;
                        send_mouse(
                            &mut mouse_writer,
                            &mouse_report(0, 0, 0, 0, 0),
                            "main mode switch",
                        )
                        .await;
                        info!("USB: main mode toggled, sent release reports");
                    }
                }
                Either::First(event) => {
                    let modes = load_modes();
                    match event {
                        InputEvent::ButtonPress(button) => {
                            state.set_pressed(button, true);
                            if modes.main_mode == MainMode::Keyboard {
                                if matches!(button, Button::A | Button::B | Button::C | Button::D) {
                                    let report =
                                        keyboard_report(state.keyboard_modifier(), encoder_keycode);
                                    send_keyboard(&mut keyboard_writer, &report, "modifier down")
                                        .await;
                                } else if matches!(button, Button::Encoder) {
                                    let current_index = CURRENT_INDEX.load(Ordering::Relaxed);
                                    let keycode = KEYMAP[current_index].keycode as u8;
                                    encoder_keycode = Some(keycode);
                                    let report =
                                        keyboard_report(state.keyboard_modifier(), encoder_keycode);
                                    send_keyboard(&mut keyboard_writer, &report, "key down").await;
                                    info!("USB: key down keycode={}", keycode);
                                }
                            } else if matches!(button, Button::A | Button::B | Button::C) {
                                let report = mouse_report(state.mouse_buttons(), 0, 0, 0, 0);
                                send_mouse(&mut mouse_writer, &report, "mouse button down").await;
                            }
                        }
                        InputEvent::ButtonRelease(button) => {
                            state.set_pressed(button, false);
                            if modes.main_mode == MainMode::Keyboard {
                                match button {
                                    Button::A | Button::B | Button::C | Button::D => {
                                        let report = keyboard_report(
                                            state.keyboard_modifier(),
                                            encoder_keycode,
                                        );
                                        send_keyboard(&mut keyboard_writer, &report, "modifier up")
                                            .await;
                                    }
                                    Button::Encoder => {
                                        encoder_keycode = None;
                                        let keyup =
                                            keyboard_release_report(state.keyboard_modifier());
                                        send_keyboard(&mut keyboard_writer, &keyup, "key release")
                                            .await;
                                        info!(
                                            "USB: key release mod={:04b}",
                                            state.keyboard_modifier()
                                        );
                                    }
                                }
                            } else if matches!(button, Button::A | Button::B | Button::C) {
                                let report = mouse_report(state.mouse_buttons(), 0, 0, 0, 0);
                                send_mouse(&mut mouse_writer, &report, "mouse button up").await;
                            }
                        }
                        InputEvent::Rotary(direction) => {
                            if modes.main_mode == MainMode::Mouse {
                                let now = Instant::now();
                                let move_step = rotary_speed.move_step_for_tick(now, &direction);

                                let dir = match direction {
                                    Direction::Clockwise => 1,
                                    Direction::CounterClockwise => -1,
                                };
                                let scroll_step = SCROLL_STEP;

                                let report = if modes.pointer_mode == PointerMode::Move {
                                    let step = move_step * dir;
                                    if modes.movement_axis == MovementAxis::Y {
                                        mouse_report(state.mouse_buttons(), 0, step, 0, 0)
                                    } else {
                                        mouse_report(state.mouse_buttons(), step, 0, 0, 0)
                                    }
                                } else if modes.movement_axis == MovementAxis::Y {
                                    mouse_report(state.mouse_buttons(), 0, 0, scroll_step * dir, 0)
                                } else {
                                    mouse_report(state.mouse_buttons(), 0, 0, 0, scroll_step * dir)
                                };

                                send_mouse(&mut mouse_writer, &report, "rotary action").await;
                            }
                        }
                    }
                }
            }
        }
    };

    join(usb_fut, hid_fut).await;
}
