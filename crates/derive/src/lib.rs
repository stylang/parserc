mod syntax;
mod tuple;

/// Derive `Syntax` trait for tuples (T,...)
#[proc_macro]
pub fn derive_tuple_syntax(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    tuple::derive_tuple_syntax(args)
}

/// Derive `Syntax` trait for `struct`s / `enum`s.
#[proc_macro_derive(Syntax, attributes(parserc))]
pub fn derive_syntax(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syntax::derive_syntax(input)
}
