use core::sync::atomic::Ordering;

use crate::{
    keymap::KEYMAP,
    shared::{Button, CURRENT_INDEX, EVENT_CH, UiEvent},
};

use defmt::*;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task]
pub async fn usb_task() {
    // Placeholder for USB functionality
    let mut subscriber = EVENT_CH.subscriber().unwrap();

    loop {
        let event = subscriber.next_message_pure().await;

        match event {
            UiEvent::ButtonRelease(Button::Encoder) => {
                let current_index = CURRENT_INDEX.load(Ordering::Relaxed);
                info!("USB Task: Key Input: {:?}", KEYMAP[current_index].keycode);
            }
            _ => (),
        }
    }
}
