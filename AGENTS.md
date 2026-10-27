# Rust style: Google Rust Style Guide

This codebase follows the [Google Rust Style Guide](https://google.github.io/styleguide/rustguide.html). Formatting and dozens of rules are enforced by rustfmt, clippy and cargo-deny. You do not need to memorize those; the tools will tell you. This file covers the rules no tool can check. Follow them when writing or reviewing Rust here.

## Before you finish

Run these and fix everything they report:

```sh
cargo +nightly fmt                                   # nightly applies the guide's unstable rustfmt options
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check bans                                # if cargo-deny is installed
```

## When a lint fires

1. Fix the code. This is the answer almost every time.
2. If the guide itself grants an exception (listed under each rule below and in the guide), suppress the lint at the narrowest scope with a reason the reviewer can check:

   ```rust
   #[expect(clippy::exhaustive_structs, reason = "options struct; callers rely on struct update syntax")]
   ```

3. Never use `#[allow(...)]` without a `reason`, never use `#[rustfmt::skip]` for personal taste, and never edit the lint configuration (`Cargo.toml` `[lints]`, `clippy.toml`, `rustfmt.toml`, `deny.toml`) to make a warning go away.

Prefer `#[expect]` over `#[allow]`: it fails once the suppression is no longer needed.

## Library code or application code?

Decide first. For **library code** (used by other projects, solving a general problem) the API rules below are binding. For **application code** (one program, a specific problem) they are strong recommendations, and simplicity wins: do not generalize prematurely.

## Rules tools cannot check

### Code organization

- **Non-leaf modules only declare and re-export.** `lib.rs`, `mod.rs` and other modules with children hold `mod` declarations and `pub use` re-exports, plus at most a little glue. Put implementations in leaf modules.
- **Name a leaf module after its primary item.** A module whose main item is `RetryPolicy` is `retry_policy.rs`.
- **No glob imports and no preludes.** Clippy catches most globs. The guide allows `use super::*` in inline modules, `use Enum::*` inside functions that mostly handle that enum, and `pub use submodule::*` in non-leaf modules. Do not define a `prelude` module.
- **Import traits locally** inside the function that needs them, unless the file uses the trait throughout.

  ```rust
  fn parse_port(text: &str) -> Result<u16, ParseIntError> {
      use std::str::FromStr;
      u16::from_str(text)
  }
  ```

- **Call free functions through their module; use types unqualified.**

  ```rust
  // Bad
  use std::mem::swap;
  let pattern = regex::Regex::new(r"\d+")?;
  swap(&mut left, &mut right);

  // Good
  use std::mem;
  use regex::Regex;
  let pattern = Regex::new(r"\d+")?;
  mem::swap(&mut left, &mut right);
  ```

  Exceptions: ambiguous names may stay qualified (`io::Error`) or be renamed (`use std::io::Error as IoError`), as may large families of types, macros and generated code.

### Formatting

- Use `#[rustfmt::skip]` only for deliberate manual alignment (for example a matrix literal) or generated code.
- Write invisible characters as escapes (`"\u{200D}"`), never literally.

### Ownership and borrowing

- **Avoid `Rc` and `Arc`.** Model ownership with unique owners and borrows first. Shared ownership is allowed only when the alternative is unreasonably complex, and the `#[expect(clippy::disallowed_types, reason = "...")]` must say why.
- **Clone only when you need a copy.** Do not use `.clone()` to get past the borrow checker; restructure first. Cloning an `Rc`/`Arc` handle is fine.

### Function inputs and outputs

- **Return iterators from library functions** (`impl Iterator<Item = T>`) instead of `Vec<T>`, unless callers almost always need a `Vec`, the function naturally builds one, results need random access, or the function's purpose is to build a collection. Application code may return `Vec<T>` when that is simpler.
- **Accept `impl IntoIterator` in library functions** that only iterate their input. Application code may take `&[T]`.
- **`impl Into<T>` parameters** only in library APIs where call-site ergonomics matter most. Application code should almost never use them.
- **Enum, not `bool`, when the meaning is not obvious at the call site.** Clippy only flags two or more bools; a single unclear bool is your call.

  ```rust
  // Bad
  file.open(path, true);
  // Good
  file.open(path, OpenMode::CreateIfMissing);
  ```

  If a `bool` stays, label it at the call site: `connect(addr, /* retry= */ false)`.

- **Pick the right error channel.** `Option` for an obvious, expected absence. `Result` for recoverable failures where the caller needs to know why (I/O, network, parsing with detail). `panic!` only for bugs and broken invariants, and document it under `# Panics`.

### Types

- **Newtype instead of type alias** when the type has different meaning, needs its own API, may change representation, or could be confused with another value of the same underlying type (`struct UserId(u64)`, not `type UserId = u64`).
- **Named fields instead of tuples.** No `(String, u32, bool)` return values or multi-field tuple structs. One-field newtypes and very local tuples are fine.

### Traits

- **Library code anticipates trait impls; application code adds them when needed.**
- **Implement the standard trait** (`FromStr`, `Display`, `From`, `Iterator`, `Default`) instead of an inherent method with the same purpose.
- **Prefer `#[derive]`** over hand-written impls.
- **Generics over `dyn Trait`** in library parameters (`T: Trait` or `impl Trait`), unless code size matters.
- **Define a trait only** for generic bounds, for `impl Trait`/`dyn Trait`, or as an extension trait. Never just to group methods; use an inherent `impl`.
- **Extension traits end in `Ext`** (`PathBufExt`) and belong to the team that owns the extended type.
- **Trait checklist for new types:**
  - Error types: `Copy` when no field prevents it, and `#[must_use]`.
  - `Copy` when it will stay `Copy`; `Clone` unless the type owns an external resource.
  - `Default` when there is `new()` or an obvious default state.
  - Prefer `PartialEq` over `Eq`, and `PartialOrd` over `Ord`, unless you need the stronger guarantee.
  - `Hash` + `Eq` for likely map keys.
- **Pair a primary `From` impl with a named constructor** (`new`, `new_from_x` or `from_x`).
- **Options structs implement `Default`** so callers can use `..Default::default()`.

### Concurrency

- **Never read through a poisoned lock.** Propagate or panic; do not call `into_inner`/`get_ref`/`get_mut` on a `PoisonError` (clippy flags those calls).

### Macros

Prefer, in order: plain functions, types and traits; `macro_rules!`; other code generation; procedural macros. Justify every macro.

### Naming

- Conversions: `as_` (cheap borrow), `to_` (expensive or owned), `into_` (consumes `self`).
- Constructors: `new()`, `new_with_x()` for alternatives, `try_new()` when it returns `Result`. Value conversions: `from_x()` alongside a `From` impl.
- Accessors: `foo()` getter (no `get_` prefix), `foo_mut()`, `set_foo(value)`, `take_foo()` (clears and returns), `replace_foo(value)` (returns the old value), `with_foo(|foo| ..)` / `with_mut_foo(|foo| ..)` for scoped access through a lock or `RefCell`.
- Lifetimes: descriptive names on types (`'source`, not `'a`). Use `'_` where it works; one or two single-letter lifetimes are fine for a single method's input-to-output relationship or a local `for<'a>` bound.
- Generic parameters: short and tied to the main bound (`R: Read`, `K`, `V`). Use `T`/`U` only for unconstrained generics.
- No abbreviations an outsider would not know, and no dropped letters (`message`, not `msg`; `count`, not `cnt`). Widely known terms (`Ip`, `Http`, `Utf8`) are fine.

### Documentation

- Doc comments (`///`) on every public item, written in full sentences with Markdown; `//!` at the top of a file for the module.
- Say what the code does not: skip `/// Removes an element.` on `remove_element`.
- Document `# Panics`, `# Errors` and `# Safety` sections. Include examples where they help readers see how the crate fits together.
- `//` for implementation notes and TODOs. Mixing `///` and `//` on the same item is fine.

### Other features

- **Unsafe:** only when there is no safe way. Wrap it in a small safe abstraction away from business logic, add a `// SAFETY:` comment explaining why it is sound, and record why safe code was not enough (with benchmarks if the reason is performance).
- **No unstable features.**
- **Never use `#[doc(hidden)]` items** from code you do not own.
- **`const fn` in public APIs** only if you will keep it `const`; removing `const` is a breaking change.
- **Operator overloading** only for mathematical or logical meaning, never as shorthand.
- **`#[non_exhaustive]`** on public enums and on public structs with all-public fields. Exceptions: fixed ontologies (`Color { Red, Green, Blue }`), two-state enums that could be a bool, options structs that use struct update syntax, or types whose owners will fix every caller themselves.
- **`#[must_use]`** on functions whose result must be used (pure functions, critical status), and on types that are errors or lazy computations.
- **No code before `main()`** (`ctor`, `inventory`, `static_init`).

### Implementation

- **Annotate types where inference hides them,** especially after `.into()`, `.parse()`, `.collect()` and long iterator chains. Put the type on the `let`, not in a turbofish.

  ```rust
  let port: u16 = text.parse()?;   // not text.parse::<u16>()?
  ```

- **Prove instead of unwrapping:** `if let` / `let ... else` instead of `is_some()` then `unwrap()`. When a panic is unavoidable, use `expect("why this cannot fail")`.
- **When matching a `#[non_exhaustive]` enum from another crate,** list every known variant and put the ones you ignore in the wildcard arm (`Shape::Triangle | _ => {}`). Use `if let` when you care about one variant.
- **Give `assert!` a message** that explains why the condition holds or what the caller did wrong, never one that restates the condition.
- **Iterator chains stay short.** Use a `for` loop when a chain mutates outside state or branches. Never build a chain in a `for` header; bind it to a named variable first. Avoid `for_each`.

  ```rust
  let urgent_bugs = bugs.iter().filter(|bug| bug.priority <= Priority::P1);
  for bug in urgent_bugs {
      assign(bug);
  }
  ```

- **No `panic::catch_unwind`**, except as defense in depth at an entry point in a codebase that already works to avoid panics.
- **`format!` / `write!` to join three or more strings** and to fill templates, not `+` chains or `str::replace` on placeholder text.
