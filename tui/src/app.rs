//! The emulator state behind the user interface and its reaction to keys.

use dcpu16::devices::{
    Console, KEYBOARD_BUFFER, KEYBOARD_BUFFER_LEN, Keyboard, KeyboardInput, VIDEO_RAM, screen,
};
use dcpu16::{DCPU16, NUM_RAM_WORDS};
use std::cell::Cell;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Selectable emulation speeds in cycles per second; F7 and F8 move between them.
/// The slowest ones let you follow single instructions.
pub const SPEEDS: [u64; 7] = [10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000];

/// Index into [`SPEEDS`] of the 100 kHz the DCPU-16 runs at in 0x10c.
const DEFAULT_SPEED: usize = 4;

/// Longest stretch of time emulated at once, so that a stalled terminal does not
/// make the program race to catch up.
const MAX_ELAPSED: Duration = Duration::from_millis(100);

/// Upper bound of the console text kept for display, in bytes.
const MAX_OUTPUT: usize = 64 * 1024;

/// Frames a device's activity light stays lit after the device was used, about 200 ms.
const ACTIVITY_FRAMES: u8 = 12;

/// How long the clock speed is measured before the shown value updates.
const MEASURE_WINDOW: Duration = Duration::from_millis(500);

/// Number of words of video RAM.
const VIDEO_WORDS: usize = screen::WIDTH * screen::HEIGHT;

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

/// A key press, whether it came from a terminal or a browser.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Key {
    /// Which key was pressed.
    pub code: KeyCode,
    /// Whether `Ctrl` was held.
    pub ctrl: bool,
}

/// The keys the interface and the programs care about.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum KeyCode {
    /// A character key.
    Char(char),
    /// Enter.
    Enter,
    /// Backspace.
    Backspace,
    /// Tab.
    Tab,
    /// Escape.
    Esc,
    /// Delete.
    Delete,
    /// Arrow up.
    Up,
    /// Arrow down.
    Down,
    /// Arrow left.
    Left,
    /// Arrow right.
    Right,
    /// A function key, `F(1)` to `F(12)`.
    F(u8),
}

impl Key {
    /// A key pressed without `Ctrl`.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn new(code: KeyCode) -> Self {
        Self { code, ctrl: false }
    }
}

/// An assembled program the interface can run.
pub struct Program {
    /// The name shown in the header, e.g. the file name.
    pub name: String,
    /// The assembled program words.
    pub words: Vec<u16>,
}

/// Recent use of the memory-mapped devices, as the number of frames each
/// activity light stays lit.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Activity {
    /// Video RAM changed.
    pub video: u8,
    /// The keyboard buffer changed: a key arrived or the program took one.
    pub keyboard: u8,
    /// The console printed a character.
    pub console: u8,
}

impl Activity {
    /// Dims every light by one frame.
    fn decay(&mut self) {
        self.video = self.video.saturating_sub(1);
        self.keyboard = self.keyboard.saturating_sub(1);
        self.console = self.console.saturating_sub(1);
    }
}

/// The parts of the machine state that reveal device activity.
struct DeviceState {
    /// The video RAM.
    video: Vec<u16>,
    /// The keyboard ring buffer.
    keyboard: Vec<u16>,
    /// The number of characters printed to the console.
    printed: usize,
}

impl DeviceState {
    /// Captures the device state of `cpu` and `output`.
    fn capture(cpu: &DCPU16, output: &ConsoleOutput) -> Self {
        let ram = cpu.ram();
        let video = usize::from(VIDEO_RAM);
        let keyboard = usize::from(KEYBOARD_BUFFER);
        Self {
            video: ram[video..video + VIDEO_WORDS].to_vec(),
            keyboard: ram[keyboard..keyboard + usize::from(KEYBOARD_BUFFER_LEN)].to_vec(),
            printed: output.printed().0,
        }
    }
}

