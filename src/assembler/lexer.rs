//! Splits assembler source into [`Token`]s.

use super::error::{AssembleError, AssembleErrorKind, Position};
use crate::Word;

/// A token of the assembler source.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(super) enum TokenKind<'a> {
    /// A label definition such as `:loop`, without the colon.
    Label(&'a str),
    /// A mnemonic, register name or label reference.
    Ident(&'a str),
    /// A decimal or hexadecimal literal.
    Number(Word),
    /// A string literal such as `"Hi\n"`, without the quotes and with escapes
    /// still in place; [`string_words`] decodes it.
    Str(&'a str),
    /// `,` between two operands.
    Comma,
    /// `+` in an offset address such as `[0x10+A]`.
    Plus,
    /// `[` opening an address.
    OpenBracket,
    /// `]` closing an address.
    CloseBracket,
    /// A line break, ending the current instruction.
    Newline,
}

/// A token together with its source text and position.
#[derive(Debug, Copy, Clone)]
pub(super) struct Token<'a> {
    /// What the token is.
    pub kind: TokenKind<'a>,
    /// The exact source text of the token.
    pub text: &'a str,
    /// Where the token starts.
    pub position: Position,
}

/// Splits the source into tokens. Whitespace and `;` comments are dropped;
/// line breaks are kept as [`TokenKind::Newline`] since instructions are line-based.
pub(super) fn tokenize(source: &str) -> Result<Vec<Token<'_>>, AssembleError> {
    let mut lexer = Lexer {
        source,
        offset: 0,
        position: Position { line: 1, column: 1 },
    };

    let mut tokens = Vec::new();
    while let Some(c) = lexer.peek() {
        let start = lexer.offset;
        let position = lexer.position;

        let kind = match c {
            // LF, CRLF and a lone CR each end a line; the CR of a CRLF pair is skipped
            // so the pair yields a single newline.
            '\r' if lexer.peek_second() == Some('\n') => {
                lexer.bump();
                continue;
            }
            '\n' | '\r' => {
                lexer.bump();
                TokenKind::Newline
            }
            ';' => {
                lexer.bump_while(|c| c != '\n' && c != '\r');
                continue;
            }
            c if c.is_whitespace() => {
                lexer.bump();
                continue;
            }
            '"' => TokenKind::Str(lexer.string()?),
            ',' => lexer.single(TokenKind::Comma),
            '+' => lexer.single(TokenKind::Plus),
            '[' => lexer.single(TokenKind::OpenBracket),
            ']' => lexer.single(TokenKind::CloseBracket),
            ':' => {
                lexer.bump();
                let name = lexer.bump_while(is_ident_char);
                if name.is_empty() {
                    return Err(AssembleError::new(
                        AssembleErrorKind::MissingLabelName,
                        position,
                    ));
                }
                TokenKind::Label(name)
            }
            c if c.is_ascii_digit() => {
                let text = lexer.bump_while(is_ident_char);
                TokenKind::Number(parse_number(text, position)?)
            }
            c if is_ident_char(c) => TokenKind::Ident(lexer.bump_while(is_ident_char)),
            c => {
                return Err(AssembleError::new(
                    AssembleErrorKind::UnexpectedCharacter(c),
                    position,
                ));
            }
        };

        tokens.push(Token {
            kind,
            text: &source[start..lexer.offset],
            position,
        });
    }

    Ok(tokens)
}

/// A cursor over the source that tracks line and column.
struct Lexer<'a> {
    /// The complete source.
    source: &'a str,
    /// The byte offset of the next character.
    offset: usize,
    /// The line and column of the next character.
    position: Position,
}

impl<'a> Lexer<'a> {
    /// Returns the next character without consuming it.
    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    /// Returns the character after the next one without consuming anything.
    fn peek_second(&self) -> Option<char> {
        self.source[self.offset..].chars().nth(1)
    }

    /// Consumes one character, updating line and column.
    fn bump(&mut self) {
        if let Some(c) = self.peek() {
            let ends_line = c == '\n' || (c == '\r' && self.peek_second() != Some('\n'));
            self.offset += c.len_utf8();
            if ends_line {
                self.position.line += 1;
                self.position.column = 1;
            } else {
                self.position.column += 1;
            }
        }
    }

