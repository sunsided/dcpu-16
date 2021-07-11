mod disassemble;
mod instruction_word;
mod instruction;
mod register;
mod instruction_argument;

use crate::instruction_word::{InstructionWord, NonBasicInstruction};
use crate::instruction::{InstructionWithOperands, Instruction};
pub use crate::register::Register;
use crate::instruction_argument::{InstructionArgumentDefinition, InstructionArgument};
use std::ops::{BitAnd, BitOr, BitXor};
use tracing::{debug, info, trace, warn};

type Word = u16;

const NUM_REGISTERS: usize = 8;
const NUM_RAM_WORDS: usize = 0x10000;

// Stack pointer is initialized to 0xffff (for 0x10000 words of memory).
const STACK_POINTER_INIT: usize = NUM_RAM_WORDS - 1;

/// Decoding of instructions or values.
trait Decode {
    /// Decodes the specified word.
    fn decode(value: Word) -> Self;
}

/// A DCPU-16 emulator.
pub struct DCPU16<'p> {
    /// RAM.
    ram: Box<[Word; NUM_RAM_WORDS]>,
    /// Registers.
    registers: [Word; NUM_REGISTERS],
    /// Program counter.
    pub program_counter: Word,
    /// Stack pointer.
    pub stack_pointer: Word,
    /// Overflow.
    pub overflow: Word,

    /// Program counter location of the last step.
    ///
    /// This value is used to determine a "crash loop" (a jump to the same instruction).
    previous_program_counter: Word,
    /// The program
    program: &'p [u16],
    /// Indicates whether the next instruction should be skipped.
    skip_next_intruction: bool
}

impl<'p> DCPU16<'p> {
    pub fn new(program: &'p [u16]) -> Self {
        assert!(program.len() < u16::MAX as usize);
        let cpu = Self {
            ram: Box::new([0; NUM_RAM_WORDS]),
            registers: [0; NUM_REGISTERS],
            program_counter: 0,
            stack_pointer: STACK_POINTER_INIT as _,
            overflow: 0,
            program,
            previous_program_counter: 0,
            skip_next_intruction: false
        };

        info!(
            "Loaded {program_length} words of program data",
            program_length = program.len()
        );
        cpu.dump_state();
        cpu
    }

    /// Gets the value of the specified register.
    pub fn register(&self, register: Register) -> Word {
        self.registers[register as usize]
    }

    /// Gets a reference to the RAM.
    pub fn ram(&self) -> &[u16; NUM_RAM_WORDS] {
        self.ram.as_ref()
    }

    /// Gets a reference to the RAM.
    pub fn ram_mut(&mut self) -> &[u16; NUM_RAM_WORDS] {
        self.ram.as_mut()
    }

    /// Executes the program until a crash loop is detected.
    pub fn run(&mut self) {
        while self.step() {}
    }

    /// Executes a single instruction of the program.
    pub fn step(&mut self) -> bool {
        self.previous_program_counter = self.program_counter;
        let instruction = self.read_instruction();

        if self.skip_next_intruction {
            self.execute_skipped_instruction(instruction);
        }
        else {
            if !self.execute_instruction(instruction) {
                return false;
            }
        }

        // We print the state after the execution.
        self.dump_state();

        if (self.program_counter as usize) < self.program.len() {
            return true;
        }

        warn!("End of program reached - terminating");
        false
    }

    /// "Executes" a skipped instruction.
    fn execute_skipped_instruction(&mut self, instruction: InstructionWithOperands) {
        debug!(
                "SKIP {operation_pc:04X}: {instruction:?}",
                operation_pc = self.previous_program_counter,
                instruction = instruction
            );
        self.skip_next_intruction = false;
    }

