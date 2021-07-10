mod instruction;
mod register;
mod value;

use crate::instruction::{Instruction, NonBasicInstruction};
use crate::register::Register;
use crate::value::Value;
use std::fmt::{Debug, Formatter};
use std::ops::{BitAnd, BitOr, BitXor};
use tracing::{debug, info, trace, warn};

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
    program_counter: Word,
    /// Program counter location of the last step.
    ///
    /// This value is used to determine a "crash loop" (a jump to the same instruction).
    previous_program_counter: Word,
    /// Stack pointer.
    stack_pointer: Word,
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
    ProgramCounter,
    StackPointer,
    Overflow,
}

impl Address {
    fn get_literal(&self) -> Option<Word> {
        match self {
            Self::Literal(value) => Some(*value),
            Self::Address(value) => Some(*value),
            _ => None,
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
    /// to look up an entire instruction that takes one additional operands.
    fn resolve_1op(cpu: &mut DCPU16, word: Word, instruction: Instruction, a: Value) -> Self {
        let (lhs_addr, lhs) = cpu.resolve_address(a);
        InstructionWithOperands::new_1op(word, instruction, a, lhs_addr, lhs)
    }

    /// Uses the CPU's resolve method (which may advance the PC)
    /// to look up an entire instruction that takes two additional operands.
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

impl<'p> DCPU16<'p> {
    pub fn new(program: &'p [u16]) -> Self {
        assert!(program.len() < u16::MAX as usize);
        let cpu = Self {
            ram: Box::new([0; NUM_RAM_WORDS]),
            registers: [0; NUM_REGISTERS],
            program_counter: 0,
            previous_program_counter: 0,
            stack_pointer: STACK_POINTER_INIT as _,
            overflow: 0,
            program,
        };

        info!(
            "Loaded {program_length} bytes words of program data",
            program_length = program.len()
        );
        cpu.dump_state();
        cpu
    }

    pub fn step(&mut self) -> bool {
        self.previous_program_counter = self.program_counter;
        let instruction = self.read_instruction();

        debug!(
            "PC={operation_pc:04X}:   {instruction:?}",
            operation_pc = self.previous_program_counter,
            instruction = instruction
        );

        match instruction.instruction {
            Instruction::NonBasic(nbi) => match nbi {
                NonBasicInstruction::Reserved => panic!(),
                NonBasicInstruction::Jsr { .. } => {
                    assert!(instruction.b.is_none());
                    self.stack_pointer -= 1;
                    self.ram[self.stack_pointer as usize] = self.program_counter;
                    self.program_counter = instruction.a.value_at_address;
                }
            },
            Instruction::Set { .. } => {
                self.store_value(
                    instruction.a.address,
                    instruction.b.unwrap().value_at_address,
                );
            }
            Instruction::Add { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let (result, overflow) = lhs.overflowing_add(rhs);
                self.overflow = if overflow { 0x0001 } else { 0x0 };
                self.store_value(a, result);
            }
            Instruction::Sub { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let (result, overflow) = lhs.overflowing_sub(rhs);
                self.overflow = if overflow { 0xffff } else { 0x0 };
                self.store_value(a, result);
            }
            Instruction::Mul { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let result = lhs.wrapping_mul(rhs);
                self.overflow = (((lhs as u32 * rhs as u32) >> 16) & 0xffff) as _;
                self.store_value(a, result);
            }
            Instruction::Div { .. } => {
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
            Instruction::Mod { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                if rhs > 0 {
                    let result = lhs % rhs;
                    self.store_value(a, result);
                } else {
                    self.store_value(a, 0);
                }
            }
            Instruction::Shl { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let result = lhs << rhs;
                self.overflow = ((((lhs as u32) << (rhs as u32)) >> 16) & 0xffff) as u16;
                self.store_value(a, result);
            }
            Instruction::Shr { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let result = lhs >> rhs;
                self.overflow = ((((lhs as u32) << 16) >> (rhs as u32)) & 0xffff) as u16;
                self.store_value(a, result);
            }
            Instruction::And { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let result = lhs.bitand(rhs);
                self.store_value(a, result);
            }
            Instruction::Bor { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let result = lhs.bitor(rhs);
                self.store_value(a, result);
            }
            Instruction::Xor { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.unwrap().unpack();
                let result = lhs.bitxor(rhs);
                self.store_value(a, result);
            }
            Instruction::Ife { .. } => {
                let lhs = instruction.a.value_at_address;
                let rhs = instruction.b.unwrap().value_at_address;
                if !(lhs == rhs) {
                    self.skip_instruction()
                }
            }
            Instruction::Ifn { .. } => {
                let lhs = instruction.a.value_at_address;
                let rhs = instruction.b.unwrap().value_at_address;
                if !(lhs != rhs) {
                    self.skip_instruction()
                }
            }
            Instruction::Ifg { .. } => {
                let lhs = instruction.a.value_at_address;
                let rhs = instruction.b.unwrap().value_at_address;
                if !(lhs > rhs) {
                    self.skip_instruction()
                }
            }
            Instruction::Ifb { .. } => {
                let lhs = instruction.a.value_at_address;
                let rhs = instruction.b.unwrap().value_at_address;
                if !(lhs.bitor(rhs) != 0) {
                    self.skip_instruction()
                }
            }
        }

        // We print the state after the execution.
        self.dump_state();

        // An operation may mutate the program counter, e.g. `SET PC, POP`.
        // The comparison of the PC before the instruction was read and after
        // it was executed can be used as a naive heuristic for crash loop detection.
        if self.previous_program_counter == self.program_counter {
            warn!("Crash loop detected - terminating");
            return false;
        }

        (self.program_counter as usize) < self.program.len()
    }

    fn read_instruction(&mut self) -> InstructionWithOperands {
        let instruction_word = self.read_word_and_advance_pc();
        let instruction = Instruction::from(instruction_word);
        assert!(instruction.len() >= 1);

        match instruction {
            Instruction::NonBasic(nbi) => match nbi {
                NonBasicInstruction::Reserved => panic!(),
                NonBasicInstruction::Jsr { a } => {
                    InstructionWithOperands::resolve_1op(self, instruction_word, instruction, a)
                }
            },
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
        }
    }

    pub fn dump_state(&self) {
        debug!(
            "Registers: A={a:04X?} B={b:04X?} C={c:04X?} X={x:04X?} Y={y:04X?} Z={z:04X?} I={i:04X?} J={j:04X?} PC⁎={pc:04X?} SP={sp:04X?} O={o:04X?}",
            a=self.registers[0],
            b=self.registers[1],
            c=self.registers[2],
            x=self.registers[3],
            y=self.registers[4],
            z=self.registers[5],
            i=self.registers[6],
            j=self.registers[7],
            pc=self.program_counter,
            sp=self.stack_pointer,
            o=self.overflow
        );
    }

    /// Reads the value from the specified address.
    fn read_value(&self, address: Address) -> Word {
        match address {
            Address::Literal(value) => value,
            Address::Register(register) => self.registers[register as usize],
            Address::Address(address) => self.ram[address as usize],
            Address::ProgramCounter => self.program_counter,
            Address::StackPointer => self.stack_pointer,
            Address::Overflow => self.overflow,
        }
    }

    /// Stores the value to the specified address.
    fn store_value(&mut self, address: Address, value: Word) {
        match address {
            // Specification:
            // If any instruction tries to assign a literal value, the assignment fails silently.
            // Other than that, the instruction behaves as normal.
            Address::Literal(_) => {
                trace!(
                    "Skipping literal assignment of word {word:04X} to literal {literal:04X}",
                    word = value,
                    literal = address.get_literal().unwrap()
                )
            }
            Address::Register(register) => self.registers[register as usize] = value,
            Address::Address(address) => self.ram[address as usize] = value,
            Address::ProgramCounter => self.program_counter = value,
            Address::StackPointer => self.stack_pointer = value,
            Address::Overflow => self.overflow = value,
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
                let address = self.stack_pointer;
                self.stack_pointer += 1;
                Address::Address(address)
            }
            Value::Peek => Address::Address(self.stack_pointer),
            Value::Push => {
                self.stack_pointer -= 1;
                Address::Address(self.stack_pointer)
            }
            Value::OfStackPointer => Address::StackPointer,
            Value::OfProgramCounter => Address::ProgramCounter,
            Value::OfOverflow => Address::Overflow,
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
        let value = self.program[self.program_counter as usize];
        self.program_counter += 1;
        value
    }

    /// Skips the next instruction.
    fn skip_instruction(&mut self) {
        // Since the read_instruction() function reads the entire instruction including
        // its arguments, executing it here ensures we're skipping over the correct
        // amount of words.
        let _ = self.read_instruction();
    }
}