    /// Consumes a single-character token and returns its kind.
    fn single(&mut self, kind: TokenKind<'a>) -> TokenKind<'a> {
        self.bump();
        kind
    }

    /// Consumes a string literal, validating its escapes and characters, and
    /// returns its contents between the quotes.
    fn string(&mut self) -> Result<&'a str, AssembleError> {
        let opening = self.position;
        self.bump();
        let start = self.offset;
        loop {
            let position = self.position;
            match self.peek() {
                Some('"') => break,
                None | Some('\n' | '\r') => {
                    return Err(AssembleError::new(
                        AssembleErrorKind::UnterminatedString,
                        opening,
                    ));
                }
                Some('\\') => {
                    self.bump();
                    match self.peek() {
                        Some(c) if unescape(c).is_some() => self.bump(),
                        // A line break or the end of input is reported as an unterminated string.
                        None | Some('\n' | '\r') => {}
                        Some(c) => {
                            return Err(AssembleError::new(
                                AssembleErrorKind::InvalidEscape(c),
                                position,
                            ));
                        }
                    }
                }
                Some(c) if Word::try_from(u32::from(c)).is_err() => {
                    return Err(AssembleError::new(
                        AssembleErrorKind::CharacterOutOfRange(c),
                        position,
                    ));
                }
                Some(_) => self.bump(),
            }
        }
        let contents = &self.source[start..self.offset];
        self.bump();
        Ok(contents)
    }

    /// Consumes characters while the predicate holds and returns them.
    fn bump_while(&mut self, predicate: impl Fn(char) -> bool) -> &'a str {
        let start = self.offset;
        while self.peek().is_some_and(&predicate) {
            self.bump();
        }
        &self.source[start..self.offset]
    }
}

/// Decodes the contents of a [`TokenKind::Str`] into one word per character.
pub(super) fn string_words(contents: &str) -> impl Iterator<Item = Word> + '_ {
    let mut chars = contents.chars();
    std::iter::from_fn(move || {
        let c = match chars.next()? {
            // The lexer only produces valid escapes.
            '\\' => chars.next().and_then(unescape)?,
            c => c,
        };
        // The lexer only accepts characters that fit into a word.
        Some(u32::from(c) as Word)
    })
}

/// Maps the character after a `\` in a string literal to the character it stands for.
fn unescape(c: char) -> Option<char> {
    Some(match c {
        '\\' => '\\',
        '"' => '"',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '0' => '\0',
        _ => return None,
    })
}

/// Whether `c` may appear in a mnemonic, register name, label or number.
fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Parses a decimal literal or a hexadecimal literal prefixed with `0x`.
fn parse_number(text: &str, position: Position) -> Result<Word, AssembleError> {
    let (digits, radix) = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(digits) => (digits, 16),
        None => (text, 10),
    };

    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return Err(AssembleError::new(
            AssembleErrorKind::InvalidNumber(text.to_string()),
            position,
        ));
    }

    Word::from_str_radix(digits, radix).map_err(|_| {
        AssembleError::new(
            AssembleErrorKind::NumberOutOfRange(text.to_string()),
            position,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind<'_>> {
        tokenize(source)
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn tokenizes_instruction_with_label_and_comment() {
        assert_eq!(
            kinds(":loop SET [0x2000+I], [A] ; copy\n"),
            vec![
                TokenKind::Label("loop"),
                TokenKind::Ident("SET"),
                TokenKind::OpenBracket,
                TokenKind::Number(0x2000),
                TokenKind::Plus,
                TokenKind::Ident("I"),
                TokenKind::CloseBracket,
                TokenKind::Comma,
                TokenKind::OpenBracket,
                TokenKind::Ident("A"),
                TokenKind::CloseBracket,
                TokenKind::Newline,
            ]
        );
    }

    #[test]
    fn tracks_positions() {
        let tokens = tokenize("SET A, 1\n  JSR x").unwrap();
        let jsr = tokens.iter().find(|t| t.text == "JSR").unwrap();
        assert_eq!(jsr.position, Position { line: 2, column: 3 });
    }

    #[test]
    fn parses_numbers() {
        assert_eq!(
            kinds("65535 0xFFFF 0x1f"),
            vec![
                TokenKind::Number(0xFFFF),
                TokenKind::Number(0xFFFF),
                TokenKind::Number(0x1F),
            ]
        );
    }

    #[test]
    fn rejects_bad_numbers() {
        let err = tokenize("65536").unwrap_err();
        assert_eq!(
            err.kind(),
            &AssembleErrorKind::NumberOutOfRange("65536".to_string())
        );

        let err = tokenize("0x10000").unwrap_err();
        assert_eq!(
            err.kind(),
            &AssembleErrorKind::NumberOutOfRange("0x10000".to_string())
        );

        for text in ["0x", "12ab", "0xg"] {
            let err = tokenize(text).unwrap_err();
            assert_eq!(
                err.kind(),
                &AssembleErrorKind::InvalidNumber(text.to_string())
            );
        }
    }

    #[test]
    fn rejects_unexpected_characters() {
        let err = tokenize("SET A, #1").unwrap_err();
        assert_eq!(err.kind(), &AssembleErrorKind::UnexpectedCharacter('#'));
        assert_eq!(err.position(), Position { line: 1, column: 8 });
    }

    #[test]
    fn rejects_empty_label() {
        let err = tokenize(": SET A, 1").unwrap_err();
        assert_eq!(err.kind(), &AssembleErrorKind::MissingLabelName);
    }
}