    /// Executes an instruction.
    fn execute_instruction(&mut self, instruction: InstructionWithOperands) -> bool {
        debug!(
                "EXEC {operation_pc:04X}: {instruction:?}",
                operation_pc = self.previous_program_counter,
                instruction = instruction
            );

        match instruction.instruction {
            InstructionWord::NonBasic(nbi) => match nbi {
                NonBasicInstruction::Reserved => panic!(),
                NonBasicInstruction::Jsr { .. } => {
                    assert!(instruction.b.is_none());
                    self.stack_pointer -= 1;
                    self.ram[self.stack_pointer as usize] = self.program_counter;
                    self.program_counter = instruction.a.resolved_value;
                }
            },
            InstructionWord::Set { .. } => {
                self.store_value(
                    instruction.a.argument,
                    instruction.b.expect("require second argument").resolved_value,
                );
            }
            InstructionWord::Add { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.expect("require second argument").unpack();
                let (result, overflow) = lhs.overflowing_add(rhs);
                self.overflow = if overflow { 0x0001 } else { 0x0 };
                self.store_value(a, result);
            }
            InstructionWord::Sub { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.expect("require second argument").unpack();
                let (result, overflow) = lhs.overflowing_sub(rhs);
                self.overflow = if overflow { 0xffff } else { 0x0 };
                self.store_value(a, result);
            }
            InstructionWord::Mul { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.expect("require second argument").unpack();
                let result = lhs.wrapping_mul(rhs);
                self.overflow = (((lhs as u32 * rhs as u32) >> 16) & 0xffff) as _;
                self.store_value(a, result);
            }
            InstructionWord::Div { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.expect("require second argument").unpack();
                if rhs > 0 {
                    let result = lhs.wrapping_div(rhs);
                    self.overflow = ((((lhs as u32) << 16) / (rhs as u32)) & 0xffff) as _;
                    self.store_value(a, result);
                } else {
                    self.overflow = 0;
                    self.store_value(a, 0);
                }
            }
            InstructionWord::Mod { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.expect("require second argument").unpack();
                if rhs > 0 {
                    let result = lhs % rhs;
                    self.store_value(a, result);
                } else {
                    self.store_value(a, 0);
                }
            }
            InstructionWord::Shl { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.expect("require second argument").unpack();
                let result = lhs << rhs;
                self.overflow = ((((lhs as u32) << (rhs as u32)) >> 16) & 0xffff) as u16;
                self.store_value(a, result);
            }
            InstructionWord::Shr { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.expect("require second argument").unpack();
                let result = lhs >> rhs;
                self.overflow = ((((lhs as u32) << 16) >> (rhs as u32)) & 0xffff) as u16;
                self.store_value(a, result);
            }
            InstructionWord::And { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.expect("require second argument").unpack();
                let result = lhs.bitand(rhs);
                self.store_value(a, result);
            }
            InstructionWord::Bor { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.expect("require second argument").unpack();
                let result = lhs.bitor(rhs);
                self.store_value(a, result);
            }
            InstructionWord::Xor { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.expect("require second argument").unpack();
                let result = lhs.bitxor(rhs);
                self.store_value(a, result);
            }
            InstructionWord::Ife { .. } => {
                let lhs = instruction.a.resolved_value;
                let rhs = instruction.b.expect("require second argument").resolved_value;
                if !(lhs == rhs) {
                    self.skip_next_intruction = true;
                }
            }
            InstructionWord::Ifn { .. } => {
                let lhs = instruction.a.resolved_value;
                let rhs = instruction.b.expect("require second argument").resolved_value;
                if !(lhs != rhs) {
                    self.skip_next_intruction = true;
                }
            }
            InstructionWord::Ifg { .. } => {
                let lhs = instruction.a.resolved_value;
                let rhs = instruction.b.expect("require second argument").resolved_value;
                if !(lhs > rhs) {
                    self.skip_next_intruction = true;
                }
            }
            InstructionWord::Ifb { .. } => {
                let lhs = instruction.a.resolved_value;
                let rhs = instruction.b.expect("require second argument").resolved_value;
                if !(lhs.bitor(rhs) != 0) {
                    self.skip_next_intruction = true;
                }
            }
        }

        // An operation may mutate the program counter, e.g. `SET PC, POP`.
        // The comparison of the PC before the instruction was read and after
        // it was executed can be used as a naive heuristic for crash loop detection.
        if self.previous_program_counter == self.program_counter {
            warn!(
                "Crash loop detected at PC={pc:04X} - terminating",
                pc = self.program_counter
            );
            return false;
        }

        true
    }

    fn read_instruction(&mut self) -> InstructionWithOperands {
        let raw_instruction = self.read_word_and_advance_pc();
        let instruction_word = InstructionWord::decode(raw_instruction);
        assert!(instruction_word.length_in_words() >= 1);

        let instruction = match instruction_word.length_in_words() {
            1 => Instruction::OneWord { raw_instruction, instruction: instruction_word },
            2 => Instruction::TwoWord { raw_instruction, instruction: instruction_word, raw_1st: self.read_word_and_advance_pc() },
            3 => Instruction::ThreeWord { raw_instruction, instruction: instruction_word, raw_1st: self.read_word_and_advance_pc(), raw_2nd: self.read_word_and_advance_pc() },
            _ => unreachable!()
        };

        InstructionWithOperands::resolve(self, instruction)
    }

    /// Reads the value at the current program counter and advances the program counter.
    fn read_word_and_advance_pc(&mut self) -> u16 {
        let value = self.program[self.program_counter as usize];
        self.program_counter += 1;
        value
    }

