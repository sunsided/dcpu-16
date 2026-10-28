//! Drawing of the screen, console, registers and code panels.

use crate::app::{App, State};
use dcpu16::Register;
use dcpu16::devices::screen::{self, Cell, PALETTE};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

/// Width of the screen panel: the screen plus its border.
const SCREEN_PANEL_WIDTH: u16 = screen::WIDTH as u16 + 2;
/// Height of the screen panel: the screen plus its border.
const SCREEN_PANEL_HEIGHT: u16 = screen::HEIGHT as u16 + 2;

/// Draws the whole interface.
pub fn draw(frame: &mut Frame, app: &App, blink_on: bool) {
    let [body, status] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
    let [left, right] =
        Layout::horizontal([Constraint::Length(SCREEN_PANEL_WIDTH), Constraint::Min(24)])
            .areas(body);
    let [screen_area, console_area] =
        Layout::vertical([Constraint::Length(SCREEN_PANEL_HEIGHT), Constraint::Min(3)]).areas(left);
    let [registers_area, code_area] =
        Layout::vertical([Constraint::Length(9), Constraint::Min(3)]).areas(right);

    draw_screen(frame, screen_area, app, blink_on);
    draw_console(frame, console_area, app);
    draw_registers(frame, registers_area, app);
    draw_code(frame, code_area, app);
    draw_status(frame, status, app);
}

/// Draws the 32×12 character screen from video RAM.
fn draw_screen(frame: &mut Frame, area: Rect, app: &App, blink_on: bool) {
    let cells: Vec<Cell> = screen::cells(app.cpu.ram()).collect();
    let lines: Vec<Line> = cells
        .chunks(screen::WIDTH)
        .map(|row| {
            row.iter()
                .map(|cell| cell_span(*cell, blink_on))
                .collect::<Line>()
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(" Screen ")),
        area,
    );
}

/// Renders one screen cell.
///
/// Cells without any color bits, such as plain ASCII written to video RAM, use
/// the terminal's colors instead of black on black, so they stay readable.
fn cell_span(cell: Cell, blink_on: bool) -> Span<'static> {
    let visible = cell.character != 0 && !(cell.blink && !blink_on);
    let c = char::from(cell.character);
    let symbol = if visible && !c.is_ascii_control() {
        c
    } else {
        ' '
    };

    let style = if cell.foreground == 0 && cell.background == 0 {
        Style::new()
    } else {
        Style::new()
            .fg(color(cell.foreground))
            .bg(color(cell.background))
    };
    Span::styled(symbol.to_string(), style)
}

/// Converts a palette index into a terminal color.
fn color(index: u8) -> Color {
    let rgb = PALETTE[usize::from(index & 0xf)];
    // Each 4-bit channel scales to 8 bits by repeating it: 0xa becomes 0xaa.
    let channel = |shift: u16| ((rgb >> shift) & 0xf) as u8 * 0x11;
    Color::Rgb(channel(8), channel(4), channel(0))
}

/// Draws the tail of the console output.
fn draw_console(frame: &mut Frame, area: Rect, app: &App) {
    let text = app.output.text();
    let height = usize::from(area.height.saturating_sub(2));
    let lines: Vec<&str> = text.lines().collect();
    // A trailing line break starts a new, still empty line.
    let mut visible: Vec<Line> = lines
        .iter()
        .skip(lines.len().saturating_sub(height))
        .map(|line| Line::raw(*line))
        .collect();
    if text.ends_with('\n') && visible.len() == height {
        visible.remove(0);
    }
    frame.render_widget(
        Paragraph::new(visible).block(Block::bordered().title(" Console ")),
        area,
    );
}

