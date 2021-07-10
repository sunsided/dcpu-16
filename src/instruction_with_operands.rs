use crate::address::Address;
use crate::instruction::Instruction;
use crate::value::Value;
use crate::{Word, DCPU16};
use std::fmt::{Debug, Formatter};

pub struct ResolvedValue {
    value: Value,
    pub address: Address,
    pub value_at_address: Word,
}

impl ResolvedValue {
    pub fn unpack(&self) -> (Address, Word) {
        (self.address, self.value_at_address)
    }
}

pub struct InstructionWithOperands {
    word: Word,
    pub instruction: Instruction,
    pub a: ResolvedValue,
    pub b: Option<ResolvedValue>,
}

impl InstructionWithOperands {
    /// Uses the CPU's resolve method (which may advance the PC)
    /// to look up an entire instruction that takes one additional operands.
    pub fn resolve_1op(cpu: &mut DCPU16, word: Word, instruction: Instruction, a: Value) -> Self {
        let (lhs_addr, lhs) = cpu.resolve_address(a);
        InstructionWithOperands::new_1op(word, instruction, a, lhs_addr, lhs)
    }

    /// Uses the CPU's resolve method (which may advance the PC)
    /// to look up an entire instruction that takes two additional operands.
    pub fn resolve_2op(
        cpu: &mut DCPU16,
        word: Word,
        instruction: Instruction,
        a: Value,
        b: Value,
    ) -> Self {
        let (lhs_addr, lhs) = cpu.resolve_address(a);
        let (rhs_addr, rhs) = cpu.resolve_address(b);
        InstructionWithOperands::new_2op(word, instruction, a, lhs_addr, lhs, b, rhs_addr, rhs)
    }

    /// Constructs a one-operand instruction.
    fn new_1op(
        word: Word,
        instruction: Instruction,
        lhs_value: Value,
        lhs_addr: Address,
        lhs: Word,
    ) -> Self {
        Self {
            word,
            instruction,
            a: ResolvedValue {
                value: lhs_value,
                address: lhs_addr,
                value_at_address: lhs,
            },
            b: None,
        }
    }

    /// Constructs a two-operand instruction.
    fn new_2op(
        word: Word,
        instruction: Instruction,
        lhs_value: Value,
        lhs_addr: Address,
        lhs: Word,
        rhs_value: Value,
        rhs_addr: Address,
        rhs: Word,
    ) -> Self {
        Self {
            word,
            instruction,
            a: ResolvedValue {
                value: lhs_value,
                address: lhs_addr,
                value_at_address: lhs,
            },
            b: Some(ResolvedValue {
                value: rhs_value,
                address: rhs_addr,
                value_at_address: rhs,
            }),
        }
    }

    /// Gets the length of the instruction including all operands.
    fn len(&self) -> usize {
        self.instruction.len()
    }
}

impl Debug for InstructionWithOperands {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        assert!(self.len() >= 1 && self.len() <= 3);

        if self.len() == 1 {
            write!(f, "{:04x?} => {:?}", self.word, self.instruction)
        } else if self.len() == 2 {
            let second_word = if self.a.value.len() == 1 {
                self.a.address.get_literal().unwrap()
            } else {
                self.b.as_ref().unwrap().address.get_literal().unwrap()
            };
            write!(
                f,
                "{:04x?} {:04x?} => {:?}",
                self.word, second_word, self.instruction
            )
        } else {
            assert_eq!(self.a.value.len(), 1);
            assert_eq!(self.b.as_ref().unwrap().value.len(), 1);
            write!(
                f,
                "{:04x?} {:04x?} {:04x?} => {:?}",
                self.word,
                self.a.address.get_literal().unwrap(),
                self.b.as_ref().unwrap().address.get_literal().unwrap(),
                self.instruction
            )
        }
    }
}
