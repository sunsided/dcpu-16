//! The emulator state behind the user interface and its reaction to keys.

use dcpu16::DCPU16;
use dcpu16::devices::{Console, Keyboard, KeyboardInput};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::cell::Cell;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Selectable emulation speeds in cycles per second; F7 and F8 move between them.
pub const SPEEDS: [u64; 5] = [1_000, 10_000, 100_000, 1_000_000, 10_000_000];

/// Index into [`SPEEDS`] of the 100 kHz the DCPU-16 runs at in 0x10c.
const DEFAULT_SPEED: usize = 2;

/// Longest stretch of time emulated at once, so that a stalled terminal does not
/// make the program race to catch up.
const MAX_ELAPSED: Duration = Duration::from_millis(100);

/// Upper bound of the console text kept for display, in bytes.
const MAX_OUTPUT: usize = 64 * 1024;

/// Whether the program executes.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum State {
    /// Executing at the selected speed.
    Running,
    /// Stopped by the user; F10 executes single instructions.
    Paused,
    /// Stopped by the program: it ended, hit a reserved opcode or jumped to itself.
    Halted,
}

/// The registers `A`–`J`, then `PC`, `SP` and `O`.
pub type Registers = [u16; 11];

/// The emulated machine and the controls around it.
pub struct App {
    /// The name of the program, shown in the header.
    pub name: String,
    /// The assembled program, kept for resets.
    program: Vec<u16>,
    /// The CPU with the keyboard and console attached.
    pub cpu: DCPU16,
    /// Types keys on the attached keyboard.
    keyboard: KeyboardInput,
    /// Everything the program printed to the console.
    pub output: ConsoleOutput,
    /// Whether the program executes.
    pub state: State,
    /// Index into [`SPEEDS`].
    speed: usize,
    /// Cycles due but not executed yet; negative after an instruction overshot.
    budget: f64,
    /// The registers before the last frame or step, to highlight what changed.
    pub previous: Registers,
    /// The address of the first instruction in the code panel. It stays put while
    /// the program counter moves within the panel, so loops do not scroll.
    pub code_top: Cell<u16>,
    /// Set once the user asked to quit.
    pub quit: bool,
}

impl App {
    /// Loads the program and starts running it.
    pub fn new(program: Vec<u16>, name: impl Into<String>) -> Self {
        let (cpu, keyboard, output) = boot(&program);
        Self {
            name: name.into(),
            previous: registers(&cpu),
            code_top: Cell::new(cpu.program_counter),
            program,
            cpu,
            keyboard,
            output,
            state: State::Running,
            speed: DEFAULT_SPEED,
            budget: 0.0,
            quit: false,
        }
    }

    /// Gets the emulation speed in cycles per second.
    pub fn speed(&self) -> u64 {
        SPEEDS[self.speed]
    }

    /// Executes the cycles that are due after `elapsed` time at the selected speed.
    pub fn advance(&mut self, elapsed: Duration) {
        if self.state != State::Running {
            return;
        }
        self.previous = registers(&self.cpu);

        self.budget += elapsed.min(MAX_ELAPSED).as_secs_f64() * self.speed() as f64;
        if self.budget < 1.0 {
            return;
        }

        let before = self.cpu.cycles();
        let running = self.cpu.run_for(self.budget as u64);
        self.budget -= (self.cpu.cycles() - before) as f64;
        if !running {
            self.state = State::Halted;
        }
    }

    /// Reacts to a key: controls on function keys and `Ctrl`, everything else goes
    /// to the program's keyboard.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('c' | 'q') if ctrl => self.quit = true,
            KeyCode::F(5) => {
                self.state = match self.state {
                    State::Running => State::Paused,
                    State::Paused => State::Running,
                    State::Halted => State::Halted,
                };
                self.budget = 0.0;
            }
            KeyCode::F(10) if self.state == State::Paused => {
                self.previous = registers(&self.cpu);
                if !self.cpu.step() {
                    self.state = State::Halted;
                }
            }
            KeyCode::F(2) => self.reset(),
            KeyCode::F(7) => self.speed = self.speed.saturating_sub(1),
            KeyCode::F(8) => self.speed = (self.speed + 1).min(SPEEDS.len() - 1),
            _ => {
                if let Some(code) = key_code(key) {
                    self.keyboard.press(code);
                }
            }
        }
    }

    /// Reloads the program into a fresh machine and runs it.
    fn reset(&mut self) {
        let (cpu, keyboard, output) = boot(&self.program);
        self.previous = registers(&cpu);
        self.code_top.set(cpu.program_counter);
        self.cpu = cpu;
        self.keyboard = keyboard;
        self.output = output;
        self.state = State::Running;
        self.budget = 0.0;
    }
}

