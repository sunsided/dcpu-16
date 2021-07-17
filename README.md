# DCPU-16 Emulator and Assembler

An emulator for the DCPU-16 16-bit processor described for the 
[0x10<sup>c</sup>] video game.

The [DCPU-16 Specification](docs/specification.txt) is no longer available on the
original website (as is the entire website) but still can be obtained from the [Wayback Machine].

An implementation of a DCPU-16 assembler is also provided in this repo and is built by default through 
the crate's `assembler` feature.

---

Cycle counts are currently not emulated.

## Example usage

See [examples/sample.rs] for a commented example application. Here's a sneak peek:

```rust
use dcpu16::{Register, DCPU16};

fn main() {
    let program = [
        0x7c01, 0x0030, 0x7de1, 0x1000, 0x0020, 0x7803, 0x1000, 0xc00d, 0x7dc1, 0x001a, 0xa861,
        0x7c01, 0x2000, 0x2161, 0x2000, 0x8463, 0x806d, 0x7dc1, 0x000d, 0x9031, 0x7c10, 0x0018,
        0x7dc1, 0x001a, 0x9037, 0x61c1, 0x7dc1, 0x001a, 0x0000, 0x0000, 0x0000, 0x0000,
    ];

    let mut cpu = DCPU16::new(&program);

    // Use cpu.step() to step through each instruction.
    // cpu.run() executes until a crash loop is detected.
    cpu.run();

    assert_eq!(cpu.program_counter, 0x001A);
    assert_eq!(cpu.register(Register::A), 0x2000);
    assert_eq!(cpu.register(Register::X), 0x40);
    
    let ram = cpu.ram();
    assert_eq!(ram[0x1000], 0x20);
}
```

### Running the emulation example

The example program can be started with

```console
RUST_LOG=dcpu16=trace cargo run --example sample
```

It executes the program given in the [DCPU-16 Specification](docs/specification.txt):

```asm
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
```

The above program can be assembled into to the following bytecode:

```hexdump
0000: 7c01 0030 7de1 1000 0020 7803 1000 c00d
0008: 7dc1 001a a861 7c01 2000 2161 2000 8463
0010: 806d 7dc1 000d 9031 7c10 0018 7dc1 001a
0018: 9037 61c1 7dc1 001a 0000 0000 0000 0000
```

When executing the bytecode instructions, the program output looks like this:

```
 INFO dcpu16: Loaded 32 words of program data
DEBUG dcpu16: Registers: A=0000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0000 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0000: 7c01 0030 ; SET A, 0x30 ("A <- 0x30")
DEBUG dcpu16: Registers: A=0030 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0002 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0002: 7de1 1000 0020 ; SET [0x1000], 0x20 ("RAM[0x1000] <- 0x20")
DEBUG dcpu16: Registers: A=0030 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0005 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0005: 7803 1000 ; SUB A, [0x1000] ("A <- A - RAM[0x1000]")
DEBUG dcpu16: Registers: A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0007 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0007: c00d ; IFN A, 0x10 ("execute next instruction if A != 0x10")
DEBUG dcpu16: Registers: A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0008 SP=FFFF O=0000
DEBUG dcpu16: SKIP 0008: 7dc1 001a ; SET PC, 0x1A ("PC <- 0x1A")
DEBUG dcpu16: Registers: A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=000A SP=FFFF O=0000
DEBUG dcpu16: EXEC 000A: a861 ; SET I, 0x0A ("I <- 0x0A")
DEBUG dcpu16: Registers: A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=000A J=0000 PC⁎=000B SP=FFFF O=0000
DEBUG dcpu16: EXEC 000B: 7c01 2000 ; SET A, 0x2000 ("A <- 0x2000")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=000A J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: EXEC 000D: 2161 2000 ; SET [0x2000+I], [A] ("RAM[0x2000 + I] <- RAM[A]")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=000A J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: EXEC 000F: 8463 ; SUB I, 0x01 ("I <- I - 0x01")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0009 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0010: 806d ; IFN I, 0x00 ("execute next instruction if I != 0x00")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0009 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0011: 7dc1 000d ; SET PC, 0x0D ("PC <- 0x0D")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0009 J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: EXEC 000D: 2161 2000 ; SET [0x2000+I], [A] ("RAM[0x2000 + I] <- RAM[A]")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0009 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: EXEC 000F: 8463 ; SUB I, 0x01 ("I <- I - 0x01")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0008 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0010: 806d ; IFN I, 0x00 ("execute next instruction if I != 0x00")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0008 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0011: 7dc1 000d ; SET PC, 0x0D ("PC <- 0x0D")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0008 J=0000 PC⁎=000D SP=FFFF O=0000
... loop repeats ...
DEBUG dcpu16: EXEC 000D: 2161 2000 ; SET [0x2000+I], [A] ("RAM[0x2000 + I] <- RAM[A]")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0001 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: EXEC 000F: 8463 ; SUB I, 0x01 ("I <- I - 0x01")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0010: 806d ; IFN I, 0x00 ("execute next instruction if I != 0x00")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: SKIP 0011: 7dc1 000d ; SET PC, 0x0D ("PC <- 0x0D")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0013 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0013: 9031 ; SET X, 0x04 ("X <- 0x04")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0004 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0014 SP=FFFF O=0000
TRACE dcpu16::instruction: Decoding non-basic instruction 7C10, opcode 01, value 1F
DEBUG dcpu16: EXEC 0014: 7c10 0018 ; JSR 0x18 ("jump to subroutine at 0x18")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0004 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0018 SP=FFFE O=0000
DEBUG dcpu16: EXEC 0018: 9037 ; SHL X, 0x04 ("X <- X << 0x04")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0040 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0019 SP=FFFE O=0000
DEBUG dcpu16: EXEC 0019: 61c1 ; SET PC, POP ("PC <- pop value from stack")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0040 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0016 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0016: 7dc1 001a ; SET PC, 0x1A ("PC <- 0x1A")
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0040 Y=0000 Z=0000 I=0000 J=0000 PC⁎=001A SP=FFFF O=0000
DEBUG dcpu16: EXEC 001A: 7dc1 001a ; SET PC, 0x1A ("PC <- 0x1A")
 WARN dcpu16: Crash loop detected at PC=001A - terminating
```

