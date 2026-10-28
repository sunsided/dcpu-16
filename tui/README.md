# dcpu16-tui

A terminal user interface for the [`dcpu16`](..) DCPU-16 emulator. It assembles a
program, runs it at the 100 kHz of the original, and shows:

- **Screen:** the 32×12 character cells at `0x8000`, with colors from the LEM1802
  palette and blinking. Cells without color bits use the terminal's colors.
- **Console:** what the program printed through the console word at `0x9010`.
- **Registers:** `A`–`J`, `PC`, `SP`, `O` and the cycle count; values that just
  changed light up.
- **Hardware:** the screen, keyboard and console with activity lights, the keyboard
  ring buffer slot by slot, the measured clock speed, and the stack.
- **Code:** the disassembled instructions from the program counter on, syntax
  highlighted and with a plain-English comment, e.g. `SET [X], 0xF0DF ; store 0xF0DF in RAM[X]`.

The look is a phosphor ship console: amber chrome and green phosphor on near-black,
which needs a terminal with 24-bit color.

Typed keys go to the keyboard ring buffer at `0x9000`, as their Unicode value. Enter
is `0x0a`, Backspace `0x08`, and the arrow keys are `0x80`–`0x83`.

```console
cargo run -p dcpu16-tui -- tui/programs/typewriter.dasm
```

| Key       | Action                                           |
|-----------|--------------------------------------------------|
| F5        | Pause or resume                                  |
| F10       | Execute one instruction while paused             |
| F2        | Reset and run the program from the start         |
| F7 / F8   | Slower / faster, from 1 kHz to 10 MHz            |
| Ctrl+C    | Quit                                             |

Example programs are in [`programs`](programs): `hello.dasm` greets on the screen
and the console, and `typewriter.dasm` types keys onto the screen.
