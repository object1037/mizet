use crate::shared::{
    Button, INPUT_CH, InputEvent, MODE_CH, MainMode, ModeChange, load_main_mode,
    toggle_main_mode, toggle_movement_axis, toggle_pointer_mode,
};

use defmt::*;
use embassy_time::Instant;

use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task]
pub async fn input_handler_task() {
    let mode_publisher = MODE_CH.publisher().unwrap();
    let mut subscriber = INPUT_CH.subscriber().unwrap();

    let mut button_d_pressed: bool = false;
    let mut button_d_press_start: Option<Instant> = None;

    loop {
        let event = subscriber.next_message_pure().await;

        match event {
            InputEvent::ButtonPress(button) => {
                if let Button::D = button {
                    button_d_pressed = true;
                    button_d_press_start = Some(Instant::now());
                }

                if let Button::Encoder = button {
                    let main_mode = load_main_mode();
                    if main_mode == MainMode::Mouse {
                        if button_d_pressed {
                            toggle_pointer_mode();
                            info!("Encoder + D: toggling move/scroll mode");
                        } else {
                            toggle_movement_axis();
                            info!("Encoder: toggling X/Y axis");
                        }
                        mode_publisher.publish(ModeChange::SubMode).await;
                    }
                }
            }
            InputEvent::ButtonRelease(Button::D) => {
                if let Some(start_time) = button_d_press_start {
                    let press_duration = Instant::now() - start_time;
                    if press_duration.as_millis() < 250 {
                        toggle_main_mode();
                        mode_publisher.publish(ModeChange::MainMode).await;
                        info!("Button D tapped: toggling keyboard/mouse mode");
                    }
                }
                button_d_pressed = false;
                button_d_press_start = None;
            }
            _ => {}
        }
    }
}
