use defmt::Format;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pubsub::PubSubChannel;

#[derive(Format, Clone, Copy)]
pub enum Button {
    A,
    B,
    C,
    D,
    Encoder,
}

#[derive(Clone)]
pub enum Direction {
    Clockwise,
    CounterClockwise,
}

#[derive(Clone)]
pub enum UiEvent {
    ButtonPress(Button),
    ButtonRelease(Button),
    Rotary(Direction),
    ModeToggle,
}

pub enum Mode {
    Keyboard,
    Mouse,
}

pub static EVENT_CH: PubSubChannel<CriticalSectionRawMutex, UiEvent, 10, 2, 6> =
    PubSubChannel::<CriticalSectionRawMutex, UiEvent, 10, 2, 6>::new();
