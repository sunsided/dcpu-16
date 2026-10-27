//! Turns assembler [`Token`]s into [`MetaInstruction`]s, one source line at a time.

use super::error::{AssembleError, AssembleErrorKind, Position};
use super::lexer::{tokenize, Token, TokenKind};
use super::{BasicOperationName, Instruction, MetaInstruction, NonBasicOperationName, Value};
use crate::instruction_argument::{InstructionArgument, SpecialRegister, StackOperation};
use crate::Register;
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

    for line in tokens.split(|t| t.kind == TokenKind::Newline) {
        let mut parser = LineParser {
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

        if let Some(instruction) = parser.instruction()? {
            meta_instructions.push(MetaInstruction::Instruction(instruction));
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

    /// Consumes and returns the next token.
    fn next(&mut self) -> Option<Token<'a>> {
        let token = self.tokens.get(self.index).copied();
        self.index += 1;
        token
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
                line: token.position.line,
                column: token.position.column + token.text.chars().count(),
            },
            None => Position { line: 1, column: 1 },
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

        if let Some(operation) = basic_operation(mnemonic) {
            let a_position = self.peek().map_or(position, |t| t.position);
            let a = match self.operand()? {
                Value::Static(argument) => argument,
                Value::LabelReference(name) => {
                    return Err(AssembleError::new(
                        AssembleErrorKind::LabelNotAllowed(name),
                        a_position,
                    ))
                }
            };
            self.expect(TokenKind::Comma, "`,`")?;
            let b = self.operand_or_label_reference()?;
            return Ok(Some(Instruction::Basic(operation, a, b)));
        }

        if let Some(operation) = nonbasic_operation(mnemonic) {
            let a = self.operand_or_label_reference()?;
            return Ok(Some(Instruction::NonBasic(operation, a)));
        }

        Err(AssembleError::new(
            AssembleErrorKind::UnknownInstruction(mnemonic.to_string()),
            position,
        ))
    }

    /// Parses an operand that may be a label reference, recording the reference for validation.
    fn operand_or_label_reference(&mut self) -> Result<Value, AssembleError> {
        let value = self.operand()?;
        if let Value::LabelReference(_) = value {
            let token = self.tokens[self.index - 1];
            if let TokenKind::Ident(name) = token.kind {
                self.label_references.push((name, token.position));
            }
        }
        Ok(value)
    }

    /// Parses an operand. Unknown names are returned as label references.
    fn operand(&mut self) -> Result<Value, AssembleError> {
        let token = match self.peek() {
            Some(&token) => token,
            None => return Err(self.expected("an operand")),
        };

        let argument = match token.kind {
            TokenKind::Number(word) => {
                self.index += 1;
                InstructionArgument::Literal(word)
            }
            TokenKind::Ident(name) => {
                self.index += 1;
                match named_argument(name) {
                    Some(argument) => argument,
                    None => return Ok(Value::LabelReference(name.to_string())),
                }
            }
            TokenKind::OpenBracket => {
                self.index += 1;
                self.address()?
            }
            _ => return Err(self.expected("an operand")),
        };
        Ok(Value::Static(argument))
    }

    /// Parses `[literal]`, `[register]` or `[literal+register]` after the opening bracket.
    fn address(&mut self) -> Result<InstructionArgument, AssembleError> {
        let argument = match self.next().map(|t| t.kind) {
            Some(TokenKind::Number(address)) => {
                if self.peek().map(|t| t.kind) == Some(TokenKind::Plus) {
                    self.index += 1;
                    let register = self.register()?;
                    InstructionArgument::AddressOffset { address, register }
                } else {
                    InstructionArgument::Address(address)
                }
            }
            Some(TokenKind::Ident(name)) => match register(name) {
                Some(register) => InstructionArgument::AddressFromRegister(register),
                None => {
                    self.index -= 1;
                    return Err(self.expected("a literal or register"));
                }
            },
            _ => {
                self.index -= 1;
                return Err(self.expected("a literal or register"));
            }
        };
        self.expect(TokenKind::CloseBracket, "`]`")?;
        Ok(argument)
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
