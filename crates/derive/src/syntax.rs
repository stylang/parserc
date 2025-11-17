use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Error, Expr, ExprClosure, Fields, Item, ItemEnum, ItemStruct, Lit, Result, Type,
    parse::Parser, parse_macro_input, spanned::Spanned,
};

pub fn derive_syntax(input: TokenStream) -> TokenStream {
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

struct ItemConfig {
    ty_input: Type,
    map_err: Option<Expr>,
    keyword: Option<Lit>,
    token: Option<ExprClosure>,
    c: Option<Lit>,
}

impl Default for ItemConfig {
    fn default() -> Self {
        Self {
            ty_input: syn::parse2(quote! { I }).unwrap(),
            map_err: None,
            keyword: None,
            token: None,
            c: None,
        }
    }
}

impl ItemConfig {
    fn parse(attrs: &[Attribute]) -> Result<ItemConfig> {
        let met_lists = attrs
            .iter()
            .filter_map(|syntax| {
                if syntax.path().is_ident("parserc") {
                    match &syntax.meta {
                        syn::Meta::Path(path) => {
                            return Some(Err(Error::new(
                                path.span(),
                                "Empty body, expect `parserc(...)`",
                            )));
                        }
                        syn::Meta::List(meta_list) => return Some(Ok(meta_list)),
                        syn::Meta::NameValue(value) => {
                            return Some(Err(Error::new(value.span(), "Unsupport syntax.")));
                        }
                    };
                }

                None
            })
            .collect::<Result<Vec<_>>>()?;

        if met_lists.is_empty() {
            return Ok(Default::default());
        };

        let mut ty_input: Option<Type> = None;
        let mut map_err: Option<Expr> = None;
        let mut keyword: Option<Lit> = None;
        let mut c: Option<Lit> = None;
        let mut token: Option<ExprClosure> = None;

        for meta_list in met_lists {
            let parser = syn::meta::parser(|meta| {
                macro_rules! error {
                ($($t:tt)+) => {
                    return Err(meta.error(format_args!($($t)+)))
                };
            }

                let Some(ident) = meta.path.get_ident() else {
                    error!("Unsupport macro `syntax` option.");
                };

                if ident == "input" {
                    if ty_input.is_some() {
                        error!("Call `input` twice.");
                    }
                    ty_input = Some(meta.value()?.parse()?);
                } else if ident == "map_err" {
                    if map_err.is_some() {
                        error!("Call `map_err` twice.");
                    }
                    map_err = Some(meta.value()?.parse()?);
                } else if ident == "keyword" {
                    if token.is_some() || c.is_some() {
                        error!("The syntax has been set as a `token` or `char`.");
                    }

                    if keyword.is_some() {
                        error!("Call `keyword` twice.");
                    }

                    keyword = Some(meta.value()?.parse()?);
                } else if ident == "token" {
                    if keyword.is_some() || c.is_some() {
                        error!("The syntax has been set as a `keyword` or `char`.");
                    }

                    if token.is_some() {
                        error!("Call `token` twice.");
                    }

                    token = Some(meta.value()?.parse()?);
                } else if ident == "char" {
                    if keyword.is_some() || token.is_some() {
                        error!("The syntax has been set as a `keyword` or `token`.");
                    }

                    if c.is_some() {
                        error!("Call `char` twice.");
                    }

                    c = Some(meta.value()?.parse()?);
                } else {
                    error!("Unsupport macro `syntax` option `{}`.", ident);
                }

                Ok(())
            });

            parser.parse2(meta_list.tokens.to_token_stream())?;
        }

        if let Some(ty_input) = ty_input {
            Ok(ItemConfig {
                ty_input,
                map_err,
                keyword,
                token,
                c,
            })
        } else {
            Ok(ItemConfig {
                map_err,
                keyword,
                token,
                c,
                ..Default::default()
            })
        }
    }
}

#[derive(Default)]
struct FieldConfig {
    crucial: bool,
    map_err: Option<Expr>,
}

impl FieldConfig {
    fn parse(attrs: &[Attribute]) -> Result<Self> {
        let met_lists = attrs
            .iter()
            .filter_map(|syntax| {
                if syntax.path().is_ident("parserc") {
                    match &syntax.meta {
                        syn::Meta::Path(path) => {
                            return Some(Err(Error::new(
                                path.span(),
                                "Empty body, expect `parserc(...)`",
                            )));
                        }
                        syn::Meta::List(meta_list) => return Some(Ok(meta_list)),
                        syn::Meta::NameValue(value) => {
                            return Some(Err(Error::new(value.span(), "Unsupport syntax.")));
                        }
                    };
                }

                None
            })
            .collect::<Result<Vec<_>>>()?;

        if met_lists.is_empty() {
            return Ok(Default::default());
        };

        let mut crucial = false;
        let mut map_err: Option<Expr> = None;

        for meta_list in met_lists {
            let parser = syn::meta::parser(|meta| {
                macro_rules! error {
                ($($t:tt)+) => {
                    return Err(meta.error(format_args!($($t)+)))
                };
            }

                let Some(ident) = meta.path.get_ident() else {
                    error!("Unsupport macro `parserc` option.");
                };

                if ident == "crucial" {
                    crucial = true;
                } else if ident == "map_err" {
                    if map_err.is_some() {
                        error!("Call `map_err` twice.");
                    }
                    map_err = Some(meta.value()?.parse()?);
                } else {
                    error!("Unsupport macro `parserc` option `{}`.", ident);
                }

                Ok(())
            });

            parser.parse2(meta_list.tokens.to_token_stream())?;
        }

        Ok(FieldConfig { crucial, map_err })
    }
}

fn derive_syntax_for_enum(item: ItemEnum) -> Result<proc_macro2::TokenStream> {
    let ItemConfig {
        ty_input,
        map_err,
        keyword,
        token,
        c,
    } = ItemConfig::parse(&item.attrs)?;

    match (keyword, token, c) {
        (None, Some(param), None) => {
            return Err(Error::new(
                param.span(),
                "Deriving `token` from an enumeration is not supported.",
            ));
        }
        (Some(param), None, None) => {
            return Err(Error::new(
                param.span(),
                "Deriving `keyword` from an enumeration is not supported.",
            ));
        }
        (None, None, Some(param)) => {
            return Err(Error::new(
                param.span(),
                "Deriving `char` from an enumeration is not supported.",
            ));
        }
        _ => {}
    }

    let ident = &item.ident;
    let ident_str = ident.to_string();

    let map_err = if let Some(map_err) = map_err {
        quote! {
            .map_err(#map_err)
        }
    } else {
        quote! {}
    };

    let (impl_generic, type_generic, where_clause) = item.generics.split_for_impl();

    let (fields, to_spans): (Vec<_>, Vec<_>) = item
        .variants
        .iter()
        .map(|varint| {
            let variant_ident = &varint.ident;

            let mut into_fatal = quote! {};

            let parse_fields = varint
                .fields
                .iter()
                .map(|field| {
                    let FieldConfig { crucial, map_err } = FieldConfig::parse(&field.attrs)?;

                    let map_err = if let Some(map_err) = map_err {
                        quote! {
                            .map_err(#map_err)
                        }
                    } else {
                        quote! {}
                    };

                    let result = match &field.ident {
                        Some(ident) => Ok(quote! {
                            #ident: input.parse()#map_err #into_fatal?
                        }),
                        None => Ok(quote! {input.parse()#map_err #into_fatal?}),
                    };

                    if crucial {
                        into_fatal = quote! {
                            .map_err(|err| err.into_fatal())
                        };
                    }

                    result
                })
                .collect::<Result<Vec<_>>>()?;

            let to_spans = varint
                .fields
                .members()
                .map(|member| match member {
                    syn::Member::Named(ident) => {
                        quote! {
                           #ident.to_span()
                        }
                    }
                    syn::Member::Unnamed(index) => {
                        let ident = format_ident!("ident_{}", index);
                        quote! {
                            #ident.to_span()
                        }
                    }
                })
                .collect::<Vec<_>>();

            let parse = if let Fields::Named(_) = &varint.fields {
                quote! {
                    Ok(#ident::#variant_ident {
                        #(#parse_fields),*
                    })
                }
            } else {
                quote! {
                    Ok(#ident::#variant_ident(#(#parse_fields),*))
                }
            };

            let field_idents = varint
                .fields
                .members()
                .map(|member| match member {
                    syn::Member::Named(ident) => ident,
                    syn::Member::Unnamed(index) => format_ident!("ident_{}", index),
                })
                .collect::<Vec<_>>();

            let match_arm = if let Fields::Named(_) = &varint.fields {
                quote! { Self::#variant_ident { #(#field_idents),* } }
            } else {
                quote! { Self::#variant_ident ( #(#field_idents),* ) }
            };

            let parse = quote! {
                let parser = | input: &mut #ty_input | {
                        use parserc::syntax::InputSyntaxExt;
                        #parse
                };

                if let Some(value) = parser.ok().parse(input)? {
                    return Ok(value);
                }
            };

            let to_span = quote! {
                #match_arm => {
                    let mut lhs = parserc::Span::None;
                    #(
                        lhs = lhs.union(&#to_spans);
                    )*

                    lhs
                }
            };

            Ok((parse, to_span))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .unzip();

    Ok(quote! {
        impl #impl_generic parserc::syntax::Syntax<#ty_input> for #ident #type_generic #where_clause {
            #[inline]
            fn parse(input: &mut #ty_input) -> Result<Self, <#ty_input as parserc::Input>::Error> {
                use parserc::syntax::InputSyntaxExt;
                use parserc::Parser;
                use parserc::ParseError;
                #(#fields)*

                Err(parserc::Kind::Syntax(#ident_str,parserc::ControlFlow::Recovable,input.to_span()).into())#map_err
            }

            #[inline]
            fn to_span(&self) -> parserc::Span {
                match self {
                    #(#to_spans),*
                }
            }
        }
    })
}

fn derive_syntax_for_struct(item: ItemStruct) -> Result<proc_macro2::TokenStream> {
    let ItemConfig {
        ty_input,
        map_err,
        keyword,
        token,
        c,
    } = ItemConfig::parse(&item.attrs)?;

    let ident = &item.ident;

    let map_err = if let Some(map_err) = map_err {
        quote! {
            .map_err(#map_err)
        }
    } else {
        quote! {}
    };

    let (impl_generic, type_generic, where_clause) = item.generics.split_for_impl();

    let mut into_fatal = quote! {};

    let parse_fields = item
        .fields
        .iter()
        .map(|field| {
            let FieldConfig { crucial, map_err } = FieldConfig::parse(&field.attrs)?;

            let map_err = if let Some(map_err) = map_err {
                quote! {
                    .map_err(#map_err)
                }
            } else {
                quote! {}
            };

            let result = match &field.ident {
                Some(ident) => Ok(quote! {
                    #ident: input.parse() #map_err #into_fatal?
                }),
                None => Ok(quote! {input.parse()#map_err #into_fatal?}),
            };

            if crucial {
                into_fatal = quote! {
                    .map_err(|err| err.into_fatal())
                };
            }

            result
        })
        .collect::<Result<Vec<_>>>()?;

    let to_spans = item
        .fields
        .members()
        .map(|member| match member {
            syn::Member::Named(ident) => {
                quote! {
                   self.#ident.to_span()
                }
            }
            syn::Member::Unnamed(index) => {
                quote! {
                    self.#index.to_span()
                }
            }
        })
        .collect::<Vec<_>>();

    let parse = if item.semi_token.is_some() {
        quote! {
            Ok(Self(#(#parse_fields),*))
        }
    } else {
        quote! {
            Ok(Self {
                #(#parse_fields),*
            })
        }
    };

    if let Some(keyword) = keyword {
        Ok(quote! {
            impl #impl_generic parserc::syntax::Syntax<#ty_input> for #ident #type_generic #where_clause {
                #[inline]
                fn parse(input: &mut #ty_input) -> Result<Self, <#ty_input as parserc::Input>::Error> {
                    use parserc::Parser;
                    parserc::keyword(#keyword).map(|input| Self(input)).parse(input)#map_err
                }

                #[inline]
                fn to_span(&self) -> parserc::Span {
                    self.0.to_span()
                }
            }
        })
    } else if let Some(token) = token {
        Ok(quote! {
            impl #impl_generic parserc::syntax::Syntax<#ty_input> for #ident #type_generic #where_clause {
                #[inline]
                fn parse(input: &mut #ty_input) -> Result<Self, <#ty_input as parserc::Input>::Error> {
                    use parserc::Parser;
                    parserc::take_while_range_from(1, #token).map(|input| Self(input)).parse(input)#map_err
                }

                #[inline]
                fn to_span(&self) -> parserc::Span {
                    self.0.to_span()
                }
            }
        })
    } else if let Some(c) = c {
        Ok(quote! {
            impl #impl_generic parserc::syntax::Syntax<#ty_input> for #ident #type_generic #where_clause {
                #[inline]
                fn parse(input: &mut #ty_input) -> Result<Self, <#ty_input as parserc::Input>::Error> {
                    use parserc::Parser;
                    parserc::next(#c).map(|input| Self(input)).parse(input)#map_err
                }

                #[inline]
                fn to_span(&self) -> parserc::Span {
                    self.0.to_span()
                }
            }
        })
    } else {
        Ok(quote! {
            impl #impl_generic parserc::syntax::Syntax<#ty_input> for #ident #type_generic #where_clause {
                #[inline]
                fn parse(input: &mut #ty_input) -> Result<Self, <#ty_input as parserc::Input>::Error> {
                    use parserc::syntax::InputSyntaxExt;
                    use parserc::ParseError;
                    #parse
                }

                #[inline]
                fn to_span(&self) -> parserc::Span {
                    let mut lhs = parserc::Span::None;
                    #(
                        lhs = lhs.union(&#to_spans);
                    )*

                    lhs
                }
            }
        })
    }
}