/// Draws the registers and the cycle count.
fn draw_registers(frame: &mut Frame, area: Rect, app: &App) {
    let cpu = &app.cpu;
    let dim = Style::new().add_modifier(Modifier::DIM);
    let value = |name: &str, value: u16| {
        [
            Span::styled(format!("{name:<3}"), dim),
            Span::raw(format!("{value:04X}   ")),
        ]
    };
    let pair = |a: (&str, u16), b: (&str, u16)| {
        Line::from_iter(value(a.0, a.1).into_iter().chain(value(b.0, b.1)))
    };
    let register = |r: Register| cpu.register(r);
    let lines = vec![
        pair(("A", register(Register::A)), ("B", register(Register::B))),
        pair(("C", register(Register::C)), ("X", register(Register::X))),
        pair(("Y", register(Register::Y)), ("Z", register(Register::Z))),
        pair(("I", register(Register::I)), ("J", register(Register::J))),
        pair(("PC", cpu.program_counter), ("SP", cpu.stack_pointer)),
        Line::from_iter(value("O", cpu.overflow)),
        Line::from(vec![
            Span::styled("cycles ", dim),
            Span::raw(cpu.cycles().to_string()),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(" Registers ")),
        area,
    );
}

/// Draws the instructions starting at the program counter.
fn draw_code(frame: &mut Frame, area: Rect, app: &App) {
    let mut address = app.cpu.program_counter;
    let mut lines = Vec::new();
    for index in 0..area.height.saturating_sub(2) {
        let disassembly = app.cpu.disassemble(address);
        let text = format!("{address:04X}  {}", disassembly.text);
        lines.push(if index == 0 {
            Line::styled(
                format!("▶ {text}"),
                Style::new().add_modifier(Modifier::REVERSED),
            )
        } else {
            Line::raw(format!("  {text}"))
        });
        address = address.wrapping_add(disassembly.length);
    }
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(" Code ")),
        area,
    );
}

/// Draws the execution state, the speed and the controls.
fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let (label, color) = match app.state {
        State::Running => (" RUNNING ", Color::Green),
        State::Paused => (" PAUSED ", Color::Yellow),
        State::Halted => (" HALTED ", Color::Red),
    };
    let line = Line::from(vec![
        Span::styled(label, Style::new().fg(Color::Black).bg(color)),
        Span::raw(format!(" {} ", format_speed(app.speed()))),
        Span::styled(
            " F5 run/pause  F10 step  F2 reset  F7/F8 speed  Ctrl+C quit",
            Style::new().add_modifier(Modifier::DIM),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

/// Formats a speed in cycles per second, e.g. `100 kHz`.
fn format_speed(hz: u64) -> String {
    match hz {
        1_000_000.. => format!("{} MHz", hz / 1_000_000),
        1_000.. => format!("{} kHz", hz / 1_000),
        _ => format!("{hz} Hz"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::time::Duration;

    /// Renders `app` and returns the lines of the terminal.
    fn render(app: &App) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| draw(frame, app, true)).unwrap();
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn shows_screen_console_registers_and_code() {
        let program = dcpu16::assemble(
            "SET [0x8000], 0xf048\nSET [0x8001], 0x0069\nSET [0x9010], 0x6f\nSET A, 0x1234\n:end SET PC, end",
        )
        .unwrap();
        let mut app = App::new(program);
        app.advance(Duration::from_millis(10));

        let lines = render(&app);
        assert!(lines[1].starts_with("│Hi "), "{}", lines[1]);
        assert!(lines[15].starts_with("│o "), "{}", lines[15]);
        assert!(lines.iter().any(|line| line.contains("A  1234")));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("▶ 000B  SET PC, 0x0B"))
        );
        assert!(lines[23].starts_with(" HALTED  100 kHz"), "{}", lines[23]);
    }

    #[test]
    fn colors_cells_from_the_palette() {
        let span = cell_span(Cell::from_word(0xe148), true);
        assert_eq!(span.content, "H");
        assert_eq!(span.style.fg, Some(Color::Rgb(0xff, 0xff, 0x55)));
        assert_eq!(span.style.bg, Some(Color::Rgb(0x00, 0x00, 0xaa)));

        // Plain characters keep the terminal colors.
        assert_eq!(cell_span(Cell::from_word(0x0048), true).style, Style::new());
    }

    #[test]
    fn hides_blinking_characters_in_the_off_phase() {
        let cell = Cell::from_word(0xf0c8);
        assert_eq!(cell_span(cell, true).content, "H");
        assert_eq!(cell_span(cell, false).content, " ");
    }

    #[test]
    fn formats_speeds() {
        assert_eq!(format_speed(1_000), "1 kHz");
        assert_eq!(format_speed(100_000), "100 kHz");
        assert_eq!(format_speed(10_000_000), "10 MHz");
        assert_eq!(format_speed(500), "500 Hz");
    }
}
