use crate::value::Value;
use crate::DurationCycles;
use std::fmt::{Debug, Formatter};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Instruction {
    /// Non-basic instruction.
    NonBasic(NonBasicInstruction),
    /// Sets `a` to `b`.
    ///
    /// Takes 1 cycle, plus the cost of `a` and `b`.
    Set { a: Value, b: Value },
    /// Sets `a` to `a+b`, sets `O` to `0x0001` if there's an overflow, `0x0` otherwise.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`.
    Add { a: Value, b: Value },
    /// Sets `a` to `a-b`, sets `O` to `0xffff` if there's an underflow, `0x0` otherwise.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`.
    Sub { a: Value, b: Value },
    /// Sets `a` to `a*b`, sets `O` to `((a*b)>>16)&0xffff`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`.
    Mul { a: Value, b: Value },
    /// Sets `a` to `a/b`, sets `O` to `((a<<16)/b)&0xffff`. if `b==0`, sets `a` and `O` to `0` instead.
    ///
    /// Takes 3 cycles, plus the cost of `a` and `b`.
    Div { a: Value, b: Value },
    /// Sets `a` to `a%b`. if `b==0`, sets `a` to `0` instead.
    ///
    /// Takes 3 cycles, plus the cost of `a` and `b`.
    Mod { a: Value, b: Value },
    /// Sets `a` to `a<<b`, sets `O` to `((a<<b)>>16)&0xffff`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`.
    Shl { a: Value, b: Value },
    /// Sets `a` to `a>>b`, sets `O` to `((a<<16)>>b)&0xffff`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`.
    Shr { a: Value, b: Value },
    /// Sets `a` to `a&b`.
    ///
    /// Takes 1 cycle, plus the cost of `a` and `b`.
    And { a: Value, b: Value },
    /// Sets `a` to `a|b`.
    ///
    /// Takes 1 cycle, plus the cost of `a` and `b`.
    Bor { a: Value, b: Value },
    /// Sets `a` to `a^b`.
    ///
    /// Takes 1 cycle, plus the cost of `a` and `b`.
    Xor { a: Value, b: Value },
    /// Performs next instruction only if `a==b`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`, plus 1 if the test fails.
    Ife { a: Value, b: Value },
    /// Performs next instruction only if `a!=b`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`, plus 1 if the test fails.
    Ifn { a: Value, b: Value },
    /// Performs next instruction only if `a>b`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`, plus 1 if the test fails.
    Ifg { a: Value, b: Value },
    /// Performs next instruction only if `(a&b)!=0`.
    ///
    /// Takes 2 cycles, plus the cost of `a` and `b`, plus 1 if the test fails.
    Ifb { a: Value, b: Value },
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
    Jsr { a: Value },
}

impl From<u16> for Instruction {
    fn from(value: u16) -> Self {
        let opcode = value & 0b1111;
        let a = Value::from((value >> 4) & 0b111_111);
        let b = Value::from((value >> 10) & 0b111_111);

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
    fn from(value: u16) -> NonBasicInstruction {
        assert_eq!(value & 0b1111, 0x0);
        let opcode = (value >> 4) & 0b111_111;
        let a = Value::from((value >> 10) & 0b111_111);
        match opcode {
            0x00 => NonBasicInstruction::Reserved,
            0x01 => NonBasicInstruction::Jsr { a },
            0x02..=0x3f => NonBasicInstruction::Reserved,
            _ => panic!(),
        }
    }
}

impl DurationCycles for Instruction {
    fn base_cycle_count(&self) -> usize {
        match self {
            Self::NonBasic(op) => op.base_cycle_count(),
            // SET, AND, BOR and XOR take 1 cycle, plus the cost of a and b.
            Self::Set { a, b } => 1 + a.base_cycle_count() + b.base_cycle_count(),
            Self::And { a, b } => 1 + a.base_cycle_count() + b.base_cycle_count(),
            Self::Bor { a, b } => 1 + a.base_cycle_count() + b.base_cycle_count(),
            Self::Xor { a, b } => 1 + a.base_cycle_count() + b.base_cycle_count(),
            // ADD, SUB, MUL, SHR, and SHL take 2 cycles, plus the cost of a and b
            Self::Add { a, b } => 2 + a.base_cycle_count() + b.base_cycle_count(),
            Self::Sub { a, b } => 2 + a.base_cycle_count() + b.base_cycle_count(),
            Self::Mul { a, b } => 2 + a.base_cycle_count() + b.base_cycle_count(),
            Self::Shr { a, b } => 2 + a.base_cycle_count() + b.base_cycle_count(),
            Self::Shl { a, b } => 2 + a.base_cycle_count() + b.base_cycle_count(),
            // DIV and MOD take 3 cycles, plus the cost of a and b
            Self::Div { a, b } => 3 + a.base_cycle_count() + b.base_cycle_count(),
            Self::Mod { a, b } => 3 + a.base_cycle_count() + b.base_cycle_count(),
            // IFE, IFN, IFG, IFB take 2 cycles, plus the cost of a and b,
            // plus 1 if the test fails
            Self::Ife { a, b } => 2 + a.base_cycle_count() + b.base_cycle_count(),
            Self::Ifn { a, b } => 2 + a.base_cycle_count() + b.base_cycle_count(),
            Self::Ifg { a, b } => 2 + a.base_cycle_count() + b.base_cycle_count(),
            Self::Ifb { a, b } => 2 + a.base_cycle_count() + b.base_cycle_count(),
        }
    }
}

impl DurationCycles for NonBasicInstruction {
    fn base_cycle_count(&self) -> usize {
        match self {
            Self::Reserved => 0,
            Self::Jsr { a } => 2 + a.base_cycle_count(),
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
            Instruction::from(0b000000_000000_0000),
            Instruction::NonBasic(NonBasicInstruction::Reserved)
        );

        assert_eq!(
            Instruction::from(0b000000_000010_0000),
            Instruction::NonBasic(NonBasicInstruction::Reserved)
        );

        assert_eq!(
            Instruction::from(0b000000_111111_0000),
            Instruction::NonBasic(NonBasicInstruction::Reserved)
        );
    }

    #[test]
    fn non_basic_instruction_jsr_works() {
        assert_eq!(
            Instruction::from(0b010001_000001_0000),
            Instruction::NonBasic(NonBasicInstruction::Jsr {
                a: Value::AtAddressFromNextWordPlusRegister {
                    register: Register::B
                }
            })
        );
    }

    #[test]
    fn set_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_0001),
            Instruction::Set {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn add_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_0010),
            Instruction::Add {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn sub_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_0011),
            Instruction::Sub {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn mul_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_0100),
            Instruction::Mul {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn div_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_0101),
            Instruction::Div {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn mod_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_0110),
            Instruction::Mod {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn shl_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_0111),
            Instruction::Shl {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn shr_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_1000),
            Instruction::Shr {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn and_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_1001),
            Instruction::And {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn bor_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_1010),
            Instruction::Bor {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn xor_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_1011),
            Instruction::Xor {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn ife_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_1100),
            Instruction::Ife {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn ifn_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_1101),
            Instruction::Ifn {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn ifg_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_1110),
            Instruction::Ifg {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }

    #[test]
    fn ifb_works() {
        assert_eq!(
            Instruction::from(0b000011_000000_1111),
            Instruction::Ifb {
                a: Value::Register {
                    register: Register::A
                },
                b: Value::Register {
                    register: Register::X
                }
            }
        );
    }
}
