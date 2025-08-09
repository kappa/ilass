# ilass

This Rust library contains the core algorithm for `ilass`, the "Automatic Language-Agnostic Subtitle Sychronization" tool. If you want to go to the command line tool instead, please click [here](https://github.com/SandroHc/ilass).

## How to use the library

Add `ilass` as a dependency to your project:

```shell
cargo add ilass
```

Or alternative edit your `Cargo.toml`:

```toml
[dependencies]
ilass = "2.0.0"
```

The library only contains one function that takes two sequences of time spans and returns the offsets to get the best possible alignment.

[Documentation](https://docs.rs/ilass)

[Crates.io](https://crates.io/crates/ilass)

### Documentation

For much more information, please see the workspace information [here](https://github.com/SandroHc/ilass).
