use dcpu16::{Register, DCPU16};

fn main() {
    // Use the RUST_LOG environment variable to configure, e.g. RUST_LOG=dcpu16=trace
    tracing_subscriber::fmt::init();

    let program = [
        0x7c01, 0x0030, 0x7de1, 0x1000, 0x0020, 0x7803, 0x1000, 0xc00d, 0x7dc1, 0x001a, 0xa861,
        0x7c01, 0x2000, 0x2161, 0x2000, 0x8463, 0x806d, 0x7dc1, 0x000d, 0x9031, 0x7c10, 0x0018,
        0x7dc1, 0x001a, 0x9037, 0x61c1, 0x7dc1, 0x001a, /* 0x0000, 0x0000, 0x0000, 0x0000 */
    ];

    let mut cpu = DCPU16::new(&program);
    cpu.run();

    // The last instruction perform a crash loop by jumping to itself (SET PC, 0x001A).
    // The length of that operation is two words, hence the following assertion.
    assert_eq!(cpu.program_counter, (program.len() - 2) as u16);

    // Same check as above, but checking against the address given in the SET operand.
    assert_eq!(cpu.program_counter, 0x001A);

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
