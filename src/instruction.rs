use crate::instruction_argument::InstructionArgumentDefinition;
use std::fmt::Debug;
use tracing::trace;
use crate::Word;

/// A decoded instruction with all extra operands.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Instruction {
    /// An instruction that has one word, i.e., does not take extra operands.
    OneWord {
        /// The decoded instruction word.
        instruction: InstructionWord,
        /// The raw word of the instruction.
        raw_instruction: Word
    },
    /// An instruction that has two words, i.e., takes one extra operand.
    TwoWord {
        /// The decoded instruction word.
        instruction: InstructionWord,
        /// The raw word of the instruction.
        raw_instruction: Word,
        /// The first extra operand.
        raw_1st: Word
    },
    /// An instruction that has three words, i.e., takes two extra operands.
    ThreeWord {
        /// The decoded instruction.
        instruction: InstructionWord,
        /// The raw word of the instruction word.
        raw_instruction: Word,
        /// The first extra operand.
        raw_1st: Word,
        /// The second extra operand.
        raw_2nd: Word
    },
}

impl Instruction {
    /// Extracts the values of the instruction into a tuple.
    pub fn unpack(&self) -> (Word, InstructionWord, Option<Word>, Option<Word>) {
        match self {
            Self::OneWord { raw_instruction, instruction} => (*raw_instruction, *instruction, None, None),
            Self::TwoWord { raw_instruction, instruction, raw_1st: a } => (*raw_instruction, *instruction, Some(*a), None),
            Self::ThreeWord { raw_instruction, instruction, raw_1st: a, raw_2nd: b } => (* raw_instruction, *instruction, Some(*a), Some(*b)),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InstructionWord {
    /// Non-basic instruction.
    NonBasic(NonBasicInstruction),
    /// Sets `a` to `b`.
    ///
    /// Takes 1 cycle, plus the cost of `a` and `b`.
    Set { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Sets `a` to `a+b`, sets `O` to `0x0001` if there's an overflow, `0x0` otherwise.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`.
    Add { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Sets `a` to `a-b`, sets `O` to `0xffff` if there's an underflow, `0x0` otherwise.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`.
    Sub { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Sets `a` to `a*b`, sets `O` to `((a*b)>>16)&0xffff`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`.
    Mul { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Sets `a` to `a/b`, sets `O` to `((a<<16)/b)&0xffff`. if `b==0`, sets `a` and `O` to `0` instead.
    ///
    /// Takes 3 cycles, plus the cost of `a` and `b`.
    Div { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Sets `a` to `a%b`. if `b==0`, sets `a` to `0` instead.
    ///
    /// Takes 3 cycles, plus the cost of `a` and `b`.
    Mod { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Sets `a` to `a<<b`, sets `O` to `((a<<b)>>16)&0xffff`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`.
    Shl { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Sets `a` to `a>>b`, sets `O` to `((a<<16)>>b)&0xffff`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`.
    Shr { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Sets `a` to `a&b`.
    ///
    /// Takes 1 cycle, plus the cost of `a` and `b`.
    And { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Sets `a` to `a|b`.
    ///
    /// Takes 1 cycle, plus the cost of `a` and `b`.
    Bor { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Sets `a` to `a^b`.
    ///
    /// Takes 1 cycle, plus the cost of `a` and `b`.
    Xor { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Performs next instruction only if `a==b`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`, plus 1 if the test fails.
    Ife { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Performs next instruction only if `a!=b`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`, plus 1 if the test fails.
    Ifn { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Performs next instruction only if `a>b`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`, plus 1 if the test fails.
    Ifg { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
    /// Performs next instruction only if `(a&b)!=0`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`, plus 1 if the test fails.
    Ifb { a: InstructionArgumentDefinition, b: InstructionArgumentDefinition },
}

/// Non-basic opcodes always have their lower four bits unset, have one value and a six bit opcode.
/// In binary, they have the format: `aaaaaaoooooo0000`
/// The value `(a)` is in the same six bit format as defined earlier.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NonBasicInstruction {
    /// Reserved for future expansion.
    Reserved,
    /// Pushes the address of the next instruction to the stack, then sets `PC` to `a`.
    /// Takes 2 cycles, plus the cost of `a`.
    Jsr { a: InstructionArgumentDefinition },
}

impl From<u16> for InstructionWord {
    /// Decodes an [`InstructionWord`] from a raw word.
    fn from(value: u16) -> Self {
        let opcode = value & 0b1111;
        let a = InstructionArgumentDefinition::from((value >> 4) & 0b111_111);
        let b = InstructionArgumentDefinition::from((value >> 10) & 0b111_111);

        match opcode {
            0x0 => Self::NonBasic(NonBasicInstruction::from(value)),
            0x1 => Self::Set { a, b },
            0x2 => Self::Add { a, b },
            0x3 => Self::Sub { a, b },
            0x4 => Self::Mul { a, b },
            0x5 => Self::Div { a, b },
            0x6 => Self::Mod { a, b },
            0x7 => Self::Shl { a, b },
            0x8 => Self::Shr { a, b },
            0x9 => Self::And { a, b },
            0xa => Self::Bor { a, b },
            0xb => Self::Xor { a, b },
            0xc => Self::Ife { a, b },
            0xd => Self::Ifn { a, b },
            0xe => Self::Ifg { a, b },
            0xf => Self::Ifb { a, b },
            _ => panic!(),
        }
    }
}

impl From<u16> for NonBasicInstruction {
    /// Decodes an [`NonBasicInstruction`] from a raw word.
    fn from(value: u16) -> NonBasicInstruction {
        assert_eq!(value & 0b1111, 0x0);
        let opcode = (value >> 4) & 0b111_111;
        let a_word = (value >> 10) & 0b111_111;
        let a = InstructionArgumentDefinition::from(a_word);

        trace!(
            "Decoding non-basic instruction {instruction:04X}, opcode {opcode:02X}, value {value:02X}",
            instruction = value,
            opcode = opcode,
            value = a_word
        );

        match opcode {
            0x00 => NonBasicInstruction::Reserved,
            0x01 => NonBasicInstruction::Jsr { a },
            0x02..=0x3f => NonBasicInstruction::Reserved,
            _ => panic!(),
        }
    }
}

impl InstructionWord {
    /// Gets the length of the instruction in words.
    pub fn length_in_words(&self) -> usize {
        let len_from_values = match self {
            Self::NonBasic(op) => op.length_in_words(),
            Self::Set { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::And { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Bor { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Xor { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Add { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Sub { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Mul { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Shr { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Shl { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Div { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Mod { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Ife { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Ifn { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Ifg { a, b } => a.num_extra_words() + b.num_extra_words(),
            Self::Ifb { a, b } => a.num_extra_words() + b.num_extra_words(),
        };

        // We're adding one to count this instruction in.
        1 + len_from_values
    }

    /// Unpacks the instruction arguments into a first value and an optional second value.
    pub fn unpack(&self) -> (InstructionArgumentDefinition, Option<InstructionArgumentDefinition>) {
        match self {
            Self::NonBasic(op) => op.unpack(),
            Self::Set { a, b } => (*a, Some(*b)),
            Self::And { a, b } => (*a, Some(*b)),
            Self::Bor { a, b } => (*a, Some(*b)),
            Self::Xor { a, b } => (*a, Some(*b)),
            Self::Add { a, b } => (*a, Some(*b)),
            Self::Sub { a, b } => (*a, Some(*b)),
            Self::Mul { a, b } => (*a, Some(*b)),
            Self::Shr { a, b } => (*a, Some(*b)),
            Self::Shl { a, b } => (*a, Some(*b)),
            Self::Div { a, b } => (*a, Some(*b)),
            Self::Mod { a, b } => (*a, Some(*b)),
            Self::Ife { a, b } => (*a, Some(*b)),
            Self::Ifn { a, b } => (*a, Some(*b)),
            Self::Ifg { a, b } => (*a, Some(*b)),
            Self::Ifb { a, b } => (*a, Some(*b)),
        }
    }
}

impl NonBasicInstruction {
    /// Gets the length of the instruction in words.
    pub fn length_in_words(&self) -> usize {
        // Note that this operation is contained in the already loaded
        // instruction, hence we do not add another offset.
        match self {
            Self::Reserved => 0,
            Self::Jsr { a } => a.num_extra_words(),
        }
    }

    /// Unpacks the instruction arguments into a first value and an optional second value.
    pub fn unpack(&self) -> (InstructionArgumentDefinition, Option<InstructionArgumentDefinition>) {
        match self {
            Self::Reserved => panic!(),
            Self::Jsr { a } => (*a, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::Register;

    #[test]
    fn non_basic_instruction_reserved_works() {
        assert_eq!(
            InstructionWord::from(0b000000_000000_0000),
            InstructionWord::NonBasic(NonBasicInstruction::Reserved)
        );

        assert_eq!(
            InstructionWord::from(0b000000_000010_0000),
            InstructionWord::NonBasic(NonBasicInstruction::Reserved)
        );

        assert_eq!(
            InstructionWord::from(0b000000_111111_0000),
            InstructionWord::NonBasic(NonBasicInstruction::Reserved)
        );
    }

    #[test]
    fn non_basic_instruction_jsr_works() {
        assert_eq!(
            InstructionWord::from(0b010001_000001_0000),
            InstructionWord::NonBasic(NonBasicInstruction::Jsr {
                a: InstructionArgumentDefinition::AtAddressFromNextWordPlusRegister {
                    register: Register::B
                }
            })
        );
    }

    #[test]
    fn set_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_0001),
            InstructionWord::Set {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn add_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_0010),
            InstructionWord::Add {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn sub_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_0011),
            InstructionWord::Sub {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn mul_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_0100),
            InstructionWord::Mul {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn div_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_0101),
            InstructionWord::Div {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn mod_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_0110),
            InstructionWord::Mod {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn shl_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_0111),
            InstructionWord::Shl {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn shr_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_1000),
            InstructionWord::Shr {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn and_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_1001),
            InstructionWord::And {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn bor_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_1010),
            InstructionWord::Bor {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn xor_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_1011),
            InstructionWord::Xor {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn ife_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_1100),
            InstructionWord::Ife {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn ifn_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_1101),
            InstructionWord::Ifn {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn ifg_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_1110),
            InstructionWord::Ifg {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn ifb_works() {
        assert_eq!(
            InstructionWord::from(0b000011_000000_1111),
            InstructionWord::Ifb {
                a: InstructionArgumentDefinition::Register {
                    register: Register::A
                },
                b: InstructionArgumentDefinition::Register {
                    register: Register::X
                }
            }
        );
    }
}
