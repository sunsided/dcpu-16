//! Instruction semantics checked against the DCPU-16 1.1 specification in `docs/specification.txt`.
#![cfg(feature = "assembler")]

use dcpu16::{DCPU16, Register, assemble};

/// Assembles and runs `source` until it ends, then hands the CPU to `check`.
fn run<R>(source: &str, check: impl FnOnce(&DCPU16) -> R) -> R {
    let program = assemble(source).unwrap();
    let mut cpu = DCPU16::new(&program);
    cpu.run();
    check(&cpu)
}

/// Runs `source` and returns the value of register `A` and the overflow register.
fn a_and_overflow(source: &str) -> (u16, u16) {
    run(source, |cpu| (cpu.register(Register::A), cpu.overflow))
}

#[test]
fn set_assigns_short_and_long_literals() {
    run("SET A, 0x1f\nSET B, 0x20\nSET C, 0xffff", |cpu| {
        assert_eq!(cpu.register(Register::A), 0x1f);
        assert_eq!(cpu.register(Register::B), 0x20);
        assert_eq!(cpu.register(Register::C), 0xffff);
    });
}

#[test]
fn add_sets_overflow_on_carry() {
    assert_eq!(a_and_overflow("SET A, 0xffff\nADD A, 2"), (0x0001, 0x0001));
    assert_eq!(a_and_overflow("SET A, 1\nADD A, 2"), (0x0003, 0x0000));
}

#[test]
fn sub_sets_overflow_on_borrow() {
    assert_eq!(a_and_overflow("SUB A, 1"), (0xffff, 0xffff));
    assert_eq!(a_and_overflow("SET A, 3\nSUB A, 1"), (0x0002, 0x0000));
}

#[test]
fn mul_puts_high_word_into_overflow() {
    assert_eq!(
        a_and_overflow("SET A, 0x1000\nMUL A, 0x20"),
        (0x0000, 0x0002)
    );
}

#[test]
fn div_puts_fraction_into_overflow_and_handles_zero() {
    assert_eq!(a_and_overflow("SET A, 7\nDIV A, 2"), (0x0003, 0x8000));
    assert_eq!(a_and_overflow("SET A, 7\nSET O, 5\nDIV A, 0"), (0, 0));
}

#[test]
fn mod_handles_zero() {
    assert_eq!(a_and_overflow("SET A, 7\nMOD A, 3").0, 1);
    assert_eq!(a_and_overflow("SET A, 7\nMOD A, 0").0, 0);
}

#[test]
fn shl_puts_shifted_out_bits_into_overflow() {
    assert_eq!(a_and_overflow("SET A, 0x1001\nSHL A, 4"), (0x0010, 0x0001));
    assert_eq!(a_and_overflow("SET A, 1\nSHL A, 16"), (0x0000, 0x0001));
    assert_eq!(a_and_overflow("SET A, 0x8001\nSHL A, 17"), (0x0000, 0x0002));
    assert_eq!(a_and_overflow("SET A, 1\nSHL A, 40"), (0x0000, 0x0000));
}

#[test]
fn shr_puts_shifted_out_bits_into_overflow() {
    assert_eq!(a_and_overflow("SET A, 0x0010\nSHR A, 5"), (0x0000, 0x8000));
    assert_eq!(a_and_overflow("SET A, 0x1234\nSHR A, 16"), (0x0000, 0x1234));
    assert_eq!(a_and_overflow("SET A, 0x1234\nSHR A, 40"), (0x0000, 0x0000));
}

#[test]
fn bitwise_operations() {
    assert_eq!(a_and_overflow("SET A, 0xc\nAND A, 0xa").0, 0x8);
    assert_eq!(a_and_overflow("SET A, 0xc\nBOR A, 0xa").0, 0xe);
    assert_eq!(a_and_overflow("SET A, 0xc\nXOR A, 0xa").0, 0x6);
}

/// Runs `condition` followed by `SET B, 1` and reports whether `B` was set.
fn performs_next(condition: &str) -> bool {
    let source = format!("SET A, 1\n{condition}\nSET B, 1");
    run(&source, |cpu| cpu.register(Register::B) == 1)
}

#[test]
fn conditionals_perform_next_instruction_only_if_true() {
    assert!(performs_next("IFE A, 1"));
    assert!(!performs_next("IFE A, 2"));
    assert!(performs_next("IFN A, 2"));
    assert!(!performs_next("IFN A, 1"));
    assert!(performs_next("IFG A, 0"));
    assert!(!performs_next("IFG A, 1"));
    assert!(performs_next("IFB A, 3"));
    assert!(!performs_next("IFB A, 2"));
}

#[test]
fn skipping_steps_over_all_words_of_the_next_instruction() {
    run("IFE A, 1\nSET [0x1000], 0x1234\nSET B, 1", |cpu| {
        assert_eq!(cpu.ram()[0x1000], 0);
        assert_eq!(cpu.register(Register::B), 1);
    });
}

