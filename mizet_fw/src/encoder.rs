use core::sync::atomic::Ordering;

use crate::keymap::{get_next_idx, get_prev_idx};
use crate::shared::{CURRENT_INDEX, EVENT_CH, UiEvent};

use embassy_rp::peripherals::PIO0;
use embassy_rp::pio_programs::rotary_encoder::{Direction, PioEncoder};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task]
pub async fn encoder_task(mut encoder: PioEncoder<'static, PIO0, 0>) {
    let publisher = EVENT_CH.publisher().unwrap();
    loop {
        let direction = encoder.read().await;
        let current_index = CURRENT_INDEX.load(Ordering::Relaxed);

        match direction {
            Direction::Clockwise => {
                CURRENT_INDEX.store(get_next_idx(current_index), Ordering::Relaxed);

                publisher
                    .publish(UiEvent::Rotary(crate::shared::Direction::Clockwise))
                    .await;
            }
            Direction::CounterClockwise => {
                CURRENT_INDEX.store(get_prev_idx(current_index), Ordering::Relaxed);

                publisher
                    .publish(UiEvent::Rotary(crate::shared::Direction::CounterClockwise))
                    .await;
            }
        };
    }
}