/// The emulated machine and the controls around it.
pub struct App {
    /// The programs to choose from with F3; resets reload the current one.
    programs: Vec<Program>,
    /// Index into `programs` of the running program.
    current: usize,
    /// Whether Ctrl+C quits; not in a browser, which has no way out.
    pub can_quit: bool,
    /// A message for the user in the header, e.g. why a requested program is not
    /// running; cleared when switching programs.
    pub notice: Option<String>,
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
    /// Recent use of the devices.
    pub activity: Activity,
    /// The measured clock speed in cycles per second, while running.
    pub measured_hz: Option<f64>,
    /// Cycles executed and time passed since the clock speed was last measured.
    meter: (u64, Duration),
    /// The number of frames drawn so far.
    frame: u64,
    /// For each address, one more than the frame in which an instruction there last
    /// executed, or zero if none did.
    trace: Vec<u64>,
    /// Set once the user asked to quit.
    pub quit: bool,
}

impl App {
    /// Loads the program and starts running it.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn new(program: Vec<u16>, name: impl Into<String>) -> Self {
        Self::with_programs(vec![Program {
            name: name.into(),
            words: program,
        }])
    }

    /// Loads the first of `programs` and starts running it; F3 switches to the next.
    ///
    /// # Panics
    ///
    /// Panics if `programs` is empty.
    pub fn with_programs(programs: Vec<Program>) -> Self {
        let (cpu, keyboard, output) = boot(&programs[0].words);
        Self {
            programs,
            current: 0,
            can_quit: true,
            notice: None,
            previous: registers(&cpu),
            code_top: Cell::new(cpu.program_counter),
            activity: Activity::default(),
            measured_hz: None,
            meter: (0, Duration::ZERO),
            frame: 0,
            trace: vec![0; NUM_RAM_WORDS],
            cpu,
            keyboard,
            output,
            state: State::Running,
            speed: DEFAULT_SPEED,
            budget: 0.0,
            quit: false,
        }
    }

    /// Gets the name of the running program.
    pub fn name(&self) -> &str {
        &self.programs[self.current].name
    }

    /// Whether F3 has other programs to switch to.
    pub fn has_program_choice(&self) -> bool {
        self.programs.len() > 1
    }

    /// Gets the emulation speed in cycles per second.
    pub fn speed(&self) -> u64 {
        SPEEDS[self.speed]
    }

    /// Executes the cycles that are due after `elapsed` time at the selected speed.
    ///
    /// Called once per frame, also while not running, so activity lights fade.
    pub fn advance(&mut self, elapsed: Duration) {
        self.frame += 1;
        self.activity.decay();
        if self.state != State::Running {
            self.measured_hz = None;
            self.meter = (0, Duration::ZERO);
            return;
        }
        self.previous = registers(&self.cpu);

        let elapsed = elapsed.min(MAX_ELAPSED);
        self.budget += elapsed.as_secs_f64() * self.speed() as f64;
        let before = self.cpu.cycles();
        if self.budget >= 1.0 {
            let budget = self.budget as u64;
            let running = self.execute(|app| app.run_for(budget));
            self.budget -= (self.cpu.cycles() - before) as f64;
            if !running {
                self.state = State::Halted;
            }
        }

        self.meter.0 += self.cpu.cycles() - before;
        self.meter.1 += elapsed;
        if self.meter.1 >= MEASURE_WINDOW {
            self.measured_hz = Some(self.meter.0 as f64 / self.meter.1.as_secs_f64());
            self.meter = (0, Duration::ZERO);
        }
    }

    /// Gets the number of frames since an instruction at `address` last executed,
    /// or `None` if none ever did.
    pub fn frames_since_executed(&self, address: u16) -> Option<u64> {
        match self.trace[usize::from(address)] {
            0 => None,
            frame => Some(self.frame + 1 - frame),
        }
    }

    /// Like [`DCPU16::run_for`], but records every executed address in the trace.
    fn run_for(&mut self, cycles: u64) -> bool {
        let end = self.cpu.cycles().saturating_add(cycles);
        while self.cpu.cycles() < end {
            if !self.step() {
                return false;
            }
        }
        true
    }

    /// Executes one instruction, or skips it, and records its address in the trace.
    fn step(&mut self) -> bool {
        self.trace[usize::from(self.cpu.program_counter)] = self.frame + 1;
        self.cpu.step()
    }

    /// Runs `run` and lights up the devices it used.
    fn execute(&mut self, run: impl FnOnce(&mut Self) -> bool) -> bool {
        let before = DeviceState::capture(&self.cpu, &self.output);
        let running = run(self);
        let after = DeviceState::capture(&self.cpu, &self.output);

        if before.video != after.video {
            self.activity.video = ACTIVITY_FRAMES;
        }
        if before.keyboard != after.keyboard {
            self.activity.keyboard = ACTIVITY_FRAMES;
        }
        if before.printed != after.printed {
            self.activity.console = ACTIVITY_FRAMES;
        }
        running
    }

    /// Reacts to a key press: controls on function keys and `Ctrl`, everything else
    /// goes to the program's keyboard.
    pub fn handle_key(&mut self, key: Key) {
        match key.code {
            KeyCode::Char('c' | 'q') if key.ctrl => self.quit = self.can_quit,
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
                if !self.execute(Self::step) {
                    self.state = State::Halted;
                }
            }
            KeyCode::F(2) => self.reset(),
            KeyCode::F(3) if self.has_program_choice() => {
                self.current = (self.current + 1) % self.programs.len();
                self.notice = None;
                self.reset();
            }
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
        let (cpu, keyboard, output) = boot(&self.programs[self.current].words);
        self.previous = registers(&cpu);
        self.code_top.set(cpu.program_counter);
        self.cpu = cpu;
        self.keyboard = keyboard;
        self.output = output;
        self.state = State::Running;
        self.budget = 0.0;
        self.activity = Activity::default();
        self.measured_hz = None;
        self.meter = (0, Duration::ZERO);
        self.trace.fill(0);
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
fn key_code(key: Key) -> Option<u16> {
    if key.ctrl {
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
        KeyCode::F(_) => return None,
    })
}

