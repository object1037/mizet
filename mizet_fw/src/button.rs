use core::sync::atomic::Ordering;

use crate::shared::{Button, EVENT_CH, IS_BUTTON_D_PRESSED, IS_KEYBOARD_MODE, IS_MOVE_MODE, IS_MOVEMENT_Y, UiEvent};

use defmt::*;
use embassy_rp::gpio::Input;
use embassy_time::Instant;

use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task(pool_size = 5)]
pub async fn button_task(mut button_input: Input<'static>, button_type: Button) {
    let publisher = EVENT_CH.publisher().unwrap();
    loop {
        button_input.wait_for_low().await;
        let start_time = Instant::now();

        publisher.publish(UiEvent::ButtonPress(button_type)).await;
        info!("Button Pressed: {:?}", button_type);

        if let Button::D = button_type {
            IS_BUTTON_D_PRESSED.store(true, Ordering::Relaxed);
        }

        button_input.wait_for_high().await;
        let end_time = Instant::now();
        let press_duration = end_time - start_time;

        if let Button::D = button_type
            && press_duration.as_millis() < 250
        {
            let is_keyboard_mode = IS_KEYBOARD_MODE.load(Ordering::Relaxed);
            IS_KEYBOARD_MODE.store(!is_keyboard_mode, Ordering::Relaxed);
            publisher.publish(UiEvent::ModeToggle).await;
        }

        if let Button::Encoder = button_type {
            let is_keyboard_mode = IS_KEYBOARD_MODE.load(Ordering::Relaxed);
            if !is_keyboard_mode {
                let is_d_pressed = IS_BUTTON_D_PRESSED.load(Ordering::Relaxed);
                if is_d_pressed {
                    let is_move_mode = IS_MOVE_MODE.load(Ordering::Relaxed);
                    IS_MOVE_MODE.store(!is_move_mode, Ordering::Relaxed);
                } else {
                    let is_movement_y = IS_MOVEMENT_Y.load(Ordering::Relaxed);
                    IS_MOVEMENT_Y.store(!is_movement_y, Ordering::Relaxed);
                }
            }
        }

        if let Button::D = button_type {
            IS_BUTTON_D_PRESSED.store(false, Ordering::Relaxed);
        }

        publisher.publish(UiEvent::ButtonRelease(button_type)).await;
        info!(
            "Button Released: {:?}. Duration: {:?} ms",
            button_type,
            press_duration.as_millis()
        );
    }
}
