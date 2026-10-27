//! The DCPU-16 assembler.
//!
//! Source is parsed into [`MetaInstruction`]s by [`parser`], then laid out in
//! two passes: label addresses are estimated optimistically and refined until
//! no instruction changes size, so references to nearby labels can use the
//! short inline literal encoding.

mod error;
mod lexer;
mod parser;

pub use self::error::{AssembleError, AssembleErrorKind, Position};
use crate::instruction_argument::{InstructionArgument, SpecialRegister, StackOperation};
use crate::{Word, MAX_PROGRAM_WORDS};
use std::collections::HashMap;
use tracing::trace;

/// Assembles the source code into an DCPU-16 program bytecode.
///
/// # Errors
///
/// Returns an [`AssembleError`] pointing at the offending source position
/// if the source cannot be parsed, refers to undefined labels, or assembles
/// to a program longer than [`DCPU16::new`](crate::DCPU16::new) accepts.
pub fn assemble<T>(source: T) -> Result<Vec<Word>, AssembleError>
where
    T: AsRef<str>,
{
    let tokens = parser::parse(source.as_ref())?;

    // Labels are unique and every reference is defined; the parser ensures both.
    let mut label_map = HashMap::new();
    for token in tokens.iter() {
        if let MetaInstruction::Label(label) = token {
            label_map.insert(label.clone(), 0x0000u16);
        }
    }

    let mut instructions = Vec::new();
    let mut positions = Vec::new();
    let mut current_position: usize = 0;

    // First pass, materialize as many instructions as possible.
    for token in tokens {
        match token {
            MetaInstruction::Instruction(instruction, position) => {
                let materialized = instruction.materialize(&label_map);

                // We assume the best-case situation here.
                // This is helpful because small values can be inlined
                // into the instruction.
                current_position += materialized.len_estimate();

                instructions.push(materialized);
                positions.push(position);
            }
            MetaInstruction::Label(label) => {
                // A program that no longer fits is rejected right after this pass,
                // so a truncated label address is never used for output.
                label_map.insert(label.clone(), current_position as Word);
            }
        }
    }
    check_program_length(&instructions, &positions)?;

    // Second pass, attempt to materialize the "flexible" instructions.
    let mut current_position: usize;
    loop {
        let mut replace_list = Vec::new();
        current_position = 0;

        for (i, entry) in instructions.iter().enumerate() {
            let current_length = entry.len_estimate();
            current_position += current_length;

            match entry {
                MaterializedInstruction::Static { .. } => continue,
                MaterializedInstruction::Flexible { instruction, .. } => {
                    let new_instruction = instruction.materialize(&label_map);
                    let new_length = new_instruction.len_estimate();

                    let difference = new_length as i64 - current_length as i64;
                    // assert!(difference >= 0);

                    // If the instruction changed in size, we need to adjust all following
                    // label positions.
                    if difference != 0 {
                        // Store the updated entry for later replacement in the list.
                        replace_list.push((i, new_instruction));

                        // Update the labels.
                        for label_pos in label_map.values_mut() {
                            if usize::from(*label_pos) > current_position {
                                *label_pos = (*label_pos as i64 + difference) as Word;
                            }
                        }
                    }
                }
            }
        }

        // If no instruction was replaced we arrived at a local optimum.
        if replace_list.is_empty() {
            break;
        }

        // Update the improved instructions. They only ever grow, so the program
        // may have outgrown the limit.
        for (idx, instruction) in replace_list {
            instructions[idx] = instruction;
        }
        check_program_length(&instructions, &positions)?;
    }

    // Go through the instructions one last time and generate the byte stream.
    let mut bytesteam = Vec::with_capacity(current_position);
    for entry in instructions {
        write_materialized_instruction_into_bytestream(&mut bytesteam, entry, &mut label_map)
    }

    Ok(bytesteam)
}

