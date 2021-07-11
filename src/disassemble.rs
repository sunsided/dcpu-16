use crate::instruction::{InstructionWord, NonBasicInstruction};
use crate::instruction_with_operands::{InstructionWithOperands, ResolvedValue};
use crate::instruction_argument::{InstructionArgument, InstructionArgumentDefinition};
use crate::Register;

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
            InstructionArgumentDefinition::Literal { value } => String::from(format!("0x{:02X}", value)),
            InstructionArgumentDefinition::NextWordLiteral => String::from(format!("0x{:02X}", self.resolved_value)),
            InstructionArgumentDefinition::AtAddressFromNextWord => String::from(format!(
                "[0x{:02X}]",
                self.argument.get_literal().unwrap()
            )),
            InstructionArgumentDefinition::OfOverflow => String::from("O"),
            InstructionArgumentDefinition::OfProgramCounter => String::from("PC"),
            InstructionArgumentDefinition::OfStackPointer => String::from("SP"),
            InstructionArgumentDefinition::AtAddressFromNextWordPlusRegister { .. } => match self.argument {
                InstructionArgument::AddressOffset { address, register } => {
                    String::from(format!("[0x{:02X}+{}]", address, register.disassemble()))
                }
                _ => panic!(),
            },
            InstructionArgumentDefinition::Pop => String::from("POP"),
            InstructionArgumentDefinition::Peek => String::from("PEEK"),
            InstructionArgumentDefinition::Push => String::from("PUSH"),
            InstructionArgumentDefinition::AtAddressFromRegister { register } => {
                String::from(format!("[{}]", register.disassemble()))
            }
        }
    }

    fn disassemble_human(&self) -> String {
        match self.argument_definition {
            InstructionArgumentDefinition::AtAddressFromNextWord => String::from(format!(
                "RAM[0x{:02X}]",
                self.argument.get_literal().unwrap()
            )),
            // Value::OfOverflow => String::from("O"),
            // Value::OfProgramCounter => String::from("PC"),
            // Value::OfStackPointer => String::from("SP"),
            InstructionArgumentDefinition::AtAddressFromNextWordPlusRegister { .. } => match self.argument {
                InstructionArgument::AddressOffset { address, register } => String::from(format!(
                    "RAM[0x{:02X} + {}]",
                    address,
                    register.disassemble_human()
                )),
                _ => panic!(),
            },
            InstructionArgumentDefinition::AtAddressFromRegister { register } => {
                String::from(format!("RAM[{}]", register.disassemble_human()))
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
            InstructionWord::Set { .. } => String::from(format!(
                "SET {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Add { .. } => String::from(format!(
                "ADD {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Sub { .. } => String::from(format!(
                "SUB {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Mul { .. } => String::from(format!(
                "MUL {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Div { .. } => String::from(format!(
                "DIV {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Mod { .. } => String::from(format!(
                "MOD {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Shl { .. } => String::from(format!(
                "SHL {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Shr { .. } => String::from(format!(
                "SHR {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::And { .. } => String::from(format!(
                "AND {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Bor { .. } => String::from(format!(
                "BOR {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Xor { .. } => String::from(format!(
                "XOR {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Ife { .. } => String::from(format!(
                "IFE {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Ifn { .. } => String::from(format!(
                "IFN {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Ifg { .. } => String::from(format!(
                "IFG {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::Ifb { .. } => String::from(format!(
                "IFB {}, {}",
                self.a.expect("require first operand").disassemble(),
                self.b.expect("require second operand").disassemble()
            )),
            InstructionWord::NonBasic(nbi) => match nbi {
                NonBasicInstruction::Reserved => panic!(),
                NonBasicInstruction::Jsr { .. } => {
                    String::from(format!("JSR {}", self.a.expect("require first operand").disassemble()))
                }
            },
        }
    }

    fn disassemble_human(&self) -> String {
        match self.instruction {
            InstructionWord::Set { .. } => String::from(format!(
                "{0} <- {1}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Add { .. } => String::from(format!(
                "{0} <- {0} + {1}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Sub { .. } => String::from(format!(
                "{0} <- {0} - {1}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Mul { .. } => String::from(format!(
                "{0} <- {0} * {1}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Div { .. } => String::from(format!(
                "{0} <- {0} / {1}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Mod { .. } => String::from(format!(
                "{0} <- {0} % {1}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Shl { .. } => String::from(format!(
                "{0} <- {0} << {1}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Shr { .. } => String::from(format!(
                "{0} <- {0} >> {1}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::And { .. } => String::from(format!(
                "{0} <- {0} & {1}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Bor { .. } => String::from(format!(
                "{0} <- {0} | {1}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Xor { .. } => String::from(format!(
                "{0} <- {0} ^ {1}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Ife { .. } => String::from(format!(
                "execute next instruction if {} == {}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Ifn { .. } => String::from(format!(
                "execute next instruction if {} != {}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Ifg { .. } => String::from(format!(
                "execute next instruction if {} > {}",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::Ifb { .. } => String::from(format!(
                "execute next instruction if ({} & {}) != 0",
                self.a.expect("require first operand").disassemble_human(),
                self.b.expect("require second operand").disassemble_human()
            )),
            InstructionWord::NonBasic(nbi) => match nbi {
                NonBasicInstruction::Reserved => panic!(),
                NonBasicInstruction::Jsr { .. } => {
                    String::from(format!("jump to subroutine at {}", self.a.expect("require first operand").disassemble()))
                }
            },
        }
    }
}
