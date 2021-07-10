use crate::register::Register;
use crate::Word;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Address {
    Register(Register),
    Literal(Word),
    Address(Word),
    ProgramCounter,
    StackPointer,
    Overflow,
}

impl Address {
    pub fn get_literal(&self) -> Option<Word> {
        match self {
            Self::Literal(value) => Some(*value),
            Self::Address(value) => Some(*value),
            _ => None,
        }
    }
}
