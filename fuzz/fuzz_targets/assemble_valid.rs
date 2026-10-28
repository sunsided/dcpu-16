//! Generates syntactically valid programs and checks that they assemble.
//!
//! Every program built here follows the assembler grammar, so assembling must
//! succeed, every instruction must encode to one to three words, and every
//! `DAT` directive to exactly as many words as it lists.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use std::fmt::Write;

/// Names of the general-purpose registers.
const REGISTERS: [&str; 8] = ["A", "B", "C", "X", "Y", "Z", "I", "J"];
/// Names of the special registers and stack operations.
const SPECIAL: [&str; 6] = ["SP", "PC", "O", "PUSH", "PEEK", "POP"];
/// Mnemonics of the basic (two-operand) instructions.
const BASIC: [&str; 15] = [
    "SET", "ADD", "SUB", "MUL", "DIV", "MOD", "SHL", "SHR", "AND", "BOR", "XOR", "IFE", "IFN",
    "IFG", "IFB",
];

/// A program: a sequence of source lines.
#[derive(Arbitrary, Debug)]
struct Program {
    /// The lines of the program.
    lines: Vec<Line>,
}

/// One source line.
#[derive(Arbitrary, Debug)]
struct Line {
    /// Whether the line defines a new label.
    label: bool,
    /// The instruction on the line, if any.
    instruction: Option<Instruction>,
    /// Whether the line ends with a comment.
    comment: bool,
    /// Whitespace and line-ending style.
    style: Style,
}

/// An instruction with its operands.
#[derive(Arbitrary, Debug)]
enum Instruction {
    /// A basic instruction; `a` may not be a label reference.
    Basic {
        /// Index into [`BASIC`].
        operation: u8,
        /// The first operand.
        a: Operand,
        /// The second operand.
        b: Operand,
    },
    /// `JSR` with its operand.
    Jsr(Operand),
    /// `DAT` with at least one item.
    Data(Datum, Vec<Datum>),
}

/// An item of a `DAT` directive.
#[derive(Arbitrary, Debug)]
enum Datum {
    /// A number, written as hex if the flag is set.
    Number(u16, bool),
    /// A string literal; each byte selects a character or escape sequence.
    String(Vec<u8>),
    /// A reference to a defined label.
    Label(u8),
}

/// An operand in any of its source forms.
#[derive(Arbitrary, Debug)]
enum Operand {
    /// A general-purpose register.
    Register(u8),
    /// A special register or stack operation.
    Special(u8),
    /// A literal, written as hex if the flag is set.
    Literal(u16, bool),
    /// `[literal]`, written as hex if the flag is set.
    Address(u16, bool),
    /// `[register]`.
    AddressFromRegister(u8),
    /// `[literal+register]`, written as hex if the flag is set.
    AddressOffset(u16, u8, bool),
    /// `[register+literal]`, written as hex if the flag is set.
    RegisterOffset(u8, u16, bool),
    /// A reference to a defined label.
    Label(u8),
    /// `[label]`, `[label+register]` or `[register+label]`, depending on the
    /// optional register and the flag.
    LabelAddress(u8, Option<u8>, bool),
}

/// Formatting choices that must not change the result.
#[derive(Arbitrary, Debug)]
struct Style {
    /// Indentation before the line content.
    indent: u8,
    /// Whether to put spaces around `,` and `+`.
    spaced: bool,
    /// Whether the line ends with CRLF instead of LF.
    crlf: bool,
}

