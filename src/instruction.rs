//! Decoded instructions together with their extra operand words.

use crate::disassemble::Disassemble;
use crate::instruction_argument::{InstructionArgument, InstructionArgumentDefinition, Location};
use crate::instruction_word::InstructionWord;
use crate::{DCPU16, Word};
use std::fmt::{Debug, Formatter};

/// A decoded instruction with all extra operands.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)] // word counts are the point; renaming would break the public API
pub enum Instruction {
    /// An instruction that has one word, i.e., does not take extra operands.
    OneWord {
        /// The decoded instruction word.
        instruction: InstructionWord,
        /// The raw word of the instruction.
        raw_instruction: Word,
    },
    /// An instruction that has two words, i.e., takes one extra operand.
    TwoWord {
        /// The decoded instruction word.
        instruction: InstructionWord,
        /// The raw word of the instruction.
        raw_instruction: Word,
        /// The first extra operand.
        raw_1st: Word,
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
        raw_2nd: Word,
    },
}

impl Instruction {
    /// Extracts the values of the instruction into a tuple.
    pub fn unpack(&self) -> (Word, InstructionWord, Option<Word>, Option<Word>) {
        match self {
            Self::OneWord {
                raw_instruction,
                instruction,
            } => (*raw_instruction, *instruction, None, None),
            Self::TwoWord {
                raw_instruction,
                instruction,
                raw_1st: a,
            } => (*raw_instruction, *instruction, Some(*a), None),
            Self::ThreeWord {
                raw_instruction,
                instruction,
                raw_1st: a,
                raw_2nd: b,
            } => (*raw_instruction, *instruction, Some(*a), Some(*b)),
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
    /// Where the value was read from and where a result is written to.
    pub location: Location,
    /// The resolved value.
    pub resolved_value: Word,
}

impl ResolvedValue {
    /// Unpacks the value into its location and the resolved value.
    pub fn unpack(&self) -> (Location, Word) {
        (self.location, self.resolved_value)
    }
}

/// An instruction and its resolved arguments.
pub struct InstructionWithOperands {
    /// The raw first word of the instruction.
    raw_instruction: Word,
    /// The decoded instruction word.
    pub instruction: InstructionWord,
    /// The resolved `a` operand.
    pub a: ResolvedValue,
    /// The resolved `b` operand; `None` for non-basic instructions.
    pub b: Option<ResolvedValue>,
}

impl InstructionWithOperands {
    /// Resolves the values for each argument of the instruction word.
    ///
    /// The operands are resolved in order, `a` before `b`, so side effects such as
    /// `PUSH` and `POP` apply in that order.
    pub fn resolve(cpu: &mut DCPU16, instruction: Instruction) -> Self {
        Self::with_operands(instruction, |definition, word| {
            cpu.resolve_argument(definition, word)
        })
    }

    /// Like [`Self::resolve`], but without side effects: the stack pointer does not move.
    /// Used to describe instructions that are skipped.
    pub fn inspect(cpu: &DCPU16, instruction: Instruction) -> Self {
        Self::with_operands(instruction, |definition, word| {
            cpu.inspect_argument(definition, word)
        })
    }

    /// Pairs each operand definition with its extra word and resolves it with `operand`.
    fn with_operands(
        instruction: Instruction,
        mut operand: impl FnMut(InstructionArgumentDefinition, Option<Word>) -> ResolvedValue,
    ) -> Self {
        let (raw_instruction, instruction_word, raw_1st, raw_2nd) = instruction.unpack();

        // Get the "a" and "b" value definitions from the original instruction.
        let (a, b) = instruction_word.unpack();

        // The extra words follow the instruction in operand order. If "a" uses an
        // inline value, e.g. a register or short literal, the first extra word
        // belongs to "b".
        let (a_word, b_word) = if a.has_extra_words() {
            (raw_1st, raw_2nd)
        } else {
            (None, raw_1st)
        };

        let a = operand(a, a_word);
        let b = b.map(|b| operand(b, b_word));

        InstructionWithOperands {
            raw_instruction,
            instruction: instruction_word,
            a,
            b,
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
            assert_eq!(
                self.b
                    .expect("require second argument")
                    .argument_definition
                    .num_extra_words(),
                1
            );
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
