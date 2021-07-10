use crate::register::Register;
use crate::Word;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Address {
    Register(Register),
    Literal(Word),
    Address(Word),
    AddressOffset { address: Word, register: Register },
    ProgramCounter,
    StackPointer,
    Overflow,
}

impl Address {
    pub fn get_literal(&self) -> Option<Word> {
        match self {
            Self::Literal(value) => Some(*value),
            Self::Address(value) => Some(*value),
            Self::AddressOffset {
                address,
                register: _,
            } => Some(*address),
            _ => None,
        }
    }
}
