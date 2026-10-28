//! Echoes typed lines through the memory-mapped keyboard and console devices.
//!
//! Type a line and press Enter; the program running on the DCPU-16 prints it back
//! in upper case. A line containing `.` ends the program.

use dcpu16::devices::{Console, Keyboard};
use dcpu16::{DCPU16, assemble};
use std::io::Read;
use std::time::Duration;

/// Reads keys from the ring buffer at 0x9000 and writes them to the console at 0x9010,
/// turning lower-case letters into upper case.
const SOURCE: &str = r#"
              SET I, 0
:prompt       SET J, 0
:prompt_loop  SET A, [prompt_text+J]
              IFE A, 0
                 SET PC, wait
              SET [0x9010], A
              ADD J, 1
              SET PC, prompt_loop

:wait         SET A, [0x9000+I]
              IFE A, 0
                 SET PC, wait
              SET [0x9000+I], 0
              ADD I, 1
              AND I, 0xf

              IFG 0x61, A              ; below "a"?
                 SET PC, print
              IFG A, 0x7a              ; above "z"?
                 SET PC, print
              SUB A, 0x20
:print        SET [0x9010], A
              IFE A, 0x2e              ; "."
                 SET PC, end
              IFE A, 0x0a              ; Enter
                 SET PC, prompt
              SET PC, wait

:end          SET [0x9010], 0x0a
:halt         SET PC, halt

:prompt_text  DAT "> ", 0
"#;

fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let program = match assemble(SOURCE) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    };

    let (keyboard, input) = Keyboard::new();
    let mut cpu = DCPU16::new(&program);
    cpu.attach(keyboard);
    cpu.attach(Console::new(std::io::stdout()));

    // The terminal delivers input line by line; every byte becomes a key.
    std::thread::spawn(move || {
        for byte in std::io::stdin().lock().bytes() {
            let Ok(byte) = byte else { break };
            if !input.press(byte.into()) {
                break;
            }
        }
    });

    // Run at the 100 kHz of the original: 1000 cycles every 10 ms.
    while cpu.run_for(1000) {
        std::thread::sleep(Duration::from_millis(10));
    }
}
