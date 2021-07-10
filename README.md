# DCPU-16 Emulator

An emulator for the DCPU-16 16-bit processor described for the 
[0x10<sup>c</sup>] video game.

The [DCPU-16 Specification](docs/specification.txt) is no longer available on the
original website (as is the entire website), can be obtained from the [Wayback Machine].

---

Cycle counts are currently not emulated.

## Example use

See [examples/sample.rs] for a commented example application. Here's an example use:

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
}
```

## Example program execution

The example program can be started with

```console
RUST_LOG=sample=trace,dcpu16=trace cargo run --example sample
```

It executes the program given in the [DCPU-16 Specification](docs/specification.txt):

```asm
; Try some basic stuff
              SET A, 0x30              ; 7c01 0030
              SET [0x1000], 0x20       ; 7de1 1000 0020
              SUB A, [0x1000]          ; 7803 1000
              IFN A, 0x10              ; c00d
                 SET PC, crash         ; 7dc1 001a [*]

; Do a loopy thing
              SET I, 10                ; a861
              SET A, 0x2000            ; 7c01 2000
:loop         SET [0x2000+I], [A]      ; 2161 2000
              SUB I, 1                 ; 8463
              IFN I, 0                 ; 806d
                 SET PC, loop          ; 7dc1 000d [*]

; Call a subroutine
              SET X, 0x4               ; 9031
              JSR testsub              ; 7c10 0018 [*]
              SET PC, crash            ; 7dc1 001a [*]

:testsub      SHL X, 4                 ; 9037
              SET PC, POP              ; 61c1

; Hang forever. X should now be 0x40 if everything went right.
:crash        SET PC, crash            ; 7dc1 001a [*]
```

When executed, the program output looks like this:

```
 INFO dcpu16: Loaded 32 words of program data
