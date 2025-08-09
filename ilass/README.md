> [!NOTE]
> This is a fork of [kaegi/alass](https://github.com/kaegi/alass).

`ilass` is a library to synchronize subtitles to movies. If you are looking for the command line tool, please click [here](https://github.com/SandroHc/ilass).

## How to use the library

Add `ilass` as a dependency to your project:

```shell
cargo add ilass
```

Alternatively, edit your `Cargo.toml`:

```toml
[dependencies]
ilass = "2.1.0"
```

The library only contains one function that takes two sequences of time spans and returns the offsets to get the best possible alignment.

[Documentation](https://docs.rs/ilass)

[Crates.io](https://crates.io/crates/ilass)

### Documentation

For much more information, please see the workspace information [here](https://github.com/SandroHc/ilass).
