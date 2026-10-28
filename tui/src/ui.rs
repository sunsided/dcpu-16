//! Drawing of the header, screen, console, registers, code and key hints.

use crate::app::{App, State, registers};
use crate::theme::{self, AMBER, BG, BG_CURSOR, BORDER, CYAN, DIM, FG, GREEN};
use dcpu16::devices::screen::{self, Cell, PALETTE};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};

/// Width of the screen panel: the screen plus its border.
const SCREEN_PANEL_WIDTH: u16 = screen::WIDTH as u16 + 2;
/// Height of the screen panel: the screen plus its border.
const SCREEN_PANEL_HEIGHT: u16 = screen::HEIGHT as u16 + 2;
/// Width of the instruction column in the code panel, before the description.
const CODE_TEXT_WIDTH: usize = 20;
/// Background of the header and footer bars.
const BAR_BG: Color = Color::Rgb(26, 20, 8);

/// Draws the whole interface.
pub fn draw(frame: &mut Frame, app: &App, blink_on: bool) {
    frame.render_widget(Block::new().style(Style::new().bg(BG).fg(FG)), frame.area());

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    let [left, right] =
        Layout::horizontal([Constraint::Length(SCREEN_PANEL_WIDTH), Constraint::Min(24)])
            .areas(body);
    let [screen_area, console_area] =
        Layout::vertical([Constraint::Length(SCREEN_PANEL_HEIGHT), Constraint::Min(3)]).areas(left);
    let [registers_area, code_area] =
        Layout::vertical([Constraint::Length(9), Constraint::Min(3)]).areas(right);

    draw_header(frame, header, app, blink_on);
    draw_screen(frame, screen_area, app, blink_on);
    draw_console(frame, console_area, app);
    draw_registers(frame, registers_area, app);
    draw_code(frame, code_area, app);
    draw_footer(frame, footer);
}

/// A rounded panel with an amber title.
fn panel(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::fg(BORDER))
        .title(Span::styled(format!(" {title} "), theme::bold(AMBER)))
}

/// Draws the title bar with the program name, the execution state and the speed.
fn draw_header(frame: &mut Frame, area: Rect, app: &App, blink_on: bool) {
    let bar = Style::new().bg(BAR_BG);
    let title = Line::from(vec![
        Span::styled(" ▓▒░ ", theme::fg(AMBER)),
        Span::styled("DCPU-16", theme::bold(AMBER)),
        Span::styled(" ░▒▓  ", theme::fg(AMBER)),
        Span::styled(app.name.as_str(), theme::fg(FG)),
    ]);
    frame.render_widget(Paragraph::new(title).style(bar), area);

    let (label, color) = match app.state {
        State::Running => ("RUNNING", GREEN),
        State::Paused => ("PAUSED", AMBER),
        State::Halted => ("HALTED", theme::RED),
    };
    // The indicator pulses while the machine runs.
    let dot = if app.state == State::Running && !blink_on {
        theme::fg(DIM)
    } else {
        theme::fg(color)
    };
    let status = Line::from(vec![
        Span::styled("● ", dot),
        Span::styled(label, theme::bold(color)),
        Span::styled(format!("  {} ", format_speed(app.speed())), theme::fg(DIM)),
    ])
    .right_aligned();
    frame.render_widget(Paragraph::new(status), area);
}

/// Draws the 32×12 character screen from video RAM inside an amber bezel.
fn draw_screen(frame: &mut Frame, area: Rect, app: &App, blink_on: bool) {
    let cells: Vec<Cell> = screen::cells(app.cpu.ram()).collect();
    let lines: Vec<Line> = cells
        .chunks(screen::WIDTH)
        .enumerate()
        .map(|(row, cells)| {
            let background = if row % 2 == 1 {
                theme::SCANLINE
            } else {
                theme::SCREEN_BG
            };
            cells
                .iter()
                .map(|cell| cell_span(*cell, blink_on, background))
                .collect::<Line>()
        })
        .collect();

    let bezel = Block::bordered()
        .border_type(BorderType::Thick)
        .border_style(theme::fg(AMBER))
        .title(Span::styled(" LEM1802 ", theme::bold(AMBER)))
        .title_bottom(Line::styled(" 0x8000 ", theme::fg(BORDER)).right_aligned());
    frame.render_widget(Paragraph::new(lines).block(bezel), area);
}

