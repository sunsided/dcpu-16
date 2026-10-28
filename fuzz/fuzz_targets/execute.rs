//! Runs arbitrary words as a program, with keys typed on the keyboard.
//!
//! Executing must never panic, whatever the program does, and no instruction
//! may take more than five cycles: three for `DIV` and `MOD`, plus one per
//! extra word, or two for a test, plus one per extra word and one if it fails.
//! The number of steps is bounded because programs may loop forever.

#![no_main]

use dcpu16::DCPU16;
use dcpu16::devices::{Console, Keyboard};
use libfuzzer_sys::fuzz_target;

/// Maximum number of instructions to execute per input.
const MAX_STEPS: usize = 10_000;

fuzz_target!(|input: (Vec<u16>, Vec<u16>)| {
    let (program, keys) = input;
    let (keyboard, keyboard_input) = Keyboard::new();
    for key in keys {
        keyboard_input.press(key);
    }

    let mut cpu = DCPU16::new(&program);
    cpu.attach(keyboard);
    cpu.attach(Console::new(std::io::sink()));
    for _ in 0..MAX_STEPS {
        let before = cpu.cycles();
        let running = cpu.step();
        assert!(
            cpu.cycles() - before <= 5,
            "one instruction took more than 5 cycles"
        );
        if !running {
            break;
        }
    }
});