/// Reads the registers shown in the registers panel.
pub fn registers(cpu: &DCPU16) -> Registers {
    use dcpu16::Register::{A, B, C, I, J, X, Y, Z};
    [
        cpu.register(A),
        cpu.register(B),
        cpu.register(C),
        cpu.register(X),
        cpu.register(Y),
        cpu.register(Z),
        cpu.register(I),
        cpu.register(J),
        cpu.program_counter,
        cpu.stack_pointer,
        cpu.overflow,
    ]
}

/// Creates a CPU running `program` with a keyboard and a console attached.
fn boot(program: &[u16]) -> (DCPU16, KeyboardInput, ConsoleOutput) {
    let output = ConsoleOutput::default();
    let (keyboard, input) = Keyboard::new();
    let mut cpu = DCPU16::new(program);
    cpu.attach(keyboard);
    cpu.attach(Console::new(output.clone()));
    (cpu, input, output)
}

/// Translates a key into the code the program receives.
///
/// Characters are sent as their Unicode value, with `\n` for Enter, `\x08` for
/// Backspace, `\t` for Tab, `\x1b` for Escape and `\x7f` for Delete. The arrow keys
/// use the codes of the DCPU-16 generic keyboard: `0x80` up, `0x81` down,
/// `0x82` left and `0x83` right. Keys combined with `Ctrl` are not sent.
fn key_code(key: KeyEvent) -> Option<u16> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }
    Some(match key.code {
        KeyCode::Char(c) => u16::try_from(u32::from(c)).ok()?,
        KeyCode::Enter => 0x0a,
        KeyCode::Backspace => 0x08,
        KeyCode::Tab => 0x09,
        KeyCode::Esc => 0x1b,
        KeyCode::Delete => 0x7f,
        KeyCode::Up => 0x80,
        KeyCode::Down => 0x81,
        KeyCode::Left => 0x82,
        KeyCode::Right => 0x83,
        _ => return None,
    })
}

/// The text the program printed through the console, shared with the console device.
#[derive(Clone, Default)]
pub struct ConsoleOutput(Arc<Mutex<String>>);

impl ConsoleOutput {
    /// Gets a copy of the text printed so far, or of its tail once it grew long.
    pub fn text(&self) -> String {
        self.0.lock().expect("console output poisoned").clone()
    }
}

impl Write for ConsoleOutput {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut text = self.0.lock().expect("console output poisoned");
        // The console writes whole UTF-8 characters at a time.
        text.push_str(&String::from_utf8_lossy(buf));
        if text.len() > MAX_OUTPUT {
            let mut cut = text.len() - MAX_OUTPUT / 2;
            while !text.is_char_boundary(cut) {
                cut += 1;
            }
            text.drain(..cut);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn app(source: &str) -> App {
        App::new(dcpu16::assemble(source).unwrap(), "test.dasm")
    }

    #[test]
    fn translates_keys() {
        assert_eq!(key_code(press(KeyCode::Char('a'))), Some(0x61));
        assert_eq!(key_code(press(KeyCode::Char('€'))), Some(0x20ac));
        assert_eq!(key_code(press(KeyCode::Char('🦀'))), None);
        assert_eq!(key_code(press(KeyCode::Enter)), Some(0x0a));
        assert_eq!(key_code(press(KeyCode::Left)), Some(0x82));
        assert_eq!(key_code(press(KeyCode::F(1))), None);
        assert_eq!(
            key_code(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL)),
            None
        );
    }

    #[test]
    fn executes_the_cycles_due_at_the_selected_speed() {
        let mut app = app(":loop ADD A, 1\nSET PC, loop");
        app.advance(Duration::from_millis(10));
        // 100 kHz for 10 ms; the last instruction may overshoot by a few cycles.
        assert!((1000..1005).contains(&app.cpu.cycles()));

        app.handle_key(press(KeyCode::F(8)));
        assert_eq!(app.speed(), 1_000_000);
        let before = app.cpu.cycles();
        app.advance(Duration::from_millis(10));
        assert!((9995..10_005).contains(&(app.cpu.cycles() - before)));
    }

