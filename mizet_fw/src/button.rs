use core::sync::atomic::Ordering;

use crate::{
    IS_KEYBOARD_MODE,
    shared::{Button, EVENT_CH, UiEvent},
};

use defmt::*;
use embassy_rp::gpio::Input;
use embassy_time::Instant;

use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task(pool_size = 5)]
pub async fn button_task(mut button_input: Input<'static>, button_type: Button) {
    loop {
        button_input.wait_for_low().await;
        let start_time = Instant::now();

        EVENT_CH.send(UiEvent::ButtonPress(button_type)).await;
        info!("Button Pressed: {:?}", button_type);

        button_input.wait_for_high().await;
        let end_time = Instant::now();
        let press_duration = end_time - start_time;

        if let Button::D = button_type
            && press_duration.as_millis() < 250
        {
            // Short press button D: Toggle mode.
            let new_mode = !IS_KEYBOARD_MODE.load(Ordering::Relaxed);
            IS_KEYBOARD_MODE.store(new_mode, Ordering::Relaxed);
            EVENT_CH.send(UiEvent::ModeToggle).await;
            continue;
        }

        EVENT_CH.send(UiEvent::ButtonRelease(button_type)).await;
        info!(
            "Button Released: {:?}. Duration: {:?} ms",
            button_type,
            press_duration.as_millis()
        );
    }
}
