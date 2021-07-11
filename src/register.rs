use crate::Word;

/// Identifier for a CPU register.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Register {
    A = 0,
    B = 1,
    C = 2,
    X = 3,
    Y = 4,
    Z = 5,
    I = 6,
    J = 7,
}

impl From<Word> for Register {
    fn from(v: Word) -> Self {
        assert!(v <= Register::J as Word);
        match v {
            x if x == Register::A as Word => Register::A,
            x if x == Register::B as Word => Register::B,
            x if x == Register::C as Word => Register::C,
            x if x == Register::X as Word => Register::X,
            x if x == Register::Y as Word => Register::Y,
            x if x == Register::Z as Word => Register::Z,
            x if x == Register::I as Word => Register::I,
            x if x == Register::J as Word => Register::J,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_works() {
        assert_eq!(Register::from(0x00), Register::A);
        assert_eq!(Register::from(0x07), Register::J);
    }
}
