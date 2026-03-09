use embassy_rp::pio_programs::rotary_encoder::Direction;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

pub enum Button {
    A,
    B,
    C,
    Mode,
}

pub enum UiEvent {
    ButtonPress(Button),
    Rotary(Direction),
    EncoderPush,
}

pub static EVENT_CH: Channel<CriticalSectionRawMutex, UiEvent, 10> = Channel::new();
