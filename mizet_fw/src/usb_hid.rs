use core::sync::atomic::Ordering;

use crate::{
    keymap::KEYMAP,
    shared::{
        Button, CURRENT_INDEX, INPUT_CH, InputEvent, MODE_CH, MainMode, ModeChange,
        MovementAxis, PointerMode, load_main_mode, load_movement_axis, load_pointer_mode,
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

const ENTER_STRIDE_DT_MS: u64 = 70;
const EXIT_STRIDE_DT_MS: u64 = 120;
const PRECISE_MOVE_STEP: i8 = 5;
const STRIDE_MOVE_STEP: i8 = 30;
const PRECISE_SCROLL_STEP: i8 = 1;
const STRIDE_SCROLL_STEP: i8 = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
enum RotaryDeltaMode {
    Precise,
    Stride,
}

impl RotaryDeltaMode {
    fn move_step(self) -> i8 {
        match self {
            Self::Precise => PRECISE_MOVE_STEP,
            Self::Stride => STRIDE_MOVE_STEP,
        }
    }

    fn scroll_step(self) -> i8 {
        match self {
            Self::Precise => PRECISE_SCROLL_STEP,
            Self::Stride => STRIDE_SCROLL_STEP,
        }
    }
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
    let mut rotary_mode = RotaryDeltaMode::Precise;
    let mut last_rotary_at: Option<Instant> = None;

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
                        rotary_mode = RotaryDeltaMode::Precise;
                        last_rotary_at = None;
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
                    let main_mode = load_main_mode();
                    match event {
                        InputEvent::ButtonPress(button) => {
                            state.set_pressed(button, true);
                            if main_mode == MainMode::Keyboard {
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
                            if main_mode == MainMode::Keyboard {
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
                            if main_mode == MainMode::Mouse {
                                let pointer_mode = load_pointer_mode();
                                let movement_axis = load_movement_axis();
                                let now = Instant::now();
                                if let Some(last) = last_rotary_at {
                                    let dt_ms = (now - last).as_millis();
                                    match rotary_mode {
                                        RotaryDeltaMode::Precise if dt_ms <= ENTER_STRIDE_DT_MS => {
                                            rotary_mode = RotaryDeltaMode::Stride;
                                            info!("USB: rotary mode -> stride (dt={}ms)", dt_ms);
                                        }
                                        RotaryDeltaMode::Stride if dt_ms >= EXIT_STRIDE_DT_MS => {
                                            rotary_mode = RotaryDeltaMode::Precise;
                                            info!("USB: rotary mode -> precise (dt={}ms)", dt_ms);
                                        }
                                        _ => {}
                                    }
                                }
                                last_rotary_at = Some(now);

                                let dir = match direction {
                                    crate::shared::Direction::Clockwise => 1,
                                    crate::shared::Direction::CounterClockwise => -1,
                                };
                                let move_step = rotary_mode.move_step();
                                let scroll_step = rotary_mode.scroll_step();

                                let report = if pointer_mode == PointerMode::Move {
                                    if movement_axis == MovementAxis::Y {
                                        mouse_report(
                                            state.mouse_buttons(),
                                            0,
                                            move_step * dir,
                                            0,
                                            0,
                                        )
                                    } else {
                                        mouse_report(
                                            state.mouse_buttons(),
                                            move_step * dir,
                                            0,
                                            0,
                                            0,
                                        )
                                    }
                                } else if movement_axis == MovementAxis::Y {
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