After the execution, the `X` register contains the word `0040` as expected (by the specification).

### Running the assembly / compilation example

See [examples/assemble.rs] for a commented example application. It can be started with

```console
RUST_LOG=dcpu16=trace cargo run --example assemble
```

Here's some example code:

```rust
use dcpu16::{assemble, DCPU16};

fn main() {
    tracing_subscriber::fmt::init();

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

    let program = assemble(&source);

    let mut cpu = DCPU16::new(program.as_slice());
    println!("{}", cpu.hexdump_program(8));

    cpu.run();

    // The last instruction perform a crash loop by jumping to itself (SET PC, 0x0016).
    // The length of that operation is one word, hence the following assertion.
    assert_eq!(cpu.program_counter, (program.len() - 1) as u16);
}
```

Here's the hexdump of the generated program. Note that this code is shorter
than the provided original bytecode due to inlining of small constants;
for example, the `:crash SET PC, crash` instruction is rendered as `7dc1 001a`
in the original code but represented using `d9c1` in this output:

```hexdump
0000: 7C01 0030 7DE1 1000 0020 7803 1000 C00D
0008: D9C1 A861 7C01 2000 2161 2000 8463 806D
0010: B1C1 9031 D010 D9C1 9037 61C1 D9C1
```

This is the tracing output of the assembler:

```
TRACE dcpu16::assembler: instruction Basic(SET, Register(A), Static(Literal(48))), len = 2
TRACE dcpu16::assembler: instruction Basic(SET, Address(4096), Static(Literal(32))), len = 3
TRACE dcpu16::assembler: instruction Basic(SUB, Register(A), Static(Address(4096))), len = 2
TRACE dcpu16::assembler: instruction Basic(IFN, Register(A), Static(Literal(16))), len = 1
TRACE dcpu16::assembler: instruction Basic(SET, SpecialRegister(ProgramCounter), LabelReference("crash")), len = 1
TRACE dcpu16::assembler: instruction Basic(SET, Register(I), Static(Literal(10))), len = 1
TRACE dcpu16::assembler: instruction Basic(SET, Register(A), Static(Literal(8192))), len = 2
TRACE dcpu16::assembler: instruction Basic(SET, AddressOffset { address: 8192, register: I }, Static(AddressFromRegister(A))), len = 2
TRACE dcpu16::assembler: instruction Basic(SUB, Register(I), Static(Literal(1))), len = 1
TRACE dcpu16::assembler: instruction Basic(IFN, Register(I), Static(Literal(0))), len = 1
TRACE dcpu16::assembler: instruction Basic(SET, SpecialRegister(ProgramCounter), LabelReference("loop")), len = 1
TRACE dcpu16::assembler: instruction Basic(SET, Register(X), Static(Literal(4))), len = 1
TRACE dcpu16::assembler: instruction NonBasic(JSR, LabelReference("testsub")), len = 1
TRACE dcpu16::assembler: instruction Basic(SET, SpecialRegister(ProgramCounter), LabelReference("crash")), len = 1
TRACE dcpu16::assembler: instruction Basic(SHL, Register(X), Static(Literal(4))), len = 1
TRACE dcpu16::assembler: instruction Basic(SET, SpecialRegister(ProgramCounter), Static(StackOperation(Pop))), len = 1
TRACE dcpu16::assembler: instruction Basic(SET, SpecialRegister(ProgramCounter), LabelReference("crash")), len = 1
```

[0x10<sup>c</sup>]: https://en.wikipedia.org/wiki/0x10c
[DCPU-16 Specification]: docs/specification.txt
[Wayback Machine]: http://web.archive.org/web/20120504005858/http://0x10c.com/doc/dcpu-16.txt
[examples/sample.rs]: examples/sample.rs
[examples/assemble.rs]: examples/assemble.rs