    #[test]
    fn caps_the_time_emulated_at_once() {
        let mut app = app(":loop ADD A, 1\nSET PC, loop");
        app.advance(Duration::from_secs(10));
        assert!(app.cpu.cycles() < 10_100);
    }

    #[test]
    fn pauses_steps_and_resets() {
        let mut app = app(":loop ADD A, 1\nSET PC, loop");
        app.handle_key(press(KeyCode::F(5)));
        assert_eq!(app.state, State::Paused);

        app.advance(Duration::from_millis(10));
        assert_eq!(app.cpu.cycles(), 0);

        app.handle_key(press(KeyCode::F(10)));
        assert_eq!(app.cpu.register(dcpu16::Register::A), 1);
        assert_eq!(app.previous[0], 0);
        assert_eq!(registers(&app.cpu)[0], 1);

        app.handle_key(press(KeyCode::F(2)));
        assert_eq!(app.state, State::Running);
        assert_eq!(app.cpu.register(dcpu16::Register::A), 0);
    }

    #[test]
    fn halts_when_the_program_stops() {
        let mut app = app(":end SET PC, end");
        app.advance(Duration::from_millis(10));
        assert_eq!(app.state, State::Halted);
    }

    #[test]
    fn sends_keys_to_the_program_and_collects_its_output() {
        // Copies one key from the keyboard to the console.
        let mut app =
            app(":wait IFE [0x9000], 0\nSET PC, wait\nSET [0x9010], [0x9000]\n:end SET PC, end");
        app.handle_key(press(KeyCode::Char('x')));
        app.advance(Duration::from_millis(10));
        assert_eq!(app.output.text(), "x");
        assert_eq!(app.state, State::Halted);
    }

    #[test]
    fn keeps_the_tail_of_long_output() {
        let mut output = ConsoleOutput::default();
        for _ in 0..MAX_OUTPUT {
            output.write_all("ä".as_bytes()).unwrap();
        }
        let text = output.text();
        assert!(text.len() <= MAX_OUTPUT);
        assert!(text.chars().all(|c| c == 'ä'));
    }

    /// Types `text` into `app` and runs it long enough to handle every key.
    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            let code = match c {
                '\n' => KeyCode::Enter,
                '\x08' => KeyCode::Backspace,
                c => KeyCode::Char(c),
            };
            app.handle_key(press(code));
            app.advance(Duration::from_millis(10));
        }
    }

    /// Gets the screen as text with trailing spaces removed from each row.
    fn screen_text(app: &App) -> Vec<String> {
        dcpu16::devices::screen::text(app.cpu.ram())
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect()
    }

    #[test]
    fn hello_program_greets_on_screen_and_console() {
        let mut app = app(include_str!("../programs/hello.dasm"));
        app.advance(Duration::from_millis(10));
        assert_eq!(screen_text(&app)[0], "Hello, world!_");
        assert_eq!(app.output.text(), "Hello, world!");
        assert_eq!(app.state, State::Halted);
    }

    #[test]
    fn typewriter_program_types_breaks_lines_and_deletes() {
        let mut app = app(include_str!("../programs/typewriter.dasm"));
        app.advance(Duration::from_millis(10));
        type_text(&mut app, "Hi\nthere\x08\x08\x08y\u{7f}");
        let screen = screen_text(&app);
        assert_eq!(screen[0], "DCPU-16 typewriter");
        assert_eq!(screen[2], "Hi");
        assert_eq!(screen[3], "thy_");
        assert_eq!(app.state, State::Running);
    }

    #[test]
    fn typewriter_program_scrolls_at_the_bottom() {
        let mut app = app(include_str!("../programs/typewriter.dasm"));
        app.advance(Duration::from_millis(10));
        for line in 0..12 {
            type_text(&mut app, &format!("line {line}\n"));
        }
        // Scrolling moves about 300 words, which takes longer than one key.
        for _ in 0..10 {
            app.advance(Duration::from_millis(100));
        }
        let screen = screen_text(&app);
        assert_eq!(screen[0], "DCPU-16 typewriter");
        assert_eq!(screen[2], "line 3");
        assert_eq!(screen[10], "line 11");
        assert_eq!(screen[11], "_");
    }

    #[test]
    fn quits_on_ctrl_c() {
        let mut app = app("SET A, 1");
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.quit);
    }
}
