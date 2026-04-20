# Zoxi

Zoxi is a small language layer on top of Rust.

You write `.zo` files, Zoxi transpiles them into Rust under `.zoxi/src`, then compiles them with `rustc`.

## What Zoxi Adds

Zoxi keeps normal Rust syntax unless it recognizes one of its language features.

- `:` return syntax instead of `->`
- `string` as an alias for `String`
- owned string literals with interpolation: `"Hello, {name}"`
- `.view()` as `as_str()`
- vector literal sugar: `[1, 2, 3]`
- map literal sugar: `{ "name": "zoxi", "age": 1 }`
- index assignment sugar: `map["key"] = value`
- iterator helpers with arrow closures: `map`, `filter`, `find`, `findIndex`

Example:

```zoxi
fn greet(name: &str): string {
    "Hello, {name}!"
}

fn main() {
    let nums = [1, 2, 3, 6, 8, 10];
    let doubled: Vec<i32> = nums.map(x => x * 2);
    println!("{} {:?}", greet("Zoxi"), doubled);
}
```

## Project Layout

Write Zoxi source in `src/**/*.zo`:

```text
my-app/
├── src/
│   └── main.zo
└── zoxi.toml
```

Generated and cached files live in `.zoxi/`:

```text
.zoxi/
├── src/                  # generated Rust
└── .cache/
    ├── transpile-state
    ├── build-state
    ├── artifacts/
    └── incremental/
```

Shared dependency caches live globally in `~/.zoxi/cache/`.

## Dependencies

Zoxi can manage registry dependencies with:

```bash
zoxi add itertools
zoxi remove itertools
```

For user projects, Zoxi does not use Cargo at runtime.

- crate metadata is fetched from crates.io
- crate archives are downloaded directly
- dependencies are compiled with `rustc`
- compiled dependency artifacts are shared globally in `~/.zoxi/cache`

## CLI

Build:

```bash
zoxi build
zoxi build -r
```

Run:

```bash
zoxi run
zoxi run -r
zoxi run -r -- arg1 arg2
```

Test:

```bash
zoxi test
zoxi test -r
```

Use `--path` to run against another project directory:

```bash
zoxi --path examples/hello run -r
```

## Development

Build the Zoxi CLI itself with Cargo:

```bash
cargo run -- --path examples/hello run -r
```

The Zoxi repository uses Cargo for developing Zoxi.
Zoxi projects use the Zoxi runtime pipeline instead.

## Status Output

The CLI prints Cargo-style progress output such as:

```text
    Checking /path/to/project/src
       Fresh transpilation cache (1 files)
   Compiling /path/to/project/.zoxi/src/main.rs
    Finished release profile [optimized]
     Running /path/to/project/.zoxi/.cache/artifacts/release/app
```

Unchanged sources are checked through file metadata so repeated runs avoid unnecessary transpile and compile work.

## Limitations

Current runtime dependency support is intentionally narrow:

- registry dependencies only
- no `path`, `git`, or `workspace` dependencies
- no `build.rs` support yet
- proc-macro support exists but is not fully battle-tested across complex dependency graphs

## Docs

See `DOC.md` for the current syntax reference and Rust translation details.

## License

MIT. See `LICENSE`.
