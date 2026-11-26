# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for
compatibility with GitHub comment style markdown rendering.
-->

## [0.14.0] - 2026-11-26

- unsyn: the DSL is now compiled. A `.us` file — `whitespace`, `lexer` and `syntax` productions — is read into a CST and turned into `parserc` syntax: a `Syntax` type per rule, the parsers its right side writes, and a `syn`-shaped visitor. This replaces the hand-written `lib.uns` self-description with a spec, `unsyn.us`, and `crates/unsyn/README.md` is the reference for the language and for what every production becomes.
- unsyn: `Builder::to_dir` writes one file per kind of code — `input.rs`, `error.rs`, `lexer.rs`, `syntax.rs`, `visit.rs` — each importing what it reads of its neighbours, and `Builder::modules` hands back the same parts without touching the filesystem. A build is build-time only: no formatting, no `cargo:` instructions, no runtime feature.
- unsyn: `Builder::project("calc_lang")` names the language, which names the two items that are no rule of it — `CalcLangInput` and `CalcLangError`. `Builder::input_at` and `Builder::error_at` say a stream trait or an error stands elsewhere, and then no file is written for it.
- unsyn: `<Ext>` is an external link and nothing more: `Builder::external` is the only way anything outside the file enters a build, and the generator holds no table of what any external means and never will.
- unsyn: added the semantic pass. Refusals include left recursion, same-offset recursion, an undefined name, a name collision, a missing `whitespace` declaration, an empty or wider set member, and an invalid range, repetition bound or code point. A second channel carries lints — what is legal but says nothing: an alternative that names no node, the separator asked for as a field, set members compared as values. A report holds spans, never file text.
- unsyn: the error a build writes carries the framework's `Kind` and one family of rules, and that family is written from the file alone: only the rules whose own code names a refusal after itself, in the order the file declares them.
- unsyn: `Box` appears where two rules stand on a cycle of containment, and nowhere else.
- unsyn: removed the `followed` operator. The language has no lookahead at all, so there was nothing to assert: `except` bounds a run of text, and `~` refuses an operand and takes one character.
- unsyn: `examples/bootstrap` closes the loop — the spec is read by the parser a build wrote for it, and the crate's own analysis, generation and visitor run on that tree and write back the same files. `cargo run -p unsyn --features bootstrap --example bootstrap`.
- parserc: added the combinator `text_of`, which answers with the run of items another parser read, whatever that parser answered with.
- parserc: added the `ToSpan` trait and made it a supertrait of `Input`, so that a stream and a node report a region the same way; `Syntax` now requires it, and `as_str`/`as_bytes` cut the source text of any node out of any stream over the same file, without taking the node apart.
- derive: `#[parserc(..)]` combinators are accepted at every position — item, struct, enum, field and variant — and not on fields alone, and a bare path is a parser (`#[parserc(read_x)]` where it used to take `#[parserc(parser = read_x)]`).
- derive: a `struct` or `enum` can be one token, and a variant can be a leaf of a choice beside arms read from parts.
- sourcespan: `Span::shift` returns a plain range instead of a `Span` recounted from another point. Added `start`, `end`, `as_range`, `width`, `is_empty`, `intersect`, `contains` and `offset`.
- opgen: added the crate, with the opcode interface definitions.

## [0.13.1] - 2026-09-22

- fix release bug

## [0.13.0] - 2026-09-22

- take_while_*: now `non-greedy` matching alogrithem.

## [0.12.7] - 2026-01-02

- fixed `semantic` option derive bug.

## [0.12.6] - 2025-12-27

- add new trait `ToSpan`

## [0.12.5] - 2025-12-25

- derive: field attribute `parserc` now supports `semantic` option.

## [0.12.4] - 2025-12-24

- parser combinator `take_while_*`: modified to `non-greedy` matching alogrithem.

## [0.12.3] - 2025-12-18

- parser combinator `take_while_*`: fixed range bugs.

## [0.12.2] - 2025-12-09

- Fixed an error: syntax::Delimiter will now throw a `fatal` error when unable
  to parse the closing delimiter.

## [0.12.1] - 2025-12-09

- remove: Kind::Delimiter

## [0.12.0] - 2025-12-09

- rename `InputSyntaxEx` to `SyntaxInput`.
- remove trait `PartialSyntax`.

## [0.11.10] - 2025-12-08

- Concrete syntax tree: add `left recursion detect` switch attribute `parserc(left_recursion)`.

## [0.11.9] - 2025-12-08

- derive: semantic signature is changed to: `fn(I,T) -> Result<T,E>`.

## [0.11.8] - 2025-12-08

- derive: now `parser` argument in `#[parserc(parser)]` can returns any type value.

## [0.11.7] - 2025-11-30

- `Punctuated`: add `len` and `is_empty` fncs.

## [0.11.6] - 2025-11-22

- derive: item `parserc` attribute add `semantic` cfg.

## [0.11.5] - 2025-11-21

- derive: `parserc` attribute add meta cfg `take_while` and `parser`.

## [0.11.4] - 2025-11-20

- ParseError: add `is_fatal` function.

## [0.11.3] - 2025-11-19

- support attribute `#[parserc(keyword = "xxx")]` on struct/enum field.

## [0.11.2] - 2025-11-19

- drive `Ord` and `Eq` for syntax `Or`.

## [0.11.1] - 2025-11-18

- fixed the `span` bug of the `next_if`,`next` functions.

## [0.11.0] - 2025-11-17

- new `proc-macro` syntax.

## [0.10.11] - 2025-11-06

- transfer ownership to `stylang` organization.

## [0.10.10] - 2025-11-04

- `take_until`: Returns error if pattern is not found.

## [0.10.9] - 2025-11-03

- `span`: implement `ops::Add` for `Span` type.

## [0.10.8] - 2025-11-03

- derive `syntax` add option `map_err`.

## [0.10.7] - 2025-11-01

- fixed derive `token!` bug.

## [0.10.6] - 2025-11-01

- fixed compile bug: remove unnecessary `E` generic parameter.

## [0.10.5] - 2025-10-30

- derive: add `token!` proc-macro.

## [0.10.4] - 2025-10-30

- syntax: add `Limits*` syntax nodes.
- syntax: add `to_span` function to the `Syntax` trait.

## [0.10.3] - 2025-10-29

- Change `ParseError` api: add `span` function.

## [0.10.2] - 2025-10-29

- Fixed some `derive` bugs.

## [0.10.1] - 2025-10-29

- Support `syntax` drive from enum types.

## [0.10.0] - 2025-10-25

- Rewrite using the new architecture
