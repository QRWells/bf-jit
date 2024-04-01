# bf-jit

- Brainfxxk Interpreter with JIT compilation.

## How to build

```sh
$ cargo build --release
```

## How to run

```sh
$ ./target/release/bf-jit <options> <path-to-bf-file>
```

or just

```sh
$ cargo run --release <options> <path-to-bf-file>
```

### Options

- `-j` or `--jit`: Enable JIT compilation
    - only available on x86_64 Linux/macOS

## Related Projects

- [seelx3/bf-cpp](https://github.com/seelx3/bf-cpp)
- [tuesdayjz/bf-interpreter](https://github.com/tuesdayjz/bf-interpreter)
- [uchijo/bf-eval](https://github.com/uchijo/bf-eval)
- [kobayashiharuto/brainfuck](https://github.com/kobayashiharuto/brainfuck)
- [mkihr-ojisan/bf](https://github.com/mkihr-ojisan/bf)
- [n0-t0/brainfuck](https://github.com/n0-t0/brainfuck)