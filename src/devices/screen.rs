//! The 32×12 character screen in video RAM.
//!
//! The screen needs no ticking: frontends read the cells straight from RAM,
//! e.g. through [`DCPU16::ram`](crate::DCPU16::ram), whenever they redraw.
//! Each cell is one word laid out as `ffff bbbb B ccccccc`: the foreground and
//! background colors index [`PALETTE`], `B` makes the character blink, and the
//! low seven bits are the character.

use super::VIDEO_RAM;
use crate::NUM_RAM_WORDS;

/// Number of character columns.
pub const WIDTH: usize = 32;
/// Number of character rows.
pub const HEIGHT: usize = 12;

/// The 16 colors cells refer to, as `0x0RGB` with four bits per channel
/// (the default palette of the LEM1802 monitor).
pub const PALETTE: [u16; 16] = [
    0x000, 0x00a, 0x0a0, 0x0aa, 0xa00, 0xa0a, 0xa50, 0xaaa, 0x555, 0x55f, 0x5f5, 0x5ff, 0xf55,
    0xf5f, 0xff5, 0xfff,
];

/// A decoded screen cell.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Cell {
    /// The 7-bit character code; `0` shows nothing.
    pub character: u8,
    /// The foreground color as an index into [`PALETTE`].
    pub foreground: u8,
    /// The background color as an index into [`PALETTE`].
    pub background: u8,
    /// Whether the character blinks.
    pub blink: bool,
}

impl Cell {
    /// Decodes a word of video RAM.
    pub fn from_word(word: u16) -> Self {
        Self {
            character: (word & 0x7f) as u8,
            foreground: (word >> 12) as u8,
            background: ((word >> 8) & 0xf) as u8,
            blink: word & 0x80 != 0,
        }
    }
}

/// Gets the screen cells row by row.
pub fn cells(ram: &[u16; NUM_RAM_WORDS]) -> impl Iterator<Item = Cell> + '_ {
    let start = usize::from(VIDEO_RAM);
    ram[start..start + WIDTH * HEIGHT]
        .iter()
        .map(|&word| Cell::from_word(word))
}

/// Renders the characters on the screen as text, one line per row, ignoring colors.
///
/// Empty cells and control characters become spaces.
pub fn text(ram: &[u16; NUM_RAM_WORDS]) -> String {
    let mut text = String::with_capacity((WIDTH + 1) * HEIGHT);
    for (index, cell) in cells(ram).enumerate() {
        if index > 0 && index % WIDTH == 0 {
            text.push('\n');
        }
        let c = char::from(cell.character);
        text.push(if c.is_ascii_control() { ' ' } else { c });
    }
    text
}
