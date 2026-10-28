//! Drawing of the header, screen, console, registers, code and key hints.

use crate::app::{App, State, registers};
use crate::theme::{self, AMBER, BG, BG_CURSOR, BORDER, CYAN, DIM, FG, GREEN};
use dcpu16::Disassembly;
use dcpu16::devices::screen::{self, Cell, PALETTE};
use dcpu16::devices::{KEYBOARD_BUFFER, KEYBOARD_BUFFER_LEN};
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
/// Width of the registers panel when the hardware panel sits next to it.
const REGISTERS_WIDTH: u16 = 24;
/// Narrowest hardware panel worth showing; below that, the registers take the row.
const MIN_HARDWARE_WIDTH: u16 = 40;
/// Number of segments in the clock gauge.
const GAUGE_SEGMENTS: usize = 10;

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
    if registers_area.width >= REGISTERS_WIDTH + MIN_HARDWARE_WIDTH {
        let [registers_area, hardware_area] =
            Layout::horizontal([Constraint::Length(REGISTERS_WIDTH), Constraint::Min(0)])
                .areas(registers_area);
        draw_registers(frame, registers_area, app);
        draw_hardware(frame, hardware_area, app);
    } else {
        draw_registers(frame, registers_area, app);
    }
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

/// Draws the state of the memory-mapped devices, the clock and the stack, each
/// with an activity light.
fn draw_hardware(frame: &mut Frame, area: Rect, app: &App) {
    let ram = app.cpu.ram();
    let activity = app.activity;
    let device = |active: bool, name: &str, address: &str| {
        let light = if active {
            theme::bold(GREEN)
        } else {
            theme::fg(BORDER)
        };
        vec![
            Span::styled("● ", light),
            Span::styled(format!("{name:<9}"), theme::fg(AMBER)),
            Span::styled(format!("{address:<7}"), theme::fg(DIM)),
        ]
    };

    // The screen: how many cells show something.
    let lit = screen::cells(ram)
        .filter(|cell| cell.character != 0)
        .count();
    let mut video = device(activity.video > 0, "LEM1802", "0x8000");
    video.extend([
        Span::styled(lit.to_string(), theme::fg(GREEN)),
        Span::styled(
            format!("/{} cells", screen::WIDTH * screen::HEIGHT),
            theme::fg(DIM),
        ),
    ]);

    // The keyboard: the ring buffer, slot by slot.
    let mut keyboard = device(activity.keyboard > 0, "KEYBOARD", "0x9000");
    keyboard.push(Span::styled("▕", theme::fg(BORDER)));
    let buffer = usize::from(KEYBOARD_BUFFER);
    keyboard.extend(
        ram[buffer..buffer + usize::from(KEYBOARD_BUFFER_LEN)]
            .iter()
            .map(|&key| key_span(key)),
    );
    keyboard.push(Span::styled("▏", theme::fg(BORDER)));

    // The console: how much was printed, and the last character.
    let (printed, last) = app.output.printed();
    let mut console = device(activity.console > 0, "CONSOLE", "0x9010");
    console.extend([
        Span::styled(printed.to_string(), theme::fg(GREEN)),
        Span::styled(" chars", theme::fg(DIM)),
    ]);
    if let Some(last) = last {
        console.extend([
            Span::styled("  last ", theme::fg(DIM)),
            Span::styled(format!("'{}'", last.escape_debug()), theme::fg(GREEN)),
        ]);
    }

    // The clock: the measured speed against the selected one.
    let mut clock = device(app.state == State::Running, "CLOCK", "");
    match app.measured_hz {
        Some(hz) => {
            let ratio = (hz / app.speed() as f64).clamp(0.0, 1.0);
            let filled = (ratio * GAUGE_SEGMENTS as f64).round() as usize;
            clock.extend([
                Span::styled("▰".repeat(filled), theme::fg(GREEN)),
                Span::styled("▱".repeat(GAUGE_SEGMENTS - filled), theme::fg(BORDER)),
                Span::styled(format!(" {}", format_rate(hz)), theme::fg(GREEN)),
            ]);
        }
        None => clock.push(Span::styled("stopped", theme::fg(DIM))),
    }

    // The stack: its depth and the values on top.
    let sp = app.cpu.stack_pointer;
    let mut stack = device(sp != app.previous[9], "STACK", "");
    if sp == 0 {
        stack.push(Span::styled("empty", theme::fg(DIM)));
    } else {
        let depth = 0x10000 - usize::from(sp);
        stack.extend([
            Span::styled(depth.to_string(), theme::fg(GREEN)),
            Span::styled(" deep ", theme::fg(DIM)),
        ]);
        let top = (0..depth.min(3)).map(|offset| ram[usize::from(sp) + offset]);
        stack.extend(top.map(|word| Span::styled(format!(" {word:04X}"), theme::fg(FG))));
        if depth > 3 {
            stack.push(Span::styled(" …", theme::fg(DIM)));
        }
    }

    let lines: Vec<Line> = [video, keyboard, console, clock, stack]
        .into_iter()
        .map(Line::from)
        .collect();
    frame.render_widget(Paragraph::new(lines).block(panel("HARDWARE")), area);
}

