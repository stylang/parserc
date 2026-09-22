# parserc

[![CI][1]][2] [![version][3]][4]

[1]: https://github.com/stylang/parserc/actions/workflows/ci.yaml/badge.svg
[2]: https://github.com/stylang/parserc/actions/workflows/ci.yaml
[3]: https://img.shields.io/crates/v/parserc.svg
[4]: https://docs.rs/parserc/latest/parserc

Yet another parser combinator framework for the `rust` programming language.

`parserc` builds hand-written tokenizers and parsers from small, composable
pieces. It targets tools that need precise source positions and explicit
control over backtracking: compilers, linters, formatters, and configuration
readers.

## Highlights

- **Composable parsers** — any `FnOnce(&mut I) -> Result<O, I::Error>` is a
  [`Parser`](https://docs.rs/parserc/latest/parserc/trait.Parser.html); chain
  them with `map` / `map_err` / `ok` / `or` / `fatal` / `boxed`.
- **Explicit backtracking** — every error carries a `ControlFlow`
  (`Recovable` / `Incomplete` / `Fatal`), so you decide what may be retried and
  what must abort the parse.
- **Precise spans** — inputs track their absolute offset in the source;
  failures report the [`Span`](https://docs.rs/parserc/latest/parserc/enum.Span.html)
  they point at, ready for diagnostics.
- **Byte and char inputs** — `bytes::TokenStream` (items are `u8`) and
  `chars::TokenStream` (items are `char`) over `&str`, with `memchr`-backed
  matching and search.
- **AST building blocks** (feature `syntax`) — nodes like `Delimiter`,
  `Punctuated`, `Or`, `Limits*`, plus a `#[derive(Syntax)]` macro.
- **Fast** — adapter combinators cost about a nanosecond, `memchr`-backed scans
  run at tens of GB/s. See [Benchmarks](#benchmarks).

## Installation

```toml
[dependencies]
parserc = "0.12"
```

Requires Rust 1.85 or later (edition 2024). Default features are `input`,
`syntax` and `serde`; for a leaner build:

```toml
parserc = { version = "0.12", default-features = false, features = ["input"] }
```

## Quick start

Match a keyword, skip whitespace, then read an identifier:

```rust
use parserc::{AsStr, Parser, chars::TokenStream, keyword, take_while, take_while_with};

type Chars = TokenStream<'static>;

fn main() {
    let mut input = Chars::from("let answer = 42");

    let kw = keyword::<_, Chars>("let").parse(&mut input).unwrap();
    let ws = take_while::<Chars, _>(|c| c.is_whitespace()).parse(&mut input).unwrap();
    let name = take_while_with::<Chars, _, _>(1.., |c| c.is_ascii_alphabetic())
        .map(|taken| taken.as_str().to_owned())
        .parse(&mut input)
        .unwrap();

    assert_eq!(kw.as_str(), "let");
    assert_eq!(ws.as_str(), " ");
    assert_eq!(name, "answer");
    // The unparsed remainder is still available on the input.
    assert_eq!(input.as_str(), " = 42");
}
```

## Error model

| `ControlFlow` | Meaning | `ok` / `or` behavior |
|---|---|---|
| `Recovable` | The parser did not match; an alternative may still succeed. | Retried |
| `Incomplete` | The input ended before the parser could finish. | Retried |
| `Fatal` | Parsing cannot continue. | Propagated as-is |

Parsers fail without consuming input; backtracking combinators rewind the
input themselves before retrying. Promote any failure with `fatal()`, and
inspect errors through `ParseError` (`to_span`, `control_flow`, `is_fatal`).
`Kind::LeftRecursion` is raised as `Fatal` when a derived syntax node detects
left recursion.

```rust
use parserc::{AsStr, ParseError, Parser, chars::TokenStream, keyword};

type Chars = TokenStream<'static>;

fn main() {
    // `or` runs the right parser only when the left one fails `non-fatal`ly ...
    let mut input = Chars::from("const x = 1");
    let kw = keyword::<_, Chars>("let")
        .or(keyword::<_, Chars>("const"))
        .parse(&mut input)
        .unwrap();
    assert_eq!(kw.as_str(), "const");

    // ... a plain failure is recoverable ...
    let mut input = Chars::from("x");
    let err = keyword::<_, Chars>("let").parse(&mut input).unwrap_err();
    assert!(!err.is_fatal());

    // ... while `fatal` stops every backtracking combinator.
    let err = keyword::<_, Chars>("let").fatal().parse(&mut input).unwrap_err();
    assert!(err.is_fatal());
}
```

## Tokenizer combinators

| Combinator | Description |
|---|---|
| `next(item)` | Matches one item equal to `item`. |
| `next_if(f)` | Matches one item accepted by `f`. |
| `keyword(kw)` | Matches a literal prefix. |
| `take_until(kw)` | Consumes everything up to `kw`, leaving it in the input. |
| `take_while(f)` | Longest run of items accepted by `f`; never fails. |
| `take_while_with(range, f)` | Like `take_while`, but the match length must fit `range`. |
| `take_till(f)` | Longest run of items rejected by `f`. |

## `Parser` adapters

| Adapter | Description |
|---|---|
| `map(f)` | Transforms the output on success. |
| `map_err(f)` | Rewrites the error on failure. |
| `ok()` | Turns a `non-fatal` failure into `None`, rewinding the input. |
| `or(p)` | Falls back to `p` after a `non-fatal` failure. |
| `fatal()` | Promotes every failure to `Fatal`. |
| `boxed()` | Boxes the output (shorthand for `map(Box::new)`). |

## Inputs

`bytes::TokenStream<'a>` and `chars::TokenStream<'a>` wrap a `&str` segment and
track its absolute offset in the whole source. Offsets and spans are measured
in bytes.

| Method | Description |
|---|---|
| `split_to(at)` | Consumes and returns the prefix `[0, at)`. |
| `split_off(at)` | Keeps `[0, at)` and returns the rest. |
| `iter()` / `iter_indices()` | Remaining items, optionally with offsets. |
| `start()` / `end()` | Absolute offsets in the whole source. |
| `to_span()` / `to_span_at(at)` | Region as a `Span`. |
| `starts_with(needle)` / `find(needle)` | `memchr`-backed match and search. |
| `as_str()` / `as_bytes()` | Raw views of the remaining input. |

## Syntax trees (feature `syntax`)

Implement `Syntax` for your AST nodes, or build them from the ready-made ones:

| Node | Description |
|---|---|
| `Char<I, C>`, `Byte<I, C>` | A single literal item. |
| `Option<T>`, `Box<T>`, `Vec<T>` | Optional, boxed, and repeated nodes. |
| `Delimiter<S, E, B>` | Body between `S` and `E`; a missing `E` is `Fatal`. |
| `Punctuated<T, P>` | `T (P T)* P?` sequences with pairs and a tail. |
| `Or<F, S>` | Ordered choice: try `F`, fall back to `S`. |
| `LimitsTo` / `Limits` / `LimitsFrom` | Bounds on the node's span length. |

```rust
use parserc::ParseError;
use parserc::chars::TokenStream;
use parserc::syntax::{Char, Delimiter, Punctuated, Syntax};

type Chars = TokenStream<'static>;
type A = Char<Chars, 'a'>;
type Comma = Char<Chars, ','>;

fn main() {
    // `a,a,a` is two `(item, separator)` pairs plus a trailing item.
    let mut input = Chars::from("a,a,a");
    let nodes = Punctuated::<A, Comma>::parse(&mut input).unwrap();
    assert_eq!(nodes.len(), 3);

    // An opened delimiter must be closed: `(aa` ends with a fatal error.
    type Parens = Delimiter<Char<Chars, '('>, Char<Chars, ')'>, Vec<A>>;
    let err = Parens::parse(&mut Chars::from("(aa")).unwrap_err();
    assert!(err.is_fatal());
}
```

For structs and enums, derive `Syntax` instead. Parsing rules go into
`#[parserc(...)]` attributes:

```rust
use parserc::{AsStr, chars::{self, CharsInput}, syntax::Syntax};

type TokenStream<'a> = chars::TokenStream<'a>;

/// An identifier: one or more ASCII alphabetic characters.
#[derive(Debug, PartialEq, Syntax)]
#[parserc(take_while = |c: char| c.is_ascii_alphabetic())]
struct Ident<I>(pub I)
where
    I: CharsInput;

fn main() {
    let mut input = TokenStream::from("hello world");
    let ident = Ident::parse(&mut input).unwrap();

    assert_eq!(ident.0.as_str(), "hello");
    assert_eq!(input.as_str(), " world");
}
```

Recognized keys are `ty_input`, `map_err`, `keyword`, `take_while`, `c` and
`semantic` on items, and `crucial`, `left_recursion`, `map_err`, `keyword`,
`take_while`, `parser` and `semantic` on fields. See
[`tests/derive.rs`](crates/parserc/tests/derive.rs) for left-recursion
detection and error promotion in action.

## Benchmarks

```sh
cargo bench -p parserc
```

Benchmarks are written with [`divan`](https://crates.io/crates/divan) and cover
the input primitives, every combinator and adapter, and composed tokenizer
loops. Each case reports latency plus a throughput counter (GB/s or
Mitem/s). Representative numbers:

| Case | Result |
|---|---|
| `adapters::bare` (one `next` parse) | ~1.3 ns |
| `adapters::boxed` | ~4.6 ns (heap allocation) |
| `combinators::take_until_absent`, 16 KiB | ~144 ns ≈ 113 GB/s |
| `combinators::take_while_full_run`, 16 KiB | ~4.0 µs ≈ 4 GB/s |
| `tokenize::csv_fields`, 2048 fields | ~4.1 µs ≈ 430 Mitem/s |

Use `cargo bench -p parserc -- --save-baseline main` and
`--load-baseline main` to compare future runs against a baseline.

## Feature flags

| Feature | Default | Enables |
|---|---|---|
| `input` | yes | `bytes::TokenStream` and `chars::TokenStream` (pulls in `memchr`) |
| `syntax` | yes | The `syntax` module and `#[derive(Syntax)]` (pulls in `parserc-derive`) |
| `serde` | yes | `serde` support for `Span` and the error types |

## Workspace

| Crate | Description |
|---|---|
| `parserc` | Combinators, inputs, error model, and AST building blocks. |
| `parserc-derive` | `#[derive(Syntax)]` and tuple syntax proc-macros. |
| `sourcespan` | `Span`, the source region type shared by the toolchain. |
| `unsyn` | A DSL for specifying concrete syntax trees. |

## Development

```sh
cargo test -p parserc     # unit tests, doctests and integration tests
cargo bench -p parserc    # divan benchmarks
```

## License

MIT.
