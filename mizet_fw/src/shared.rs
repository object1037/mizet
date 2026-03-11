use defmt::Format;
use embassy_rp::pio_programs::rotary_encoder::Direction;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

#[derive(Format, Clone, Copy)]
pub enum Button {
    A,
    B,
    C,
    D,
    Encoder,
}

pub enum UiEvent {
    ButtonPress(Button),
    ButtonRelease(Button),
    Rotary(Direction),
    ModeToggle,
}

pub static EVENT_CH: Channel<CriticalSectionRawMutex, UiEvent, 10> = Channel::new();
