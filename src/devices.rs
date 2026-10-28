//! Hardware that talks to programs through memory-mapped RAM.
//!
//! The addresses follow the conventions that grew around the DCPU-16 1.1
//! specification, which itself defines no I/O:
//!
//! | Address           | Device                                              |
//! |-------------------|-----------------------------------------------------|
//! | `0x8000`–`0x817f` | [`screen`]: 32×12 character cells                   |
//! | `0x9000`–`0x900f` | [`Keyboard`]: ring buffer of typed keys             |
//! | `0x9010`          | [`Console`]: write a character to print it          |
//!
//! Devices are attached with [`DCPU16::attach`](crate::DCPU16::attach) and
//! ticked after every executed instruction.

mod console;
mod keyboard;
pub mod screen;

pub use self::console::Console;
pub use self::keyboard::{Keyboard, KeyboardInput};
use crate::NUM_RAM_WORDS;

/// First word of the video RAM read by [`screen`].
pub const VIDEO_RAM: u16 = 0x8000;
/// First word of the [`Keyboard`] ring buffer.
pub const KEYBOARD_BUFFER: u16 = 0x9000;
/// Number of words in the [`Keyboard`] ring buffer.
pub const KEYBOARD_BUFFER_LEN: u16 = 16;
/// The word the [`Console`] prints characters from.
pub const CONSOLE_OUTPUT: u16 = 0x9010;

/// Hardware attached to the CPU that communicates through RAM.
pub trait Device {
    /// Called after every executed instruction; may read and write RAM.
    fn tick(&mut self, ram: &mut [u16; NUM_RAM_WORDS]);
}
