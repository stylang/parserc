# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for
compatibility with GitHub comment style markdown rendering.
-->

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