/// Renders a slot of the keyboard buffer: the key, or `·` when the slot is free.
fn key_span(key: u16) -> Span<'static> {
    let symbol = match key {
        0 => return Span::styled("·", theme::fg(BORDER)),
        0x0a => '⏎',
        0x08 => '⌫',
        0x09 => '⇥',
        0x80 => '↑',
        0x81 => '↓',
        0x82 => '←',
        0x83 => '→',
        key => char::from_u32(key.into())
            .filter(|c| !c.is_control())
            .unwrap_or('?'),
    };
    Span::styled(symbol.to_string(), theme::bold(CYAN))
}

/// Frames over which the execution trail in the code panel fades, about a second.
const TRAIL_FRAMES: u64 = 60;

/// Lines the code panel keeps visible above the program counter when it scrolls.
const CODE_CONTEXT: usize = 3;
/// Lines the code panel keeps visible below the program counter before it scrolls.
const CODE_LOOKAHEAD: usize = 3;

/// Draws the instructions around the program counter, with syntax highlighting
/// and a plain-English description of each.
///
/// The listing stays in place while the program counter moves within it, so a loop
/// only moves the caret. Once the program counter gets close to the bottom, the
/// listing scrolls to keep a few instructions above it; when it jumps somewhere
/// else, the listing starts at the program counter.
fn draw_code(frame: &mut Frame, area: Rect, app: &App) {
    let rows = usize::from(area.height.saturating_sub(2));
    let pc = app.cpu.program_counter;

    let mut listing = code_listing(app, app.code_top.get(), rows);
    let top = match listing.iter().position(|(address, _)| *address == pc) {
        // Small panels cannot keep both margins; visible is good enough there.
        Some(_) if rows <= CODE_CONTEXT + CODE_LOOKAHEAD => None,
        Some(index) if index + CODE_LOOKAHEAD < rows => None,
        Some(index) => Some(listing[index - CODE_CONTEXT].0),
        None => Some(pc),
    };
    if let Some(top) = top {
        app.code_top.set(top);
        listing = code_listing(app, top, rows);
    }

    let lines: Vec<Line> = listing
        .into_iter()
        .map(|(address, disassembly)| {
            let trail = app.frames_since_executed(address);
            code_line(address, &disassembly, address == pc, trail)
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).block(panel("CODE")), area);
}

/// Disassembles `rows` consecutive instructions starting at `top`.
fn code_listing(app: &App, top: u16, rows: usize) -> Vec<(u16, Disassembly)> {
    let mut address = top;
    (0..rows)
        .map(|_| {
            let disassembly = app.cpu.disassemble(address);
            let line = (address, disassembly);
            address = address.wrapping_add(line.1.length);
            line
        })
        .collect()
}

/// Renders one instruction of the code panel; `current` marks the program counter,
/// and `trail` is the number of frames since the instruction last executed.
fn code_line(
    address: u16,
    disassembly: &Disassembly,
    current: bool,
    trail: Option<u64>,
) -> Line<'static> {
    let mut spans = vec![
        Span::styled(if current { "▶" } else { " " }, theme::bold(AMBER)),
        trail_span(trail),
        Span::styled(
            format!(" {address:04X}  "),
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
    if current {
        line.style(Style::new().bg(BG_CURSOR))
    } else {
        line
    }
}

/// Renders the execution trail mark of an instruction: bright amber right after it
/// executed, fading to the border color over [`TRAIL_FRAMES`] frames.
fn trail_span(frames: Option<u64>) -> Span<'static> {
    match frames {
        Some(frames) if frames < TRAIL_FRAMES => {
            let t = frames as f64 / TRAIL_FRAMES as f64;
            Span::styled("▍", theme::fg(blend(AMBER, BORDER, t)))
        }
        Some(_) => Span::styled("▍", theme::fg(BORDER)),
        None => Span::raw(" "),
    }
}

/// Mixes two RGB colors: `t` = 0 gives `from`, `t` = 1 gives `to`.
fn blend(from: Color, to: Color, t: f64) -> Color {
    match (from, to) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let mix =
                |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
            Color::Rgb(mix(r1, r2), mix(g1, g2), mix(b1, b2))
        }
        _ => from,
    }
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

    let typing = Line::from(vec![
        Span::styled("other keys ", theme::fg(DIM)),
        Span::styled("→ program ", theme::bold(CYAN)),
    ])
    .right_aligned();
    frame.render_widget(Paragraph::new(typing), area);
}

