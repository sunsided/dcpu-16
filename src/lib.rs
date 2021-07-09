mod instruction;
mod register;
mod value;

use crate::register::Register;

type Word = u16;

const NUM_REGISTERS: usize = 8;
const NUM_RAM_WORDS: usize = 0x10000;

// Stack pointer is initialized to 0xffff (for 0x10000 words of memory).
const STACK_POINTER_INIT: usize = NUM_RAM_WORDS - 1;

trait DurationCycles {
    fn base_cycle_count(&self) -> usize;
}

/// A DCPU-16 emulator.
pub struct DCPU16 {
    /// RAM.
    ram: Box<[Word; NUM_RAM_WORDS]>,
    /// Registers.
    registers: [Word; NUM_REGISTERS],
    /// Program counter.
    pc: Word,
    /// Stack pointer.
    sp: Word,
    /// Overflow.
    o: Word,
}

impl Default for DCPU16 {
    fn default() -> Self {
        Self {
            ram: Box::new([0; NUM_RAM_WORDS]),
            registers: [0; NUM_REGISTERS],
            pc: 0,
            sp: STACK_POINTER_INIT as _,
            o: 0,
        }
    }
}

impl DCPU16 {
    pub fn load(&mut self, _program: &[u16]) {
        self.pc = 0;
        self.sp = STACK_POINTER_INIT as _;
    }

    fn next_word(&mut self) -> u16 {
        let value = self.ram[self.pc as usize];
        self.pc += 1;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_program() {
        let _program: [u16; 32] = [
            0x7c01, 0x0030, 0x7de1, 0x1000, 0x0020, 0x7803, 0x1000, 0xc00d, 0x7dc1, 0x001a, 0xa861,
            0x7c01, 0x2000, 0x2161, 0x2000, 0x8463, 0x806d, 0x7dc1, 0x000d, 0x9031, 0x7c10, 0x0018,
            0x7dc1, 0x001a, 0x9037, 0x61c1, 0x7dc1, 0x001a, 0x0000, 0x0000, 0x0000, 0x0000,
        ];

        assert!(true)
    }
}
