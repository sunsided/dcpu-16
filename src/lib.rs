//! An emulator for the DCPU-16 16-bit processor from the 0x10c game,
//! with an optional assembler behind the `assembler` feature.
//!
//! Load a program with [`DCPU16::new`] and execute it with [`DCPU16::run`],
//! [`DCPU16::run_for`] or [`DCPU16::step`]. Memory-mapped [`devices`] such as a
//! keyboard and a console can be attached with [`DCPU16::attach`]. With the
//! `assembler` feature, `assemble` turns DCPU-16 assembly source into program words.

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

#[cfg(feature = "assembler")]
mod assembler;
pub mod devices;
mod disassemble;
mod instruction;
mod instruction_argument;
mod instruction_word;
mod register;

#[cfg(feature = "assembler")]
pub use crate::assembler::{AssembleError, AssembleErrorKind, Position, assemble};
use crate::devices::Device;
use crate::disassemble::Disassemble;
use crate::instruction::{Instruction, InstructionWithOperands, ResolvedValue};
use crate::instruction_argument::{
    InstructionArgument, InstructionArgumentDefinition, Location, SpecialRegister, StackOperation,
};
use crate::instruction_word::{InstructionWord, NonBasicInstruction};
pub use crate::register::Register;
use std::ops::{BitAnd, BitOr, BitXor};
use tracing::{debug, info, trace, warn};

/// The DCPU-16 is a 16-bit machine; every register, RAM cell and program entry is one word.
type Word = u16;

/// Number of general-purpose registers (`A`, `B`, `C`, `X`, `Y`, `Z`, `I`, `J`).
const NUM_REGISTERS: usize = 8;
/// Number of words of RAM.
pub const NUM_RAM_WORDS: usize = 0x10000;

/// Largest program, in words, that [`DCPU16::new`] accepts.
const MAX_PROGRAM_WORDS: usize = u16::MAX as usize - 1;

/// Initial stack pointer. `PUSH` is `[--SP]`, so the first value pushed lands in
/// the last RAM word, `0xffff`, and the stack grows downwards from there.
const STACK_POINTER_INIT: Word = 0;

/// Decoding of instructions or values.
trait Decode {
    /// Decodes the specified word.
    fn decode(value: Word) -> Self;
}

/// An instruction rendered as assembly by [`DCPU16::disassemble`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Disassembly {
    /// The instruction in assembly syntax, e.g. `SET [0x1000], 0x20`.
    pub text: String,
    /// The number of words the instruction occupies, from one to three.
    pub length: u16,
}

/// A DCPU-16 emulator.
pub struct DCPU16 {
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
    /// Number of program words loaded at address `0`; execution ends when the
    /// program counter moves past them.
    program_length: usize,
    /// Indicates whether the next instruction should be skipped.
    skip_next_instruction: bool,
    /// Number of cycles executed so far.
    cycles: u64,
    /// Devices that are ticked after every instruction.
    devices: Vec<Box<dyn Device + Send>>,
}

