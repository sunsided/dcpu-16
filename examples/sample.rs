use dcpu16::{Register, DCPU16};

fn main() {
    // Configure based on RUST_LOG env var.
    tracing_subscriber::fmt::init();

    let program: [u16; 32] = [
        0x7c01, 0x0030, 0x7de1, 0x1000, 0x0020, 0x7803, 0x1000, 0xc00d, 0x7dc1, 0x001a, 0xa861,
        0x7c01, 0x2000, 0x2161, 0x2000, 0x8463, 0x806d, 0x7dc1, 0x000d, 0x9031, 0x7c10, 0x0018,
        0x7dc1, 0x001a, 0x9037, 0x61c1, 0x7dc1, 0x001a, 0x0000, 0x0000, 0x0000, 0x0000,
    ];

    let mut cpu = DCPU16::new(&program);
    loop {
        if !cpu.step() {
            break;
        }
    }

    assert_eq!(cpu.program_counter, 0x001A);
    assert_eq!(cpu.register(Register::A), 0x2000);
    assert_eq!(cpu.register(Register::X), 0x40);
}