impl Operand {
    /// Renders the operand. Label references fall back to a literal if no label
    /// exists yet or if `allow_label` is false.
    fn render(&self, out: &mut String, labels: usize, allow_label: bool, spaced: bool) {
        let number = |value: u16, hex: bool| {
            if hex {
                format!("0x{value:x}")
            } else {
                value.to_string()
            }
        };
        let plus = if spaced { " + " } else { "+" };
        let register = |r: u8| REGISTERS[r as usize % REGISTERS.len()];

        match *self {
            Self::Register(r) => out.push_str(register(r)),
            Self::Special(s) => out.push_str(SPECIAL[s as usize % SPECIAL.len()]),
            Self::Literal(v, hex) => out.push_str(&number(v, hex)),
            Self::Address(v, hex) => {
                let _ = write!(out, "[{}]", number(v, hex));
            }
            Self::AddressFromRegister(r) => {
                let _ = write!(out, "[{}]", register(r));
            }
            Self::AddressOffset(v, r, hex) => {
                let _ = write!(out, "[{}{plus}{}]", number(v, hex), register(r));
            }
            Self::RegisterOffset(r, v, hex) => {
                let _ = write!(out, "[{}{plus}{}]", register(r), number(v, hex));
            }
            Self::Label(l) if allow_label && labels > 0 => {
                let _ = write!(out, "label{}", l as usize % labels);
            }
            Self::Label(l) => out.push_str(&number(l.into(), false)),
            Self::LabelAddress(l, _, _) if labels == 0 => {
                let _ = write!(out, "[{}]", number(l.into(), false));
            }
            Self::LabelAddress(l, None, _) => {
                let _ = write!(out, "[label{}]", l as usize % labels);
            }
            Self::LabelAddress(l, Some(r), register_first) => {
                let label = format!("label{}", l as usize % labels);
                if register_first {
                    let _ = write!(out, "[{}{plus}{label}]", register(r));
                } else {
                    let _ = write!(out, "[{label}{plus}{}]", register(r));
                }
            }
        }
    }
}

/// Characters and escape sequences a string literal is built from, each one word.
const STRING_PARTS: [&str; 12] = [
    "a", "Z", "0", " ", ",", ";", "\\\"", "\\\\", "\\n", "\\0", "€", "]",
];

impl Datum {
    /// Renders the item and returns the number of words it encodes to.
    /// Label references fall back to a number if no label exists.
    fn render(&self, out: &mut String, labels: usize) -> usize {
        match self {
            Self::Number(v, true) => {
                let _ = write!(out, "0x{v:x}");
                1
            }
            Self::Number(v, false) => {
                let _ = write!(out, "{v}");
                1
            }
            Self::String(parts) => {
                out.push('"');
                for part in parts {
                    out.push_str(STRING_PARTS[*part as usize % STRING_PARTS.len()]);
                }
                out.push('"');
                parts.len()
            }
            Self::Label(l) if labels > 0 => {
                let _ = write!(out, "label{}", *l as usize % labels);
                1
            }
            Self::Label(l) => {
                let _ = write!(out, "{l}");
                1
            }
        }
    }
}

fuzz_target!(|program: Program| {
    // Labels referenced before their definition are fine, but every reference
    // must name a label that is defined somewhere, so count them up front.
    let labels = program.lines.iter().filter(|line| line.label).count();

    let mut source = String::new();
    let mut defined = 0;
    let mut instructions = 0;
    let mut data_words = 0;
    for line in &program.lines {
        let style = &line.style;
        let comma = if style.spaced { " , " } else { ", " };
        source.push_str(&" ".repeat(style.indent as usize % 9));

        if line.label {
            let _ = write!(source, ":label{defined} ");
            defined += 1;
        }

        match &line.instruction {
            Some(Instruction::Basic { operation, a, b }) => {
                source.push_str(BASIC[*operation as usize % BASIC.len()]);
                source.push(' ');
                a.render(&mut source, labels, false, style.spaced);
                source.push_str(comma);
                b.render(&mut source, labels, true, style.spaced);
                instructions += 1;
            }
            Some(Instruction::Jsr(a)) => {
                source.push_str("JSR ");
                a.render(&mut source, labels, true, style.spaced);
                instructions += 1;
            }
            Some(Instruction::Data(first, rest)) => {
                source.push_str("DAT ");
                data_words += first.render(&mut source, labels);
                for datum in rest {
                    source.push_str(comma);
                    data_words += datum.render(&mut source, labels);
                }
            }
            None => {}
        }

        if line.comment {
            source.push_str(" ; comment");
        }
        source.push_str(if style.crlf { "\r\n" } else { "\n" });
    }

    let words = match dcpu16::assemble(&source) {
        Ok(words) => words,
        Err(error) => panic!("valid program rejected: {error}\n{source}"),
    };
    assert!(
        (instructions + data_words..=3 * instructions + data_words).contains(&words.len()),
        "{instructions} instructions and {data_words} data words encoded to {} words\n{source}",
        words.len()
    );
});
