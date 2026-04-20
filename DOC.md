# Zoxi Syntax Reference

This document describes the Zoxi syntax that is currently implemented by the transpiler and the Rust it becomes.

## Core Rule

If a syntax form is not listed below, it is treated as normal Rust syntax and passed through unchanged.

## Files And Project Layout

Zoxi source files:

```zoxi
src/main.zo
src/lib.zo
src/foo/bar.zo
```

Generated Rust files:

```rust
.zoxi/src/main.rs
.zoxi/src/lib.rs
.zoxi/src/foo/bar.rs
```

Project manifest files:

```text
zoxi.toml -> .zoxi/Cargo.toml
zoxi.lock -> .zoxi/Cargo.lock
```

## Function Return Syntax

Zoxi lets you use `:` after a function signature or closure parameter list instead of Rust's `->`.

Zoxi:

```zoxi
fn greet(name: &str): string {
    "Hello, {name}!"
}
```

Rust:

```rust
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

Also works for closure-style signatures:

```zoxi
|x|: i32 { x + 1 }
```

Rust:

```rust
|x| -> i32 { x + 1 }
```

## `string` Alias

Zoxi:

```zoxi
let name: string = "Zoxi";
```

Rust:

```rust
let name: String = String::from("Zoxi");
```

## String Literals

Normal string literals become owned `String` values in expression positions.

Zoxi:

```zoxi
let name = "Zoxi";
```

Rust:

```rust
let name = String::from("Zoxi");
```

### String Interpolation

`{...}` inside a string becomes `format!`.

Zoxi:

```zoxi
let msg = "Hello, {name}!";
let full = "{first} {last} ({age})";
```

Rust:

```rust
let msg = format!("Hello, {}!", name);
let full = format!("{} {} ({})", first, last, age);
```

Nested expressions are supported inside interpolation:

Zoxi:

```zoxi
let msg = "sum = {a + b}";
```

Rust:

```rust
let msg = format!("sum = {}", a + b);
```

### Places Where String Literals Stay Rust Strings

String literals are kept as normal Rust string literals in contexts where Rust expects `&str`, such as:

```zoxi
println!("hello {}");
foo("bar");
arr["key"];
&"name";
```

These remain string literals instead of being wrapped in `String::from(...)`.

## `.view()` Alias

Zoxi:

```zoxi
name.view()
```

Rust:

```rust
name.as_str()
```

## Vector Literal Sugar

Top-level array literals in expression positions become `Vec` literals.

Zoxi:

```zoxi
let nums = [1, 2, 3];
let nested = [[1, 2], [3, 4]];
```

Rust:

```rust
let nums = vec![1, 2, 3];
let nested = vec![vec![1, 2], vec![3, 4]];
```

Array syntax is kept as normal Rust array syntax when it is clearly being used as an array expression rather than vector sugar.

## HashMap Literal Sugar

Top-level `{ key: value }` expression literals become `HashMap` construction.

Zoxi:

```zoxi
let user = { "name": "Zoxi", "age": 2 };
let empty = {};
```

Rust:

```rust
let user = std::collections::HashMap::from([
    (String::from("name"), String::from("Zoxi")),
    (String::from("age"), 2),
]);
let empty = std::collections::HashMap::new();
```

Nested literals inside map keys and values are also transpiled.

## HashMap Index Assignment Sugar

Zoxi:

```zoxi
scores["alice"] = 10;
```

Rust:

```rust
scores.insert("alice", 10);
```

This rewrite applies to assignment expressions using `ident[key] = value`.

## Iterator Helper Syntax

Zoxi supports JavaScript-like collection helpers with arrow closures.

### `map`

Zoxi:

```zoxi
let doubled: Vec<i32> = nums.map(x => x * 2);
```

Rust:

```rust
let doubled: Vec<i32> = nums.iter().map(|x| (*x) * 2).collect();
```

### `filter`

Zoxi:

```zoxi
let positives: Vec<i32> = nums.filter(x => x > 0);
```

Rust:

```rust
let positives: Vec<i32> = nums.iter().filter(|x| (**x) > 0).collect();
```

### `filter(...).map(...)`

Zoxi:

```zoxi
let result: Vec<i32> = nums.filter(x => x > 0).map(x => x * 2);
```

Rust:

```rust
let result: Vec<i32> = nums
    .iter()
    .filter(|x| (**x) > 0)
    .map(|x| (*x) * 2)
    .collect();
