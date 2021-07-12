use crate::instruction_argument::{InstructionArgument, SpecialRegister, StackOperation};
use crate::{Register, Word};
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;

type Opcode = u8;

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
            if label_map.insert(label.clone(), 0xFFFFu16).is_some() {
                panic!("Label '{}' defined multiple times", label);
            }
        }
    }

    println!("{:?}", tokens);

    let mut instructions = Vec::new();
    let mut current_position: Word = 0x0000;

    for token in tokens {
        match token {
            MetaInstruction::Instruction(instruction) => {
                let materialized = instruction.materialize();
                if let Some(len) = materialized.len() {
                    // We update the actual length.
                    current_position += len as Word;
                } else {
                    // We assume the best-case situation here.
                    // This is helpful because small values can be inlined
                    // into the instruction.
                    current_position += 1;
                }
                instructions.push(materialized);
            }
            MetaInstruction::Label(label) => {
                label_map.insert(label.clone(), current_position);
            }
        }
    }

    println!("{:?}", instructions);

    Vec::default()
}

fn get_meta_instructions<T>(source: T) -> Vec<MetaInstruction>
where
    T: AsRef<str>,
{
    let mut program =
        AssembleParser::parse(Rule::program, source.as_ref()).expect("unsuccessful parse");

    // Get the top-level program rule.
    let program = program.next().unwrap();

    let mut tokens = Vec::new();

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

        tokens.push(token);
    }
    tokens
}

#[derive(Debug, Clone)]
enum MetaInstruction {
    /// An instruction.
    Instruction(Instruction),
    /// A label.
    Label(String),
}

#[derive(Debug, Clone)]
enum Instruction {
    /// A basic (two-operand) instruction.
    BasicInstruction(BasicOperationName, Value, Value),
    /// A non-basic (one-operand) instruction.
    NonBasicInstruction(NonBasicOperationName, Value),
}

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
    ) -> (Opcode, Option<Word>, Option<Word>) {
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
            | ((b_baked.inline as u32 & 0b111_111) << 10)) as Opcode;
        if a_baked.literal.is_some() {
            (instruction, a_baked.literal, b_baked.literal)
        } else {
            (instruction, b_baked.literal, None)
        }
    }
}

impl NonBasicOperationName {
    fn bake(&self, a: InstructionArgument) -> (Opcode, Option<Word>) {
        let opcode = match self {
            Self::JSR => 0x1,
        };

        let a_baked = a.bake();

        let instruction = (((opcode as u32 & 0b111_111) << 4)
            | ((a_baked.inline as u32 & 0b111_111) << 10)) as Opcode;
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
    let register = match pair.as_str() {
        "A" => Register::A,
        "B" => Register::B,
        "C" => Register::C,
        "X" => Register::X,
        "Y" => Register::Y,
        "Z" => Register::Z,
        "I" => Register::I,
        "J" => Register::J,
        _ => unreachable!(),
    };

    Value::Static(InstructionArgument::Register(register))
}

fn parse_literal(pair: Pair<Rule>) -> Value {
    let item = pair.into_inner().next().unwrap();
    let word = match item.as_rule() {
        Rule::value_dec => {
            u16::from_str_radix(item.as_str(), 10).expect("invalid format for decimal literal")
        }
        Rule::value_hex => u16::from_str_radix(item.as_str().trim_start_matches("0x"), 16)
            .expect("invalid format for hex literal"),
        _ => unreachable!(),
    };
    Value::Static(InstructionArgument::Literal(word))
}

fn parse_address(pair: Pair<Rule>) -> Value {
    Value::Static(InstructionArgument::Address(0xdead))
}

fn parse_address_with_offset(pair: Pair<Rule>) -> Value {
    let arg = InstructionArgument::AddressOffset {
        address: 0x1337,
        register: Register::A,
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
        opcode: Opcode,
        arg1: Option<Word>,
        arg2: Option<Word>,
    },
    /// An instruction that is flexible in size, e.g. because
    /// it corresponds to a jump label that could be represented as
    /// an inline literal.
    Flexible { instruction: Instruction },
}

impl MaterializedInstruction {
    fn len(&self) -> Option<usize> {
        match self {
            Self::Static {
                opcode: _,
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
                Some(size)
            }
            _ => None,
        }
    }
}

impl Instruction {
    fn materialize(&self) -> MaterializedInstruction {
        match self {
            Instruction::NonBasicInstruction(nbi, a) => {
                if let Value::Static(arg) = a {
                    let (opcode, arg1) = nbi.bake(*arg);
                    MaterializedInstruction::Static {
                        opcode,
                        arg1,
                        arg2: None,
                    }
                } else {
                    MaterializedInstruction::Flexible {
                        instruction: self.clone(),
                    }
                }
            }
            Instruction::BasicInstruction(bi, a, b) => {
                // Both arguments must be fixed-sized for this to be static.
                if let Value::Static(arg1) = a {
                    if let Value::Static(arg2) = b {
                        let (opcode, arg1, arg2) = bi.bake(*arg1, *arg2);
                        MaterializedInstruction::Static { opcode, arg1, arg2 }
                    } else {
                        // b is a label reference, skip.
                        MaterializedInstruction::Flexible {
                            instruction: self.clone(),
                        }
                    }
                } else {
                    panic!("LHS must not be a label reference");
                }
            }
        }
    }
}
