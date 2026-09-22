//! Procedural macros for the `parserc` parser combinator library.
//!
//! The generated code refers to `parserc::syntax`, so `parserc` must be
//! available in the final dependency graph. From user code these macros are
//! reached through `parserc::syntax::Syntax`.

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

/// Implements `parserc::syntax::Syntax` for `struct`s and `enum`s.
///
/// Fields are parsed in declaration order and must implement `Syntax`
/// themselves. Per-item and per-field options live in `#[parserc(...)]`
/// attributes; the recognized keys are `ty_input`, `map_err`, `keyword`,
/// `take_while`, `c` and `semantic` on items, and `crucial`, `left_recursion`,
/// `map_err`, `keyword`, `take_while`, `parser` and `semantic` on fields.
#[proc_macro_derive(Syntax, attributes(parserc))]
pub fn derive_syntax(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syntax::derive_syntax(input)
}
