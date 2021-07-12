use crate::instruction_argument::InstructionArgument;
use crate::{Register, Word};
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "assemble.pest"]
struct AssembleParser;

/// Assembles the source code into an DCPU-16 program bytecode.
pub fn assemble<T>(source: T) -> Vec<Word>
where
    T: AsRef<str>,
{
    let mut program =
        AssembleParser::parse(Rule::program, source.as_ref()).expect("unsuccessful parse");

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
                let value_a = parse_value(inner.next().unwrap());

                // value b
                let value_b = parse_value(inner.next().unwrap());

                println!("{} {:?}, {:?}", instruction, value_a, value_b);
            }
            Rule::nonbasic_instruction => {
                let inner = record.into_inner();
                println!("{:?}", inner);
            }
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    unimplemented!()
}

fn parse_value(pair: Pair<Rule>) -> InstructionArgument {
    match pair.as_rule() {
        Rule::literal => InstructionArgument::Literal(0x1337),
        Rule::register => InstructionArgument::Register(Register::A),
        Rule::address => InstructionArgument::Address(0x1337),
        Rule::address_with_offset => InstructionArgument::AddressOffset {
            address: 0x1337,
            register: Register::A,
        },
        Rule::special_register => InstructionArgument::ProgramCounter,
        // This should have discrete variants for push/pop/peek
        Rule::stack_op => InstructionArgument::StackPointer,
        Rule::label_ref => unimplemented!(),
        _ => unreachable!(),
    }
}
