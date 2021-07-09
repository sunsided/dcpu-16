mod instruction;
mod register;
mod value;

use crate::register::Register;

type Word = u16;

const NUM_REGISTERS: usize = 8;
const NUM_RAM_WORDS: usize = 0x10000;

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

impl DCPU16 {
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
    fn it_works() {
        assert!(true)
    }
}