/// Ensures the program fits into [`MAX_PROGRAM_WORDS`], otherwise reports the
/// source position of the first instruction that crosses the limit.
fn check_program_length(
    instructions: &[MaterializedInstruction],
    positions: &[Position],
) -> Result<(), AssembleError> {
    let mut end = 0;
    for (instruction, position) in instructions.iter().zip(positions) {
        end += instruction.len_estimate();
        if end > MAX_PROGRAM_WORDS {
            return Err(AssembleError::new(
                AssembleErrorKind::ProgramTooLarge {
                    max_words: MAX_PROGRAM_WORDS,
                },
                *position,
            ));
        }
    }
    Ok(())
}

/// Writes a materialized instruction into the bytestream.
/// A final pass of jump label address substitution is performed.
fn write_materialized_instruction_into_bytestream(
    bytesteam: &mut Vec<u16>,
    entry: MaterializedInstruction,
    label_map: &mut HashMap<String, u16>,
) {
    let length = entry.len_estimate();
    match entry {
        MaterializedInstruction::Static {
            instruction,
            instruction_word,
            arg1,
            arg2,
        } => {
            trace!(
                "instruction {instruction:?}, len = {words}",
                instruction = instruction,
                words = length
            );
            write_instruction_word_into_bytestream(bytesteam, instruction_word, arg1, arg2)
        }
        MaterializedInstruction::Flexible { instruction, .. } => {
            // We perform a final pass of baking the actual jump addresses
            // into the instructions.
            if let MaterializedInstruction::Flexible {
                instruction,
                instruction_word,
                arg1,
                arg2,
            } = instruction.materialize(label_map)
            {
                trace!(
                    "instruction {instruction:?}, len = {words}",
                    instruction = instruction,
                    words = length
                );
                write_instruction_word_into_bytestream(bytesteam, instruction_word, arg1, arg2)
            } else {
                unreachable!();
            }
        }
    }
}

/// Writes an individual instruction word into the bytestream.
fn write_instruction_word_into_bytestream(
    bytesteam: &mut Vec<u16>,
    instruction_word: u16,
    arg1: Option<u16>,
    arg2: Option<u16>,
) {
    bytesteam.push(instruction_word);
    if let Some(arg1) = arg1 {
        bytesteam.push(arg1);
    }
    if let Some(arg2) = arg2 {
        bytesteam.push(arg2);
    }
}

/// A [`MetaInstruction`] captures the both instruction and
/// jump label definitions in the original token stream.
#[derive(Debug, Clone)]
enum MetaInstruction {
    /// An instruction and the position of its mnemonic.
    Instruction(Instruction, Position),
    /// A label.
    Label(String),
}

/// An actual instruction with both its operands.
#[derive(Debug, Clone)]
enum Instruction {
    /// A basic (two-operand) instruction.
    Basic(BasicOperationName, InstructionArgument, Value),
    /// A non-basic (one-operand) instruction.
    NonBasic(NonBasicOperationName, Value),
}

/// A [`Value`] may either refer to an actual argument or
/// a reference to a label.
#[derive(Debug, Clone)]
enum Value {
    /// A value.
    Static(InstructionArgument),
    /// A reference to a label.
    LabelReference(String),
}

/// A basic operation with two arguments.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum BasicOperationName {
    /// `SET`: Sets `a` to `b`.
    Set,
    /// `ADD`: Sets `a` to `a+b`.
    Add,
    /// `SUB`: Sets `a` to `a-b`.
    Sub,
    /// `MUL`: Sets `a` to `a*b`.
    Mul,
    /// `DIV`: Sets `a` to `a/b`.
    Div,
    /// `MOD`: Sets `a` to `a%b`.
    Mod,
    /// `SHL`: Sets `a` to `a<<b`.
    Shl,
    /// `SHR`: Sets `a` to `a>>b`.
    Shr,
    /// `AND`: Sets `a` to `a&b`.
    And,
    /// `BOR`: Sets `a` to `a|b`.
    Bor,
    /// `XOR`: Sets `a` to `a^b`.
    Xor,
    /// `IFE`: Performs the next instruction only if `a==b`.
    Ife,
    /// `IFN`: Performs the next instruction only if `a!=b`.
    Ifn,
    /// `IFG`: Performs the next instruction only if `a>b`.
    Ifg,
    /// `IFB`: Performs the next instruction only if `(a&b)!=0`.
    Ifb,
}

