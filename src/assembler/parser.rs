//! Turns assembler [`Token`]s into [`MetaInstruction`]s, one source line at a time.

use super::error::{AssembleError, AssembleErrorKind, Position};
use super::lexer::{Token, TokenKind, string_words, tokenize};
use super::{
    BasicOperationName, DataWord, Instruction, MetaInstruction, NonBasicOperationName, Operand,
};
use crate::instruction_argument::{InstructionArgument, SpecialRegister, StackOperation};
use crate::{Register, Word};
use std::collections::HashSet;

/// Parses the source and generates a stream of [`MetaInstruction`] instances.
///
/// Every line holds an optional label definition, followed by an optional instruction.
/// All label references are checked against the defined labels.
pub(super) fn parse(source: &str) -> Result<Vec<MetaInstruction>, AssembleError> {
    let tokens = tokenize(source)?;

    let mut meta_instructions = Vec::new();
    let mut defined_labels = HashSet::new();
    let mut label_references = Vec::new();

    // Every line break yields exactly one newline token, so the n-th slice is line n + 1.
    for (index, line) in tokens.split(|t| t.kind == TokenKind::Newline).enumerate() {
        let mut parser = LineParser {
            line: index + 1,
            tokens: line,
            index: 0,
            label_references: &mut label_references,
        };

        if let Some((name, position)) = parser.label() {
            if !defined_labels.insert(name) {
                return Err(AssembleError::new(
                    AssembleErrorKind::DuplicateLabel(name.to_string()),
                    position,
                ));
            }
            meta_instructions.push(MetaInstruction::Label(name.to_string()));
        }

        let position = parser.peek().map(|token| token.position);
        if let Some(instruction) = parser.instruction()? {
            // An instruction always starts with its mnemonic token.
            let position = position.expect("instruction without tokens");
            meta_instructions.push(MetaInstruction::Instruction(instruction, position));
        }

        parser.end_of_line()?;
    }

    for (name, position) in label_references {
        if !defined_labels.contains(name) {
            return Err(AssembleError::new(
                AssembleErrorKind::UndefinedLabel(name.to_string()),
                position,
            ));
        }
    }

    Ok(meta_instructions)
}

/// Parses the tokens of a single line.
struct LineParser<'t, 'a> {
    /// The line number, starting at 1.
    line: usize,
    /// The tokens of the line, without the trailing newline.
    tokens: &'t [Token<'a>],
    /// The index of the next token.
    index: usize,
    /// Label references seen so far, checked once all labels are known.
    label_references: &'t mut Vec<(&'a str, Position)>,
}

