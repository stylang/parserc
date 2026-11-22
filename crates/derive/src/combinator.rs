//! How one item, arm or field is read, and the `#[parserc(...)]` attributes
//! that say so.
//!
//! Every combinator of `parserc`'s `c` module is a variant of [`Combinator`],
//! and every one of them is accepted at every position a parser is needed —
//! on a `struct`, on an `enum`, on one of its variants, and on a field of any
//! of those. The option is spelled as the function is named, so
//! `#[parserc(next_if = …)]` writes a call of `parserc::next_if` and nothing
//! else decides it. There is therefore one list of what may be written, one
//! parser for that list, and one translation of it to code: a position that
//! takes a combinator takes all of them, and the checks a combinator needs
//! (that what it reads is the input the item is generic over) are made here,
//! once, rather than at whichever call site happened to remember them.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Attribute, Error, Expr, Lit, Result, Type, parse::Parser as _, spanned::Spanned as _};

/// Where a `#[parserc(...)]` stands, which decides what it may name.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Position {
    /// On the `struct` or `enum` itself.
    Item,
    /// On one variant of an `enum`.
    Arm,
    /// On one field, of an `enum` variant or of a `struct`.
    Field,
}

impl Position {
    /// What this position is called in an error, so the message can name the
    /// place the author actually wrote.
    fn describe(self) -> &'static str {
        match self {
            Position::Item => "an item",
            Position::Arm => "a variant",
            Position::Field => "a field",
        }
    }
}

/// How one item, arm or field is read.
///
/// All but [`Combinator::Syntax`] and [`Combinator::Parser`] read the input
/// itself and hand back a run of it, so the thing they are written on has to
/// *be* the input — the check [`Combinator::reads_input`] exists for.
pub(super) enum Combinator {
    /// Nothing was written: the type's own [`Syntax`] impl reads it.
    ///
    /// [`Syntax`]: ../parserc/syntax/trait.Syntax.html
    Syntax,
    /// `parser = …`, an arbitrary [`Parser`], written as given.
    ///
    /// [`Parser`]: ../parserc/trait.Parser.html
    Parser(Expr),
    /// `next = 'x'`, one item equal to the one here.
    Next(Lit),
    /// `next_if = f`, one item that `f` accepts.
    NextIf(Expr),
    /// `keyword = "kw"`, the items of a word in order.
    Keyword(Lit),
    /// `take_until = "kw"`, the run up to the next `kw`, leaving it unread.
    TakeUntil(Expr),
    /// `take_while = f`, the longest leading run `f` accepts, none of it
    /// included.
    TakeWhile(Expr),
    /// `take_while_with = (range, f)`, the same bounded by `range`.
    TakeWhileWith { range: Expr, cond: Expr },
    /// `take_till = f`, the longest leading run `f` rejects.
    TakeTill(Expr),
}

