use crate::{DurationCycles, Register, Word};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Value {
    /// register (A, B, C, X, Y, Z, I or J, in that order)
    Register {
        register: Register,
    },
    /// \[register\]
    AtAddressFromRegister {
        register: Register,
    },
    /// \[next word + register\]
    AtAddressFromNextWordPlusRegister {
        register: Register,
    },
    /// POP / \[SP++\]
    Pop,
    /// PEEK / \[SP\]
    Peek,
    /// PUSH / \[--SP\]
    Push,
    OfStackPointer,
    OfProgramCounter,
    OfOverflow,
    /// \[next word\]
    AtAddressFromNextWord,
    /// next word (literal)
    NextWordLiteral,
    /// literal value 0x00-0x1f (literal)
    Literal {
        value: Word,
    },
}

impl DurationCycles for Value {
    fn base_cycle_count(&self) -> usize {
        // All values that read a word (0x10-0x17, 0x1e, and 0x1f) take 1 cycle to look up.
        // The rest take 0 cycles.
        match self {
            Self::AtAddressFromNextWordPlusRegister { .. } => 1,
            Self::AtAddressFromNextWord { .. } => 1,
            Self::NextWordLiteral { .. } => 1,
            _ => 0,
        }
    }
}

impl From<u16> for Value {
    fn from(value: u16) -> Self {
        assert!(value < 0x40);
        match value {
            0x00..=0x07 => Value::Register {
                register: Register::from(value),
            },
            0x08..=0x0f => Value::AtAddressFromRegister {
                register: Register::from(value - 0x08),
            },
            0x10..=0x17 => Value::AtAddressFromNextWordPlusRegister {
                register: Register::from(value - 0x10),
            },
            0x18 => Value::Pop,
            0x19 => Value::Peek,
            0x1a => Value::Push,
            0x1b => Value::OfStackPointer,
            0x1c => Value::OfProgramCounter,
            0x1d => Value::OfOverflow,
            0x1e => Value::AtAddressFromNextWord,
            0x1f => Value::NextWordLiteral,
            0x20..=0x3f => Value::Literal {
                value: value - 0x20,
            },
            _ => panic!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_in_register_works() {
        assert_eq!(
            Value::from(0x00),
            Value::Register {
                register: Register::A
            }
        );
        assert_eq!(
            Value::from(0x01),
            Value::Register {
                register: Register::B
            }
        );
        assert_eq!(
            Value::from(0x07),
            Value::Register {
                register: Register::J
            }
        );
    }

    #[test]
    fn value_at_register_works() {
        assert_eq!(
            Value::from(0x08),
            Value::AtAddressFromRegister {
                register: Register::A
            }
        );
        assert_eq!(
            Value::from(0x09),
            Value::AtAddressFromRegister {
                register: Register::B
            }
        );
        assert_eq!(
            Value::from(0x0f),
            Value::AtAddressFromRegister {
                register: Register::J
            }
        );
    }

    #[test]
    fn value_at_next_word_plus_register_works() {
        assert_eq!(
            Value::from(0x10),
            Value::AtAddressFromNextWordPlusRegister {
                register: Register::A
            }
        );
        assert_eq!(
            Value::from(0x11),
            Value::AtAddressFromNextWordPlusRegister {
                register: Register::B
            }
        );
        assert_eq!(
            Value::from(0x17),
            Value::AtAddressFromNextWordPlusRegister {
                register: Register::J
            }
        );
    }

    #[test]
    fn value_pop_peek_push_works() {
        assert_eq!(Value::from(0x18), Value::Pop);
        assert_eq!(Value::from(0x19), Value::Peek);
        assert_eq!(Value::from(0x1a), Value::Push);
    }

    #[test]
    fn value_sp_pc_o_works() {
        assert_eq!(Value::from(0x1b), Value::OfStackPointer);
        assert_eq!(Value::from(0x1c), Value::OfProgramCounter);
        assert_eq!(Value::from(0x1d), Value::OfOverflow);
    }

    #[test]
    fn value_next_word_works() {
        assert_eq!(Value::from(0x1e), Value::AtAddressFromNextWord);
        assert_eq!(Value::from(0x1f), Value::NextWordLiteral);
    }

    #[test]
    fn value_literal_works() {
        assert_eq!(Value::from(0x20), Value::Literal { value: 0x00 });
        assert_eq!(Value::from(0x3f), Value::Literal { value: 0x1f });
    }
}
