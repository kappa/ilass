# ilass-core

This Rust library contains the core algorithm for `ilass`, the "Automatic Language-Agnostic Subtitle Sychronization" tool. If you want to go to the command line tool instead, please click [here](https://github.com/SandroHc/ilass).


## How to use the library
Add this to your `Cargo.toml`:

```toml
[dependencies]
ilass-core = "2.0.0"
```

The library only contains one function that takes two sequences of time spans and returns the offsets to get the best possible alignment.

[Documentation](https://docs.rs/ilass-core)

[Crates.io](https://crates.io/crates/ilass-core)

### Documentaion

For much more information, please see the workspace information [here](https://github.com/SandroHc/ilass).