impl<'t, 'a> LineParser<'t, 'a> {
    /// Returns the next token without consuming it.
    fn peek(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.index)
    }

    /// Builds an error describing the current token (or the end of the line).
    fn expected(&self, expected: &'static str) -> AssembleError {
        let (found, position) = match self.peek() {
            Some(token) => (format!("`{}`", token.text), token.position),
            None => ("end of line".to_string(), self.end_position()),
        };
        AssembleError::new(AssembleErrorKind::Expected { expected, found }, position)
    }

    /// The position just after the last token of the line.
    fn end_position(&self) -> Position {
        match self.tokens.last() {
            Some(token) => Position {
                line: self.line,
                column: token.position.column + token.text.chars().count(),
            },
            None => Position {
                line: self.line,
                column: 1,
            },
        }
    }

    /// Consumes the next token if it is of `kind`, otherwise reports what was `expected`.
    fn expect(&mut self, kind: TokenKind<'_>, expected: &'static str) -> Result<(), AssembleError> {
        match self.peek() {
            Some(token) if token.kind == kind => {
                self.index += 1;
                Ok(())
            }
            _ => Err(self.expected(expected)),
        }
    }

    /// Consumes a leading label definition, if any.
    fn label(&mut self) -> Option<(&'a str, Position)> {
        match self.peek() {
            Some(&Token {
                kind: TokenKind::Label(name),
                position,
                ..
            }) => {
                self.index += 1;
                Some((name, position))
            }
            _ => None,
        }
    }

    /// Parses an instruction with its operands; returns `None` on an empty line.
    fn instruction(&mut self) -> Result<Option<Instruction>, AssembleError> {
        let (mnemonic, position) = match self.peek() {
            Some(&Token {
                kind: TokenKind::Ident(mnemonic),
                position,
                ..
            }) => (mnemonic, position),
            None => return Ok(None),
            Some(_) => return Err(self.expected("an instruction")),
        };
        self.index += 1;

        if mnemonic == "DAT" {
            return Ok(Some(Instruction::Data(self.data()?)));
        }

        if let Some(operation) = basic_operation(mnemonic) {
            let a_position = self.peek().map_or(position, |t| t.position);
            let a = match self.operand()? {
                Operand::Label(name) => {
                    return Err(AssembleError::new(
                        AssembleErrorKind::LabelNotAllowed(name),
                        a_position,
                    ));
                }
                a => a,
            };
            self.expect(TokenKind::Comma, "`,`")?;
            let b = self.operand()?;
            return Ok(Some(Instruction::Basic(operation, a, b)));
        }

        if let Some(operation) = nonbasic_operation(mnemonic) {
            let a = self.operand()?;
            return Ok(Some(Instruction::NonBasic(operation, a)));
        }

        Err(AssembleError::new(
            AssembleErrorKind::UnknownInstruction(mnemonic.to_string()),
            position,
        ))
    }

    /// Parses the comma-separated numbers, strings and label references of a `DAT` directive.
    fn data(&mut self) -> Result<Vec<DataWord>, AssembleError> {
        let mut words = Vec::new();
        loop {
            match self.peek().map(|t| t.kind) {
                Some(TokenKind::Number(word)) => {
                    self.index += 1;
                    words.push(DataWord::Literal(word));
                }
                Some(TokenKind::Str(contents)) => {
                    self.index += 1;
                    words.extend(string_words(contents).map(DataWord::Literal));
                }
                Some(TokenKind::Ident(name)) if named_argument(name).is_none() => {
                    words.push(DataWord::Label(self.label_reference()));
                }
                _ => return Err(self.expected("a number, string or label")),
            }

            if self.peek().map(|t| t.kind) != Some(TokenKind::Comma) {
                return Ok(words);
            }
            self.index += 1;
        }
    }

    /// Consumes a label reference, recording it for validation, and returns its name.
    ///
    /// The next token must be an identifier.
    fn label_reference(&mut self) -> String {
        let token = self.tokens[self.index];
        self.index += 1;
        let TokenKind::Ident(name) = token.kind else {
            unreachable!("label reference without an identifier");
        };
        self.label_references.push((name, token.position));
        name.to_string()
    }

    /// Parses an operand. Names other than registers, special registers and stack
    /// operations are label references.
    fn operand(&mut self) -> Result<Operand, AssembleError> {
        let token = match self.peek() {
            Some(&token) => token,
            None => return Err(self.expected("an operand")),
        };

        let argument = match token.kind {
            TokenKind::Number(word) => {
                self.index += 1;
                InstructionArgument::Literal(word)
            }
            TokenKind::Ident(name) => match named_argument(name) {
                Some(argument) => {
                    self.index += 1;
                    argument
                }
                None => return Ok(Operand::Label(self.label_reference())),
            },
            TokenKind::OpenBracket => {
                self.index += 1;
                return self.address();
            }
            _ => return Err(self.expected("an operand")),
        };
        Ok(Operand::Static(argument))
    }

    /// Parses the address after the opening bracket: a literal, register or label,
    /// or a register added to a literal or label in either order.
    fn address(&mut self) -> Result<Operand, AssembleError> {
        let operand = match self.address_term()? {
            AddressTerm::Literal(address) => match self.offset_register()? {
                Some(register) => {
                    Operand::Static(InstructionArgument::AddressOffset { address, register })
                }
                None => Operand::Static(InstructionArgument::Address(address)),
            },
            AddressTerm::Label(label) => Operand::LabelAddress {
                label,
                register: self.offset_register()?,
            },
            AddressTerm::Register(register) => {
                if self.peek().map(|t| t.kind) == Some(TokenKind::Plus) {
                    self.index += 1;
                    match self.address_term()? {
                        AddressTerm::Literal(address) => {
                            Operand::Static(InstructionArgument::AddressOffset {
                                address,
                                register,
                            })
                        }
                        AddressTerm::Label(label) => Operand::LabelAddress {
                            label,
                            register: Some(register),
                        },
                        AddressTerm::Register(_) => {
                            self.index -= 1;
                            return Err(self.expected("a literal or label"));
                        }
                    }
                } else {
                    Operand::Static(InstructionArgument::AddressFromRegister(register))
                }
            }
        };
        self.expect(TokenKind::CloseBracket, "`]`")?;
        Ok(operand)
    }

    /// Parses a literal, general-purpose register or label reference inside brackets.
    fn address_term(&mut self) -> Result<AddressTerm, AssembleError> {
        match self.peek().map(|t| t.kind) {
            Some(TokenKind::Number(address)) => {
                self.index += 1;
                Ok(AddressTerm::Literal(address))
            }
            Some(TokenKind::Ident(name)) => {
                if let Some(register) = register(name) {
                    self.index += 1;
                    Ok(AddressTerm::Register(register))
                } else if named_argument(name).is_none() {
                    Ok(AddressTerm::Label(self.label_reference()))
                } else {
                    Err(self.expected("a literal, register or label"))
                }
            }
            _ => Err(self.expected("a literal, register or label")),
        }
    }

    /// Consumes `+register` after a literal or label, if present.
    fn offset_register(&mut self) -> Result<Option<Register>, AssembleError> {
        if self.peek().map(|t| t.kind) != Some(TokenKind::Plus) {
            return Ok(None);
        }
        self.index += 1;
        self.register().map(Some)
    }

    /// Consumes a general-purpose register name.
    fn register(&mut self) -> Result<Register, AssembleError> {
        match self.peek().map(|t| t.kind) {
            Some(TokenKind::Ident(name)) => match register(name) {
                Some(register) => {
                    self.index += 1;
                    Ok(register)
                }
                None => Err(self.expected("a register")),
            },
            _ => Err(self.expected("a register")),
        }
    }

    /// Ensures that no tokens are left on the line.
    fn end_of_line(&self) -> Result<(), AssembleError> {
        match self.peek() {
            None => Ok(()),
            Some(_) => Err(self.expected("end of line")),
        }
    }
}

