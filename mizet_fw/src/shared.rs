use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

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
pub enum InputEvent {
    ButtonPress(Button),
    ButtonRelease(Button),
    Rotary(Direction),
}

#[derive(Clone, Copy)]
pub enum ModeChange {
    MainMode,
    SubMode,
}

#[repr(u8)]
#[derive(Format, Clone, Copy, PartialEq, Eq)]
pub enum MainMode {
    Keyboard = 0,
    Mouse = 1,
}

impl From<MainMode> for u8 {
    fn from(value: MainMode) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for MainMode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Keyboard),
            1 => Ok(Self::Mouse),
            _ => Err(()),
        }
    }
}

#[repr(u8)]
#[derive(Format, Clone, Copy, PartialEq, Eq)]
pub enum PointerMode {
    Move = 0,
    Scroll = 1,
}

impl From<PointerMode> for u8 {
    fn from(value: PointerMode) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for PointerMode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Move),
            1 => Ok(Self::Scroll),
            _ => Err(()),
        }
    }
}

#[repr(u8)]
#[derive(Format, Clone, Copy, PartialEq, Eq)]
pub enum MovementAxis {
    Y = 0,
    X = 1,
}

impl From<MovementAxis> for u8 {
    fn from(value: MovementAxis) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for MovementAxis {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Y),
            1 => Ok(Self::X),
            _ => Err(()),
        }
    }
}

pub static INPUT_CH: PubSubChannel<CriticalSectionRawMutex, InputEvent, 4, 3, 6> =
    PubSubChannel::<CriticalSectionRawMutex, InputEvent, 4, 3, 6>::new();

pub static MODE_CH: PubSubChannel<CriticalSectionRawMutex, ModeChange, 1, 2, 1> =
    PubSubChannel::<CriticalSectionRawMutex, ModeChange, 1, 2, 1>::new();

pub static MAIN_MODE: AtomicU8 = AtomicU8::new(MainMode::Keyboard as u8);
pub static POINTER_MODE: AtomicU8 = AtomicU8::new(PointerMode::Move as u8);
pub static MOVEMENT_AXIS: AtomicU8 = AtomicU8::new(MovementAxis::Y as u8);
pub static CURRENT_INDEX: AtomicUsize = AtomicUsize::new(0);

pub fn load_main_mode() -> MainMode {
    MainMode::try_from(MAIN_MODE.load(Ordering::Relaxed)).unwrap_or(MainMode::Keyboard)
}

pub fn store_main_mode(mode: MainMode) {
    MAIN_MODE.store(mode.into(), Ordering::Relaxed);
}

pub fn toggle_main_mode() -> MainMode {
    let next = match load_main_mode() {
        MainMode::Keyboard => MainMode::Mouse,
        MainMode::Mouse => MainMode::Keyboard,
    };
    store_main_mode(next);
    next
}

pub fn load_pointer_mode() -> PointerMode {
    PointerMode::try_from(POINTER_MODE.load(Ordering::Relaxed)).unwrap_or(PointerMode::Move)
}

pub fn store_pointer_mode(mode: PointerMode) {
    POINTER_MODE.store(mode.into(), Ordering::Relaxed);
}

pub fn toggle_pointer_mode() -> PointerMode {
    let next = match load_pointer_mode() {
        PointerMode::Move => PointerMode::Scroll,
        PointerMode::Scroll => PointerMode::Move,
    };
    store_pointer_mode(next);
    next
}

pub fn load_movement_axis() -> MovementAxis {
    MovementAxis::try_from(MOVEMENT_AXIS.load(Ordering::Relaxed)).unwrap_or(MovementAxis::Y)
}

pub fn store_movement_axis(axis: MovementAxis) {
    MOVEMENT_AXIS.store(axis.into(), Ordering::Relaxed);
}

pub fn toggle_movement_axis() -> MovementAxis {
    let next = match load_movement_axis() {
        MovementAxis::Y => MovementAxis::X,
        MovementAxis::X => MovementAxis::Y,
    };
    store_movement_axis(next);
    next
}
