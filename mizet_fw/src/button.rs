use crate::shared::{Button, EVENT_CH, UiEvent};

use defmt::*;
use embassy_rp::gpio::Input;

use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task(pool_size = 5)]
pub async fn button_task(mut button_input: Input<'static>, button_type: Button) {
    loop {
        button_input.wait_for_low().await;
        EVENT_CH.send(UiEvent::ButtonPress(button_type)).await;
        info!("Button Pressed: {:?}", button_type);

        button_input.wait_for_high().await;
        EVENT_CH.send(UiEvent::ButtonRelease(button_type)).await;
        info!("Button Released: {:?}", button_type);
    }
}
