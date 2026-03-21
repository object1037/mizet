use crate::shared::{INPUT_CH, InputEvent};

use embassy_rp::peripherals::PIO0;
use embassy_rp::pio_programs::rotary_encoder::{Direction, PioEncoder};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task]
pub async fn encoder_task(mut encoder: PioEncoder<'static, PIO0, 0>) {
    let publisher = INPUT_CH.publisher().unwrap();
    loop {
        match encoder.read().await {
            Direction::Clockwise => {
                publisher
                    .publish(InputEvent::Rotary(crate::shared::Direction::Clockwise))
                    .await;
            }
            Direction::CounterClockwise => {
                publisher
                    .publish(InputEvent::Rotary(
                        crate::shared::Direction::CounterClockwise,
                    ))
                    .await;
            }
        };
    }
}
