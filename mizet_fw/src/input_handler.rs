use core::sync::atomic::Ordering;

use crate::shared::{
    Button, EVENT_CH, IS_KEYBOARD_MODE, IS_MOVE_MODE, IS_MOVEMENT_Y, MODE_CH, ModeChange, UiEvent,
};

use defmt::*;
use embassy_time::Instant;

use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task]
pub async fn input_handler_task() {
    let mode_publisher = MODE_CH.publisher().unwrap();
    let mut subscriber = EVENT_CH.subscriber().unwrap();

    let mut button_d_pressed: bool = false;
    let mut button_d_press_start: Option<Instant> = None;

    loop {
        let event = subscriber.next_message_pure().await;

        match event {
            UiEvent::ButtonPress(button) => {
                if let Button::D = button {
                    button_d_pressed = true;
                    button_d_press_start = Some(Instant::now());
                }

                if let Button::Encoder = button {
                    let is_keyboard_mode = IS_KEYBOARD_MODE.load(Ordering::Relaxed);
                    if !is_keyboard_mode {
                        if button_d_pressed {
                            let is_move_mode = IS_MOVE_MODE.load(Ordering::Relaxed);
                            IS_MOVE_MODE.store(!is_move_mode, Ordering::Relaxed);
                            info!("Encoder + D: toggling move/scroll mode");
                        } else {
                            let is_movement_y = IS_MOVEMENT_Y.load(Ordering::Relaxed);
                            IS_MOVEMENT_Y.store(!is_movement_y, Ordering::Relaxed);
                            info!("Encoder: toggling X/Y axis");
                        }
                        mode_publisher.publish(ModeChange::SubMode).await;
                    }
                }
            }
            UiEvent::ButtonRelease(button) => {
                if let Button::D = button {
                    if let Some(start_time) = button_d_press_start {
                        let press_duration = Instant::now() - start_time;
                        if press_duration.as_millis() < 250 {
                            let is_keyboard_mode = IS_KEYBOARD_MODE.load(Ordering::Relaxed);
                            IS_KEYBOARD_MODE.store(!is_keyboard_mode, Ordering::Relaxed);
                            mode_publisher.publish(ModeChange::MainMode).await;
                            info!("Button D tapped: toggling keyboard/mouse mode");
                        }
                    }
                    button_d_pressed = false;
                    button_d_press_start = None;
                }
            }
            _ => {}
        }
    }
}
