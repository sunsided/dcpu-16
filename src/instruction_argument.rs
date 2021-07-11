use crate::{Register, Word, Decode};

/// The argument of an instruction.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InstructionArgument {
    /// The value of a register.
    Register(Register),
    /// A literal value.
    Literal(Word),
    /// The value at the specified address in RAM.
    ///
    /// Since the RAM contains the stack, this may also refer to a stack value.
    Address(Word),
    /// The value at the specified address in RAM, offset by the value in the specified register.
    ///
    /// Since the RAM contains the stack, this may also refer to a stack value.
    AddressOffset {
        /// The base address.
        address: Word,
        /// The register containing the value by which to offset the base address.
        register: Register
    },
    /// The value of the program counter.
    ProgramCounter,
    /// The value of the stack pointer.
    StackPointer,
    /// The value of the overflow register.
    Overflow,
}

impl InstructionArgument {
    /// Gets the literal value of the argument, if it exists.
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

/// The argument of an instruction, i.e., the type of an "a" or "b" value of an instruction.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InstructionArgumentDefinition {
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

impl InstructionArgumentDefinition {
    /// Gets the number of extra words to read.
    pub fn num_extra_words(&self) -> usize {
        match self {
            Self::AtAddressFromNextWordPlusRegister { .. } => 1,
            Self::AtAddressFromNextWord => 1,
            Self::NextWordLiteral => 1,
            _ => 0,
        }
    }

    /// Determines if the instruction has extra words to read.
    pub fn has_extra_words(&self) -> bool {
        self.num_extra_words() > 0
    }
}

impl Decode for InstructionArgumentDefinition {
    fn decode(value: Word) -> Self {
        assert!(value < 0x40);
        match value {
            0x00..=0x07 => InstructionArgumentDefinition::Register {
                register: Register::from(value),
            },
            0x08..=0x0f => InstructionArgumentDefinition::AtAddressFromRegister {
                register: Register::from(value - 0x08),
            },
            0x10..=0x17 => InstructionArgumentDefinition::AtAddressFromNextWordPlusRegister {
                register: Register::from(value - 0x10),
            },
            0x18 => InstructionArgumentDefinition::Pop,
            0x19 => InstructionArgumentDefinition::Peek,
            0x1a => InstructionArgumentDefinition::Push,
            0x1b => InstructionArgumentDefinition::OfStackPointer,
            0x1c => InstructionArgumentDefinition::OfProgramCounter,
            0x1d => InstructionArgumentDefinition::OfOverflow,
            0x1e => InstructionArgumentDefinition::AtAddressFromNextWord,
            0x1f => InstructionArgumentDefinition::NextWordLiteral,
            0x20..=0x3f => InstructionArgumentDefinition::Literal {
                value: value - 0x20,
            },
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_in_register_works() {
        assert_eq!(
            InstructionArgumentDefinition::decode(0x00),
            InstructionArgumentDefinition::Register {
                register: Register::A
            }
        );
        assert_eq!(
            InstructionArgumentDefinition::decode(0x01),
            InstructionArgumentDefinition::Register {
                register: Register::B
            }
        );
        assert_eq!(
            InstructionArgumentDefinition::decode(0x07),
            InstructionArgumentDefinition::Register {
                register: Register::J
            }
        );
    }

    #[test]
    fn value_at_register_works() {
        assert_eq!(
            InstructionArgumentDefinition::decode(0x08),
            InstructionArgumentDefinition::AtAddressFromRegister {
                register: Register::A
            }
        );
        assert_eq!(
            InstructionArgumentDefinition::decode(0x09),
            InstructionArgumentDefinition::AtAddressFromRegister {
                register: Register::B
            }
        );
        assert_eq!(
            InstructionArgumentDefinition::decode(0x0f),
            InstructionArgumentDefinition::AtAddressFromRegister {
                register: Register::J
            }
        );
    }

    #[test]
    fn value_at_next_word_plus_register_works() {
        assert_eq!(
            InstructionArgumentDefinition::decode(0x10),
            InstructionArgumentDefinition::AtAddressFromNextWordPlusRegister {
                register: Register::A
            }
        );
        assert_eq!(
            InstructionArgumentDefinition::decode(0x11),
            InstructionArgumentDefinition::AtAddressFromNextWordPlusRegister {
                register: Register::B
            }
        );
        assert_eq!(
            InstructionArgumentDefinition::decode(0x17),
            InstructionArgumentDefinition::AtAddressFromNextWordPlusRegister {
                register: Register::J
            }
        );
    }

    #[test]
    fn value_pop_peek_push_works() {
        assert_eq!(InstructionArgumentDefinition::decode(0x18), InstructionArgumentDefinition::Pop);
        assert_eq!(InstructionArgumentDefinition::decode(0x19), InstructionArgumentDefinition::Peek);
        assert_eq!(InstructionArgumentDefinition::decode(0x1a), InstructionArgumentDefinition::Push);
    }

    #[test]
    fn value_sp_pc_o_works() {
        assert_eq!(InstructionArgumentDefinition::decode(0x1b), InstructionArgumentDefinition::OfStackPointer);
        assert_eq!(InstructionArgumentDefinition::decode(0x1c), InstructionArgumentDefinition::OfProgramCounter);
        assert_eq!(InstructionArgumentDefinition::decode(0x1d), InstructionArgumentDefinition::OfOverflow);
    }

    #[test]
    fn value_next_word_works() {
        assert_eq!(InstructionArgumentDefinition::decode(0x1e), InstructionArgumentDefinition::AtAddressFromNextWord);
        assert_eq!(InstructionArgumentDefinition::decode(0x1f), InstructionArgumentDefinition::NextWordLiteral);
    }

    #[test]
    fn value_literal_works() {
        assert_eq!(InstructionArgumentDefinition::decode(0x20), InstructionArgumentDefinition::Literal { value: 0x00 });
        assert_eq!(InstructionArgumentDefinition::decode(0x3f), InstructionArgumentDefinition::Literal { value: 0x1f });
    }
}
