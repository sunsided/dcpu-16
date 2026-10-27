//! Feeds arbitrary text to the assembler.
//!
//! Assembling must never panic, and every error must point at a position
//! that exists in the source.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|source: &str| {
    let Err(error) = dcpu16::assemble(source) else {
        return;
    };

    // LF, CRLF and a lone CR each end a line.
    let line_breaks = source.replace("\r\n", "\n").matches(['\n', '\r']).count();
    let position = error.position();
    assert!(
        (1..=line_breaks + 1).contains(&position.line),
        "{position:?} outside of {} lines",
        line_breaks + 1
    );
    assert!(position.column >= 1, "{position:?} has no column");

    // Formatting the error must not panic either.
    let _ = error.to_string();
});
