//! Rendering of instructions and operands as assembly mnemonics and human-readable text.

use crate::Register;
use crate::instruction::{InstructionWithOperands, ResolvedValue};
use crate::instruction_argument::{InstructionArgument, InstructionArgumentDefinition};
use crate::instruction_word::{InstructionWord, NonBasicInstruction};

/// Renders a value as DCPU-16 assembly.
pub trait Disassemble {
    /// Gets the mnemonic for the given instruction.
    fn disassemble(&self) -> String;

    /// Gets a human-readable string for the given instruction.
    fn disassemble_human(&self) -> String {
        self.disassemble()
    }
}

impl Disassemble for Register {
    fn disassemble(&self) -> String {
        match self {
            Register::A => String::from("A"),
            Register::B => String::from("B"),
            Register::C => String::from("C"),
            Register::X => String::from("X"),
            Register::Y => String::from("Y"),
            Register::Z => String::from("Z"),
            Register::I => String::from("I"),
            Register::J => String::from("J"),
        }
    }
}

impl Disassemble for ResolvedValue {
    fn disassemble(&self) -> String {
        match self.argument_definition {
            InstructionArgumentDefinition::Register { register } => register.disassemble(),
            InstructionArgumentDefinition::Literal { value } => {
                format!("0x{:02X}", value)
            }
            InstructionArgumentDefinition::NextWordLiteral => {
                format!("0x{:02X}", self.resolved_value)
            }
            InstructionArgumentDefinition::AtAddressFromNextWord => {
                format!("[0x{:02X}]", self.argument.get_literal().unwrap())
            }
            InstructionArgumentDefinition::OfOverflow => String::from("O"),
            InstructionArgumentDefinition::OfProgramCounter => String::from("PC"),
            InstructionArgumentDefinition::OfStackPointer => String::from("SP"),
            InstructionArgumentDefinition::AtAddressFromNextWordPlusRegister { .. } => {
                match self.argument {
                    InstructionArgument::AddressOffset { address, register } => {
                        format!("[0x{:02X}+{}]", address, register.disassemble())
                    }
                    _ => unreachable!(),
                }
            }
            InstructionArgumentDefinition::Pop => String::from("POP"),
            InstructionArgumentDefinition::Peek => String::from("PEEK"),
            InstructionArgumentDefinition::Push => String::from("PUSH"),
            InstructionArgumentDefinition::AtAddressFromRegister { register } => {
                format!("[{}]", register.disassemble())
            }
        }
    }

    fn disassemble_human(&self) -> String {
        match self.argument_definition {
            InstructionArgumentDefinition::AtAddressFromNextWord => {
                format!("RAM[0x{:02X}]", self.argument.get_literal().unwrap())
            }
            // Value::OfOverflow => String::from("O"),
            // Value::OfProgramCounter => String::from("PC"),
            // Value::OfStackPointer => String::from("SP"),
            InstructionArgumentDefinition::AtAddressFromNextWordPlusRegister { .. } => {
                match self.argument {
                    InstructionArgument::AddressOffset { address, register } => {
                        format!("RAM[0x{:02X} + {}]", address, register.disassemble_human())
                    }
                    _ => unreachable!(),
                }
            }
            InstructionArgumentDefinition::AtAddressFromRegister { register } => {
                format!("RAM[{}]", register.disassemble_human())
            }
            InstructionArgumentDefinition::Pop => String::from("pop value from stack"),
            InstructionArgumentDefinition::Peek => String::from("current stack value"),
            InstructionArgumentDefinition::Push => String::from("push value to stack"),
            _ => self.disassemble(),
        }
    }
}

impl Disassemble for InstructionWithOperands {
    fn disassemble(&self) -> String {
        match self.instruction {
            InstructionWord::Set { .. } => format!(
                "SET {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Add { .. } => format!(
                "ADD {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Sub { .. } => format!(
                "SUB {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Mul { .. } => format!(
                "MUL {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Div { .. } => format!(
                "DIV {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Mod { .. } => format!(
                "MOD {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Shl { .. } => format!(
                "SHL {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Shr { .. } => format!(
                "SHR {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::And { .. } => format!(
                "AND {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Bor { .. } => format!(
                "BOR {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Xor { .. } => format!(
                "XOR {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Ife { .. } => format!(
                "IFE {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Ifn { .. } => format!(
                "IFN {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Ifg { .. } => format!(
                "IFG {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::Ifb { .. } => format!(
                "IFB {}, {}",
                self.a.disassemble(),
                self.b.expect("require second argument").disassemble()
            ),
            InstructionWord::NonBasic(nbi) => match nbi {
                NonBasicInstruction::Reserved => panic!(),
                NonBasicInstruction::Jsr { .. } => {
                    format!("JSR {}", self.a.disassemble())
                }
            },
        }
    }