/// Renders one screen cell.
///
/// Cells without any color bits, such as plain ASCII written to video RAM, glow in
/// green phosphor on `background` instead of black on black.
fn cell_span(cell: Cell, blink_on: bool, background: Color) -> Span<'static> {
    let visible = cell.character != 0 && !(cell.blink && !blink_on);
    let c = char::from(cell.character);
    let symbol = if visible && !c.is_ascii_control() {
        c
    } else {
        ' '
    };

    let style = if cell.foreground == 0 && cell.background == 0 {
        Style::new().fg(GREEN).bg(background)
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
        Paragraph::new(visible)
            .style(theme::fg(GREEN))
            .block(panel("CONSOLE")),
        area,
    );
}

/// Draws the registers, highlighting those that changed, and the cycle count.
fn draw_registers(frame: &mut Frame, area: Rect, app: &App) {
    const NAMES: [&str; 11] = ["A", "B", "C", "X", "Y", "Z", "I", "J", "PC", "SP", "O"];
    let current = registers(&app.cpu);
    let register = |index: usize| {
        let style = if current[index] == app.previous[index] {
            theme::fg(GREEN)
        } else {
            theme::bold(CYAN)
        };
        [
            Span::styled(format!("{:<3}", NAMES[index]), theme::fg(DIM)),
            Span::styled(format!("{:04X}", current[index]), style),
            Span::raw("   "),
        ]
    };
    let row = |indices: &[usize]| Line::from_iter(indices.iter().flat_map(|&i| register(i)));

    let lines = vec![
        row(&[0, 1]),
        row(&[2, 3]),
        row(&[4, 5]),
        row(&[6, 7]),
        row(&[8, 9]),
        row(&[10]),
        Line::from(vec![
            Span::styled("CYC ", theme::fg(DIM)),
            Span::styled(app.cpu.cycles().to_string(), theme::fg(FG)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines).block(panel("REGISTERS")), area);
}

/// Draws the instructions from the program counter on, with syntax highlighting
/// and a plain-English description of each.
fn draw_code(frame: &mut Frame, area: Rect, app: &App) {
    let mut address = app.cpu.program_counter;
    let mut lines = Vec::new();
    for index in 0..area.height.saturating_sub(2) {
        let disassembly = app.cpu.disassemble(address);
        let current = index == 0;

        let mut spans = vec![
            Span::styled(if current { "▶ " } else { "  " }, theme::bold(AMBER)),
            Span::styled(
                format!("{address:04X}  "),
                theme::fg(if current { AMBER } else { DIM }),
            ),
        ];
        spans.extend(highlight(&disassembly.text));
        let padding = CODE_TEXT_WIDTH.saturating_sub(disassembly.text.chars().count());
        spans.push(Span::raw(" ".repeat(padding + 1)));
        spans.push(Span::styled(
            format!("; {}", disassembly.description),
            theme::fg(DIM).add_modifier(Modifier::ITALIC),
        ));

        let line = Line::from(spans);
        lines.push(if current {
            line.style(Style::new().bg(BG_CURSOR))
        } else {
            line
        });
        address = address.wrapping_add(disassembly.length);
    }
    frame.render_widget(Paragraph::new(lines).block(panel("CODE")), area);
}

/// Splits a disassembled instruction into colored tokens: the mnemonic, registers,
/// special registers and stack operations, numbers, and memory brackets.
fn highlight(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = text;
    let mut mnemonic = true;
    while let Some(c) = rest.chars().next() {
        let length = if c.is_ascii_alphanumeric() {
            rest.find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(rest.len())
        } else {
            c.len_utf8()
        };
        let (token, tail) = rest.split_at(length);
        rest = tail;

        let style = if c.is_ascii_alphanumeric() {
            let style = match token {
                _ if mnemonic && token == "DAT" => theme::bold(theme::PURPLE),
                _ if mnemonic => theme::bold(CYAN),
                _ if c.is_ascii_digit() => theme::fg(theme::YELLOW),
                "A" | "B" | "C" | "X" | "Y" | "Z" | "I" | "J" => theme::fg(GREEN),
                "SP" | "PC" | "O" | "PUSH" | "POP" | "PEEK" => theme::fg(theme::ORANGE),
                _ => theme::fg(FG),
            };
            mnemonic = false;
            style
        } else {
            match c {
                '[' | ']' | '+' => theme::fg(theme::PURPLE),
                _ => theme::fg(DIM),
            }
        };
        spans.push(Span::styled(token.to_string(), style));
    }
    spans
}

/// Draws the key hints as keycaps.
fn draw_footer(frame: &mut Frame, area: Rect) {
    const KEYS: [(&str, &str); 5] = [
        ("F5", "run/pause"),
        ("F10", "step"),
        ("F2", "reset"),
        ("F7/F8", "speed"),
        ("^C", "quit"),
    ];
    let spans = KEYS.iter().flat_map(|(key, action)| {
        [
            Span::styled(
                format!(" {key} "),
                Style::new().fg(BG).bg(AMBER).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {action}  "), theme::fg(DIM)),
        ]
    });
    let line = Line::from_iter(std::iter::once(Span::raw(" ")).chain(spans));
    frame.render_widget(Paragraph::new(line).style(Style::new().bg(BAR_BG)), area);
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
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
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
    fn shows_all_panels() {
        let program = dcpu16::assemble(
            "SET [0x8000], 0xf048\nSET [0x8001], 0x0069\nSET [0x9010], 0x6f\nSET A, 0x1234\n:end SET PC, end",
        )
        .unwrap();
        let mut app = App::new(program, "demo.dasm");
        app.advance(Duration::from_millis(10));

        let lines = render(&app);
        assert!(lines[0].contains("DCPU-16") && lines[0].contains("demo.dasm"));
        assert!(lines[0].contains("● HALTED  100 kHz"), "{}", lines[0]);
        assert!(lines[2].starts_with("┃Hi "), "{}", lines[2]);
        assert!(lines[16].starts_with("│o "), "{}", lines[16]);
        assert!(lines.iter().any(|line| line.contains("A  1234")));
        assert!(
            lines.iter().any(
                |line| line.contains("▶ 000B  SET PC, 0x0B") && line.contains("; jump to 0x0B")
            ),
            "{}",
            lines.join("\n")
        );
        assert!(lines[23].contains(" F5  run/pause"), "{}", lines[23]);
    }

    #[test]
    fn highlights_instruction_tokens() {
        let spans = highlight("SET [0x10+A], PC");
        let tokens: Vec<(&str, Option<Color>)> = spans
            .iter()
            .map(|span| (span.content.as_ref(), span.style.fg))
            .collect();
        assert_eq!(
            tokens,
            [
                ("SET", Some(CYAN)),
                (" ", Some(DIM)),
                ("[", Some(theme::PURPLE)),
                ("0x10", Some(theme::YELLOW)),
                ("+", Some(theme::PURPLE)),
                ("A", Some(GREEN)),
                ("]", Some(theme::PURPLE)),
                (",", Some(DIM)),
                (" ", Some(DIM)),
                ("PC", Some(theme::ORANGE)),
            ]
        );
        assert_eq!(highlight("DAT 0x0000")[0].style.fg, Some(theme::PURPLE));
    }

    #[test]
    fn colors_cells_from_the_palette() {
        let span = cell_span(Cell::from_word(0xe148), true, theme::SCREEN_BG);
        assert_eq!(span.content, "H");
        assert_eq!(span.style.fg, Some(Color::Rgb(0xff, 0xff, 0x55)));
        assert_eq!(span.style.bg, Some(Color::Rgb(0x00, 0x00, 0xaa)));

        // Plain characters glow in green phosphor on the scanline background.
        let plain = cell_span(Cell::from_word(0x0048), true, theme::SCANLINE);
        assert_eq!(plain.style.fg, Some(GREEN));
        assert_eq!(plain.style.bg, Some(theme::SCANLINE));
    }

    #[test]
    fn hides_blinking_characters_in_the_off_phase() {
        let cell = Cell::from_word(0xf0c8);
        assert_eq!(cell_span(cell, true, theme::SCREEN_BG).content, "H");
        assert_eq!(cell_span(cell, false, theme::SCREEN_BG).content, " ");
    }

    #[test]
    fn formats_speeds() {
        assert_eq!(format_speed(1_000), "1 kHz");
        assert_eq!(format_speed(100_000), "100 kHz");
        assert_eq!(format_speed(10_000_000), "10 MHz");
        assert_eq!(format_speed(500), "500 Hz");
    }
}
