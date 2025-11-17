# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for
compatibility with GitHub comment style markdown rendering.
-->

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