```

### `find`

Zoxi:

```zoxi
let first = nums.find(x => x > 5);
```

Rust:

```rust
let first = nums.iter().find(|x| (**x) > 5);
```

### `findIndex`

Zoxi:

```zoxi
let idx = nums.findIndex(x => x > 5);
```

Rust:

```rust
let idx = nums.iter().position(|x| (*x) > 5);
```

### Iterator Helper Rules

These helpers are currently supported:

```text
map
filter
find
findIndex
```

Arrow closure syntax:

```zoxi
x => x * 2
(x) => x * 2
```

becomes:

```rust
|x| (*x) * 2
```

or another equivalent closure form depending on the helper and iterator type.

Additional rules:

- If the receiver does not already end with `.iter()`, `.iter_mut()`, or `.into_iter()`, Zoxi inserts `.iter()` automatically.
- If the last helper in the chain is `map` or `filter`, Zoxi appends `.collect()` automatically.
- If the last helper is `find` or `findIndex`, no `.collect()` is added.
- When the receiver already uses `.into_iter()`, fewer dereferences are inserted in the generated closure body.

## Automatic `String` Binding Conversion

If a binding is explicitly typed as `String`, Zoxi wraps plain string RHS expressions with `String::from(...)` when needed.

Zoxi:

```zoxi
let title: String = "Hello";
```

Rust:

```rust
let title: String = String::from("Hello");
```

These forms are left unchanged:

```zoxi
let a: String = String::from("x");
let b: String = format!("{}", value);
let c: String = name.to_string();
let d: String = String::new();
```

## Static `String` Simplification

Zoxi:

```zoxi
static APP_NAME: String = "zoxi";
```

Rust:

```rust
static APP_NAME: &str = "zoxi";
```

## Result Tail Expression Sugar

In functions returning `Result<...>`, a final bare expression is automatically wrapped in `Ok(...)`.

Zoxi:

```zoxi
fn answer(): Result<i32, Error> {
    42
}
```

Rust:

```rust
fn answer() -> Result<i32, Error> {
    Ok(42)
}
```

No wrapping happens if the final statement is already one of these:

- empty
- ends with `;`
- starts with `return`
- starts with `Ok(`
- starts with `Err(`

## Combined Example

Zoxi:

```zoxi
fn greet(name: &str): string {
    "Hello, {name}!"
}

fn main() {
    let nums = [1, 2, 3, 6, 8, 10];
    let info = { "app": "zoxi", "count": nums.len() };

    let doubled: Vec<i32> = nums.map(x => x * 2);
    let positives_doubled: Vec<i32> = nums.filter(x => x > 2).map(x => x * 2);
    let first_large = nums.find(x => x > 5);
    let first_large_index = nums.findIndex(x => x > 5);

    println!("{} {:?} {:?} {:?}", greet("world"), info, first_large, first_large_index);
    println!("{:?} {:?}", doubled, positives_doubled);
}
```

Rust:

```rust
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

fn main() {
    let nums = vec![1, 2, 3, 6, 8, 10];
    let info = std::collections::HashMap::from([
        (String::from("app"), String::from("zoxi")),
        (String::from("count"), nums.len()),
    ]);

    let doubled: Vec<i32> = nums.iter().map(|x| (*x) * 2).collect();
    let positives_doubled: Vec<i32> = nums
        .iter()
        .filter(|x| (**x) > 2)
        .map(|x| (*x) * 2)
        .collect();
    let first_large = nums.iter().find(|x| (**x) > 5);
    let first_large_index = nums.iter().position(|x| (*x) > 5);

    println!(
        "{} {:?} {:?} {:?}",
        greet("world"),
        info,
        first_large,
        first_large_index,
    );
    println!("{:?} {:?}", doubled, positives_doubled);
}
```