#[test]
fn skipped_instruction_has_no_side_effects() {
    run("IFN A, A\nSET PUSH, 1\nSET B, 1", |cpu| {
        assert_eq!(cpu.stack_pointer, 0);
        assert_eq!(cpu.register(Register::B), 1);
    });
}

#[test]
fn stack_starts_at_the_last_ram_word() {
    run("SET PUSH, 10", |cpu| {
        assert_eq!(cpu.stack_pointer, 0xffff);
        assert_eq!(cpu.ram()[0xffff], 10);
    });
}

#[test]
fn push_and_pop_are_balanced() {
    run("SET PUSH, 1\nSET PUSH, 2\nSET A, POP\nSET B, POP", |cpu| {
        assert_eq!(cpu.register(Register::A), 2);
        assert_eq!(cpu.register(Register::B), 1);
        assert_eq!(cpu.stack_pointer, 0);
    });
}

#[test]
fn peek_reads_the_top_without_popping() {
    run("SET PUSH, 7\nSET A, PEEK", |cpu| {
        assert_eq!(cpu.register(Register::A), 7);
        assert_eq!(cpu.stack_pointer, 0xffff);
    });
}

#[test]
fn push_as_read_write_operand_moves_sp_once() {
    run("ADD PUSH, 3", |cpu| {
        assert_eq!(cpu.stack_pointer, 0xffff);
        assert_eq!(cpu.ram()[0xffff], 3);
    });
}

#[test]
fn pop_on_empty_stack_wraps() {
    run("SET A, POP\nSET B, 1", |cpu| {
        assert_eq!(cpu.stack_pointer, 1);
        assert_eq!(cpu.register(Register::B), 1);
    });
}

#[test]
fn jsr_pushes_return_address_and_set_pc_pop_returns() {
    run(
        "JSR sub\nSET C, 1\nSET PC, end\n:sub SET B, 1\nSET PC, POP\n:end SET PC, end",
        |cpu| {
            assert_eq!(cpu.register(Register::B), 1);
            assert_eq!(cpu.register(Register::C), 1);
            assert_eq!(cpu.stack_pointer, 0);
        },
    );
}

#[test]
fn register_addressing() {
    run("SET I, 0x100\nSET [I], 9\nSET A, [I]", |cpu| {
        assert_eq!(cpu.ram()[0x100], 9);
        assert_eq!(cpu.register(Register::A), 9);
    });
}

#[test]
fn offset_addressing_wraps_around_the_address_space() {
    run("SET I, 2\nSET [0xffff+I], 7\nSET A, [0xffff+I]", |cpu| {
        assert_eq!(cpu.ram()[0x0001], 7);
        assert_eq!(cpu.register(Register::A), 7);
    });
}

#[test]
fn special_registers_are_readable_and_writable() {
    run(
        "SET SP, 0x1000\nSET PUSH, 3\nSET O, 5\nSET A, O\nSET B, SP",
        |cpu| {
            assert_eq!(cpu.ram()[0x0fff], 3);
            assert_eq!(cpu.register(Register::A), 5);
            assert_eq!(cpu.register(Register::B), 0x0fff);
        },
    );
}

#[test]
fn assigning_to_a_literal_fails_silently() {
    run("SET 0x30, 1\nADD 5, 1\nSET A, 2", |cpu| {
        assert_eq!(cpu.register(Register::A), 2);
    });
}

#[test]
fn program_is_loaded_into_ram() {
    let program = assemble("SET A, [0]\nSET B, [1]").unwrap();
    let mut cpu = DCPU16::new(&program);
    cpu.run();
    assert_eq!(cpu.register(Register::A), program[0]);
    assert_eq!(cpu.register(Register::B), program[1]);
    assert_eq!(cpu.ram()[..program.len()], program[..]);
}

#[test]
fn program_can_modify_its_own_code() {
    // Overwrites `SET C, 2` at address 4 with `SET C, 3` (0x8c21) before it runs.
    run("SET [4], 0x8c21\nSET B, 1\nSET C, 2", |cpu| {
        assert_eq!(cpu.register(Register::C), 3);
    });
}

#[test]
fn ram_is_writable_through_ram_mut() {
    let program = [0x0000];
    let mut cpu = DCPU16::new(&program);
    cpu.ram_mut()[3] = 4;
    assert_eq!(cpu.ram()[3], 4);
}

#[test]
fn reserved_opcode_halts_without_panicking() {
    // 0x0000 is the reserved non-basic opcode; `SET A, 1` after it must not run.
    let program = [0x0000, 0x8401];
    let mut cpu = DCPU16::new(&program);
    assert!(!cpu.step());
    assert_eq!(cpu.register(Register::A), 0);
}

