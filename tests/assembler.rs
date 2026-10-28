//! Expected outputs of the original syntax were captured from the previous pest-based
//! assembler; `DAT` and label references inside brackets are checked against the
//! equivalent numeric forms.
#![cfg(feature = "assembler")]

use dcpu16::{AssembleErrorKind, DCPU16, Position, assemble};

fn error(source: &str) -> (AssembleErrorKind, Position) {
    let error = assemble(source).unwrap_err();
    (error.kind().clone(), error.position())
}

fn at(line: usize, column: usize) -> Position {
    Position { line, column }
}

#[test]
fn assembles_specification_example() {
    let source = r"
        ; Try some basic stuff
                      SET A, 0x30              ; 7c01 0030
                      SET [0x1000], 0x20       ; 7de1 1000 0020
                      SUB A, [0x1000]          ; 7803 1000
                      IFN A, 0x10              ; c00d
                         SET PC, crash         ; d9c1*

        ; Do a loopy thing
                      SET I, 10                ; a861
                      SET A, 0x2000            ; 7c01 2000
        :loop         SET [0x2000+I], [A]      ; 2161 2000
                      SUB I, 1                 ; 8463
                      IFN I, 0                 ; 806d
                         SET PC, loop          ; b1c1*

        ; Call a subroutine
                      SET X, 0x4               ; 9031
                      JSR testsub              ; d010*
                      SET PC, crash            ; d9c1*

        :testsub      SHL X, 4                 ; 9037
                      SET PC, POP              ; 61c1

        ; Hang forever. X should now be 0x40 if everything went right.
        :crash        SET PC, crash            ; d9c1*
    ";

    assert_eq!(
        assemble(source).unwrap(),
        [
            0x7c01, 0x0030, 0x7de1, 0x1000, 0x0020, 0x7803, 0x1000, 0xc00d, 0xd9c1, 0xa861, 0x7c01,
            0x2000, 0x2161, 0x2000, 0x8463, 0x806d, 0xb1c1, 0x9031, 0xd010, 0xd9c1, 0x9037, 0x61c1,
            0xd9c1,
        ]
    );
}

#[test]
fn assembles_all_operand_forms() {
    let source = "SET A, B\nSET B, C\nSET X, Y\nSET Z, I\nSET J, A\nSET [A], [J]\n\
        SET [0x10+B], [5+C]\nSET [0x1000], [1234]\nSET PUSH, POP\nSET PEEK, SP\nSET PC, O\n\
        SET A, 0x1f\nSET A, 0x20\nSET A, 31\nSET A, 32\nSET A, 0xffff\nSET A, 65535\n\
        SET 0x30, 0x1\nSET 5, 0x40";

    assert_eq!(
        assemble(source).unwrap(),
        [
            0x0401, 0x0811, 0x1031, 0x1851, 0x0071, 0x3c81, 0x4911, 0x0010, 0x0005, 0x79e1, 0x1000,
            0x04d2, 0x61a1, 0x6d91, 0x75c1, 0xfc01, 0x7c01, 0x0020, 0xfc01, 0x7c01, 0x0020, 0x7c01,
            0xffff, 0x7c01, 0xffff, 0x85f1, 0x0030, 0x7e51, 0x0040,
        ]
    );
}

#[test]
fn assembles_all_opcodes() {
    let source = "SET A, 1\nADD A, 1\nSUB A, 1\nMUL A, 1\nDIV A, 1\nMOD A, 1\nSHL A, 1\n\
        SHR A, 1\nAND A, 1\nBOR A, 1\nXOR A, 1\nIFE A, 1\nIFN A, 1\nIFG A, 1\nIFB A, 1\n\
        JSR 1\nJSR 0x1234\nJSR [A]\nJSR POP";

    assert_eq!(
        assemble(source).unwrap(),
        [
            0x8401, 0x8402, 0x8403, 0x8404, 0x8405, 0x8406, 0x8407, 0x8408, 0x8409, 0x840a, 0x840b,
            0x840c, 0x840d, 0x840e, 0x840f, 0x8410, 0x7c10, 0x1234, 0x2010, 0x6010,
        ]
    );
}

