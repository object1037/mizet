# mizet: RP2040-based input device with OLED display and rotary encoder

mizet is an input device that acts as a USB HID keyboard and mouse, featuring an OLED display and a rotary encoder.

## Project Structure

- `hardware/`: Contains hardware design files, including schematics and PCB layouts.
- `mizet_fw/`: Contains the firmware source code for the device.

## Hardware Overview

The device has 4 buttons (A, B, C, D), a rotary encoder with a push switch, and an OLED display.

## Firmware Overview

The firmware uses the Embassy framework.
- `mizet_fw/src/main.rs`: The main entry point of the firmware, responsible for initializing hardware and spawning tasks.
- `mizet_fw/src/display.rs`: Contains code related to the OLED display, including rendering the UI and handling display updates.
- `mizet_fw/src/keymap.rs`: Contains the key mapping.
- `mizet_fw/src/button.rs`: Contains task for handling button inputs.
- `mizet_fw/src/encoder.rs`: Contains task for handling the rotary encoder input.
- `mizet_fw/src/usb_hid.rs`: Contains task for handling USB HID communication.
- `mizet_fw/src/shared.rs`: Contains shared data structures and state used across different tasks in the firmware.

## Modes

The device has two main modes: Keyboard Mode and Mouse Mode.

### Keyboard Mode

- button A, B, C act as Ctrl, Alt, Gui modifiers, respectively.
- button D acts as a Shift key when held, and as a mode switch button when tapped.
- The rotary encoder is used to scroll through a list of keys displayed on the OLED. Pressing the encoder's push switch sends the selected key as a keyboard input.

### Mouse Mode

Mouse mode has two sub-modes: Movement Mode and Scroll Mode. The current sub-mode is indicated on the OLED display.

- button A, B, C act as Right Click, Middle Click, and Left Click, respectively.
- button D acts as a mode switch button when tapped.
- To switch between Movement Mode and Scroll Mode, hold button D and press the encoder's push switch.
- To switch the scroll/movement axis, press the encoder's push switch.
- In Movement Mode, the rotary encoder controls the mouse cursor movement.
- In Scroll Mode, the rotary encoder controls scrolling.
