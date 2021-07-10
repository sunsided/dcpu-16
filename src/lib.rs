mod instruction;
mod register;
mod value;

use crate::instruction::Instruction;
use crate::register::Register;
use crate::value::Value;
use crate::Address::Literal;
use std::fmt::{Debug, Formatter};
use std::ops::{Add, BitAnd, BitOr, BitXor};

type Word = u16;

const NUM_REGISTERS: usize = 8;
const NUM_RAM_WORDS: usize = 0x10000;

// Stack pointer is initialized to 0xffff (for 0x10000 words of memory).
const STACK_POINTER_INIT: usize = NUM_RAM_WORDS - 1;

trait DurationCycles {
    fn base_cycle_count(&self) -> usize;
}

/// A DCPU-16 emulator.
pub struct DCPU16<'p> {
    /// RAM.
    ram: Box<[Word; NUM_RAM_WORDS]>,
    /// Registers.
    registers: [Word; NUM_REGISTERS],
    /// Program counter.
    pc: Word,
    /// Stack pointer.
    sp: Word,
    /// Overflow.
    overflow: Word,
    /// The program
    program: &'p [u16],
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Address {
    Register(Register),
    Literal(Word),
    Address(Word),
}

impl Address {
    fn get_literal(&self) -> Option<Word> {
        match self {
            Self::Register(..) => None,
            Self::Literal(value) => Some(*value),
            Self::Address(value) => Some(*value),
        }
    }
}

struct ResolvedValue {
    value: Value,
    address: Address,
    value_at_address: Word,
}

impl ResolvedValue {
    fn unpack(&self) -> (Address, Word) {
        (self.address, self.value_at_address)
    }
}

struct InstructionWithOperands {
    word: Word,
    instruction: Instruction,
    a: ResolvedValue,
    b: Option<ResolvedValue>,
}