/// The text the program printed through the console, shared with the console device.
#[derive(Clone, Default)]
pub struct ConsoleOutput(Arc<Mutex<Printed>>);

/// The console text and how much was printed.
#[derive(Default)]
struct Printed {
    /// The text printed so far, or its tail once it grew long.
    text: String,
    /// The number of characters printed so far, including those cut from `text`.
    count: usize,
}

impl ConsoleOutput {
    /// Gets a copy of the text printed so far, or of its tail once it grew long.
    pub fn text(&self) -> String {
        self.lock().text.clone()
    }

    /// Gets the number of characters printed so far and the last of them.
    pub fn printed(&self) -> (usize, Option<char>) {
        let printed = self.lock();
        (printed.count, printed.text.chars().next_back())
    }

    /// Locks the shared output.
    fn lock(&self) -> std::sync::MutexGuard<'_, Printed> {
        self.0.lock().expect("console output poisoned")
    }
}

impl Write for ConsoleOutput {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut printed = self.lock();
        // The console writes whole UTF-8 characters at a time.
        let chunk = String::from_utf8_lossy(buf);
        printed.count += chunk.chars().count();
        let text = &mut printed.text;
        text.push_str(&chunk);
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

    fn press(code: KeyCode) -> Key {
        Key::new(code)
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
            key_code(Key {
                code: KeyCode::Char('x'),
                ctrl: true
            }),
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
        assert_eq!(screen[1], "Type! Enter: new line");
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
    fn lights_up_the_devices_a_program_uses() {
        let mut app = app("SET [0x8000], 0x41\nSET [0x9010], 0x42\n:end SET PC, end");
        app.advance(Duration::from_millis(10));
        assert_eq!(app.activity.video, ACTIVITY_FRAMES);
        assert_eq!(app.activity.console, ACTIVITY_FRAMES);
        assert_eq!(app.activity.keyboard, 0);
        assert_eq!(app.output.printed(), (1, Some('B')));

        // The lights fade frame by frame once the devices are idle.
        app.advance(Duration::from_millis(10));
        assert_eq!(app.activity.video, ACTIVITY_FRAMES - 1);
    }

    #[test]
    fn lights_up_the_keyboard_when_a_key_arrives() {
        let mut app = app(":loop SET PC, loop");
        app.handle_key(press(KeyCode::Char('k')));
        app.advance(Duration::from_millis(10));
        assert_eq!(app.activity.keyboard, ACTIVITY_FRAMES);
        assert_eq!(app.cpu.ram()[0x9000], u16::from(b'k'));
    }

    #[test]
    fn measures_the_clock_speed() {
        let mut app = app(":loop ADD A, 1\nSET PC, loop");
        // 31 frames of 16 ms stay below the 500 ms window, the 32nd completes it.
        for _ in 0..31 {
            app.advance(Duration::from_millis(16));
            assert_eq!(app.measured_hz, None);
        }
        app.advance(Duration::from_millis(16));
        let hz = app.measured_hz.unwrap();
        assert!((99_000.0..101_000.0).contains(&hz), "{hz}");

        app.handle_key(press(KeyCode::F(5)));
        app.advance(Duration::from_millis(16));
        assert_eq!(app.measured_hz, None);
    }

    #[test]
    fn traces_executed_instructions() {
        let mut app = app("SET A, 1\n:loop ADD B, 1\nSET PC, loop");
        assert_eq!(app.frames_since_executed(0), None);

        app.advance(Duration::from_millis(16));
        assert_eq!(app.frames_since_executed(0), Some(0));
        assert_eq!(app.frames_since_executed(1), Some(0));
        assert_eq!(app.frames_since_executed(2), Some(0));
        assert_eq!(app.frames_since_executed(3), None);

        // The loop keeps running; the first instruction ages.
        for _ in 0..10 {
            app.advance(Duration::from_millis(16));
        }
        assert_eq!(app.frames_since_executed(0), Some(10));
        assert_eq!(app.frames_since_executed(1), Some(0));

        app.handle_key(press(KeyCode::F(2)));
        assert_eq!(app.frames_since_executed(1), None);
    }

    #[test]
    fn slows_down_to_10_hz() {
        let mut app = app(":loop ADD A, 1\nSET PC, loop");
        for _ in 0..4 {
            app.handle_key(press(KeyCode::F(7)));
        }
        assert_eq!(app.speed(), 10);
        app.advance(Duration::from_millis(100));
        assert_eq!(app.cpu.cycles(), 2);
    }

    #[test]
    fn quits_on_ctrl_c() {
        let mut app = app("SET A, 1");
        let ctrl_c = Key {
            code: KeyCode::Char('c'),
            ctrl: true,
        };
        app.handle_key(ctrl_c);
        assert!(app.quit);

        // A browser has no way out, so Ctrl+C stays with the browser.
        let mut app = super::App::new(vec![0x8401], "test.dasm");
        app.can_quit = false;
        app.handle_key(ctrl_c);
        assert!(!app.quit);
    }

    #[test]
    fn switches_programs_with_f3() {
        let program = |source: &str, name: &str| Program {
            name: name.into(),
            words: dcpu16::assemble(source).unwrap(),
        };
        let mut app = App::with_programs(vec![
            program("SET A, 1", "one.dasm"),
            program("SET A, 2", "two.dasm"),
        ]);
        assert!(app.has_program_choice());
        app.advance(Duration::from_millis(10));
        assert_eq!(app.cpu.register(dcpu16::Register::A), 1);

        app.handle_key(press(KeyCode::F(3)));
        assert_eq!(app.name(), "two.dasm");
        assert_eq!(app.state, State::Running);
        app.advance(Duration::from_millis(10));
        assert_eq!(app.cpu.register(dcpu16::Register::A), 2);

        app.handle_key(press(KeyCode::F(3)));
        assert_eq!(app.name(), "one.dasm");
    }
}
