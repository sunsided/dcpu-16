//! Terminal entry point: loads the program file given on the command line and
//! runs the interface with crossterm.

use crate::app::{App, Key, KeyCode};
use crate::{blink_on, ui};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEvent, KeyEventKind, KeyModifiers};
use std::process::ExitCode;
use std::time::{Duration, Instant};

/// Time between two frames, for roughly 60 frames per second.
const FRAME: Duration = Duration::from_millis(16);

/// Usage shown for missing or unknown arguments.
const USAGE: &str = "usage: dcpu16-tui <program.dasm>";

/// Assembles the program named on the command line and runs it until the user quits.
pub fn main() -> ExitCode {
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

    let name = std::path::Path::new(&path)
        .file_name()
        .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
    let terminal = ratatui::init();
    let result = run(terminal, App::new(program, name));
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

        let blink = blink_on(now - start);
        terminal.draw(|frame| ui::draw(frame, frame.area(), &app, blink))?;

        // Handle input until the next frame is due.
        let deadline = now + FRAME;
        loop {
            let timeout = deadline.saturating_duration_since(Instant::now());
            if !event::poll(timeout)? {
                break;
            }
            if let Event::Key(event) = event::read()?
                && let Some(key) = key(event)
            {
                app.handle_key(key);
            }
        }
    }

    Ok(())
}

/// Translates a crossterm key press; releases and keys the interface ignores give `None`.
fn key(event: KeyEvent) -> Option<Key> {
    if event.kind == KeyEventKind::Release {
        return None;
    }
    use ratatui::crossterm::event::KeyCode as Code;
    let code = match event.code {
        Code::Char(c) => KeyCode::Char(c),
        Code::Enter => KeyCode::Enter,
        Code::Backspace => KeyCode::Backspace,
        Code::Tab => KeyCode::Tab,
        Code::Esc => KeyCode::Esc,
        Code::Delete => KeyCode::Delete,
        Code::Up => KeyCode::Up,
        Code::Down => KeyCode::Down,
        Code::Left => KeyCode::Left,
        Code::Right => KeyCode::Right,
        Code::F(n) => KeyCode::F(n),
        _ => return None,
    };
    Some(Key {
        code,
        ctrl: event.modifiers.contains(KeyModifiers::CONTROL),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_crossterm_keys() {
        use ratatui::crossterm::event::KeyCode as Code;
        let press = |code, modifiers| KeyEvent::new(code, modifiers);

        assert_eq!(
            key(press(Code::Char('a'), KeyModifiers::NONE)),
            Some(Key::new(KeyCode::Char('a')))
        );
        assert_eq!(
            key(press(Code::Char('c'), KeyModifiers::CONTROL)),
            Some(Key {
                code: KeyCode::Char('c'),
                ctrl: true
            })
        );
        assert_eq!(
            key(press(Code::F(5), KeyModifiers::NONE)),
            Some(Key::new(KeyCode::F(5)))
        );
        assert_eq!(key(press(Code::Home, KeyModifiers::NONE)), None);

        let mut release = press(Code::Char('a'), KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(key(release), None);
    }
}
