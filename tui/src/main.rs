//! A terminal user interface for the DCPU-16 emulator.
//!
//! Assembles a program, runs it at the 100 kHz of the original and shows the
//! screen, the console output, the registers and the code around the program
//! counter. Typed keys go to the program's keyboard.
//!
//! The same interface runs in a terminal (`native`) and, built with trunk for
//! `wasm32`, in a browser (`web`).

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

mod app;
#[cfg(not(target_arch = "wasm32"))]
mod native;
mod theme;
mod ui;
#[cfg(target_arch = "wasm32")]
mod web;

use std::time::Duration;

/// How long each phase of blinking characters lasts.
const BLINK: Duration = Duration::from_millis(500);

/// Whether blinking characters show after `elapsed` time.
fn blink_on(elapsed: Duration) -> bool {
    (elapsed.as_millis() / BLINK.as_millis()).is_multiple_of(2)
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    native::main()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    web::main();
}