DEBUG dcpu16: Registers: A=0000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0000 SP=FFFF O=0000
DEBUG dcpu16: PC=0000:   7c01 0030 => Set { a: Register { register: A }, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=0030 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0002 SP=FFFF O=0000
DEBUG dcpu16: PC=0002:   7de1 1000 0020 => Set { a: AtAddressFromNextWord, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=0030 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0005 SP=FFFF O=0000
DEBUG dcpu16: PC=0005:   7803 1000 => Sub { a: Register { register: A }, b: AtAddressFromNextWord }
DEBUG dcpu16: Registers: A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0007 SP=FFFF O=0000
DEBUG dcpu16: PC=0007:   c00d => Ifn { a: Register { register: A }, b: Literal { value: 16 } }
DEBUG dcpu16: Registers: A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=000A SP=FFFF O=0000
DEBUG dcpu16: PC=000A:   a861 => Set { a: Register { register: I }, b: Literal { value: 10 } }
DEBUG dcpu16: Registers: A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=000A J=0000 PC⁎=000B SP=FFFF O=0000
DEBUG dcpu16: PC=000B:   7c01 2000 => Set { a: Register { register: A }, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=000A J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: PC=000D:   2161 200a => Set { a: AtAddressFromNextWordPlusRegister { register: I }, b: AtAddressFromRegister { register: A } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=000A J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: PC=000F:   8463 => Sub { a: Register { register: I }, b: Literal { value: 1 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0009 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: PC=0010:   806d => Ifn { a: Register { register: I }, b: Literal { value: 0 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0009 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: PC=0011:   7dc1 000d => Set { a: OfProgramCounter, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0009 J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: PC=000D:   2161 2009 => Set { a: AtAddressFromNextWordPlusRegister { register: I }, b: AtAddressFromRegister { register: A } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0009 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: PC=000F:   8463 => Sub { a: Register { register: I }, b: Literal { value: 1 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0008 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: PC=0010:   806d => Ifn { a: Register { register: I }, b: Literal { value: 0 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0008 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: PC=0011:   7dc1 000d => Set { a: OfProgramCounter, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0008 J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: PC=000D:   2161 2008 => Set { a: AtAddressFromNextWordPlusRegister { register: I }, b: AtAddressFromRegister { register: A } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0008 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: PC=000F:   8463 => Sub { a: Register { register: I }, b: Literal { value: 1 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0007 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: PC=0010:   806d => Ifn { a: Register { register: I }, b: Literal { value: 0 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0007 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: PC=0011:   7dc1 000d => Set { a: OfProgramCounter, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0007 J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: PC=000D:   2161 2007 => Set { a: AtAddressFromNextWordPlusRegister { register: I }, b: AtAddressFromRegister { register: A } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0007 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: PC=000F:   8463 => Sub { a: Register { register: I }, b: Literal { value: 1 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0006 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: PC=0010:   806d => Ifn { a: Register { register: I }, b: Literal { value: 0 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0006 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: PC=0011:   7dc1 000d => Set { a: OfProgramCounter, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0006 J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: PC=000D:   2161 2006 => Set { a: AtAddressFromNextWordPlusRegister { register: I }, b: AtAddressFromRegister { register: A } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0006 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: PC=000F:   8463 => Sub { a: Register { register: I }, b: Literal { value: 1 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0005 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: PC=0010:   806d => Ifn { a: Register { register: I }, b: Literal { value: 0 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0005 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: PC=0011:   7dc1 000d => Set { a: OfProgramCounter, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0005 J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: PC=000D:   2161 2005 => Set { a: AtAddressFromNextWordPlusRegister { register: I }, b: AtAddressFromRegister { register: A } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0005 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: PC=000F:   8463 => Sub { a: Register { register: I }, b: Literal { value: 1 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0004 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: PC=0010:   806d => Ifn { a: Register { register: I }, b: Literal { value: 0 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0004 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: PC=0011:   7dc1 000d => Set { a: OfProgramCounter, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0004 J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: PC=000D:   2161 2004 => Set { a: AtAddressFromNextWordPlusRegister { register: I }, b: AtAddressFromRegister { register: A } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0004 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: PC=000F:   8463 => Sub { a: Register { register: I }, b: Literal { value: 1 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0003 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: PC=0010:   806d => Ifn { a: Register { register: I }, b: Literal { value: 0 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0003 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: PC=0011:   7dc1 000d => Set { a: OfProgramCounter, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0003 J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: PC=000D:   2161 2003 => Set { a: AtAddressFromNextWordPlusRegister { register: I }, b: AtAddressFromRegister { register: A } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0003 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: PC=000F:   8463 => Sub { a: Register { register: I }, b: Literal { value: 1 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0002 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: PC=0010:   806d => Ifn { a: Register { register: I }, b: Literal { value: 0 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0002 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: PC=0011:   7dc1 000d => Set { a: OfProgramCounter, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0002 J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: PC=000D:   2161 2002 => Set { a: AtAddressFromNextWordPlusRegister { register: I }, b: AtAddressFromRegister { register: A } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0002 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: PC=000F:   8463 => Sub { a: Register { register: I }, b: Literal { value: 1 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0001 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: PC=0010:   806d => Ifn { a: Register { register: I }, b: Literal { value: 0 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0001 J=0000 PC⁎=0011 SP=FFFF O=0000
DEBUG dcpu16: PC=0011:   7dc1 000d => Set { a: OfProgramCounter, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0001 J=0000 PC⁎=000D SP=FFFF O=0000
DEBUG dcpu16: PC=000D:   2161 2001 => Set { a: AtAddressFromNextWordPlusRegister { register: I }, b: AtAddressFromRegister { register: A } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0001 J=0000 PC⁎=000F SP=FFFF O=0000
DEBUG dcpu16: PC=000F:   8463 => Sub { a: Register { register: I }, b: Literal { value: 1 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0010 SP=FFFF O=0000
DEBUG dcpu16: PC=0010:   806d => Ifn { a: Register { register: I }, b: Literal { value: 0 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0013 SP=FFFF O=0000
DEBUG dcpu16: PC=0013:   9031 => Set { a: Register { register: X }, b: Literal { value: 4 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0004 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0014 SP=FFFF O=0000
TRACE dcpu16::instruction: Decoding non-basic instruction 7C10, opcode 01, value 1F
DEBUG dcpu16: PC=0014:   7c10 0018 => NonBasic(Jsr { a: NextWordLiteral })
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0004 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0018 SP=FFFE O=0000
DEBUG dcpu16: PC=0018:   9037 => Shl { a: Register { register: X }, b: Literal { value: 4 } }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0040 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0019 SP=FFFE O=0000
DEBUG dcpu16: PC=0019:   61c1 => Set { a: OfProgramCounter, b: Pop }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0040 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0016 SP=FFFF O=0000
DEBUG dcpu16: PC=0016:   7dc1 001a => Set { a: OfProgramCounter, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0040 Y=0000 Z=0000 I=0000 J=0000 PC⁎=001A SP=FFFF O=0000
DEBUG dcpu16: PC=001A:   7dc1 001a => Set { a: OfProgramCounter, b: NextWordLiteral }
DEBUG dcpu16: Registers: A=2000 B=0000 C=0000 X=0040 Y=0000 Z=0000 I=0000 J=0000 PC⁎=001A SP=FFFF O=0000
 WARN dcpu16: Crash loop detected at PC=001A - terminating
```

After the execution, the `X` register contains the word `0040` as expected (see the
[DCPU-16 Specification] for the example program).

[0x10<sup>c</sup>]: https://en.wikipedia.org/wiki/0x10c
[DCPU-16 Specification]: docs/specification.txt
[Wayback Machine]: http://web.archive.org/web/20120504005858/http://0x10c.com/doc/dcpu-16.txt
[examples/sample.rs]: examples/sample.rs
