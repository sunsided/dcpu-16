//! The demo programs built into the browser build, and the choice of what to
//! run from the page's query parameters.

// Only the browser build loads programs this way; the tests cover it natively.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use crate::app::Program;

/// The demo programs, in the order F3 cycles through them.
const BUNDLED: [(&str, &str); 2] = [
    (
        "typewriter.dasm",
        include_str!("../programs/typewriter.dasm"),
    ),
    ("hello.dasm", include_str!("../programs/hello.dasm")),
];

/// Name of a program passed as source without a name.
const DEFAULT_NAME: &str = "shared.dasm";

/// What to run, and a notice for the user if a request could not be met.
pub struct Selection {
    /// The programs, the one to start with first; never empty.
    pub programs: Vec<Program>,
    /// Why a requested program is not running, if it is not.
    pub notice: Option<String>,
}

/// Chooses the programs to run.
///
/// `source` is assembly source to run first, named `name`. `program` names a demo
/// to start with instead of the first, with or without the `.dasm` extension.
/// The demos always follow, so F3 can switch to them, and stand in for source
/// that does not assemble.
pub fn select(program: Option<&str>, source: Option<&str>, name: Option<&str>) -> Selection {
    let mut programs: Vec<Program> = BUNDLED
        .iter()
        .map(|(name, source)| Program {
            name: (*name).to_string(),
            words: dcpu16::assemble(source).expect("demo programs assemble"),
        })
        .collect();
    let mut notice = None;

    if let Some(program) = program {
        let wanted = program.strip_suffix(".dasm").unwrap_or(program);
        match programs
            .iter()
            .position(|p| p.name.strip_suffix(".dasm") == Some(wanted))
        {
            Some(index) => programs.rotate_left(index),
            None => notice = Some(format!("no demo program named `{program}`")),
        }
    }

    if let Some(source) = source {
        let name = name.filter(|name| !name.is_empty()).unwrap_or(DEFAULT_NAME);
        match dcpu16::assemble(source) {
            Ok(words) => programs.insert(
                0,
                Program {
                    name: name.to_string(),
                    words,
                },
            ),
            Err(error) => {
                let position = error.position();
                notice = Some(format!(
                    "{name}:{}:{}: {}",
                    position.line,
                    position.column,
                    error.kind()
                ));
            }
        }
    }

    Selection { programs, notice }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(selection: &Selection) -> Vec<&str> {
        selection.programs.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn starts_with_the_first_demo_by_default() {
        let selection = select(None, None, None);
        assert_eq!(names(&selection), ["typewriter.dasm", "hello.dasm"]);
        assert_eq!(selection.notice, None);
    }

    #[test]
    fn starts_with_a_demo_by_name() {
        for name in ["hello", "hello.dasm"] {
            let selection = select(Some(name), None, None);
            assert_eq!(names(&selection), ["hello.dasm", "typewriter.dasm"]);
            assert_eq!(selection.notice, None);
        }
    }

    #[test]
    fn reports_an_unknown_demo() {
        let selection = select(Some("tetris"), None, None);
        assert_eq!(names(&selection), ["typewriter.dasm", "hello.dasm"]);
        assert_eq!(
            selection.notice.as_deref(),
            Some("no demo program named `tetris`")
        );
    }

    #[test]
    fn runs_source_first() {
        let selection = select(None, Some("SET A, 1"), Some("mine.dasm"));
        assert_eq!(
            names(&selection),
            ["mine.dasm", "typewriter.dasm", "hello.dasm"]
        );
        assert_eq!(selection.programs[0].words, [0x8401]);

        let selection = select(Some("hello"), Some("SET A, 1"), None);
        assert_eq!(
            names(&selection),
            ["shared.dasm", "hello.dasm", "typewriter.dasm"]
        );
    }

    #[test]
    fn reports_source_that_does_not_assemble() {
        let selection = select(None, Some("SET A,"), None);
        assert_eq!(names(&selection), ["typewriter.dasm", "hello.dasm"]);
        assert_eq!(
            selection.notice.as_deref(),
            Some("shared.dasm:1:7: expected an operand, found end of line")
        );
    }
}
