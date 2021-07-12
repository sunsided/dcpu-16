use dcpu16::{assemble, Register, DCPU16};

fn main() {
    // Use the RUST_LOG environment variable to configure, e.g. RUST_LOG=dcpu16=trace
    tracing_subscriber::fmt::init();

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

        ; trash only to mess with the labels
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a
        ;             SET PC, crash            ; 7dc1 001a

        :testsub      SHL X, 4                 ; 9037
                      SET PC, POP              ; 61c1

        ; Hang forever. X should now be 0x40 if everything went right.
        :crash        SET PC, crash            ; 7dc1 001a
    ";

    let program = assemble(&source);

    let mut cpu = DCPU16::new(program.as_slice());
    println!("{}", cpu.hexdump_program(8));

    cpu.run();

    // The last instruction perform a crash loop by jumping to itself (SET PC, 0x0016).
    // The length of that operation is one word, hence the following assertion.
    assert_eq!(cpu.program_counter, (program.len() - 1) as u16);

    // Some register tests.
    assert_eq!(cpu.register(Register::A), 0x2000);
    assert_eq!(cpu.register(Register::X), 0x40);

    // Some RAM value tests.
    let ram = cpu.ram();
    assert_eq!(ram[0x1000], 0x20);
    assert_eq!(ram[0x2000 + 0x0A], ram[0x2000]);

    // Print the RAM contents.
    println!("{}", cpu.hexdump_ram(32));
}