#[test]
fn truncated_instruction_reads_zero_without_panicking() {
    // `SET A, <next word>` with the next word missing.
    let program = [0x7c01];
    let mut cpu = DCPU16::new(&program);
    cpu.run();
    assert_eq!(cpu.register(Register::A), 0);
}

#[test]
fn skipping_a_reserved_opcode_continues_execution() {
    // `IFE A, 1` fails, so the reserved opcode is skipped and `SET B, 1` runs.
    let program = [0x840c, 0x0000, 0x8411];
    let mut cpu = DCPU16::new(&program);
    cpu.run();
    assert_eq!(cpu.register(Register::B), 1);
}

/// Runs `source` and returns the number of cycles it took.
fn cycles(source: &str) -> u64 {
    run(source, |cpu| cpu.cycles())
}

#[test]
fn counts_cycles_per_operation() {
    assert_eq!(cycles("SET A, 1"), 1);
    assert_eq!(cycles("AND A, 1\nBOR A, 1\nXOR A, 1"), 3);
    assert_eq!(
        cycles("ADD A, 1\nSUB A, 1\nMUL A, 1\nSHL A, 1\nSHR A, 1"),
        10
    );
    assert_eq!(cycles("DIV A, 1\nMOD A, 1"), 6);
    assert_eq!(cycles("JSR 3\nSET A, 1\nSET A, 1\nSET A, 1"), 3);
}

#[test]
fn counts_a_cycle_per_extra_word() {
    assert_eq!(cycles("SET A, 0x20"), 2);
    assert_eq!(cycles("SET [0x1000], 0x20"), 3);
    assert_eq!(cycles("SET [0x10+A], [B]"), 2);
    assert_eq!(cycles("DIV [0x1000], 0x20"), 5);
}

#[test]
fn failed_test_takes_an_extra_cycle_and_skipping_is_free() {
    assert_eq!(cycles("IFE A, 0\nSET B, 1"), 3);
    assert_eq!(cycles("IFE A, 1\nSET [0x1000], 0x20\nSET B, 1"), 4);
}

#[test]
fn run_for_stops_after_the_given_cycles() {
    // Each iteration takes 3 cycles: ADD takes 2, the short jump 1.
    let program = assemble(":loop ADD A, 1\nSET PC, loop").unwrap();
    let mut cpu = DCPU16::new(&program);
    assert!(cpu.run_for(10));
    assert_eq!(cpu.cycles(), 11);
    assert_eq!(cpu.register(Register::A), 4);

    assert!(cpu.run_for(3));
    assert_eq!(cpu.register(Register::A), 5);
}

#[test]
fn run_for_reports_when_execution_stops() {
    let program = assemble("SET A, 1\n:end SET PC, end").unwrap();
    let mut cpu = DCPU16::new(&program);
    assert!(!cpu.run_for(100));
    assert_eq!(cpu.cycles(), 2);
}

#[test]
fn disassembles_without_executing() {
    let mut program = assemble("SET [0x1000], 0x20\nSET PUSH, [A]\nJSR 0x10").unwrap();
    program.push(0x0000);
    let cpu = DCPU16::new(&program);

    let mut address = 0;
    let mut lines = Vec::new();
    while usize::from(address) < program.len() {
        let disassembly = cpu.disassemble(address);
        lines.push(disassembly.text);
        address += disassembly.length;
    }
    assert_eq!(
        lines,
        [
            "SET [0x1000], 0x20",
            "SET PUSH, [A]",
            "JSR 0x10",
            "DAT 0x0000"
        ]
    );
    assert_eq!(cpu.program_counter, 0);
    assert_eq!(cpu.stack_pointer, 0);
}

#[test]
fn describes_instructions_in_plain_english() {
    let source = "SET [X], 0xf0df\nSET A, [0x10+I]\nSET PUSH, A\nSET B, POP\nSET PC, 0x40\n\
        SET PC, POP\nADD PEEK, 1\nIFE A, 0x10\nIFB A, 1\nJSR 0x18\nMOD A, B";
    let program = assemble(source).unwrap();
    let cpu = DCPU16::new(&program);

    let mut address = 0;
    let mut descriptions = Vec::new();
    while usize::from(address) < program.len() {
        let disassembly = cpu.disassemble(address);
        descriptions.push(disassembly.description);
        address += disassembly.length;
    }
    assert_eq!(
        descriptions,
        [
            "store 0xF0DF in RAM[X]",
            "store RAM[0x10+I] in A",
            "push A",
            "pop into B",
            "jump to 0x40",
            "return from subroutine",
            "add 0x01 to RAM[SP]",
            "run next if A == 0x10",
            "run next if A & 0x01 != 0",
            "call subroutine at 0x18",
            "set A to A modulo B",
        ]
    );
}