impl InstructionWithOperands {
    /// Uses the CPU's resolve method (which may advance the PC)
    /// to look up an entire instruction.
    fn resolve_2op(
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

impl<'p> DCPU16<'p> {
    pub fn new(program: &'p [u16]) -> Self {
        assert!(program.len() < u16::MAX as usize);
        Self {
            ram: Box::new([0; NUM_RAM_WORDS]),
            registers: [0; NUM_REGISTERS],
            pc: 0,
            sp: STACK_POINTER_INIT as _,
            overflow: 0,
            program,
        }
    }

    pub fn step(&mut self) -> bool {
        let location = self.pc;
        let instruction = self.read_instruction();
        println!("{:04X?}: {:?}", location, instruction);

        match instruction.instruction {
            Instruction::Set { .. } => {
                self.store_value(
                    instruction.a.address,
                    instruction.b.unwrap().value_at_address,
                );
            }
            Instruction::Add { a, b } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let (result, overflow) = lhs.overflowing_add(rhs);
                self.overflow = if overflow { 0x0001 } else { 0x0 };
                self.store_value(a, result);
            }
            Instruction::Sub { a, b } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let (result, overflow) = lhs.overflowing_sub(rhs);
                self.overflow = if overflow { 0xffff } else { 0x0 };
                self.store_value(a, result);
            }
            Instruction::Mul { a, b } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let result = lhs.wrapping_mul(rhs);
                self.overflow = (((lhs as u32 * rhs as u32) >> 16) & 0xffff) as _;
                self.store_value(a, result);
            }
            Instruction::Div { a, b } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                if rhs > 0 {
                    let result = lhs.wrapping_div(rhs);
                    self.overflow = ((((lhs as u32) << 16) / (rhs as u32)) & 0xffff) as _;
                    self.store_value(a, result);
                } else {
                    self.overflow = 0;
                    self.store_value(a, 0);
                }
            }
            Instruction::Mod { a, b } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                if rhs > 0 {
                    let result = lhs % rhs;
                    self.store_value(a, result);
                } else {
                    self.store_value(a, 0);
                }
            }
            Instruction::Shl { a, b } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let result = lhs << rhs;
                self.overflow = ((((lhs as u32) << (rhs as u32)) >> 16) & 0xffff) as u16;
                self.store_value(a, result);
            }
            Instruction::Shr { a, b } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let result = lhs >> rhs;
                self.overflow = ((((lhs as u32) << 16) >> (rhs as u32)) & 0xffff) as u16;
                self.store_value(a, result);
            }
            Instruction::And { a, b } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let result = lhs.bitand(rhs);
                self.store_value(a, result);
            }
            Instruction::Bor { a, b } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let result = lhs.bitor(rhs);
                self.store_value(a, result);
            }
            Instruction::Xor { a, b } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let result = lhs.bitxor(rhs);
                self.store_value(a, result);
            }
            Instruction::Ife { a, b } => {
                let lhs = instruction.a.value_at_address;
                let rhs = instruction.b.unwrap().value_at_address;
                if !(lhs == rhs) {
                    self.skip_instruction()
                }
            }
            Instruction::Ifn { a, b } => {
                let lhs = instruction.a.value_at_address;
                let rhs = instruction.b.unwrap().value_at_address;
                if !(lhs != rhs) {
                    self.skip_instruction()
                }
            }
            Instruction::Ifg { a, b } => {
                let lhs = instruction.a.value_at_address;
                let rhs = instruction.b.unwrap().value_at_address;
                if !(lhs > rhs) {
                    self.skip_instruction()
                }
            }
            Instruction::Ifb { a, b } => {
                let lhs = instruction.a.value_at_address;
                let rhs = instruction.b.unwrap().value_at_address;
                if !(lhs.bitor(rhs) != 0) {
                    self.skip_instruction()
                }
            }
            _ => panic!(),
        }

        // We print the state after the execution.
        self.dump_state();
        println!();

        (self.pc as usize) < self.program.len()
    }

    fn read_instruction(&mut self) -> InstructionWithOperands {
        let instruction_word = self.read_word_and_advance_pc();
        let instruction = Instruction::from(instruction_word);
        assert!(instruction.len() >= 1);

        match instruction {
            Instruction::Set { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Add { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Sub { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Mul { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Div { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Mod { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Shl { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Shr { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::And { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Bor { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Xor { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Ife { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Ifn { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Ifg { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            Instruction::Ifb { a, b } => {
                InstructionWithOperands::resolve_2op(self, instruction_word, instruction, a, b)
            }
            _ => panic!(),
        }
    }

    pub fn dump_state(&mut self) {
        println!(
            "A={:04X?} B={:04X?} C={:04X?} X={:04X?} Y={:04X?} Z={:04X?} I={:04X?} J={:04X?} PC⁎={:04X?} SP={:04X?} O={:04X?}",
            self.registers[0],
            self.registers[1],
            self.registers[2],
            self.registers[3],
            self.registers[4],
            self.registers[5],
            self.registers[6],
            self.registers[7],
            self.pc,
            self.sp,
            self.overflow
        );
    }

    /// Reads the value from the specified address.
    fn read_value(&self, address: Address) -> Word {
        match address {
            Address::Literal(value) => value,
            Address::Register(register) => self.registers[register as usize],
            Address::Address(address) => self.ram[address as usize],
        }
    }

    /// Stores the value to the specified address.
    fn store_value(&mut self, address: Address, value: Word) {
        match address {
            // Specification:
            // If any instruction tries to assign a literal value, the assignment fails silently.
            // Other than that, the instruction behaves as normal.
            Address::Literal(_) => {}
            Address::Register(register) => self.registers[register as usize] = value,
            Address::Address(address) => self.ram[address as usize] = value,
        }
    }

    /// Shorthand for [`interpret_address()`] followed by [`read_value()`].
    /// Returns the address and the value at the address.
    fn resolve_address(&mut self, value: Value) -> (Address, Word) {
        let address = self.interpret_address(value);
        (address, self.read_value(address))
    }

    /// Resolves an value into an [`Address`].
    fn interpret_address(&mut self, value: Value) -> Address {
        match value {
            Value::Register { register } => Address::Register(register),
            Value::AtAddressFromRegister { register } => {
                Address::Address(self.registers[register as usize])
            }
            Value::AtAddressFromNextWordPlusRegister { register } => {
                let word = self.read_word_and_advance_pc();
                let register = self.registers[register as usize];
                Address::Address(word + register)
            }
            Value::Pop => {
                let address = self.sp;
                self.sp += 1;
                Address::Address(address)
            }
            Value::Peek => Address::Address(self.sp),
            Value::Push => {
                self.sp -= 1;
                Address::Address(self.sp)
            }
            Value::OfStackPointer => Address::Literal(self.sp),
            Value::OfProgramCounter => Address::Literal(self.pc),
            Value::OfOverflow => Address::Literal(self.overflow),
            Value::AtAddressFromNextWord => {
                let word = self.read_word_and_advance_pc();
                Address::Address(word)
            }
            Value::NextWordLiteral => {
                let word = self.read_word_and_advance_pc();
                Address::Literal(word)
            }
            Value::Literal { value } => Address::Address(value),
        }
    }

    /// Reads the value at the current program counter and advances the program counter.
    fn read_word_and_advance_pc(&mut self) -> u16 {
        let value = self.program[self.pc as usize];
        self.pc += 1;
        value
    }

    /// Skips the next instruction.
    fn skip_instruction(&mut self) {
        // For this to work we need to skip up to three words, depending on the actual instruction.
        panic!()
        // self.read_word_and_advance_pc();
    }
}
