use crate::shared::{EVENT_CH, UiEvent};

use embassy_rp::peripherals::PIO0;
use embassy_rp::pio_programs::rotary_encoder::{Direction, PioEncoder};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task]
pub async fn encoder_task(mut encoder: PioEncoder<'static, PIO0, 0>) {
    loop {
        match encoder.read().await {
            Direction::Clockwise => {
                EVENT_CH.send(UiEvent::Rotary(Direction::Clockwise)).await;
            }
            Direction::CounterClockwise => {
                EVENT_CH
                    .send(UiEvent::Rotary(Direction::CounterClockwise))
                    .await;
            }
        };
    }
}
