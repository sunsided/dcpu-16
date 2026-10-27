//! Runs arbitrary words as a program.
//!
//! Executing must never panic, whatever the program does. The number of steps
//! is bounded because programs may loop forever.

#![no_main]

use dcpu16::DCPU16;
use libfuzzer_sys::fuzz_target;

/// Maximum number of instructions to execute per input.
const MAX_STEPS: usize = 10_000;

fuzz_target!(|program: Vec<u16>| {
    let mut cpu = DCPU16::new(&program);
    for _ in 0..MAX_STEPS {
        if !cpu.step() {
            break;
        }
    }
});