#[test]
fn uses_long_form_for_distant_labels() {
    let mut source = String::from(":start SET PC, end\n");
    for _ in 0..20 {
        source.push_str("SET [0x1000], 0x1234\n");
    }
    source.push_str(":end SET PC, start\nJSR end\nSET A, end\n");

    let mut expected = vec![0x7dc1, 0x003e];
    for _ in 0..20 {
        expected.extend([0x7de1, 0x1000, 0x1234]);
    }
    expected.extend([0x81c1, 0x7c10, 0x003e, 0x7c01, 0x003e]);

    assert_eq!(assemble(source).unwrap(), expected);
}

#[test]
fn moves_labels_directly_behind_a_growing_instruction() {
    // `SET PC, far` only grows to two words once `far` is known to be distant,
    // which must also move `next`, the label right behind it, to address 2.
    let mut source = String::from("SET PC, far\n:next SET A, next\n");
    source.push_str(&"SET [0x1000], 0x1234\n".repeat(20));
    source.push_str(":far SET PC, far\n");

    let program = assemble(source).unwrap();
    assert_eq!(program[..3], [0x7dc1, 0x003f, 0x8801]);
}

#[test]
fn assembles_data() {
    assert_eq!(
        assemble("DAT 1, 0x20, 0xffff").unwrap(),
        [0x0001, 0x0020, 0xffff]
    );
    assert_eq!(
        assemble(r#"DAT "Hi", 0"#).unwrap(),
        [u16::from(b'H'), u16::from(b'i'), 0]
    );
    assert_eq!(
        assemble(r#"DAT "\"\\\n\r\t\0", """#).unwrap(),
        [0x22, 0x5c, 0x0a, 0x0d, 0x09, 0x00]
    );
    assert_eq!(assemble("DAT \"€\"").unwrap(), [0x20ac]);
}

#[test]
fn assembles_label_addresses_in_data() {
    assert_eq!(
        assemble(":a DAT b, a\n:b DAT \"xy\", b").unwrap(),
        [0x0002, 0x0000, 0x0078, 0x0079, 0x0002]
    );
}

#[test]
fn data_can_be_read_by_label() {
    let program = assemble(
        "SET A, [msg]\nSET B, [msg+I]\nSET I, 1\nSET C, [I+msg]\n:end SET PC, end\n:msg DAT 7, 9",
    )
    .unwrap();
    let mut cpu = DCPU16::new(&program);
    cpu.run();
    assert_eq!(cpu.register(dcpu16::Register::A), 7);
    assert_eq!(cpu.register(dcpu16::Register::B), 7);
    assert_eq!(cpu.register(dcpu16::Register::C), 9);
}

#[test]
fn assembles_label_addresses_like_numeric_addresses() {
    // The label sits at 0x0a; brackets always use the next-word form, even for small addresses.
    let padding = "SET A, 1\n".repeat(10);
    let labelled = format!("{padding}:x SET [x], [x+A]\nSET [B+x], x\nJSR [x]\nSET A, [x+J]");
    let numeric =
        format!("{padding}SET [0x0a], [0x0a+A]\nSET [0x0a+B], 0x0a\nJSR [0x0a]\nSET A, [0x0a+J]");
    assert_eq!(assemble(labelled).unwrap(), assemble(numeric).unwrap());
}

#[test]
fn accepts_register_before_literal_offset() {
    assert_eq!(
        assemble("SET [A+0x10], [B+5]").unwrap(),
        assemble("SET [0x10+A], [5+B]").unwrap()
    );
}

#[test]
fn accepts_blank_lines_comments_and_bare_labels() {
    let source = "\n\n  ; only comment\n:a\n:b SET A, b ; c\n:c\n   SET PC, a\n:d";
    assert_eq!(assemble(source).unwrap(), [0x8001, 0x81c1]);
}

#[test]
fn accepts_crlf_and_trailing_whitespace() {
    assert_eq!(
        assemble("SET A, 1   \r\nSET B, 2\t\r\n").unwrap(),
        [0x8401, 0x8811]
    );
}

#[test]
fn accepts_lone_carriage_return_line_endings() {
    assert_eq!(
        assemble("SET A, 1 ; one\rSET B, 2\r\rSET C, 3").unwrap(),
        [0x8401, 0x8811, 0x8c21]
    );
}

#[test]
fn reports_positions_after_lone_carriage_returns() {
    assert_eq!(
        error("SET A, 1\r\rMOV A, 1"),
        (
            AssembleErrorKind::UnknownInstruction("MOV".into()),
            at(3, 1)
        )
    );
}

#[test]
fn accepts_underscores_in_label_names() {
    assert_eq!(
        assemble(":my_label SET PC, my_label").unwrap(),
        assemble(":mylabel SET PC, mylabel").unwrap()
    );
}

#[test]
fn reports_end_of_line_on_the_offending_line() {
    assert_eq!(
        error("SET A, 1\n\nJSR"),
        (
            AssembleErrorKind::Expected {
                expected: "an operand",
                found: "end of line".into()
            },
            at(3, 4)
        )
    );
}

#[test]
fn labels_may_start_with_register_names() {
    // A greedy register match must not swallow the start of a label name.
    assert_eq!(
        assemble(":Crash SET PC, Crash").unwrap(),
        assemble(":crash SET PC, crash").unwrap()
    );
}

/// 21844 three-word instructions: 65532 words.
fn three_word_instructions() -> String {
    "SET [0x1000], 0x1234\n".repeat(21844)
}

#[test]
fn accepts_program_of_maximum_length() {
    let source = three_word_instructions() + "SET A, 1\nSET A, 1\n";
    let program = assemble(source).unwrap();
    assert_eq!(program.len(), 65534);

    // The emulator must accept everything the assembler produces.
    DCPU16::new(&program);
}

#[test]
fn reports_program_exceeding_maximum_length() {
    let source = three_word_instructions() + "SET A, 1\nSET A, 1\nSET A, 1\n";
    assert_eq!(
        error(&source),
        (
            AssembleErrorKind::ProgramTooLarge { max_words: 65534 },
            at(21847, 1)
        )
    );
}

#[test]
fn reports_program_outgrowing_maximum_length_through_label_references() {
    // Fits while both jumps are estimated as one word, but `end` ends up beyond
    // the inline literal range, so the first jump grows to two words.
    let source = format!(
        ":start SET PC, end\n{}:end SET PC, start\n",
        three_word_instructions()
    );
    assert_eq!(
        error(&source),
        (
            AssembleErrorKind::ProgramTooLarge { max_words: 65534 },
            at(21846, 6)
        )
    );
}

#[test]
fn reports_unknown_instruction() {
    assert_eq!(
        error("SET A, 1\n  MOV A, 1"),
        (
            AssembleErrorKind::UnknownInstruction("MOV".into()),
            at(2, 3)
        )
    );
}

#[test]
fn reports_undefined_label() {
    assert_eq!(
        error("SET PC, nowhere"),
        (
            AssembleErrorKind::UndefinedLabel("nowhere".into()),
            at(1, 9)
        )
    );
}

#[test]
fn reports_duplicate_label() {
    assert_eq!(
        error(":a SET A, 1\n:a SET A, 2"),
        (AssembleErrorKind::DuplicateLabel("a".into()), at(2, 1))
    );
}

#[test]
fn reports_label_as_first_operand() {
    assert_eq!(
        error(":a SET a, 1"),
        (AssembleErrorKind::LabelNotAllowed("a".into()), at(1, 8))
    );
}

#[test]
fn reports_out_of_range_literal() {
    assert_eq!(
        error("SET A, 99999"),
        (
            AssembleErrorKind::NumberOutOfRange("99999".into()),
            at(1, 8)
        )
    );
}

#[test]
fn reports_missing_operand() {
    assert_eq!(
        error("SET A,"),
        (
            AssembleErrorKind::Expected {
                expected: "an operand",
                found: "end of line".into()
            },
            at(1, 7)
        )
    );
}

#[test]
fn reports_missing_comma() {
    assert_eq!(
        error("SET A 1"),
        (
            AssembleErrorKind::Expected {
                expected: "`,`",
                found: "`1`".into()
            },
            at(1, 7)
        )
    );
}

#[test]
fn reports_unclosed_bracket() {
    assert_eq!(
        error("SET [A, 1"),
        (
            AssembleErrorKind::Expected {
                expected: "`]`",
                found: "`,`".into()
            },
            at(1, 7)
        )
    );
}

#[test]
fn reports_non_register_offset() {
    assert_eq!(
        error("SET [0x10+PC], 1"),
        (
            AssembleErrorKind::Expected {
                expected: "a register",
                found: "`PC`".into()
            },
            at(1, 11)
        )
    );
}

#[test]
fn reports_undefined_label_in_brackets_and_data() {
    assert_eq!(
        error("SET A, [B+nowhere]"),
        (
            AssembleErrorKind::UndefinedLabel("nowhere".into()),
            at(1, 11)
        )
    );
    assert_eq!(
        error("DAT 1, nowhere"),
        (
            AssembleErrorKind::UndefinedLabel("nowhere".into()),
            at(1, 8)
        )
    );
}

#[test]
fn reports_invalid_address_expressions() {
    for (source, expected, found, column) in [
        ("SET A, [PC]", "a literal, register or label", "`PC`", 9),
        ("SET A, []", "a literal, register or label", "`]`", 9),
        ("SET A, [A+B]", "a literal or label", "`B`", 11),
        ("SET A, [x+1]", "a register", "`1`", 11),
        ("SET A, [1+2]", "a register", "`2`", 11),
    ] {
        assert_eq!(
            error(&format!("{source}\n:x")),
            (
                AssembleErrorKind::Expected {
                    expected,
                    found: found.into()
                },
                at(1, column)
            ),
            "{source}"
        );
    }
}

#[test]
fn reports_invalid_data() {
    for (source, found, column) in [
        ("DAT", "end of line", 4),
        ("DAT 1,", "end of line", 7),
        ("DAT A", "`A`", 5),
        ("DAT [1]", "`[`", 5),
    ] {
        assert_eq!(
            error(source),
            (
                AssembleErrorKind::Expected {
                    expected: "a number, string or label",
                    found: found.into()
                },
                at(1, column)
            ),
            "{source}"
        );
    }
}

#[test]
fn reports_invalid_strings() {
    assert_eq!(
        error("DAT 1, \"abc\nDAT 2"),
        (AssembleErrorKind::UnterminatedString, at(1, 8))
    );
    assert_eq!(
        error("DAT \"abc\\\""),
        (AssembleErrorKind::UnterminatedString, at(1, 5))
    );
    assert_eq!(
        error("DAT \"a\\qb\""),
        (AssembleErrorKind::InvalidEscape('q'), at(1, 7))
    );
    assert_eq!(
        error("DAT \"a🦀\""),
        (AssembleErrorKind::CharacterOutOfRange('🦀'), at(1, 7))
    );
}

#[test]
fn reports_trailing_tokens() {
    assert_eq!(
        error("JSR 1, 2"),
        (
            AssembleErrorKind::Expected {
                expected: "end of line",
                found: "`,`".into()
            },
            at(1, 6)
        )
    );
}

#[test]
fn reports_two_labels_on_one_line() {
    assert_eq!(
        error(":a :b SET A, 1"),
        (
            AssembleErrorKind::Expected {
                expected: "an instruction",
                found: "`:b`".into()
            },
            at(1, 4)
        )
    );
}

#[test]
fn formats_errors_with_position() {
    let error = assemble("SET A, 1\nSET A, nowhere").unwrap_err();
    assert_eq!(
        error.to_string(),
        "line 2, column 8: label `nowhere` is not defined"
    );
}
