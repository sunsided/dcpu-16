//! A keyboard that feeds typed keys into a ring buffer in RAM.

use super::{Device, KEYBOARD_BUFFER, KEYBOARD_BUFFER_LEN};
use crate::NUM_RAM_WORDS;
use std::sync::mpsc::{Receiver, Sender, channel};

/// Writes typed keys into the ring buffer at [`KEYBOARD_BUFFER`].
///
/// Keys are written to the slots in turn, wrapping around after the last one.
/// A program reads the slots in the same order: a non-zero word is a key, which
/// the program takes by clearing the slot to zero. Keys wait until the next
/// slot is free, so none are lost when the program falls behind.
///
/// Key codes are up to the sender; ASCII with `\n` for Enter is the convention.
pub struct Keyboard {
    /// Keys sent through [`KeyboardInput`] that are not in RAM yet.
    keys: Receiver<u16>,
    /// The slot the next key is written to.
    next_slot: u16,
}

/// Sends keys to a [`Keyboard`], e.g. from an input thread.
#[derive(Clone)]
pub struct KeyboardInput {
    /// The sending side of the keyboard's queue.
    keys: Sender<u16>,
}

impl Keyboard {
    /// Creates a keyboard and the handle to type on it.
    pub fn new() -> (Self, KeyboardInput) {
        let (sender, receiver) = channel();
        let keyboard = Self {
            keys: receiver,
            next_slot: 0,
        };
        (keyboard, KeyboardInput { keys: sender })
    }
}

impl KeyboardInput {
    /// Types a key. Returns `false` if the keyboard no longer exists.
    pub fn press(&self, key: u16) -> bool {
        self.keys.send(key).is_ok()
    }
}

impl Device for Keyboard {
    fn tick(&mut self, ram: &mut [u16; NUM_RAM_WORDS]) {
        loop {
            let slot = usize::from(KEYBOARD_BUFFER + self.next_slot);
            if ram[slot] != 0 {
                return;
            }
            let Ok(key) = self.keys.try_recv() else {
                return;
            };
            ram[slot] = key;
            self.next_slot = (self.next_slot + 1) % KEYBOARD_BUFFER_LEN;
        }
    }
}
