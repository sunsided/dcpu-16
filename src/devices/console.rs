//! A console that prints the characters a program writes to one word of RAM.

use super::{CONSOLE_OUTPUT, Device};
use crate::NUM_RAM_WORDS;
use std::io::Write;
use tracing::warn;

/// Prints every character a program writes to [`CONSOLE_OUTPUT`].
///
/// A non-zero word is printed as the Unicode character it encodes and then cleared
/// to zero, which tells the program the console is ready for the next character.
/// Words that are no valid character, such as surrogates, print as `U+FFFD`.
pub struct Console<W> {
    /// Where characters are written to.
    writer: W,
}

impl<W: Write> Console<W> {
    /// Creates a console that writes to `writer`, e.g. [`std::io::stdout`].
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Gets the writer back, e.g. to inspect what was printed.
    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl<W: Write> Device for Console<W> {
    fn tick(&mut self, ram: &mut [u16; NUM_RAM_WORDS]) {
        let word = std::mem::take(&mut ram[usize::from(CONSOLE_OUTPUT)]);
        if word == 0 {
            return;
        }

        let c = char::from_u32(word.into()).unwrap_or(char::REPLACEMENT_CHARACTER);
        let mut buffer = [0; 4];
        // Flushing makes characters appear right away, even without a line break.
        let result = self
            .writer
            .write_all(c.encode_utf8(&mut buffer).as_bytes())
            .and_then(|()| self.writer.flush());
        if let Err(error) = result {
            warn!("Console output failed: {error}");
        }
    }
}
