//! Colors and styles of the phosphor ship-console look: amber chrome and
//! green phosphor on near-black, with a single cyan accent.

use ratatui::style::{Color, Modifier, Style};

/// Background of the whole interface.
pub const BG: Color = Color::Rgb(9, 10, 14);
/// Background of the current line in the code panel.
pub const BG_CURSOR: Color = Color::Rgb(46, 34, 8);
/// Background of the emulated screen.
pub const SCREEN_BG: Color = Color::Rgb(0, 0, 0);
/// Background of every other screen row, for a faint scanline effect.
pub const SCANLINE: Color = Color::Rgb(7, 12, 8);

/// Primary text: warm white.
pub const FG: Color = Color::Rgb(222, 214, 196);
/// Secondary text: labels, hints, comments.
pub const DIM: Color = Color::Rgb(122, 112, 94);
/// Panel borders.
pub const BORDER: Color = Color::Rgb(96, 72, 30);

/// Amber phosphor: titles, keycaps, the screen bezel.
pub const AMBER: Color = Color::Rgb(255, 176, 0);
/// Green phosphor: values and screen text.
pub const GREEN: Color = Color::Rgb(57, 255, 120);
/// Electric cyan: mnemonics and changed values.
pub const CYAN: Color = Color::Rgb(0, 220, 255);
/// Laser yellow: numbers.
pub const YELLOW: Color = Color::Rgb(255, 228, 77);
/// Ultraviolet: memory brackets and data.
pub const PURPLE: Color = Color::Rgb(168, 92, 255);
/// Sunset orange: special registers and stack operations.
pub const ORANGE: Color = Color::Rgb(255, 140, 40);
/// Warning red: a halted machine.
pub const RED: Color = Color::Rgb(255, 56, 96);

/// Plain style with foreground `color`.
pub fn fg(color: Color) -> Style {
    Style::new().fg(color)
}

/// Bold style with foreground `color`.
pub fn bold(color: Color) -> Style {
    fg(color).add_modifier(Modifier::BOLD)
}
