# DCPU-16 Emulator

An emulator for the DCPU-16 16-bit processor described for the 
<em>0x10<sup>c</sup></em> video game.

## Example Output

The example program can be started with

```console
$ cargo run --example sample
```

It executes the program given in the [Specification](docs/specification.txt):

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

When executing, the program outputs the following:

```
Initial state:
A=0000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0000 SP=FFFF O=0000

0000: 7c01 0030 => Set { a: Register { register: A }, b: NextWordLiteral }
A=0030 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0002 SP=FFFF O=0000

0002: 7de1 1000 0020 => Set { a: AtAddressFromNextWord, b: NextWordLiteral }
A=0030 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0005 SP=FFFF O=0000

0005: 7803 1000 => Sub { a: Register { register: A }, b: AtAddressFromNextWord }
A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0007 SP=FFFF O=0000

0007: c00d => Ifn { a: Register { register: A }, b: Literal { value: 16 } }
A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0008 SP=FFFF O=0000

0008: 7dc1 001a => Set { a: OfProgramCounter, b: NextWordLiteral }
A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=000A SP=FFFF O=0000

000A: a861 => Set { a: Register { register: I }, b: Literal { value: 10 } }
A=0010 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=000B SP=FFFF O=0000

000B: 7c01 2000 => Set { a: Register { register: A }, b: NextWordLiteral }
A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=000D SP=FFFF O=0000

000D: 2161 2000 => Set { a: AtAddressFromNextWordPlusRegister { register: I }, b: AtAddressFromRegister { register: A } }
A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=000F SP=FFFF O=0000

000F: 8463 => Sub { a: Register { register: I }, b: Literal { value: 1 } }
A=2000 B=0000 C=0000 X=0000 Y=0000 Z=0000 I=0000 J=0000 PC⁎=0010 SP=FFFF O=0000
```
