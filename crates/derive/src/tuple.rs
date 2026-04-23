use proc_macro::TokenStream;
use quote::quote;
use syn::{Error, LitInt, parse_macro_input};

pub fn derive_tuple_syntax(args: TokenStream) -> TokenStream {
    let len = parse_macro_input!(args as LitInt);

    let len = match len.base10_parse::<usize>() {
        Ok(num) => {
            if num < 3 {
                return Error::new(len.span(), "length argument must greater than 2.")
                    .into_compile_error()
                    .into();
            }

            num
        }
        Err(err) => return err.into_compile_error().into(),
    };

    let mut stmts = vec![];

    for i in 2..len {
        let mut types = vec![];

        let mut pos = vec![];

        for j in 0..i {
            types.push(
                format!("T{}", j)
                    .parse::<proc_macro2::TokenStream>()
                    .unwrap(),
            );

            pos.push(
                format!("self.{}", j)
                    .parse::<proc_macro2::TokenStream>()
                    .unwrap(),
            );
        }

        stmts.push(quote! {
            impl<I,#(#types),*> Syntax<I> for (#(#types),*)
            where
                I: Input,
                I::Error: ParseError,
                #(#types: Syntax<I>),*
            {
                #[inline]
                fn parse(input: &mut I) -> std::result::Result<Self, I::Error> {
                    #(
                        let #types = #types::parse(input)?;
                    )*

                    Ok((#(#types),*))
                }

                #[inline]
                fn to_span(&self) -> Span {
                    let mut lhs = Span::None;

                    #(
                        let lhs = lhs.join(&#pos.to_span());
                    )*

                    lhs
                }
            }
        });
    }

    quote! {
        #(#stmts)*
    }
    .into()
}
