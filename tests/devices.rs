//! Memory-mapped devices driven by assembled programs.
#![cfg(feature = "assembler")]

use dcpu16::devices::{Console, Device, Keyboard, screen};
use dcpu16::{DCPU16, assemble};
use std::sync::{Arc, Mutex};

/// A writer whose output stays readable after the console moved into the CPU.
#[derive(Clone, Default)]
struct Output(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Output {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

/// Prints the zero-terminated string at `message`.
const HELLO: &str = r#"
              SET I, 0
:loop         SET A, [message+I]
              IFE A, 0
                 SET PC, end
              SET [0x9010], A
              ADD I, 1
              SET PC, loop
:end          SET PC, end
:message      DAT "Hello, wörld!\n", 0
"#;

#[test]
fn console_prints_characters_and_clears_the_word() {
    let output = Output::default();
    let mut cpu = DCPU16::new(&assemble(HELLO).unwrap());
    cpu.attach(Console::new(output.clone()));
    cpu.run();

    assert_eq!(output.text(), "Hello, wörld!\n");
    assert_eq!(cpu.ram()[0x9010], 0);
}

#[test]
fn console_replaces_invalid_characters() {
    let mut console = Console::new(Vec::new());
    let mut ram = Box::new([0; dcpu16::NUM_RAM_WORDS]);
    ram[0x9010] = 0xd800;
    console.tick(&mut ram);
    assert_eq!(String::from_utf8(console.into_inner()).unwrap(), "\u{fffd}");
}

/// Echoes keys to the console until `q` arrives.
const ECHO: &str = r#"
              SET I, 0
:loop         SET A, [0x9000+I]
              IFE A, 0
                 SET PC, loop
              SET [0x9000+I], 0
              ADD I, 1
              AND I, 0xf
              SET [0x9010], A
              IFN A, 0x71            ; "q"
                 SET PC, loop
:end          SET PC, end
"#;

#[test]
fn keyboard_feeds_keys_through_the_ring_buffer() {
    let output = Output::default();
    let (keyboard, input) = Keyboard::new();
    let mut cpu = DCPU16::new(&assemble(ECHO).unwrap());
    cpu.attach(keyboard);
    cpu.attach(Console::new(output.clone()));

    // More keys than the buffer holds, so it has to wrap around and wait for free slots.
    let text = "The quick brown fox jumps over the lazy dog";
    for key in text.bytes() {
        assert!(input.press(key.into()));
    }

    // The program waits for input in a loop, so it runs until `q` arrives.
    cpu.run();
    assert_eq!(output.text(), "The q");

    // The keys behind `q` are still queued or in the buffer.
    assert_eq!(cpu.ram()[0x9005], u16::from(b'u'));
}

#[test]
fn keyboard_input_reports_a_dropped_keyboard() {
    let (keyboard, input) = Keyboard::new();
    drop(keyboard);
    assert!(!input.press(0x41));
}

#[test]
fn screen_decodes_cells_and_renders_text() {
    let mut cpu = DCPU16::new(
        &assemble("SET [0x8000], 0xf148\nSET [0x8001], 0x00e9\nSET [0x8020], 0x0021").unwrap(),
    );
    cpu.run();

    let cell = screen::cells(cpu.ram()).next().unwrap();
    assert_eq!(
        cell,
        screen::Cell {
            character: b'H',
            foreground: 0xf,
            background: 0x1,
            blink: false,
        }
    );
    assert!(screen::Cell::from_word(0x00e9).blink);

    let text = screen::text(cpu.ram());
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), screen::HEIGHT);
    assert_eq!(lines[0], format!("Hi{}", " ".repeat(screen::WIDTH - 2)));
    assert_eq!(lines[1].trim_end(), "!");
}