    fn disassemble_human(&self) -> String {
        match self.instruction {
            InstructionWord::Set { .. } => format!(
                "{0} <- {1}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Add { .. } => format!(
                "{0} <- {0} + {1}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Sub { .. } => format!(
                "{0} <- {0} - {1}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Mul { .. } => format!(
                "{0} <- {0} * {1}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Div { .. } => format!(
                "{0} <- {0} / {1}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Mod { .. } => format!(
                "{0} <- {0} % {1}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Shl { .. } => format!(
                "{0} <- {0} << {1}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Shr { .. } => format!(
                "{0} <- {0} >> {1}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::And { .. } => format!(
                "{0} <- {0} & {1}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Bor { .. } => format!(
                "{0} <- {0} | {1}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Xor { .. } => format!(
                "{0} <- {0} ^ {1}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Ife { .. } => format!(
                "execute next instruction if {} == {}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Ifn { .. } => format!(
                "execute next instruction if {} != {}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Ifg { .. } => format!(
                "execute next instruction if {} > {}",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::Ifb { .. } => format!(
                "execute next instruction if ({} & {}) != 0",
                self.a.disassemble_human(),
                self.b.expect("require second argument").disassemble_human()
            ),
            InstructionWord::NonBasic(nbi) => match nbi {
                NonBasicInstruction::Reserved => panic!(),
                NonBasicInstruction::Jsr { .. } => {
                    format!("jump to subroutine at {}", self.a.disassemble())
                }
            },
        }
    }
}

impl ResolvedValue {
    /// Names the operand for [`InstructionWithOperands::describe`]: memory reads
    /// as `RAM[...]`, including the stack operations.
    fn describe(&self) -> String {
        match self.argument_definition {
            InstructionArgumentDefinition::AtAddressFromNextWord => {
                format!("RAM[0x{:02X}]", self.argument.get_literal().unwrap())
            }
            InstructionArgumentDefinition::AtAddressFromRegister { register } => {
                format!("RAM[{}]", register.disassemble())
            }
            InstructionArgumentDefinition::AtAddressFromNextWordPlusRegister { .. } => {
                match self.argument {
                    InstructionArgument::AddressOffset { address, register } => {
                        format!("RAM[0x{:02X}+{}]", address, register.disassemble())
                    }
                    _ => unreachable!(),
                }
            }
            InstructionArgumentDefinition::Push => String::from("RAM[--SP]"),
            InstructionArgumentDefinition::Pop => String::from("RAM[SP++]"),
            InstructionArgumentDefinition::Peek => String::from("RAM[SP]"),
            _ => self.disassemble(),
        }
    }
}

impl InstructionWithOperands {
    /// Describes what the instruction does in plain English, e.g. `store 0x20 in RAM[X]`.
    pub fn describe(&self) -> String {
        let a = self.a.describe();
        let b = || self.b.expect("require second argument");
        match self.instruction {
            InstructionWord::Set { .. } => {
                let b = b();
                match (self.a.argument_definition, b.argument_definition) {
                    (
                        InstructionArgumentDefinition::OfProgramCounter,
                        InstructionArgumentDefinition::Pop,
                    ) => String::from("return from subroutine"),
                    (InstructionArgumentDefinition::OfProgramCounter, _) => {
                        format!("jump to {}", b.describe())
                    }
                    (InstructionArgumentDefinition::Push, _) => format!("push {}", b.describe()),
                    (_, InstructionArgumentDefinition::Pop) => format!("pop into {a}"),
                    _ => format!("store {} in {a}", b.describe()),
                }
            }
            InstructionWord::Add { .. } => format!("add {} to {a}", b().describe()),
            InstructionWord::Sub { .. } => format!("subtract {} from {a}", b().describe()),
            InstructionWord::Mul { .. } => format!("multiply {a} by {}", b().describe()),
            InstructionWord::Div { .. } => format!("divide {a} by {}", b().describe()),
            InstructionWord::Mod { .. } => format!("set {a} to {a} modulo {}", b().describe()),
            InstructionWord::Shl { .. } => format!("shift {a} left by {}", b().describe()),
            InstructionWord::Shr { .. } => format!("shift {a} right by {}", b().describe()),
            InstructionWord::And { .. } => {
                format!("keep the bits of {a} set in {}", b().describe())
            }
            InstructionWord::Bor { .. } => format!("set the bits of {} in {a}", b().describe()),
            InstructionWord::Xor { .. } => format!("flip the bits of {} in {a}", b().describe()),
            InstructionWord::Ife { .. } => format!("run next if {a} == {}", b().describe()),
            InstructionWord::Ifn { .. } => format!("run next if {a} != {}", b().describe()),
            InstructionWord::Ifg { .. } => format!("run next if {a} > {}", b().describe()),
            InstructionWord::Ifb { .. } => format!("run next if {a} & {} != 0", b().describe()),
            InstructionWord::NonBasic(NonBasicInstruction::Jsr { .. }) => {
                format!("call subroutine at {a}")
            }
            InstructionWord::NonBasic(NonBasicInstruction::Reserved) => {
                String::from("reserved opcode")
            }
        }
    }
}
