use crate::instruction_argument::{InstructionArgument, SpecialRegister, StackOperation};
use crate::{Register, Word};
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use tracing::trace;

#[derive(Parser)]
#[grammar = "assemble.pest"]
struct AssembleParser;

/// Assembles the source code into an DCPU-16 program bytecode.
pub fn assemble<T>(source: T) -> Vec<Word>
where
    T: AsRef<str>,
{
    let tokens = get_meta_instructions(source);

    let mut label_map = HashMap::new();
    for token in tokens.iter() {
        if let MetaInstruction::Label(label) = token {
            if label_map.insert(label.clone(), 0x0000u16).is_some() {
                panic!("Label '{}' defined multiple times", label);
            }
        }
    }

    let mut instructions = Vec::new();
    let mut current_position: Word = 0x0000;

    // First pass, materialize as many instructions as possible.
    for token in tokens {
        match token {
            MetaInstruction::Instruction(instruction) => {
                let materialized = instruction.materialize(&label_map);

                // We assume the best-case situation here.
                // This is helpful because small values can be inlined
                // into the instruction.
                let len = materialized.len();
                current_position += len as Word;

                instructions.push(materialized);
            }
            MetaInstruction::Label(label) => {
                label_map.insert(label.clone(), current_position);
            }
        }
    }

    // Second pass, attempt to materialize the "flexible" instructions.
    let mut current_position: Word;
    loop {
        let mut replace_list = Vec::new();
        current_position = 0x0000;

        for (i, entry) in instructions.iter().enumerate() {
            let current_length = entry.len();
            current_position += current_length as Word;

            match entry {
                MaterializedInstruction::Static { .. } => continue,
                MaterializedInstruction::Flexible { instruction, .. } => {
                    let new_instruction = instruction.materialize(&label_map);
                    let new_length = new_instruction.len();

                    let difference = new_length as i64 - current_length as i64;
                    // assert!(difference >= 0);

                    // If the instruction changed in size, we need to adjust all following
                    // label positions.
                    if difference != 0 {
                        // Store the updated entry for later replacement in the list.
                        replace_list.push((i, new_instruction));

                        // Update the labels.
                        for (_label, label_pos) in label_map.iter_mut() {
                            if *label_pos > current_position {
                                *label_pos = (*label_pos as i64 + difference) as Word;
                            }
                        }
                    }
                }
            }
        }

        // If no instruction was replaced we achieved an optimum.
        if replace_list.len() == 0 {
            break;
        }

        // Update the improved instructions.
        for (idx, instruction) in replace_list {
            instructions[idx] = instruction;
        }
    }

    // Go through the instructions one last time and generate the byte stream.
    let mut bytesteam = Vec::with_capacity(current_position as usize);
    for entry in instructions {
        write_materialized_instruction_into_bytestream(&mut bytesteam, entry, &mut label_map)
    }

    bytesteam
}

