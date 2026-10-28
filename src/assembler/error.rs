//! Errors reported by the assembler, with their source position.

use std::fmt::{Display, Formatter};

/// A position in the assembler source.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Position {
    /// The line, starting at 1.
    pub line: usize,
    /// The column in characters, starting at 1.
    pub column: usize,
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// An error raised while assembling source code.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AssembleError {
    /// The reason why assembling failed.
    kind: AssembleErrorKind,
    /// Where in the source the error was detected.
    position: Position,
}

/// The reason why assembling failed.
#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum AssembleErrorKind {
    /// A character that cannot start any token.
    UnexpectedCharacter(char),
    /// A `:` that is not followed by a label name.
    MissingLabelName,
    /// A malformed numeric literal.
    InvalidNumber(String),
    /// A numeric literal that does not fit into a 16-bit word.
    NumberOutOfRange(String),
    /// A string literal without its closing `"` on the same line.
    UnterminatedString,
    /// An unknown escape sequence in a string literal, given by the character after the `\`.
    InvalidEscape(char),
    /// A character in a string literal that does not fit into a 16-bit word.
    CharacterOutOfRange(char),
    /// An unknown instruction mnemonic.
    UnknownInstruction(String),
    /// A token other than the expected one.
    Expected {
        /// A description of what was expected.
        expected: &'static str,
        /// A description of what was found instead.
        found: String,
    },
    /// A label that is defined more than once.
    DuplicateLabel(String),
    /// A reference to a label that is never defined.
    UndefinedLabel(String),
    /// A bare label reference as the first operand of a basic instruction,
    /// which would assign to a literal.
    LabelNotAllowed(String),
    /// The assembled program would be longer than the emulator accepts.
    ProgramTooLarge {
        /// The maximum program length in words.
        max_words: usize,
    },
}

impl AssembleError {
    /// Creates an error of the given kind at the given position.
    pub(super) fn new(kind: AssembleErrorKind, position: Position) -> Self {
        Self { kind, position }
    }

    /// Gets the reason why assembling failed.
    pub fn kind(&self) -> &AssembleErrorKind {
        &self.kind
    }

    /// Gets the position in the source at which the error was detected.
    pub fn position(&self) -> Position {
        self.position
    }
}

impl Display for AssembleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.position, self.kind)
    }
}

impl Display for AssembleErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedCharacter(c) => write!(f, "unexpected character {:?}", c),
            Self::MissingLabelName => write!(f, "expected a label name after `:`"),
            Self::InvalidNumber(text) => write!(f, "invalid number `{}`", text),
            Self::NumberOutOfRange(text) => {
                write!(f, "number `{}` does not fit into a 16-bit word", text)
            }
            Self::UnterminatedString => write!(f, "string literal is missing its closing `\"`"),
            Self::InvalidEscape(c) => write!(f, "unknown escape sequence `\\{}`", c),
            Self::CharacterOutOfRange(c) => {
                write!(f, "character {:?} does not fit into a 16-bit word", c)
            }
            Self::UnknownInstruction(name) => write!(f, "unknown instruction `{}`", name),
            Self::Expected { expected, found } => {
                write!(f, "expected {}, found {}", expected, found)
            }
            Self::DuplicateLabel(name) => write!(f, "label `{}` is defined multiple times", name),
            Self::UndefinedLabel(name) => write!(f, "label `{}` is not defined", name),
            Self::ProgramTooLarge { max_words } => write!(
                f,
                "program exceeds the maximum length of {} words",
                max_words
            ),
            Self::LabelNotAllowed(name) => write!(
                f,
                "label `{}` cannot be assigned to; use `[{}]` to write to its address",
                name, name
            ),
        }
    }
}

impl std::error::Error for AssembleError {}
