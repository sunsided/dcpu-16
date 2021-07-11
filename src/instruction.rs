use crate::disassemble::Disassemble;
use crate::instruction_word::{InstructionWord};
use crate::instruction_argument::{InstructionArgumentDefinition, InstructionArgument};
use crate::{Word, DCPU16};
use std::fmt::{Debug, Formatter};

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

/// A resolved value containing both the argument definition, as well as the resolved value.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ResolvedValue {
    /// The definition of the argument from the instruction.
    pub argument_definition: InstructionArgumentDefinition,
    /// The interpreted address from the value.
    pub argument: InstructionArgument,
    /// The resolved value.
    pub resolved_value: Word,
}

impl ResolvedValue {
    /// Unpacks the value into a system address and the resolved value.
    pub fn unpack(&self) -> (InstructionArgument, Word) {
        (self.argument, self.resolved_value)
    }
}

/// An instruction and its resolved arguments.
pub struct InstructionWithOperands {
    raw_instruction: Word,
    pub instruction: InstructionWord,
    pub a: ResolvedValue,
    pub b: Option<ResolvedValue>,
}

impl InstructionWithOperands {
    /// Resolves the values for each argument of the instruction word.
    pub fn resolve(cpu: &mut DCPU16, instruction: Instruction) -> Self {
        let (raw_instruction, instruction_word, raw_1st, raw_2nd) = instruction.unpack();

        // Get the "a" and "b" value definitions from the original instruction.
        let (a, b) = instruction_word.unpack();

        // Most instructions have two operands.
        if let Some(b) = b {
            // The "a" value always exists, however it may use an "inline" value, e.g. a
            // register or default literal. In that case the "first operand" provided to the
            // instruction really belongs to the second value, i.e., "b".
            if a.has_extra_words() {
                let (lhs_arg, lhs) = cpu.resolve_argument(a, raw_1st);
                let (rhs_arg, rhs) = cpu.resolve_argument(b, raw_2nd);

                InstructionWithOperands {
                    raw_instruction,
                    instruction: instruction_word,
                    a: ResolvedValue {
                        argument_definition: a,
                        argument: lhs_arg,
                        resolved_value: lhs
                    },
                    b: Some(ResolvedValue {
                        argument_definition: b,
                        argument: rhs_arg,
                        resolved_value: rhs
                    }),
                }
            }
            else {
                // Since we know that the "a" value has no extra operand, we pass it to the second.
                let (lhs_arg, lhs) = cpu.resolve_argument(a, None);
                let (rhs_arg, rhs) = cpu.resolve_argument(b, raw_1st);
                assert!(raw_2nd.is_none());

                InstructionWithOperands {
                    raw_instruction,
                    instruction: instruction_word,
                    a: ResolvedValue {
                        argument_definition: a,
                        argument: lhs_arg,
                        resolved_value: lhs
                    },
                    b: Some(ResolvedValue {
                        argument_definition: b,
                        argument: rhs_arg,
                        resolved_value: rhs
                    }),
                }
            }
        }
        else {
            // A simpler version of above, we just need to anticipate the first operand.
            let (lhs_arg, lhs) = cpu.resolve_argument(a, raw_1st);
            assert!(a.has_extra_words() && raw_1st.is_some() || !a.has_extra_words());
            assert!(raw_2nd.is_none());

            InstructionWithOperands {
                raw_instruction,
                instruction: instruction_word,
                a: ResolvedValue {
                    argument_definition: a,
                    argument: lhs_arg,
                    resolved_value: lhs
                },
                b: None
            }
        }
    }

    /// Gets the length of the instruction including all operands.
    fn length_in_words(&self) -> usize {
        self.instruction.length_in_words()
    }
}

impl Debug for InstructionWithOperands {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        assert!(self.length_in_words() >= 1 && self.length_in_words() <= 3);

        if self.length_in_words() == 1 {
            write!(
                f,
                "{:04x?} ; {} ({:?})",
                self.raw_instruction,
                self.disassemble(),
                self.disassemble_human()
            )
        } else if self.length_in_words() == 2 {
            // The first value may be "inline", e.g. a default literal, a register etc.
            // In this case we need to look up the second operand, which then must be a literal value.
            let next_word = if self.a.argument_definition.num_extra_words() == 1 {
                self.a.argument.get_literal().unwrap()
            } else {
                self.b
                    .expect("require second argument")
                    .argument
                    .get_literal()
                    .unwrap()
            };
            write!(
                f,
                "{:04x?} {:04x?} ; {} ({:?})",
                self.raw_instruction,
                next_word,
                self.disassemble(),
                self.disassemble_human()
            )
        } else {
            assert_eq!(self.a.argument_definition.num_extra_words(), 1);
            assert_eq!(self.b.expect("require second argument").argument_definition.num_extra_words(), 1);
            write!(
                f,
                "{:04x?} {:04x?} {:04x?} ; {} ({:?})",
                self.raw_instruction,
                self.a.argument.get_literal().unwrap(),
                self.b
                    .expect("require second argument")
                    .argument
                    .get_literal()
                    .unwrap(),
                self.disassemble(),
                self.disassemble_human()
            )
        }
    }
}