/// Writes a materialized instruction into the bytestream.
/// A final pass of jump label address substitution is performed.
fn write_materialized_instruction_into_bytestream(
    mut bytesteam: &mut Vec<u16>,
    entry: MaterializedInstruction,
    label_map: &mut HashMap<String, u16>,
) {
    let length = entry.len();
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
            write_instruction_word_into_bytestream(&mut bytesteam, instruction_word, arg1, arg2)
        }
        MaterializedInstruction::Flexible { instruction, .. } => {
            // We perform a final pass of baking the actual jump addresses
            // into the instructions.
            if let MaterializedInstruction::Flexible {
                instruction,
                instruction_word,
                arg1,
                arg2,
            } = instruction.materialize(&label_map)
            {
                trace!(
                    "instruction {instruction:?}, len = {words}",
                    instruction = instruction,
                    words = length
                );
                write_instruction_word_into_bytestream(&mut bytesteam, instruction_word, arg1, arg2)
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
    if arg1.is_some() {
        bytesteam.push(arg1.unwrap());
    }
    if arg2.is_some() {
        bytesteam.push(arg2.unwrap());
    }
}

/// Parses the source and generates a stream of [`MetaInstruction`] instances.
fn get_meta_instructions<T>(source: T) -> Vec<MetaInstruction>
where
    T: AsRef<str>,
{
    // Get the top-level program rule.
    let mut program =
        AssembleParser::parse(Rule::program, source.as_ref()).expect("unsuccessful parse");
    let program = program.next().unwrap();

    let mut meta_instructions = Vec::new();
    for record in program.into_inner() {
        let token = match record.as_rule() {
            Rule::label => {
                let inner = record.into_inner();
                MetaInstruction::Label(String::from(inner.as_str()))
            }
            Rule::basic_instruction => {
                let mut instruction = record.into_inner();

                let op = instruction.next().unwrap();
                let a = instruction.next().unwrap();
                let b = instruction.next().unwrap();

                let operation = parse_basic_operation(op);
                let value_a = parse_value(a);
                let value_b = parse_value(b);

                let instruction = Instruction::BasicInstruction(operation, value_a, value_b);
                MetaInstruction::Instruction(instruction)
            }
            Rule::nonbasic_instruction => {
                let mut instruction = record.into_inner();

                let op = instruction.next().unwrap();
                let a = instruction.next().unwrap();
                assert!(instruction.next().is_none());

                let operation = parse_nonbasic_operation(op);
                let value_a = parse_value(a);

                let instruction = Instruction::NonBasicInstruction(operation, value_a);
                MetaInstruction::Instruction(instruction)
            }
            Rule::EOI => {
                break;
            }
            _ => {
                println!("record = {:?}", record);
                unreachable!()
            }
        };

        meta_instructions.push(token);
    }
    meta_instructions
}

/// A [`MetaInstruction`] captures the both instruction and
/// jump label definitions in the original token stream.
#[derive(Debug, Clone)]
enum MetaInstruction {
    /// An instruction.
    Instruction(Instruction),
    /// A label.
    Label(String),
}

/// An actual instruction with both its operands.
#[derive(Debug, Clone)]
enum Instruction {
    /// A basic (two-operand) instruction.
    BasicInstruction(BasicOperationName, Value, Value),
    /// A non-basic (one-operand) instruction.
    NonBasicInstruction(NonBasicOperationName, Value),
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

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum BasicOperationName {
    SET,
    ADD,
    SUB,
    MUL,
    DIV,
    MOD,
    SHL,
    SHR,
    AND,
    BOR,
    XOR,
    IFE,
    IFN,
    IFG,
    IFB,
}

impl BasicOperationName {
    fn bake(
        &self,
        a: InstructionArgument,
        b: InstructionArgument,
    ) -> (Word, Option<Word>, Option<Word>) {
        let opcode = match self {
            Self::SET => 0x1,
            Self::ADD => 0x2,
            Self::SUB => 0x3,
            Self::MUL => 0x4,
            Self::DIV => 0x5,
            Self::MOD => 0x6,
            Self::SHL => 0x7,
            Self::SHR => 0x8,
            Self::AND => 0x9,
            Self::BOR => 0xA,
            Self::XOR => 0xB,
            Self::IFE => 0xC,
            Self::IFN => 0xD,
            Self::IFG => 0xE,
            Self::IFB => 0xF,
        };

        let a_baked = a.bake();
        let b_baked = b.bake();

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
    fn bake(&self, a: InstructionArgument) -> (Word, Option<Word>) {
        let opcode = match self {
            Self::JSR => 0x1,
        };

        let a_baked = a.bake();

        let instruction = (((opcode as u32 & 0b111_111) << 4)
            | ((a_baked.inline as u32 & 0b111_111) << 10)) as Word;
        (instruction, a_baked.literal)
    }
}

struct MaterializedValue {
    pub inline: u8,
    pub literal: Option<Word>,
}

impl InstructionArgument {
    fn bake(&self) -> MaterializedValue {
        match self {
            Self::Register(register) => MaterializedValue {
                inline: (*register as usize + 0x00) as u8,
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

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum NonBasicOperationName {
    JSR,
}

fn parse_basic_operation(pair: Pair<Rule>) -> BasicOperationName {
    match pair.as_str() {
        "SET" => BasicOperationName::SET,
        "ADD" => BasicOperationName::ADD,
        "SUB" => BasicOperationName::SUB,
        "MUL" => BasicOperationName::MUL,
        "DIV" => BasicOperationName::DIV,
        "MOD" => BasicOperationName::MOD,
        "SHL" => BasicOperationName::SHL,
        "SHR" => BasicOperationName::SHR,
        "AND" => BasicOperationName::AND,
        "BOR" => BasicOperationName::BOR,
        "XOR" => BasicOperationName::XOR,
        "IFE" => BasicOperationName::IFE,
        "IFN" => BasicOperationName::IFN,
        "IFG" => BasicOperationName::IFG,
        "IFB" => BasicOperationName::IFB,
        _ => unimplemented!(),
    }
}

fn parse_nonbasic_operation(pair: Pair<Rule>) -> NonBasicOperationName {
    match pair.as_str() {
        "JSR" => NonBasicOperationName::JSR,
        _ => unimplemented!(),
    }
}

fn parse_value(pair: Pair<Rule>) -> Value {
    match pair.as_rule() {
        Rule::literal => parse_literal(pair),
        Rule::register => parse_register(pair),
        Rule::address => parse_address(pair),
        Rule::address_with_offset => parse_address_with_offset(pair),
        Rule::special_register => parse_special_register(pair),
        Rule::stack_op => parse_stack_op(pair),
        Rule::label_ref => parse_label_ref(pair),
        _ => {
            println!("{:?}", pair);
            unreachable!()
        }
    }
}

fn parse_register(pair: Pair<Rule>) -> Value {
    Value::Static(InstructionArgument::Register(parse_register_raw(pair)))
}

fn parse_register_raw(pair: Pair<Rule>) -> Register {
    match pair.as_str() {
        "A" => Register::A,
        "B" => Register::B,
        "C" => Register::C,
        "X" => Register::X,
        "Y" => Register::Y,
        "Z" => Register::Z,
        "I" => Register::I,
        "J" => Register::J,
        _ => unreachable!(),
    }
}

fn parse_literal(pair: Pair<Rule>) -> Value {
    let item = pair.into_inner().next().unwrap();
    let word = parse_literal_raw(item);
    Value::Static(InstructionArgument::Literal(word))
}

fn parse_literal_raw(pair: Pair<Rule>) -> Word {
    match pair.as_rule() {
        Rule::value_dec => {
            u16::from_str_radix(pair.as_str(), 10).expect("invalid format for decimal literal")
        }
        Rule::value_hex => u16::from_str_radix(pair.as_str().trim_start_matches("0x"), 16)
            .expect("invalid format for hex literal"),
        _ => unreachable!(),
    }
}

fn parse_address(pair: Pair<Rule>) -> Value {
    let mut address = pair.into_inner();

    // Skip opening bracket.
    address.next();

    let literal = address.next().unwrap();
    match literal.as_rule() {
        Rule::literal => {
            let item = literal.into_inner().next().unwrap();
            let word = parse_literal_raw(item);
            Value::Static(InstructionArgument::Address(word))
        }
        Rule::register => {
            let register = parse_register_raw(literal);
            Value::Static(InstructionArgument::AddressFromRegister(register))
        }
        _ => unreachable!(),
    }
}

fn parse_address_with_offset(pair: Pair<Rule>) -> Value {
    let mut address = pair.into_inner();

    // Skip opening bracket.
    address.next();

    let literal = address.next().unwrap().into_inner().next().unwrap();
    let register = address.next().unwrap();

    let base = parse_literal_raw(literal);
    let offset = parse_register_raw(register);

    let arg = InstructionArgument::AddressOffset {
        address: base,
        register: offset,
    };
    Value::Static(arg)
}

fn parse_special_register(pair: Pair<Rule>) -> Value {
    let special = match pair.as_str() {
        "SP" => SpecialRegister::StackPointer,
        "PC" => SpecialRegister::ProgramCounter,
        "O" => SpecialRegister::Overflow,
        _ => {
            println!("{:?}", pair);
            unreachable!()
        }
    };
    Value::Static(InstructionArgument::SpecialRegister(special))
}

fn parse_stack_op(pair: Pair<Rule>) -> Value {
    let op = match pair.as_str() {
        "POP" => StackOperation::Pop,
        "PEEK" => StackOperation::Peek,
        "PUSH" => StackOperation::Push,
        _ => {
            println!("{:?}", pair);
            unreachable!()
        }
    };
    Value::Static(InstructionArgument::StackOperation(op))
}

fn parse_label_ref(pair: Pair<Rule>) -> Value {
    Value::LabelReference(String::from(pair.as_str()))
}

#[derive(Debug)]
enum MaterializedInstruction {
    /// An instruction that is fixed in size.
    Static {
        instruction: Instruction,
        instruction_word: Word,
        arg1: Option<Word>,
        arg2: Option<Word>,
    },
    /// An instruction that is flexible in size, e.g. because
    /// it corresponds to a jump label that could be represented as
    /// an inline literal.
    Flexible {
        instruction: Instruction,
        instruction_word: Word,
        arg1: Option<Word>,
        arg2: Option<Word>,
    },
}

impl MaterializedInstruction {
    fn len(&self) -> usize {
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
    fn materialize(&self, label_map: &HashMap<String, Word>) -> MaterializedInstruction {
        match self {
            Instruction::NonBasicInstruction(nbi, a) => {
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
            Instruction::BasicInstruction(bi, a, b) => {
                // Both arguments must be fixed-sized for this to be static.
                if let Value::Static(arg1) = a {
                    match b {
                        Value::Static(arg2) => {
                            let (opcode, arg1, arg2) = bi.bake(*arg1, *arg2);
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
                            let (opcode, arg1, arg2) = bi.bake(*arg1, arg2);

                            MaterializedInstruction::Flexible {
                                instruction: self.clone(),
                                instruction_word: opcode,
                                arg1,
                                arg2,
                            }
                        }
                    }
                } else {
                    panic!("LHS must not be a label reference");
                }
            }
        }
    }
}
