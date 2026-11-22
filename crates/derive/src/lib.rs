//! Procedural macros for the `parserc` parser combinator library.
//!
//! The generated code refers to `parserc::syntax`, so `parserc` must be
//! available in the final dependency graph. From user code these macros are
//! reached through `parserc::syntax::Syntax`.

mod combinator;
mod syntax;
mod tuple;

/// Implements `parserc::syntax::Syntax` for tuples of `Syntax` nodes.
///
/// `N` must be greater than `2`; the macro generates impls for tuples of
/// `2..N` elements. `parserc` invokes it as `derive_tuple_syntax!(16)`.
#[proc_macro]
pub fn derive_tuple_syntax(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    tuple::derive_tuple_syntax(args)
}

/// Implements `parserc::syntax::Syntax` for `struct`s and `enums`.
///
/// A `struct` is read by parsing its fields in declaration order, and an `enum`
/// by trying each variant in declaration order; the types those fields name
/// must implement `Syntax` themselves. Per-item, per-variant and per-field
/// options live in `#[parserc(...)]` attributes.
///
/// Every combinator of `parserc` may be written at every one of those
/// three positions, under the name of the function it calls: `next`, `next_if`,
/// `keyword`, `take_until`, `take_while`, `take_while_with`, `take_till`, and
/// `parser` for a parser given whole. `take_while_with` is written as the pair
/// it is called with — `take_while_with = (1.., |c: char| …)`.
///
/// Three options are about a position rather than about a read: `input = Type`
/// names the type an item is generic over and belongs to an item alone, and
/// `crucial` and `left_recursion` say what one failure does to the reads beside
/// it and belong to a field alone. `map_err` and `semantic` are accepted
/// anywhere.
#[proc_macro_derive(Syntax, attributes(parserc))]
pub fn derive_syntax(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syntax::derive_syntax(input)
}
