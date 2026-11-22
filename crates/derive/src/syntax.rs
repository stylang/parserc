use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Error, Expr, Field, Fields, Item, ItemEnum, ItemStruct, Result, Type, parse_macro_input,
    spanned::Spanned,
};

use crate::combinator::{Combinator, Config, Position, check_reads_input};

pub fn derive_syntax(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse_macro_input!(input as Item);

    let derived = match item {
        Item::Enum(item) => derive_syntax_for_enum(item),
        Item::Struct(item) => derive_syntax_for_struct(item),
        _ => {
            return Error::new(
                item.span(),
                "proc_macro `Syntax` can only derive `struct` or `enum`.",
            )
            .into_compile_error()
            .into();
        }
    };

    match derived {
        Ok(token_stream) => token_stream.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// What one `struct` or `enum` declares, once its attributes are read.
struct Item_ {
    ident: syn::Ident,
    generics: syn::Generics,
    /// The `input` type, the item's own if it says one.
    ty_input: Type,
    map_err: Option<Expr>,
    semantic: Option<Expr>,
    /// How the whole item is read, if it is not read from its parts.
    combinator: Combinator,
}

impl Item_ {
    fn new_enum(item: &ItemEnum) -> Result<Self> {
        let config = Config::parse(&item.attrs, Position::Item)?;

        Ok(Self {
            ident: item.ident.clone(),
            generics: item.generics.clone(),
            ty_input: config.ty_input,
            map_err: config.map_err,
            semantic: config.semantic,
            combinator: config.combinator,
        })
    }

    fn new_struct(item: &ItemStruct) -> Result<Self> {
        let config = Config::parse(&item.attrs, Position::Item)?;

        Ok(Self {
            ident: item.ident.clone(),
            generics: item.generics.clone(),
            ty_input: config.ty_input,
            map_err: config.map_err,
            semantic: config.semantic,
            combinator: config.combinator,
        })
    }

    fn map_err_tokens(&self) -> TokenStream {
        match &self.map_err {
            Some(map_err) => quote! { .map_err(#map_err) },
            None => quote! {},
        }
    }

    /// The `impl` header parts, split once so both shapes of generated item
    /// write them the same way.
    fn impl_parts(
        &self,
    ) -> (
        syn::ImplGenerics<'_>,
        syn::TypeGenerics<'_>,
        Option<&syn::WhereClause>,
    ) {
        self.generics.split_for_impl()
    }
}

/// The whole of one generated `Syntax` impl for a thing that is read as a run
/// of the input rather than from parts: a `struct`, an `enum` of one variant,
/// or a variant on its own.
///
/// `ctor` is what the run is put into, and `to_span` how the span of one of
/// those is read back off it.
fn leaf_impl(
    item: &Item_,
    ctor: TokenStream,
    body_to_span: TokenStream,
) -> Result<proc_macro2::TokenStream> {
    let Item_ {
        ident,
        ty_input,
        combinator,
        ..
    } = item;

    let map_err = item.map_err_tokens();
    let (impl_generics, type_generics, where_clause) = item.impl_parts();
    // An item is read once, with nothing beside it to fall back on: all that
    // stands between the wrap and the read is its own `map_err`.
    let read = combinator.read_into(ctor, map_err);

    Ok(quote! {
        impl #impl_generics parserc::syntax::Syntax<#ty_input> for #ident #type_generics #where_clause {
            #[inline]
            fn parse(input: &mut #ty_input) -> Result<Self, <#ty_input as parserc::Input>::Error> {
                use parserc::{ParseError, Parser};

                #read
            }

            #[inline]
            fn to_span(&self) -> parserc::Span {
                use parserc::ToSpan;

                #body_to_span
            }
        }
    })
}

/// The one field an item or arm read as a run of the input has to have, and
/// the way to name its type.
///
/// Refused here rather than left to the compiler: without this the generated
/// `Self(taken)` against a struct of three fields fails in a body the author
/// never wrote, pointing at nothing they can act on.
fn only_field<'a>(
    fields: &'a Fields,
    ty_input: &Type,
    combinator: &Combinator,
    what: &str,
) -> Result<&'a Field> {
    let Fields::Unnamed(unnamed) = fields else {
        return Err(Error::new(
            fields.span(),
            format!(
                "`{}` on {} reads one run of the input, which it can only hold in one unnamed field",
                combinator.name(),
                what,
            ),
        ));
    };

    if unnamed.unnamed.len() != 1 {
        return Err(Error::new(
            fields.span(),
            format!(
                "`{}` on {} reads one run of the input and has nowhere else to put it, so it needs exactly one field",
                combinator.name(),
                what,
            ),
        ));
    }

    let field = unnamed.unnamed.first().expect("one field");

    if field.ty.to_token_stream().to_string() != ty_input.to_token_stream().to_string() {
        return Err(Error::new(
            field.ty.span(),
            format!(
                "`{}` reads the input, so {what}'s one field has to be its type `{}`",
                combinator.name(),
                ty_input.to_token_stream(),
            ),
        ));
    }

    Ok(field)
}

/// The read of one field, and the expression that builds it into what holds it.
///
/// This is the whole of what a field says, written once for the fields of a
/// `struct` and the fields of an `enum` variant, which are the same question:
/// how is this operand read, what does a failure here do to the reads around
/// it, and what is asked of the value once it arrives.
///
/// `into_fatal` is what a failure of *this* field is turned into, and is the
/// caller's to carry between fields: `crucial` says that from here on a failure
/// stops the item, which is a fact about the fields after it and not about the
/// one it is written on.
fn field_read(
    field: &Field,
    ty_input: &Type,
    map_err_default: &TokenStream,
    into_fatal: &mut proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream> {
    let config = Config::parse(&field.attrs, Position::Field)?;

    let Config {
        crucial,
        map_err,
        left_recursion,
        combinator,
        semantic,
        ..
    } = config;

    check_reads_input(&combinator, &field.ty, ty_input)?;

    let map_err = match &map_err {
        Some(map_err) => quote! { .map_err(#map_err) },
        None => map_err_default.clone(),
    };

    let read = combinator.read(ty_input);

    // A guard against a field reached again while it is being read, which is
    // what a left-recursive rule would otherwise loop on forever.
    let read = if left_recursion {
        quote! {
            {
                thread_local! {
                    static GUARD: std::cell::RefCell<bool> = std::cell::RefCell::new(false);
                }

                if GUARD.replace(true) {
                    Err(parserc::Kind::LeftRecursion(
                        parserc::ControlFlow::Recovable,
                        input.to_span_at(1),
                    )
                    .into()) #map_err #into_fatal
                } else {
                    let taken = #read #map_err #into_fatal;

                    GUARD.replace(false);

                    taken
                }
            }
        }
    } else {
        quote! { #read #map_err #into_fatal }
    };

    let read = match &semantic {
        Some(semantic) => quote! { #semantic(input.clone(), #read?) },
        None => quote! { #read? },
    };

    if crucial {
        *into_fatal = quote! { .map_err(|err| err.into_fatal()) };
    }

    Ok(match &field.ident {
        Some(ident) => quote! { #ident: #read },
        None => quote! { #read },
    })
}

/// The span of every field of an item or arm, joined, in the shape `armed`
/// says they are named: bound by `match` for an arm, reached through `self`
/// for a `struct`.
fn spans_of(fields: &Fields, armed: bool) -> Vec<proc_macro2::TokenStream> {
    fields
        .members()
        .map(|member| {
            let access = match &member {
                syn::Member::Named(name) if armed => quote! { #name },
                syn::Member::Named(name) => quote! { self.#name },
                syn::Member::Unnamed(index) if armed => {
                    let name = format_ident!("ident_{}", index);

                    quote! { #name }
                }
                syn::Member::Unnamed(index) => quote! { self.#index },
            };

            quote! { #access.to_span() }
        })
        .collect()
}

/// The pattern one variant's fields bind to, in the same order as
/// [`spans_of`] read them back.
fn arm_pattern(variant: &syn::Variant, ident: &syn::Ident) -> proc_macro2::TokenStream {
    let variant_ident = &variant.ident;
    let names = variant.fields.members().map(|member| match member {
        syn::Member::Named(name) => quote! { #name },
        syn::Member::Unnamed(index) => {
            let name = format_ident!("ident_{}", index);

            quote! { #name }
        }
    });

    match &variant.fields {
        Fields::Named(_) => quote! { #ident::#variant_ident { #(#names),* } },
        _ => quote! { #ident::#variant_ident( #(#names),* ) },
    }
}

fn derive_syntax_for_struct(item: ItemStruct) -> Result<proc_macro2::TokenStream> {
    let parsed = Item_::new_struct(&item)?;

    // An item that says how it is read itself is one field of the input and
    // nothing else; the attributes have already refused the combination that
    // would have meant something else.
    if !matches!(parsed.combinator, Combinator::Syntax) {
        only_field(
            &item.fields,
            &parsed.ty_input,
            &parsed.combinator,
            "a `struct`",
        )?;

        let ident = &parsed.ident;

        return leaf_impl(&parsed, quote! { #ident }, quote! { self.0.to_span() });
    }

    let map_err_default = parsed.map_err_tokens();
    let mut into_fatal = quote! {};

    let parse_fields = item
        .fields
        .iter()
        .map(|field| field_read(field, &parsed.ty_input, &map_err_default, &mut into_fatal))
        .collect::<Result<Vec<_>>>()?;

    let to_spans = spans_of(&item.fields, false);

    let built = if item.semi_token.is_some() {
        quote! { Self(#(#parse_fields),*) }
    } else {
        quote! { Self { #(#parse_fields),* } }
    };

    let built = match &parsed.semantic {
        Some(semantic) => quote! { #semantic(input.clone(), #built) },
        None => quote! { Ok(#built) },
    };

    let ident = &parsed.ident;
    let ty_input = &parsed.ty_input;
    let (impl_generics, type_generics, where_clause) = parsed.impl_parts();

    Ok(quote! {
        impl #impl_generics parserc::syntax::Syntax<#ty_input> for #ident #type_generics #where_clause {
            #[inline]
            fn parse(input: &mut #ty_input) -> Result<Self, <#ty_input as parserc::Input>::Error> {
                use parserc::Parser;
                use parserc::ParseError;
                use parserc::syntax::SyntaxInput;

                #built
            }

            #[inline]
            fn to_span(&self) -> parserc::Span {
                use parserc::ToSpan;

                let mut lhs = parserc::Span::None;
                #(
                    lhs = lhs.union(&#to_spans);
                )*

                lhs
            }
        }
    })
}

fn derive_syntax_for_enum(item: ItemEnum) -> Result<proc_macro2::TokenStream> {
    let parsed = Item_::new_enum(&item)?;
    let ident = &parsed.ident;

    // An `enum` that says how it is read itself is exactly as much of a run of
    // the input as a `struct` is: one variant, holding one field of it. What it
    // cannot be is a choice *and* a leaf at once, which is what the old code
    // answered by refusing the option on every `enum`.
    if !matches!(parsed.combinator, Combinator::Syntax) {
        if item.variants.len() != 1 {
            return Err(Error::new(
                item.variants.span(),
                format!(
                    "`{}` on an `enum` reads the item as one run of the input, which leaves nothing \
                     for a second variant to answer with",
                    parsed.combinator.name(),
                ),
            ));
        }

        let variant = item.variants.first().expect("one variant");

        only_field(
            &variant.fields,
            &parsed.ty_input,
            &parsed.combinator,
            "an `enum`",
        )?;

        let variant_ident = &variant.ident;

        let to_span = match &variant.fields {
            Fields::Named(_) => {
                return Err(Error::new(
                    variant.fields.span(),
                    "an `enum` read as one run of the input holds it in one unnamed field",
                ));
            }
            _ => quote! { let #ident::#variant_ident(taken) = self; taken.to_span() },
        };

        return leaf_impl(&parsed, quote! { #ident::#variant_ident }, to_span);
    }

    let map_err = parsed.map_err_tokens();
    let ty_input = &parsed.ty_input;

    let arms = item
        .variants
        .iter()
        .map(|variant| arm_read(variant, &parsed))
        .collect::<Result<Vec<_>>>()?;

    let (parse_arms, to_spans): (Vec<_>, Vec<_>) = arms.into_iter().unzip();

    let (impl_generics, type_generics, where_clause) = parsed.impl_parts();
    let name = ident.to_string();

    Ok(quote! {
        impl #impl_generics parserc::syntax::Syntax<#ty_input> for #ident #type_generics #where_clause {
            #[inline]
            fn parse(input: &mut #ty_input) -> Result<Self, <#ty_input as parserc::Input>::Error> {
                use parserc::Parser;
                use parserc::ParseError;
                use parserc::syntax::SyntaxInput;

                #(#parse_arms)*

                Err(parserc::Kind::Syntax(
                    #name,
                    parserc::ControlFlow::Recovable,
                    input.to_span_at(1),
                )
                .into())#map_err
            }

            #[inline]
            fn to_span(&self) -> parserc::Span {
                use parserc::ToSpan;

                match self {
                    #(#to_spans),*
                }
            }
        }
    })
}

/// The two halves of one variant: the statements that try it, and the `match`
/// arm that reads its span back.
fn arm_read(
    variant: &syn::Variant,
    item: &Item_,
) -> Result<(proc_macro2::TokenStream, proc_macro2::TokenStream)> {
    let ident = &item.ident;
    let variant_ident = &variant.ident;
    let ty_input = &item.ty_input;
    let config = Config::parse(&variant.attrs, Position::Arm)?;

    if config.crucial || config.left_recursion {
        return Err(Error::new(
            variant.span(),
            "unreachable: `Config::parse` refuses these off a field",
        ));
    }

    let map_err = match &config.map_err {
        Some(map_err) => quote! { .map_err(#map_err) },
        None => quote! {},
    };

    let pattern = arm_pattern(variant, ident);
    let to_spans = spans_of(&variant.fields, true);
    let to_span = quote! {
        #pattern => {
            let mut lhs = parserc::Span::None;
            #(
                lhs = lhs.union(&#to_spans);
            )*

            lhs
        }
    };

    // A variant that says how it is read is a leaf of the choice, holding one
    // run of the input where the others hold their parts.
    if !matches!(config.combinator, Combinator::Syntax) {
        only_field(&variant.fields, ty_input, &config.combinator, "a variant")?;

        let read = config.combinator.read_into(
            quote! { #ident::#variant_ident },
            // One arm failing is the next arm's turn, exactly as it is when the
            // arm is read from its parts. The relabelling goes first so that a
            // failure this arm was told to report fatally is not turned back
            // into a retry by the `.ok()` after it.
            quote! { #map_err.ok() },
        );

        let parse = quote! {
            if let Some(taken) = #read? {
                return Ok(taken);
            }
        };

        return Ok((parse, to_span));
    }

    let mut into_fatal = quote! {};
    let map_err_default = &map_err;

    let parse_fields = variant
        .fields
        .iter()
        .map(|field| field_read(field, ty_input, map_err_default, &mut into_fatal))
        .collect::<Result<Vec<_>>>()?;

    let built = match &variant.fields {
        Fields::Named(_) => quote! { #ident::#variant_ident { #(#parse_fields),* } },
        _ => quote! { #ident::#variant_ident( #(#parse_fields),* ) },
    };

    let built = match &config.semantic {
        Some(semantic) => quote! { #semantic(input.clone(), #built) },
        None => quote! { Ok(#built) },
    };

    let parse = quote! {
        let parser = |input: &mut #ty_input| {
            use parserc::syntax::SyntaxInput;

            #built
        };

        if let Some(taken) = parser.ok().parse(input)? {
            return Ok(taken);
        }
    };

    Ok((parse, to_span))
}