impl DCPU16 {
    /// Creates a CPU with cleared registers and loads `program` into RAM at address `0`,
    /// where it executes from. Like any other RAM, the program can read and overwrite itself.
    ///
    /// # Panics
    ///
    /// Panics if the program is longer than 65534 words.
    pub fn new(program: &[u16]) -> Self {
        assert!(program.len() <= MAX_PROGRAM_WORDS);
        let mut ram = Box::new([0; NUM_RAM_WORDS]);
        ram[..program.len()].copy_from_slice(program);

        let cpu = Self {
            ram,
            registers: [0; NUM_REGISTERS],
            program_counter: 0,
            stack_pointer: STACK_POINTER_INIT,
            overflow: 0,
            program_length: program.len(),
            previous_program_counter: 0,
            skip_next_instruction: false,
            cycles: 0,
            devices: Vec::new(),
        };

        info!(
            "Loaded {program_length} words of program data",
            program_length = program.len()
        );
        cpu.dump_registers();
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

    /// Gets a mutable reference to the RAM.
    pub fn ram_mut(&mut self) -> &mut [u16; NUM_RAM_WORDS] {
        self.ram.as_mut()
    }

    /// Gets the number of cycles executed so far, as defined by the specification.
    pub fn cycles(&self) -> u64 {
        self.cycles
    }

    /// Attaches a device, which is ticked after every executed instruction from now on.
    pub fn attach(&mut self, device: impl Device + Send + 'static) {
        self.devices.push(Box::new(device));
    }

    /// Disassembles the instruction stored at `address` without executing it.
    ///
    /// Reserved opcodes render as `DAT` with the raw word.
    pub fn disassemble(&self, address: u16) -> Disassembly {
        let instruction = self.instruction_at(address);
        let (raw_instruction, instruction_word, _, _) = instruction.unpack();
        let text = if instruction_word == InstructionWord::NonBasic(NonBasicInstruction::Reserved) {
            format!("DAT 0x{raw_instruction:04X}")
        } else {
            InstructionWithOperands::inspect(self, instruction).disassemble()
        };
        Disassembly {
            text,
            length: instruction_word.length_in_words() as u16,
        }
    }

    /// Executes the program until it ends, hits a reserved opcode or a crash loop is detected.
    pub fn run(&mut self) {
        while self.step() {}
    }

    /// Executes instructions until at least `cycles` more cycles have passed or execution stops.
    ///
    /// Returns `false` if execution stopped, like [`step`](Self::step) does. Calling this
    /// repeatedly with a pause in between paces the program, e.g. 1000 cycles every 10 ms
    /// for the 100 kHz the DCPU-16 runs at in 0x10c.
    pub fn run_for(&mut self, cycles: u64) -> bool {
        let end = self.cycles.saturating_add(cycles);
        while self.cycles < end {
            if !self.step() {
                return false;
            }
        }
        true
    }

    /// Executes a single instruction of the program.
    ///
    /// Returns `false` once execution should stop: at the end of the program,
    /// on a reserved opcode, or when an instruction jumps to itself.
    pub fn step(&mut self) -> bool {
        self.previous_program_counter = self.program_counter;
        let instruction = self.read_instruction();

        if self.skip_next_instruction {
            // Skipped instructions are not resolved, so their operands have no side effects.
            // Skipping costs nothing beyond the extra cycle the failed test already took.
            self.execute_skipped_instruction(instruction);
        } else {
            let (_, instruction_word, _, _) = instruction.unpack();
            if instruction_word == InstructionWord::NonBasic(NonBasicInstruction::Reserved) {
                warn!(
                    "Reserved opcode at PC={pc:04X} - terminating",
                    pc = self.previous_program_counter
                );
                return false;
            }

            self.cycles += instruction_word.cycles();
            let instruction = InstructionWithOperands::resolve(self, instruction);
            let keep_running = self.execute_instruction(instruction);
            if self.skip_next_instruction {
                // A failed test takes one extra cycle.
                self.cycles += 1;
            }
            self.tick_devices();
            if !keep_running {
                return false;
            }
        }

        // We print the state after the execution.
        self.dump_registers();

        if (self.program_counter as usize) < self.program_length {
            return true;
        }

        warn!("End of program reached - terminating");
        false
    }

    /// Lets every attached device access the RAM.
    fn tick_devices(&mut self) {
        for device in &mut self.devices {
            device.tick(&mut self.ram);
        }
    }

    /// "Executes" a skipped instruction.
    fn execute_skipped_instruction(&mut self, instruction: Instruction) {
        let (raw_instruction, instruction_word, _, _) = instruction.unpack();
        if instruction_word == InstructionWord::NonBasic(NonBasicInstruction::Reserved) {
            // Reserved opcodes have no operands to describe.
            debug!(
                "SKIP {operation_pc:04X}: {raw_instruction:04x} ; reserved",
                operation_pc = self.previous_program_counter,
            );
        } else {
            let instruction = InstructionWithOperands::inspect(self, instruction);
            debug!(
                "SKIP {operation_pc:04X}: {instruction:?}",
                operation_pc = self.previous_program_counter,
                instruction = instruction
            );
        }
        self.skip_next_instruction = false;
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
                NonBasicInstruction::Reserved => unreachable!("reserved opcodes stop in step()"),
                NonBasicInstruction::Jsr { .. } => {
                    assert!(instruction.b.is_none());
                    self.stack_pointer = self.stack_pointer.wrapping_sub(1);
                    self.ram[self.stack_pointer as usize] = self.program_counter;
                    self.program_counter = instruction.a.resolved_value;
                }
            },
            InstructionWord::Set { .. } => {
                self.store_value(
                    instruction.a.location,
                    instruction
                        .b
                        .expect("require second argument")
                        .resolved_value,
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
                // Shifting by 32 or more leaves nothing in either word.
                let shifted = u32::from(lhs).checked_shl(rhs.into()).unwrap_or(0);
                self.overflow = (shifted >> 16) as Word;
                self.store_value(a, shifted as Word);
            }
            InstructionWord::Shr { .. } => {
                let (a, lhs) = instruction.a.unpack();
                let (_, rhs) = instruction.b.expect("require second argument").unpack();
                // `a` sits in the high word so the bits shifted out land in the low word.
                let shifted = (u32::from(lhs) << 16).checked_shr(rhs.into()).unwrap_or(0);
                self.overflow = shifted as Word;
                self.store_value(a, (shifted >> 16) as Word);
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
                let rhs = instruction
                    .b
                    .expect("require second argument")
                    .resolved_value;
                if !(lhs == rhs) {
                    self.skip_next_instruction = true;
                }
            }
            InstructionWord::Ifn { .. } => {
                let lhs = instruction.a.resolved_value;
                let rhs = instruction
                    .b
                    .expect("require second argument")
                    .resolved_value;
                if !(lhs != rhs) {
                    self.skip_next_instruction = true;
                }
            }
            InstructionWord::Ifg { .. } => {
                let lhs = instruction.a.resolved_value;
                let rhs = instruction
                    .b
                    .expect("require second argument")
                    .resolved_value;
                if !(lhs > rhs) {
                    self.skip_next_instruction = true;
                }
            }
            InstructionWord::Ifb { .. } => {
                let lhs = instruction.a.resolved_value;
                let rhs = instruction
                    .b
                    .expect("require second argument")
                    .resolved_value;
                if lhs.bitand(rhs) == 0 {
                    self.skip_next_instruction = true;
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

    /// Reads the instruction at the program counter, including its extra operand words,
    /// and advances the program counter past it. Operands are not resolved yet.
    fn read_instruction(&mut self) -> Instruction {
        let instruction = self.instruction_at(self.program_counter);
        let (_, instruction_word, _, _) = instruction.unpack();
        self.program_counter = self
            .program_counter
            .wrapping_add(instruction_word.length_in_words() as Word);
        instruction
    }

    /// Reads the instruction at `address`, including its extra operand words,
    /// which wrap around at the end of RAM.
    fn instruction_at(&self, address: Word) -> Instruction {
        let word = |offset: Word| self.ram[usize::from(address.wrapping_add(offset))];
        let raw_instruction = word(0);
        let instruction_word = InstructionWord::decode(raw_instruction);

        match instruction_word.length_in_words() {
            1 => Instruction::OneWord {
                raw_instruction,
                instruction: instruction_word,
            },
            2 => Instruction::TwoWord {
                raw_instruction,
                instruction: instruction_word,
                raw_1st: word(1),
            },
            3 => Instruction::ThreeWord {
                raw_instruction,
                instruction: instruction_word,
                raw_1st: word(1),
                raw_2nd: word(2),
            },
            _ => unreachable!("instructions are one to three words long"),
        }
    }

    /// Decodes an operand, resolves it to its [`Location`] and reads the value there.
    fn resolve_argument(
        &mut self,
        definition: InstructionArgumentDefinition,
        operand: Option<Word>,
    ) -> ResolvedValue {
        let argument = InstructionArgument::from(definition, operand);
        let location = self.locate(argument);
        ResolvedValue {
            argument_definition: definition,
            argument,
            location,
            resolved_value: self.read_value(location),
        }
    }

    /// Like [`Self::resolve_argument`], but without moving the stack pointer.
    fn inspect_argument(
        &self,
        definition: InstructionArgumentDefinition,
        operand: Option<Word>,
    ) -> ResolvedValue {
        let argument = InstructionArgument::from(definition, operand);
        let location = self.location_of(argument);
        ResolvedValue {
            argument_definition: definition,
            argument,
            location,
            resolved_value: self.read_value(location),
        }
    }

    /// Resolves an argument to the location it reads from and writes to, then
    /// applies its side effect.
    ///
    /// This is the only place where `PUSH` and `POP` move the stack pointer,
    /// so each operand moves it exactly once.
    fn locate(&mut self, argument: InstructionArgument) -> Location {
        let location = self.location_of(argument);
        match argument {
            InstructionArgument::StackOperation(StackOperation::Pop) => {
                self.stack_pointer = self.stack_pointer.wrapping_add(1);
            }
            InstructionArgument::StackOperation(StackOperation::Push) => {
                self.stack_pointer = self.stack_pointer.wrapping_sub(1);
            }
            _ => {}
        }
        location
    }

    /// Computes the location an argument reads from and writes to, without side effects.
    /// All address arithmetic wraps around the 16-bit address space.
    fn location_of(&self, argument: InstructionArgument) -> Location {
        match argument {
            InstructionArgument::Literal(value) => Location::Literal(value),
            InstructionArgument::Register(register) => Location::Register(register),
            InstructionArgument::Address(address) => Location::Ram(address),
            InstructionArgument::AddressFromRegister(register) => {
                Location::Ram(self.registers[register as usize])
            }
            InstructionArgument::AddressOffset { address, register } => {
                Location::Ram(address.wrapping_add(self.registers[register as usize]))
            }
            InstructionArgument::SpecialRegister(register) => Location::SpecialRegister(register),
            InstructionArgument::StackOperation(operation) => match operation {
                StackOperation::Peek | StackOperation::Pop => Location::Ram(self.stack_pointer),
                StackOperation::Push => Location::Ram(self.stack_pointer.wrapping_sub(1)),
            },
        }
    }

    /// Reads the value at the specified location.
    fn read_value(&self, location: Location) -> Word {
        match location {
            Location::Literal(value) => value,
            Location::Register(register) => self.registers[register as usize],
            Location::Ram(address) => self.ram[address as usize],
            Location::SpecialRegister(register) => match register {
                SpecialRegister::ProgramCounter => self.program_counter,
                SpecialRegister::StackPointer => self.stack_pointer,
                SpecialRegister::Overflow => self.overflow,
            },
        }
    }

    /// Stores the value to the specified location.
    fn store_value(&mut self, location: Location, value: Word) {
        match location {
            // Specification:
            // If any instruction tries to assign a literal value, the assignment fails silently.
            // Other than that, the instruction behaves as normal.
            Location::Literal(literal) => {
                trace!(
                    "Skipping literal assignment of word {word:04X} to literal {literal:04X}",
                    word = value,
                    literal = literal
                )
            }
            Location::Register(register) => self.registers[register as usize] = value,
            Location::Ram(address) => self.ram[address as usize] = value,
            Location::SpecialRegister(register) => match register {
                SpecialRegister::ProgramCounter => self.program_counter = value,
                SpecialRegister::StackPointer => self.stack_pointer = value,
                SpecialRegister::Overflow => self.overflow = value,
            },
        }
    }

    /// Logs all register values at debug level.
    fn dump_registers(&self) {
        debug!(
            "Registers: A={a:04X?} B={b:04X?} C={c:04X?} X={x:04X?} Y={y:04X?} Z={z:04X?} I={i:04X?} J={j:04X?} PC⁎={pc:04X?} SP={sp:04X?} O={o:04X?}",
            a = self.registers[0],
            b = self.registers[1],
            c = self.registers[2],
            x = self.registers[3],
            y = self.registers[4],
            z = self.registers[5],
            i = self.registers[6],
            j = self.registers[7],
            pc = self.program_counter,
            sp = self.stack_pointer,
            o = self.overflow
        );
    }

    /// Formats the program region of RAM as a hex dump with `words_per_row` words per line,
    /// each line prefixed with its start address. Words the program has overwritten
    /// show their current value.
    ///
    /// # Panics
    ///
    /// Panics if `words_per_row` is zero.
    pub fn hexdump_program(&self, words_per_row: usize) -> String {
        assert!(words_per_row > 0);
        let program = &self.ram[..self.program_length];
        let newline = String::from('\n');
        let length_of_newline = newline.len();
        debug_assert_eq!(length_of_newline, 1);

        let row_length = (4 + 1) + (1 + 4) * words_per_row + length_of_newline;
        let row_count = program.len() / words_per_row;
        let remainder = program.len() % words_per_row;
        let mut expected_num_characters = row_length * row_count;
        if remainder > 0 {
            expected_num_characters += row_length;
        }

        let mut dump = String::with_capacity(expected_num_characters);

        for row in 0..row_count {
            let row_start = row * words_per_row;
            dump.push_str(format!("{:04X}:", row_start).as_str());
            for word in 0..words_per_row {
                dump.push_str(format!(" {:04X}", program[row_start + word]).as_str());
            }
            dump.push_str(newline.as_str())
        }

        if remainder > 0 {
            for row in row_count..(row_count + 1) {
                let row_start = row * words_per_row;
                dump.push_str(format!("{:04X}:", row_start).as_str());
                for word in 0..remainder {
                    dump.push_str(format!(" {:04X}", program[row_start + word]).as_str());
                }
                for _ in remainder..words_per_row {
                    dump.push_str("     ");
                }

                dump.push_str(newline.as_str())
            }
        }

        assert_eq!(dump.len(), expected_num_characters);
        dump
    }

    /// Formats the entire RAM as a hex dump with `words_per_row` words per line,
    /// each line prefixed with its start address.
    ///
    /// # Panics
    ///
    /// Panics if `words_per_row` is zero.
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