/// A non-basic operation with one argument.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum NonBasicOperationName {
    /// `JSR`: Pushes the address of the next instruction and jumps to `a`.
    Jsr,
}

impl BasicOperationName {
    /// Bakes the instruction and its arguments into bytecode.
    fn bake(
        &self,
        a: InstructionArgument,
        b: InstructionArgument,
    ) -> (Word, Option<Word>, Option<Word>) {
        let opcode = match self {
            Self::Set => 0x1,
            Self::Add => 0x2,
            Self::Sub => 0x3,
            Self::Mul => 0x4,
            Self::Div => 0x5,
            Self::Mod => 0x6,
            Self::Shl => 0x7,
            Self::Shr => 0x8,
            Self::And => 0x9,
            Self::Bor => 0xA,
            Self::Xor => 0xB,
            Self::Ife => 0xC,
            Self::Ifn => 0xD,
            Self::Ifg => 0xE,
            Self::Ifb => 0xF,
        };

        let a_baked = a.bake_argument();
        let b_baked = b.bake_argument();

        let instruction = ((opcode & 0b1111)
            | ((a_baked.inline as u32 & 0b111_111) << 4)
            | ((b_baked.inline as u32 & 0b111_111) << 10)) as Word;
        if a_baked.literal.is_some() {
            (instruction, a_baked.literal, b_baked.literal)
        } else {
            (instruction, b_baked.literal, None)
        }
    }
}

impl NonBasicOperationName {
    /// Bakes the instruction and its arguments into bytecode.
    fn bake(&self, a: InstructionArgument) -> (Word, Option<Word>) {
        let opcode = match self {
            Self::Jsr => 0x1,
        };

        let a_baked = a.bake_argument();

        let instruction = (((opcode as u32 & 0b111_111) << 4)
            | ((a_baked.inline as u32 & 0b111_111) << 10)) as Word;
        (instruction, a_baked.literal)
    }
}

/// A value has an inline component and may additional
/// refer to an extra word.
struct MaterializedValue {
    /// The inline component that is embedded into the instruction word.
    pub inline: u8,
    /// The optional additional word.
    pub literal: Option<Word>,
}

impl InstructionArgument {
    /// Bakes the argument into a [`MaterializedValue`].
    fn bake_argument(&self) -> MaterializedValue {
        match self {
            Self::Register(register) => MaterializedValue {
                inline: (*register as usize) as u8,
                literal: None,
            },
            Self::AddressFromRegister(register) => MaterializedValue {
                inline: (*register as usize + 0x08) as u8,
                literal: None,
            },
            Self::AddressOffset { address, register } => MaterializedValue {
                inline: (*register as usize + 0x10) as u8,
                literal: Some(*address),
            },
            Self::StackOperation(op) => match op {
                StackOperation::Pop => MaterializedValue {
                    inline: 0x18,
                    literal: None,
                },
                StackOperation::Peek => MaterializedValue {
                    inline: 0x19,
                    literal: None,
                },
                StackOperation::Push => MaterializedValue {
                    inline: 0x1a,
                    literal: None,
                },
            },
            Self::SpecialRegister(sr) => match sr {
                SpecialRegister::StackPointer => MaterializedValue {
                    inline: 0x1b,
                    literal: None,
                },
                SpecialRegister::ProgramCounter => MaterializedValue {
                    inline: 0x1c,
                    literal: None,
                },
                SpecialRegister::Overflow => MaterializedValue {
                    inline: 0x1d,
                    literal: None,
                },
            },
            Self::Address(word) => MaterializedValue {
                inline: 0x1e,
                literal: Some(*word),
            },
            Self::Literal(word) => {
                if *word > 0x1f {
                    MaterializedValue {
                        inline: 0x1f,
                        literal: Some(*word),
                    }
                } else {
                    MaterializedValue {
                        inline: (word + 0x20) as u8,
                        literal: None,
                    }
                }
            }
        }
    }
}

