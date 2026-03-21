use crate::shared::{Button, INPUT_CH, InputEvent};

use embassy_rp::gpio::Input;
use embassy_time::{Duration, Timer};

use {defmt_rtt as _, panic_probe as _};

const DEBOUNCE_DELAY_MS: u64 = 8;

#[embassy_executor::task(pool_size = 5)]
pub async fn button_task(mut button_input: Input<'static>, button_type: Button) {
    let publisher = INPUT_CH.publisher().unwrap();
    loop {
        button_input.wait_for_low().await;
        Timer::after(Duration::from_millis(DEBOUNCE_DELAY_MS)).await;
        if button_input.is_low() {
            publisher
                .publish(InputEvent::ButtonPress(button_type))
                .await;
        }

        button_input.wait_for_high().await;
        Timer::after(Duration::from_millis(DEBOUNCE_DELAY_MS)).await;
        if button_input.is_high() {
            publisher
                .publish(InputEvent::ButtonRelease(button_type))
                .await;
        }
    }
}
