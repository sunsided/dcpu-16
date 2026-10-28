//! A terminal user interface for the DCPU-16 emulator.
//!
//! Assembles a program, runs it at the 100 kHz of the original and shows the
//! screen, the console output, the registers and the code around the program
//! counter. Typed keys go to the program's keyboard.

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

mod app;
mod ui;

use crate::app::App;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event};
use std::process::ExitCode;
use std::time::{Duration, Instant};

/// Time between two frames, for roughly 60 frames per second.
const FRAME: Duration = Duration::from_millis(16);

/// How long each phase of blinking characters lasts.
const BLINK: Duration = Duration::from_millis(500);

/// Usage shown for missing or unknown arguments.
const USAGE: &str = "usage: dcpu16-tui <program.dasm>";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let path = match (args.next(), args.next()) {
        (Some(path), None) if path != "-h" && path != "--help" => path,
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{path}: {error}");
            return ExitCode::FAILURE;
        }
    };

    let program = match dcpu16::assemble(&source) {
        Ok(program) => program,
        Err(error) => {
            let position = error.position();
            eprintln!(
                "{path}:{}:{}: {}",
                position.line,
                position.column,
                error.kind()
            );
            return ExitCode::FAILURE;
        }
    };

    let terminal = ratatui::init();
    let result = run(terminal, App::new(program));
    ratatui::restore();

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the emulation and redraws until the user quits.
fn run(mut terminal: DefaultTerminal, mut app: App) -> std::io::Result<()> {
    let start = Instant::now();
    let mut last_frame = start;

    while !app.quit {
        let now = Instant::now();
        app.advance(now - last_frame);
        last_frame = now;

        let blink_on = ((now - start).as_millis() / BLINK.as_millis()).is_multiple_of(2);
        terminal.draw(|frame| ui::draw(frame, &app, blink_on))?;

        // Handle input until the next frame is due.
        let deadline = now + FRAME;
        loop {
            let timeout = deadline.saturating_duration_since(Instant::now());
            if !event::poll(timeout)? {
                break;
            }
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }
    }

    Ok(())
}