    /// Shorthand for [`interpret_argument()`] followed by [`read_value()`].
    /// Returns the address and the value at the address.
    fn resolve_argument(&mut self, value: InstructionArgumentDefinition, operand: Option<Word>) -> (InstructionArgument, Word) {
        let argument = self.interpret_argument(value, operand);
        (argument, self.read_value(argument))
    }

    /// Resolves an value into an [`InstructionArgument`].
    fn interpret_argument(&mut self, value: InstructionArgumentDefinition, operand: Option<Word>) -> InstructionArgument {
        match value {
            InstructionArgumentDefinition::Register { register } => InstructionArgument::Register(register),
            InstructionArgumentDefinition::AtAddressFromRegister { register } => {
                InstructionArgument::Address(self.registers[register as usize])
            }
            InstructionArgumentDefinition::AtAddressFromNextWordPlusRegister { register } => {
                InstructionArgument::AddressOffset { address: operand.expect("operand required"), register }
            }
            InstructionArgumentDefinition::Pop => {
                let address = self.stack_pointer;
                self.stack_pointer += 1;
                InstructionArgument::Address(address)
            }
            InstructionArgumentDefinition::Peek => InstructionArgument::Address(self.stack_pointer),
            InstructionArgumentDefinition::Push => {
                self.stack_pointer -= 1;
                InstructionArgument::Address(self.stack_pointer)
            }
            InstructionArgumentDefinition::OfStackPointer => InstructionArgument::StackPointer,
            InstructionArgumentDefinition::OfProgramCounter => InstructionArgument::ProgramCounter,
            InstructionArgumentDefinition::OfOverflow => InstructionArgument::Overflow,
            InstructionArgumentDefinition::AtAddressFromNextWord => {
                InstructionArgument::Address(operand.expect("operand required"))
            }
            InstructionArgumentDefinition::NextWordLiteral => {
                InstructionArgument::Literal(operand.expect("operand required"))
            }
            InstructionArgumentDefinition::Literal { value } => InstructionArgument::Literal(value),
        }
    }

    /// Reads the value from the specified argument.
    fn read_value(&self, address: InstructionArgument) -> Word {
        match address {
            InstructionArgument::Literal(value) => value,
            InstructionArgument::Register(register) => self.registers[register as usize],
            InstructionArgument::Address(address) => self.ram[address as usize],
            InstructionArgument::AddressOffset { address, register } => {
                let register_value = self.registers[register as usize];
                self.ram[address as usize + register_value as usize]
            }
            InstructionArgument::ProgramCounter => self.program_counter,
            InstructionArgument::StackPointer => self.stack_pointer,
            InstructionArgument::Overflow => self.overflow,
        }
    }

    /// Stores the value to the specified address.
    fn store_value(&mut self, address: InstructionArgument, value: Word) {
        match address {
            // Specification:
            // If any instruction tries to assign a literal value, the assignment fails silently.
            // Other than that, the instruction behaves as normal.
            InstructionArgument::Literal(_) => {
                trace!(
                    "Skipping literal assignment of word {word:04X} to literal {literal:04X}",
                    word = value,
                    literal = address.get_literal().unwrap()
                )
            }
            InstructionArgument::Register(register) => self.registers[register as usize] = value,
            InstructionArgument::Address(address) => self.ram[address as usize] = value,
            InstructionArgument::AddressOffset { address, register } => {
                let register_value = self.registers[register as usize];
                self.ram[address as usize + register_value as usize] = value
            }
            InstructionArgument::ProgramCounter => self.program_counter = value,
            InstructionArgument::StackPointer => self.stack_pointer = value,
            InstructionArgument::Overflow => self.overflow = value,
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

    pub fn hexdump_ram(&self, words_per_row: usize) -> String {
        assert!(words_per_row > 0);
        debug_assert_eq!(NUM_RAM_WORDS, 65536);
        let newline = String::from('\n');
        let length_of_newline = newline.len();
        debug_assert_eq!(length_of_newline, 1);

        let row_length = (4 + 1) + (1 + 4) * words_per_row + length_of_newline;
        let row_count = NUM_RAM_WORDS / words_per_row;
        let expected_num_characters = row_length * row_count;

        let mut dump = String::with_capacity(expected_num_characters);

        for row in 0..row_count {
            let row_start = row * words_per_row;
            dump.push_str(format!("{:04X}:", row_start).as_str());
            for word in 0..words_per_row {
                dump.push_str(format!(" {:04X}", self.ram[row_start + word]).as_str());
            }
            dump.push_str(newline.as_str())
        }

        assert_eq!(dump.len(), expected_num_characters);
        dump
    }
}
