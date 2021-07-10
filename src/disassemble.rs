use crate::address::Address;
use crate::instruction::{Instruction, NonBasicInstruction};
use crate::instruction_with_operands::{InstructionWithOperands, ResolvedValue};
use crate::value::Value;
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
        match self.value_type {
            Value::Register { register } => register.disassemble(),
            Value::Literal { value } => String::from(format!("0x{:02X}", value)),
            Value::NextWordLiteral => String::from(format!("0x{:02X}", self.resolved_value)),
            Value::AtAddressFromNextWord => String::from(format!(
                "[0x{:02X}]",
                self.value_address.get_literal().unwrap()
            )),
            Value::OfOverflow => String::from("O"),
            Value::OfProgramCounter => String::from("PC"),
            Value::OfStackPointer => String::from("SP"),
            Value::AtAddressFromNextWordPlusRegister { .. } => match self.value_address {
                Address::AddressOffset { address, register } => {
                    String::from(format!("[0x{:02X}+{}]", address, register.disassemble()))
                }
                _ => panic!(),
            },
            Value::Pop => String::from("POP"),
            Value::Peek => String::from("PEEK"),
            Value::Push => String::from("PUSH"),
            Value::AtAddressFromRegister { register } => {
                String::from(format!("[{}]", register.disassemble()))
            }
        }
    }

    fn disassemble_human(&self) -> String {
        match self.value_type {
            Value::AtAddressFromNextWord => String::from(format!(
                "RAM[0x{:02X}]",
                self.value_address.get_literal().unwrap()
            )),
            // Value::OfOverflow => String::from("O"),
            // Value::OfProgramCounter => String::from("PC"),
            // Value::OfStackPointer => String::from("SP"),
            Value::AtAddressFromNextWordPlusRegister { .. } => match self.value_address {
                Address::AddressOffset { address, register } => String::from(format!(
                    "RAM[0x{:02X} + {}]",
                    address,
                    register.disassemble_human()
                )),
                _ => panic!(),
            },
            Value::AtAddressFromRegister { register } => {
                String::from(format!("RAM[{}]", register.disassemble_human()))
            }
            Value::Pop => String::from("pop value from stack"),
            Value::Peek => String::from("current stack value"),
            Value::Push => String::from("push value to stack"),
            _ => self.disassemble(),
        }
    }
}

impl Disassemble for InstructionWithOperands {
    fn disassemble(&self) -> String {
        match self.instruction {
            Instruction::Set { .. } => String::from(format!(
                "SET {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Add { .. } => String::from(format!(
                "ADD {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Sub { .. } => String::from(format!(
                "SUB {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Mul { .. } => String::from(format!(
                "MUL {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Div { .. } => String::from(format!(
                "DIV {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Mod { .. } => String::from(format!(
                "MOD {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Shl { .. } => String::from(format!(
                "SHL {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Shr { .. } => String::from(format!(
                "SHR {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::And { .. } => String::from(format!(
                "AND {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Bor { .. } => String::from(format!(
                "BOR {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Xor { .. } => String::from(format!(
                "XOR {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Ife { .. } => String::from(format!(
                "IFE {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Ifn { .. } => String::from(format!(
                "IFN {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Ifg { .. } => String::from(format!(
                "IFG {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::Ifb { .. } => String::from(format!(
                "IFB {}, {}",
                self.a.disassemble(),
                self.b.as_ref().unwrap().disassemble()
            )),
            Instruction::NonBasic(nbi) => match nbi {
                NonBasicInstruction::Reserved => panic!(),
                NonBasicInstruction::Jsr { .. } => {
                    String::from(format!("JSR {}", self.a.disassemble()))
                }
            },
        }
    }

    fn disassemble_human(&self) -> String {
        match self.instruction {
            Instruction::Set { .. } => String::from(format!(
                "{0} <- {1}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Add { .. } => String::from(format!(
                "{0} <- {0} + {1}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Sub { .. } => String::from(format!(
                "{0} <- {0} - {1}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Mul { .. } => String::from(format!(
                "{0} <- {0} * {1}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Div { .. } => String::from(format!(
                "{0} <- {0} / {1}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Mod { .. } => String::from(format!(
                "{0} <- {0} % {1}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Shl { .. } => String::from(format!(
                "{0} <- {0} << {1}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Shr { .. } => String::from(format!(
                "{0} <- {0} >> {1}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::And { .. } => String::from(format!(
                "{0} <- {0} & {1}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Bor { .. } => String::from(format!(
                "{0} <- {0} | {1}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Xor { .. } => String::from(format!(
                "{0} <- {0} ^ {1}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Ife { .. } => String::from(format!(
                "execute next instruction if {} == {}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Ifn { .. } => String::from(format!(
                "execute next instruction if {} != {}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Ifg { .. } => String::from(format!(
                "execute next instruction if {} > {}",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::Ifb { .. } => String::from(format!(
                "execute next instruction if ({} & {}) != 0",
                self.a.disassemble_human(),
                self.b.as_ref().unwrap().disassemble_human()
            )),
            Instruction::NonBasic(nbi) => match nbi {
                NonBasicInstruction::Reserved => panic!(),
                NonBasicInstruction::Jsr { .. } => {
                    String::from(format!("jump to subroutine at {}", self.a.disassemble()))
                }
            },
        }
    }
}
