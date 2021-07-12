use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "assemble.pest"]
pub struct AssembleParser;

fn main() {
    let source = r"
        ; Try some basic stuff
                      SET A, 0x30              ; 7c01 0030
                      SET [0x1000], 0x20       ; 7de1 1000 0020
                      SUB A, [0x1000]          ; 7803 1000
                      IFN A, 0x10              ; c00d
                         SET PC, crash         ; 7dc1 001a

        ; Do a loopy thing
                      SET I, 10                ; a861
                      SET A, 0x2000            ; 7c01 2000
        :loop         SET [0x2000+I], [A]      ; 2161 2000
                      SUB I, 1                 ; 8463
                      IFN I, 0                 ; 806d
                         SET PC, loop          ; 7dc1 000d

        ; Call a subroutine
                      SET X, 0x4               ; 9031
                      JSR testsub              ; 7c10 0018
                      SET PC, crash            ; 7dc1 001a

        :testsub      SHL X, 4                 ; 9037
                      SET PC, POP              ; 61c1

        ; Hang forever. X should now be 0x40 if everything went right.
        :crash        SET PC, crash            ; 7dc1 001a
    ";

    let mut program = AssembleParser::parse(Rule::program, source)
        .expect("unsuccessful parse");

    // Get the top-level program rule.
    let program = program.next().unwrap();

    for record in program.into_inner() {
        match record.as_rule() {
            Rule::comment => {}
            Rule::label => {
                let inner = record.into_inner();
                println!("{:?}", inner);
            }
            Rule::basic_instruction => {
                let mut inner = record.into_inner();
                let instruction = inner.next().unwrap().as_str();

                // value a
                let value_a = match inner.next().unwrap().as_rule() {
                    Rule::literal => "literal",
                    Rule::register =>"register",
                    Rule::address => "address",
                    Rule::address_with_offset => "address_with_offset",
                    Rule::special_register => "special_register",
                    Rule::stack_op => "stack_op",
                    _ => unreachable!()
                };

                // value b
                let value_b = match inner.next().unwrap().as_rule() {
                    Rule::literal => "literal",
                    Rule::register =>"register",
                    Rule::address => "address",
                    Rule::address_with_offset => "address_with_offset",
                    Rule::special_register => "special_register",
                    Rule::stack_op => "stack_op",
                    Rule::label_ref => "label_ref",
                    _ => unreachable!()
                };

                println!("{} {}, {}", instruction, value_a, value_b);
            }
            Rule::nonbasic_instruction => {
                let inner = record.into_inner();
                println!("{:?}", inner);
            }
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }
}