/// One side of an address expression inside brackets.
enum AddressTerm {
    /// A numeric address or offset.
    Literal(Word),
    /// A general-purpose register.
    Register(Register),
    /// A label reference, already recorded for validation.
    Label(String),
}

/// Resolves the mnemonic of a basic (two-operand) instruction.
fn basic_operation(mnemonic: &str) -> Option<BasicOperationName> {
    Some(match mnemonic {
        "SET" => BasicOperationName::Set,
        "ADD" => BasicOperationName::Add,
        "SUB" => BasicOperationName::Sub,
        "MUL" => BasicOperationName::Mul,
        "DIV" => BasicOperationName::Div,
        "MOD" => BasicOperationName::Mod,
        "SHL" => BasicOperationName::Shl,
        "SHR" => BasicOperationName::Shr,
        "AND" => BasicOperationName::And,
        "BOR" => BasicOperationName::Bor,
        "XOR" => BasicOperationName::Xor,
        "IFE" => BasicOperationName::Ife,
        "IFN" => BasicOperationName::Ifn,
        "IFG" => BasicOperationName::Ifg,
        "IFB" => BasicOperationName::Ifb,
        _ => return None,
    })
}

/// Resolves the mnemonic of a non-basic (one-operand) instruction.
fn nonbasic_operation(mnemonic: &str) -> Option<NonBasicOperationName> {
    match mnemonic {
        "JSR" => Some(NonBasicOperationName::Jsr),
        _ => None,
    }
}

/// Resolves the name of a general-purpose register.
fn register(name: &str) -> Option<Register> {
    Some(match name {
        "A" => Register::A,
        "B" => Register::B,
        "C" => Register::C,
        "X" => Register::X,
        "Y" => Register::Y,
        "Z" => Register::Z,
        "I" => Register::I,
        "J" => Register::J,
        _ => return None,
    })
}

/// Resolves register, special register and stack operation names.
fn named_argument(name: &str) -> Option<InstructionArgument> {
    if let Some(register) = register(name) {
        return Some(InstructionArgument::Register(register));
    }
    Some(match name {
        "SP" => InstructionArgument::SpecialRegister(SpecialRegister::StackPointer),
        "PC" => InstructionArgument::SpecialRegister(SpecialRegister::ProgramCounter),
        "O" => InstructionArgument::SpecialRegister(SpecialRegister::Overflow),
        "PUSH" => InstructionArgument::StackOperation(StackOperation::Push),
        "PEEK" => InstructionArgument::StackOperation(StackOperation::Peek),
        "POP" => InstructionArgument::StackOperation(StackOperation::Pop),
        _ => return None,
    })
}