/// Helper value that represents an instruction prior to being baked into
/// the bytestream.
///
/// At this point the instructions are generally clear, but due to the possibility of inlining
/// of small constants into the instruction word all addresses referring to jump labels need
/// to be known and iteratively optimized before the final instruction can be written.
/// The [`MaterializedInstruction::Flexible`] variant captures this notion.
#[derive(Debug)]
enum MaterializedInstruction {
    /// An instruction that is fixed in size.
    Static {
        /// The instruction this was materialized from.
        instruction: Instruction,
        /// The encoded instruction word.
        instruction_word: Word,
        /// The first extra operand word, if any.
        arg1: Option<Word>,
        /// The second extra operand word, if any.
        arg2: Option<Word>,
    },
    /// An instruction that is flexible in size, e.g. because
    /// it corresponds to a jump label that could be represented as
    /// an inline literal.
    Flexible {
        /// The instruction this was materialized from.
        instruction: Instruction,
        /// The encoded instruction word.
        instruction_word: Word,
        /// The first extra operand word, if any.
        arg1: Option<Word>,
        /// The second extra operand word, if any.
        arg2: Option<Word>,
    },
}

impl MaterializedInstruction {
    /// Gets the length of the instruction in words.
    /// For the [`MaterializedInstruction::Flexible`] variant a
    /// best-case estimate is provided.
    fn len_estimate(&self) -> usize {
        match self {
            Self::Static {
                instruction: _,
                instruction_word: _,
                arg1,
                arg2,
            } => {
                let mut size = 1;
                if arg1.is_some() {
                    size += 1;
                }
                if arg2.is_some() {
                    size += 1;
                }
                size
            }
            Self::Flexible {
                instruction: _,
                instruction_word: _,
                arg1,
                arg2,
            } => {
                let mut size = 1;
                if arg1.is_some() {
                    size += 1;
                }
                if arg2.is_some() {
                    size += 1;
                }
                size
            }
        }
    }
}

impl Instruction {
    /// Materializes an instruction given the map of jump labels to program addresses.
    fn materialize(&self, label_map: &HashMap<String, Word>) -> MaterializedInstruction {
        match self {
            Instruction::NonBasic(nbi, a) => {
                match a {
                    Value::Static(arg) => {
                        let (opcode, arg1) = nbi.bake(*arg);
                        MaterializedInstruction::Static {
                            instruction: self.clone(),
                            instruction_word: opcode,
                            arg1,
                            arg2: None,
                        }
                    }
                    Value::LabelReference(reference) => {
                        // We now optimistically generate the instruction based on the current
                        // best guess for the label address in the label map.
                        let address = label_map[reference];
                        let arg = InstructionArgument::Literal(address);
                        let (opcode, arg1) = nbi.bake(arg);

                        MaterializedInstruction::Flexible {
                            instruction: self.clone(),
                            instruction_word: opcode,
                            arg1,
                            arg2: None,
                        }
                    }
                }
            }
            Instruction::Basic(bi, a, b) => {
                // Both arguments must be fixed-sized for this to be static.
                match b {
                    Value::Static(arg2) => {
                        let (opcode, arg1, arg2) = bi.bake(*a, *arg2);
                        MaterializedInstruction::Static {
                            instruction: self.clone(),
                            instruction_word: opcode,
                            arg1,
                            arg2,
                        }
                    }
                    Value::LabelReference(reference) => {
                        // We now optimistically generate the instruction based on the current
                        // best guess for the label address in the label map.
                        let address = label_map[reference];
                        let arg2 = InstructionArgument::Literal(address);
                        let (opcode, arg1, arg2) = bi.bake(*a, arg2);

                        MaterializedInstruction::Flexible {
                            instruction: self.clone(),
                            instruction_word: opcode,
                            arg1,
                            arg2,
                        }
                    }
                }
            }
        }
    }
}