impl Combinator {
    /// The option's own name, which is the function's name in `parserc`.
    pub(super) fn name(&self) -> &'static str {
        match self {
            Combinator::Syntax => "the type's own Syntax impl",
            Combinator::Parser(_) => "parser",
            Combinator::Next(_) => "next",
            Combinator::NextIf(_) => "next_if",
            Combinator::Keyword(_) => "keyword",
            Combinator::TakeUntil(_) => "take_until",
            Combinator::TakeWhile(_) => "take_while",
            Combinator::TakeWhileWith { .. } => "take_while_with",
            Combinator::TakeTill(_) => "take_till",
        }
    }

    /// Whether this combinator reads the input and answers with a run of it,
    /// which is what asks the thing it is written on to be the input type.
    ///
    /// [`Combinator::Parser`] is exempt only because its answer is whatever the
    /// expression the author wrote produces, which cannot be known from here.
    fn reads_input(&self) -> bool {
        !matches!(self, Combinator::Syntax | Combinator::Parser(_))
    }

    /// The combinator expression, without the `.parse(input)` on it: what an
    /// item or arm maps into its own constructor, and what a field parses
    /// straight through.
    ///
    /// [`Combinator::Parser`] answers with the expression the author wrote, so
    /// that it can be mapped into a constructor like any combinator: leaving it
    /// empty here would strand the `.map` after nothing at all.
    fn expression(&self) -> TokenStream {
        match self {
            Combinator::Syntax => TokenStream::new(),
            Combinator::Parser(parser) => parser.to_token_stream(),
            Combinator::Next(lit) => quote! { parserc::next(#lit) },
            Combinator::NextIf(cond) => quote! { parserc::next_if(#cond) },
            Combinator::Keyword(keyword) => quote! { parserc::keyword(#keyword) },
            Combinator::TakeUntil(keyword) => quote! { parserc::take_until(#keyword) },
            Combinator::TakeWhile(cond) => quote! { parserc::take_while(#cond) },
            Combinator::TakeWhileWith { range, cond } => {
                quote! { parserc::take_while_with(#range, #cond) }
            }
            Combinator::TakeTill(cond) => quote! { parserc::take_till(#cond) },
        }
    }

    /// The statement that reads one match into a field: the combinator
    /// applied to `input`, or the type's own parser where there is none.
    pub(super) fn read(&self, ty_input: &Type) -> TokenStream {
        match self {
            Combinator::Syntax => {
                quote! { <#ty_input as parserc::syntax::SyntaxInput>::parse(input) }
            }
            Combinator::Parser(parser) => quote! { #parser.parse(input) },
            combinator => {
                let expression = combinator.expression();

                quote! { #expression.parse(input) }
            }
        }
    }

    /// The parser that reads one match and wraps it in `ctor`, which is what
    /// an item or an arm holds a run of text in.
    ///
    /// The run the combinator took is named `taken` rather than `input` inside
    /// the closure: the parameter of the surrounding `fn parse` is `input`, and
    /// shadowing it there is how the generated code used to read as though the
    /// input were being rebound.
    ///
    /// `tail` is everything between the wrap and the read — an arm's `map_err`
    /// and its `.ok()`, an item's `map_err` alone. It is the caller's because
    /// their order is the order an error has to be relabelled in: before the
    /// choice between alternatives is made, not after, or a failure an arm was
    /// told to report fatally would be turned back into a retry.
    pub(super) fn read_into(&self, ctor: TokenStream, tail: TokenStream) -> TokenStream {
        let expression = self.expression();

        quote! { #expression.map(|taken| #ctor(taken))#tail.parse(input) }
    }
}

/// Everything one `#[parserc(...)]` says about one position.
pub(super) struct Config {
    /// The input the item is generic over. `input = …` on an item; the
    /// default `I` everywhere else.
    pub ty_input: Type,
    /// `map_err = …`, on the read this position performs.
    pub map_err: Option<Expr>,
    /// `crucial`, on a field: what fails here stops the whole item.
    pub crucial: bool,
    /// `left_recursion`, on a field: reaching this field again while reading
    /// it is an error rather than a loop.
    pub left_recursion: bool,
    /// How this position is read.
    pub combinator: Combinator,
    /// `semantic = …`, a function of the input and what was built, asked of
    /// what this position assembled.
    pub semantic: Option<Expr>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ty_input: syn::parse2(quote! { I }).unwrap(),
            map_err: None,
            crucial: false,
            left_recursion: false,
            combinator: Combinator::Syntax,
            semantic: None,
        }
    }
}

impl Config {
    /// Reads every `#[parserc(...)]` on one item, arm or field.
    ///
    /// Each option is held in a local until the whole list has been read, so
    /// that "set twice" is a question about one `Option` rather than about
    /// whether a default value happens to look like the one the author wrote.
    pub(super) fn parse(attrs: &[Attribute], position: Position) -> Result<Self> {
        let mut config = Config::default();
        let mut ty_input: Option<Type> = None;
        // Which option last said how this is read, so a second one is refused
        // by both of its names rather than by an unexplained "not both".
        let mut combinator: Option<String> = None;

        for meta_list in parserc_lists(attrs)? {
            let parser = syn::meta::parser(|meta| {
                // A bare path with nothing after it may be a flag (`crucial`),
                // which the arms below claim, or the parser itself — the form
                // `#[parserc(read_ident_or_keyword)]`, which says
                // `parser = read_ident_or_keyword`. Whether the first is one is
                // a question about the key, so the arms are asked first and only
                // a key that names nothing is read as a path.
                let bare = meta.input.is_empty();
                let key = meta.path.get_ident().map(ToString::to_string);

                match key.as_deref() {
                    // Names the type an item is generic over, so nothing else
                    // can answer for it and no other position has one.
                    Some("input") => {
                        if position != Position::Item {
                            return Err(meta.error(format!(
                                "`input` names the type an item is generic over, which only an item \
                                 has: it cannot be set on {}",
                                position.describe()
                            )));
                        }

                        if ty_input.is_some() {
                            return Err(meta.error("`input` was already set on this item"));
                        }

                        ty_input = Some(meta.value()?.parse()?);
                    }
                    Some("map_err") => {
                        if config.map_err.is_some() {
                            return Err(meta.error("`map_err` was already set here"));
                        }

                        config.map_err = Some(meta.value()?.parse()?);
                    }
                    // Both of these say what one failure does to the reads
                    // around it, which only a field of a sequence has.
                    Some(flag @ "crucial") | Some(flag @ "left_recursion") => {
                        if position != Position::Field {
                            return Err(meta.error(format!(
                                "`{flag}` says what a failure here does to the reads beside it, and \
                                 only a field has those: it cannot be set on {}",
                                position.describe()
                            )));
                        }

                        if flag == "crucial" {
                            config.crucial = true;
                        } else {
                            config.left_recursion = true;
                        }
                    }
                    Some("semantic") => {
                        if config.semantic.is_some() {
                            return Err(meta.error("`semantic` was already set here"));
                        }

                        config.semantic = Some(meta.value()?.parse()?);
                    }
                    Some(name @ ("parser" | "next" | "next_if" | "keyword" | "take_until"))
                    | Some(name @ ("take_while" | "take_while_with" | "take_till")) => {
                        if let Some(previous) = &combinator {
                            return Err(meta.error(format!(
                                "`{name}` and `{previous}` both say how this is read; keep one"
                            )));
                        }

                        let value = meta.value()?;

                        // Read as the token the author wrote, not as an `Expr`
                        // wrapped around it: a literal handed over by a
                        // `macro_rules!` metavariable reaches here as an opaque
                        // token subtree, which `Lit` parses and `Expr` refuses.
                        config.combinator = match name {
                            "next" => Combinator::Next(value.parse()?),
                            "keyword" => Combinator::Keyword(value.parse()?),
                            "parser" => Combinator::Parser(value.parse()?),
                            "next_if" => Combinator::NextIf(value.parse()?),
                            "take_until" => Combinator::TakeUntil(value.parse()?),
                            "take_while" => Combinator::TakeWhile(value.parse()?),
                            "take_till" => Combinator::TakeTill(value.parse()?),
                            "take_while_with" => {
                                let (range, cond) = pair(value, name)?;

                                Combinator::TakeWhileWith { range, cond }
                            }
                            other => unreachable!("`{other}` is not an option of a combinator"),
                        };

                        combinator = Some(name.to_owned());
                    }
                    // No key named this, and there is nothing after it to be a
                    // value of: it is a path standing for the parser. syn reads
                    // a nested attribute's path and stops at it, so this form can
                    // only reach a function by name — a call of one, a fold, or
                    // anything else with tokens of its own, is `parser = …`.
                    _ if bare => {
                        if let Some(previous) = &combinator {
                            return Err(meta.error(format!(
                                "this path and `{previous}` both say how this is read; keep one"
                            )));
                        }

                        let path = meta.path.clone();

                        config.combinator = Combinator::Parser(
                            syn::ExprPath {
                                attrs: Vec::new(),
                                qself: None,
                                path,
                            }
                            .into(),
                        );
                        combinator = Some("this path".to_owned());
                    }
                    _ => {
                        let named = key.as_deref().unwrap_or("a path");

                        return Err(meta.error(format!(
                            "unsupported option on {} `{named}`",
                            position.describe()
                        )));
                    }
                }

                Ok(())
            });

            parser.parse2(meta_list.tokens.to_token_stream())?;
        }

        if let Some(ty_input) = ty_input {
            config.ty_input = ty_input;
        }

        Ok(config)
    }
}

/// Whether `combinator` reads the input and answers with a run of it, which is
/// what asks the thing it is written on to be the input type. Refused here,
/// where the answer is known, rather than at whichever call site remembered to
/// ask.
pub(super) fn check_reads_input(
    combinator: &Combinator,
    field_ty: &Type,
    ty_input: &Type,
) -> Result<()> {
    if !combinator.reads_input() {
        return Ok(());
    }

    let read = combinator.name();
    let written = ty_input.to_token_stream().to_string();

    if field_ty.to_token_stream().to_string() != written {
        return Err(Error::new(
            field_ty.span(),
            format!(
                "`{read}` reads the input and hands back a run of it, so it can only be set on a \
                 thing whose type is the input `{written}`"
            ),
        ));
    }

    Ok(())
}

/// The option's value, which has to be the two arguments the combinator takes.
///
/// Read as a comma list first, and a lone parenthesised pair second, so that
/// both `take_while_with = (1.., |c| …)` and the same two arguments handed over
/// by a `macro_rules!` metavariable — whose parentheses a substitution can drop
/// — arrive at the same answer. Reading in this order is what keeps a fork and a
/// rewind out of it: `1..` is a valid expression on its own, so an `Expr`-first
/// reading would swallow `1.., f` and have nothing left to rewind from.
fn pair(value: &syn::parse::ParseBuffer<'_>, key: &str) -> Result<(Expr, Expr)> {
    let elems = value.parse_terminated(<Expr as syn::parse::Parse>::parse, syn::Token![,])?;

    let parts = match elems.len() {
        2 => elems.iter().cloned().collect::<Vec<_>>(),
        1 => match elems.first().expect("one element") {
            Expr::Tuple(tuple) if tuple.elems.len() == 2 => {
                tuple.elems.iter().cloned().collect::<Vec<_>>()
            }
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };

    let [range, cond] = parts.as_slice() else {
        return Err(Error::new(
            elems.span(),
            format!("`{key}` is called with a range and a condition: `{key} = (1.., |c| …)`"),
        ));
    };

    Ok((range.clone(), cond.clone()))
}

/// The `parserc(...)` lists of `attrs`, refusing the bare and the named-value
/// forms rather than reading them as nothing.
fn parserc_lists(attrs: &[Attribute]) -> Result<Vec<&syn::MetaList>> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("parserc"))
        .map(|attr| match &attr.meta {
            syn::Meta::Path(path) => {
                Err(Error::new(path.span(), "empty body, expect `parserc(…)`"))
            }
            syn::Meta::NameValue(value) => Err(Error::new(
                value.span(),
                "expect `parserc(key = value)`, not `parserc = value`",
            )),
            syn::Meta::List(list) => Ok(list),
        })
        .collect()
}
