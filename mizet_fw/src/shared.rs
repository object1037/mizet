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

const MAIN_MODE_BIT: u8 = 0b0000_0001;
const POINTER_MODE_BIT: u8 = 0b0000_0010;
const MOVEMENT_AXIS_BIT: u8 = 0b0000_0100;

pub static DEVICE_STATE: AtomicU8 = AtomicU8::new(0);
pub static CURRENT_INDEX: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy)]
pub struct Modes {
    pub main_mode: MainMode,
    pub pointer_mode: PointerMode,
    pub movement_axis: MovementAxis,
}

fn main_mode_from_state(state: u8) -> MainMode {
    if state & MAIN_MODE_BIT == 0 {
        MainMode::Keyboard
    } else {
        MainMode::Mouse
    }
}

fn pointer_mode_from_state(state: u8) -> PointerMode {
    if state & POINTER_MODE_BIT == 0 {
        PointerMode::Move
    } else {
        PointerMode::Scroll
    }
}

fn movement_axis_from_state(state: u8) -> MovementAxis {
    if state & MOVEMENT_AXIS_BIT == 0 {
        MovementAxis::Y
    } else {
        MovementAxis::X
    }
}

fn update_device_state(f: impl FnOnce(u8) -> u8) -> u8 {
    critical_section::with(|_| {
        let current = DEVICE_STATE.load(Ordering::Relaxed);
        let next = f(current);
        DEVICE_STATE.store(next, Ordering::Relaxed);
        next
    })
}

pub fn load_main_mode() -> MainMode {
    main_mode_from_state(DEVICE_STATE.load(Ordering::Relaxed))
}

pub fn toggle_main_mode() -> MainMode {
    let next = update_device_state(|state| state ^ MAIN_MODE_BIT);
    main_mode_from_state(next)
}

pub fn toggle_pointer_mode() -> PointerMode {
    let next = update_device_state(|state| state ^ POINTER_MODE_BIT);
    pointer_mode_from_state(next)
}

pub fn toggle_movement_axis() -> MovementAxis {
    let next = update_device_state(|state| state ^ MOVEMENT_AXIS_BIT);
    movement_axis_from_state(next)
}

pub fn load_modes() -> Modes {
    let state = DEVICE_STATE.load(Ordering::Relaxed);
    Modes {
        main_mode: main_mode_from_state(state),
        pointer_mode: pointer_mode_from_state(state),
        movement_axis: movement_axis_from_state(state),
    }
}
