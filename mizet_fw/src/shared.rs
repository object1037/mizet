use core::sync::atomic::{AtomicBool, AtomicUsize};

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
}

#[derive(Clone, Copy)]
pub enum ModeChange {
    MainMode,
    SubMode,
}

pub static EVENT_CH: PubSubChannel<CriticalSectionRawMutex, UiEvent, 4, 3, 6> =
    PubSubChannel::<CriticalSectionRawMutex, UiEvent, 4, 3, 6>::new();

pub static MODE_CH: PubSubChannel<CriticalSectionRawMutex, ModeChange, 1, 2, 1> =
    PubSubChannel::<CriticalSectionRawMutex, ModeChange, 1, 2, 1>::new();

pub static IS_KEYBOARD_MODE: AtomicBool = AtomicBool::new(true);
pub static IS_MOVE_MODE: AtomicBool = AtomicBool::new(true);
pub static IS_MOVEMENT_Y: AtomicBool = AtomicBool::new(true);
pub static CURRENT_INDEX: AtomicUsize = AtomicUsize::new(0);