/// Formats a measured speed in cycles per second, e.g. `99.8 kHz`.
fn format_rate(hz: f64) -> String {
    if hz >= 1_000_000.0 {
        format!("{:.1} MHz", hz / 1_000_000.0)
    } else if hz >= 1_000.0 {
        format!("{:.1} kHz", hz / 1_000.0)
    } else {
        format!("{hz:.0} Hz")
    }
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
            lines
                .iter()
                .any(|line| line.contains("▶▍ 000B  SET PC, 0x0B")
                    && line.contains("; jump to 0x0B")),
            "{}",
            lines.join("\n")
        );
        assert!(lines[23].contains(" F5  run/pause"), "{}", lines[23]);
        assert!(
            lines[23].ends_with("other keys → program "),
            "{}",
            lines[23]
        );

        let hardware = lines[2..7].join("\n");
        assert!(
            hardware.contains("LEM1802  0x8000 2/384 cells"),
            "{hardware}"
        );
        assert!(
            hardware.contains("KEYBOARD 0x9000 ▕················▏"),
            "{hardware}"
        );
        assert!(
            hardware.contains("CONSOLE  0x9010 1 chars  last 'o'"),
            "{hardware}"
        );
        assert!(hardware.contains("CLOCK           stopped"), "{hardware}");
        assert!(hardware.contains("STACK           empty"), "{hardware}");
    }

    #[test]
    fn fades_the_execution_trail() {
        assert_eq!(trail_span(None).content, " ");
        assert_eq!(trail_span(Some(0)).style.fg, Some(AMBER));
        assert_eq!(trail_span(Some(TRAIL_FRAMES)).style.fg, Some(BORDER));

        let halfway = trail_span(Some(TRAIL_FRAMES / 2)).style.fg;
        assert_ne!(halfway, Some(AMBER));
        assert_ne!(halfway, Some(BORDER));
    }

    #[test]
    fn renders_keyboard_slots() {
        assert_eq!(key_span(0).content, "·");
        assert_eq!(key_span(0x61).content, "a");
        assert_eq!(key_span(0x0a).content, "⏎");
        assert_eq!(key_span(0x82).content, "←");
        assert_eq!(key_span(0x07).content, "?");
    }

    #[test]
    fn formats_measured_rates() {
        assert_eq!(format_rate(99_812.0), "99.8 kHz");
        assert_eq!(format_rate(1_250_000.0), "1.2 MHz");
        assert_eq!(format_rate(512.4), "512 Hz");
    }

    /// Gets the address and whether it is marked as current for each code line.
    fn code_rows(lines: &[String]) -> Vec<(String, bool)> {
        // The code panel starts below the registers panel, inside its border.
        lines[11..22]
            .iter()
            .map(|line| {
                // The code starts right of the screen panel and the code panel's border.
                let code: String = line
                    .chars()
                    .skip(usize::from(SCREEN_PANEL_WIDTH) + 1)
                    .collect();
                (
                    code.chars().skip(3).take(4).collect(),
                    code.starts_with('▶'),
                )
            })
            .collect()
    }

    #[test]
    fn code_panel_stays_put_while_looping() {
        let program = dcpu16::assemble(
            "SET A, 0\n:loop ADD A, 1\nIFN A, 0x30\nSET PC, loop\n:end SET PC, end",
        )
        .unwrap();
        let mut app = App::new(program, "loop.dasm");
        app.state = State::Paused;

        for step in 0..12 {
            let rows = code_rows(&render(&app));
            assert_eq!(rows[0].0, "0000", "step {step} scrolled: {rows:?}");
            let current: Vec<_> = rows.iter().filter(|(_, current)| *current).collect();
            assert_eq!(current.len(), 1, "step {step}: {rows:?}");
            assert_eq!(
                current[0].0,
                format!("{:04X}", app.cpu.program_counter),
                "step {step}"
            );
            app.cpu.step();
        }
    }

    #[test]
    fn code_panel_scrolls_near_the_bottom_and_follows_jumps() {
        let program = dcpu16::assemble("ADD A, 1\n".repeat(40)).unwrap();
        let mut app = App::new(program, "straight.dasm");
        app.state = State::Paused;

        // 11 rows: the caret may move down to row 8, keeping 3 rows below it.
        for _ in 0..7 {
            app.cpu.step();
        }
        let rows = code_rows(&render(&app));
        assert_eq!(rows[0].0, "0000");
        assert!(rows[7].1);

        app.cpu.step();
        let rows = code_rows(&render(&app));
        assert_eq!(rows[0].0, "0005", "{rows:?}");
        assert!(rows[3].1);

        // A jump out of the listing starts it at the program counter.
        app.cpu.program_counter = 0x20;
        let rows = code_rows(&render(&app));
        assert_eq!(rows[0].0, "0020");
        assert!(rows[0].1);
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
