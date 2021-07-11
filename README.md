# DCPU-16 Emulator

An emulator for the DCPU-16 16-bit processor described for the 
[0x10<sup>c</sup>] video game.

The [DCPU-16 Specification](docs/specification.txt) is no longer available on the
original website (as is the entire website) but still can be obtained from the [Wayback Machine].

---

Cycle counts are currently not emulated. This project also does not contain an
assembler, i.e., the program must be provided as bytecode. 

## Example use

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

## Example program execution

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
DEBUG dcpu16: EXEC 0000: 7c01 0030 ; SET A, 0x30 => "A <- 0x30"
DEBUG dcpu16: Registers: A=0030 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0002 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0002: 7de1 1000 0020 ; SET [0x1000], 0x20 => "RAM[0x1000] <- 0x20"
DEBUG dcpu16: Registers: A=0030 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0005 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0005: 7803 1000 ; SUB A, [0x1000] => "A <- A - RAM[0x1000]"
DEBUG dcpu16: Registers: A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0007 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0007: c00d ; IFN A, 0x10 => "execute next instruction if A != 0x10"
DEBUG dcpu16: Registers: A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0008 SP=FFFF O=0000
DEBUG dcpu16: SKIP 0008: 7dc1 001a ; SET PC, 0x1A => "PC <- 0x1A"
DEBUG dcpu16: Registers: A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=000A SP=FFFF O=0000
DEBUG dcpu16: EXEC 000A: a861 ; SET I, 0x0A => "I <- 0x0A"
DEBUG dcpu16: Registers: A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=000A J=0000 PC⁎=000B SP=FFFF O=0000
DEBUG dcpu16: EXEC 000B: 7c01 2000 ; SET A, 0x2000 => "A <- 0x2000"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=000A J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: EXEC 000D: 2161 2000 ; SET [0x2000+I], [A] => "RAM[0x2000 + I] <- RAM[A]"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=000A J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: EXEC 000F: 8463 ; SUB I, 0x01 => "I <- I - 0x01"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0009 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0010: 806d ; IFN I, 0x00 => "execute next instruction if I != 0x00"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0009 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0011: 7dc1 000d ; SET PC, 0x0D => "PC <- 0x0D"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0009 J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: EXEC 000D: 2161 2000 ; SET [0x2000+I], [A] => "RAM[0x2000 + I] <- RAM[A]"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0009 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: EXEC 000F: 8463 ; SUB I, 0x01 => "I <- I - 0x01"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0008 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0010: 806d ; IFN I, 0x00 => "execute next instruction if I != 0x00"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0008 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0011: 7dc1 000d ; SET PC, 0x0D => "PC <- 0x0D"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0008 J=0000 PC⁎=000D SP=FFFF O=0000
... loop repeats ...
DEBUG dcpu16: EXEC 000D: 2161 2000 ; SET [0x2000+I], [A] => "RAM[0x2000 + I] <- RAM[A]"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0001 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: EXEC 000F: 8463 ; SUB I, 0x01 => "I <- I - 0x01"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0010: 806d ; IFN I, 0x00 => "execute next instruction if I != 0x00"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: SKIP 0011: 7dc1 000d ; SET PC, 0x0D => "PC <- 0x0D"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0013 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0013: 9031 ; SET X, 0x04 => "X <- 0x04"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0004 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0014 SP=FFFF O=0000
TRACE dcpu16::instruction: Decoding non-basic instruction 7C10, opcode 01, value 1F
DEBUG dcpu16: EXEC 0014: 7c10 0018 ; JSR 0x18 => "jump to subroutine at 0x18"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0004 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0018 SP=FFFE O=0000
DEBUG dcpu16: EXEC 0018: 9037 ; SHL X, 0x04 => "X <- X << 0x04"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0040 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0019 SP=FFFE O=0000
DEBUG dcpu16: EXEC 0019: 61c1 ; SET PC, POP => "PC <- pop value from stack"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0040 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0016 SP=FFFF O=0000
DEBUG dcpu16: EXEC 0016: 7dc1 001a ; SET PC, 0x1A => "PC <- 0x1A"
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0040 Y=0000 Z=0000 I=0000 J=0000 PC⁎=001A SP=FFFF O=0000
DEBUG dcpu16: EXEC 001A: 7dc1 001a ; SET PC, 0x1A => "PC <- 0x1A"
 WARN dcpu16: Crash loop detected at PC=001A - terminating
```

After the execution, the `X` register contains the word `0040` as expected (by the specification).

[0x10<sup>c</sup>]: https://en.wikipedia.org/wiki/0x10c
[DCPU-16 Specification]: docs/specification.txt
[Wayback Machine]: http://web.archive.org/web/20120504005858/http://0x10c.com/doc/dcpu-16.txt
[examples/sample.rs]: examples/sample.rs
