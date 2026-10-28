//! The DCPU-16 assembler.
//!
//! Source is parsed into [`MetaInstruction`]s by [`parser`], then laid out in
//! repeated passes: label addresses start out optimistically low and are
//! refined until no instruction changes size, so references to nearby labels
//! can use the short inline literal encoding.

mod error;
mod lexer;
mod parser;

pub use self::error::{AssembleError, AssembleErrorKind, Position};
use crate::instruction_argument::{InstructionArgument, SpecialRegister, StackOperation};
use crate::{MAX_PROGRAM_WORDS, Register, Word};
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
    let lines = parser::parse(source.as_ref())?;

    // Labels are unique and every reference is defined; the parser ensures both.
    // Starting with every label at address 0 encodes all label literals in their
    // short inline form. Each pass then places the labels behind the instructions
    // as encoded, which only ever moves them up, so instructions only ever grow
    // and the passes settle on the shortest consistent layout.
    let mut labels: HashMap<&str, Word> = lines
        .iter()
        .filter_map(|line| match line {
            MetaInstruction::Label(name) => Some((name.as_str(), 0)),
            MetaInstruction::Instruction(..) => None,
        })
        .collect();

    let mut words = Vec::new();
    let mut encoded = Vec::new();
    for pass in 1.. {
        words.clear();
        encoded.clear();
        let mut addresses = HashMap::with_capacity(labels.len());
        for line in &lines {
            match line {
                MetaInstruction::Label(name) => {
                    // The length check below keeps every address within a word.
                    addresses.insert(name.as_str(), words.len() as Word);
                }
                MetaInstruction::Instruction(instruction, position) => {
                    let start = words.len();
                    instruction.encode(&labels, &mut words);
                    encoded.push((instruction, start..words.len()));
                    if words.len() > MAX_PROGRAM_WORDS {
                        return Err(AssembleError::new(
                            AssembleErrorKind::ProgramTooLarge {
                                max_words: MAX_PROGRAM_WORDS,
                            },
                            *position,
                        ));
                    }
                }
            }
        }

        if addresses == labels {
            for (instruction, range) in &encoded {
                trace!(
                    "{:04X}: {instruction:?} => {:04X?}",
                    range.start,
                    &words[range.clone()]
                );
            }
            trace!("assembled {} words in {pass} passes", words.len());
            break;
        }
        labels = addresses;
    }

    Ok(words)
}

/// A [`MetaInstruction`] captures both instructions and
/// label definitions in the original token stream.
#[derive(Debug, Clone)]
enum MetaInstruction {
    /// An instruction and the position of its mnemonic.
    Instruction(Instruction, Position),
    /// A label.
    Label(String),
}

/// An actual instruction with its operands, or raw data.
#[derive(Debug, Clone)]
enum Instruction {
    /// A basic (two-operand) instruction.
    Basic(BasicOperationName, Operand, Operand),
    /// A non-basic (one-operand) instruction.
    NonBasic(NonBasicOperationName, Operand),
    /// `DAT`: words emitted into the program as they are.
    Data(Vec<DataWord>),
}

/// An operand, which may depend on label addresses that are only known after layout.
#[derive(Debug, Clone)]
enum Operand {
    /// An operand that does not refer to a label.
    Static(InstructionArgument),
    /// The address of a label as a literal value, e.g. `SET PC, loop`.
    Label(String),
    /// The memory at a label's address, optionally offset by a register,
    /// e.g. `[data]` or `[data+I]`.
    LabelAddress {
        /// The referenced label.
        label: String,
        /// The register added to the label's address, if any.
        register: Option<Register>,
    },
}

/// A word of a `DAT` directive.
#[derive(Debug, Clone)]
enum DataWord {
    /// A literal word; string literals contribute one per character.
    Literal(Word),
    /// The address of a label.
    Label(String),
}

impl Instruction {
    /// Appends the encoded instruction to `words`, using `labels` for label addresses.
    fn encode(&self, labels: &HashMap<&str, Word>, words: &mut Vec<Word>) {
        match self {
            Self::Basic(operation, a, b) => {
                let (word, first, second) = operation.bake(a.resolve(labels), b.resolve(labels));
                words.push(word);
                words.extend(first);
                words.extend(second);
            }
            Self::NonBasic(operation, a) => {
                let (word, first) = operation.bake(a.resolve(labels));
                words.push(word);
                words.extend(first);
            }
            Self::Data(data) => words.extend(data.iter().map(|word| match word {
                DataWord::Literal(word) => *word,
                DataWord::Label(label) => labels[label.as_str()],
            })),
        }
    }
}

impl Operand {
    /// Resolves label references to the addresses in `labels`.
    fn resolve(&self, labels: &HashMap<&str, Word>) -> InstructionArgument {
        match self {
            Self::Static(argument) => *argument,
            Self::Label(label) => InstructionArgument::Literal(labels[label.as_str()]),
            Self::LabelAddress { label, register } => {
                let address = labels[label.as_str()];
                match register {
                    Some(register) => InstructionArgument::AddressOffset {
                        address,
                        register: *register,
                    },
                    None => InstructionArgument::Address(address),
                }
            }
        }
    }
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
