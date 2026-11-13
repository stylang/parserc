//! The Rust one type is written as: its declaration, its `parse` and its
//! `to_span`.
//!
//! [`Renderer`] is the last step of the generator and the only one that knows
//! anything about Rust syntax. It reads the two IRs and writes items in each
//! one's own terms: a [`Scanner`] is a tuple of the text it took, whose `parse`
//! is the program [`scan`] ran over the stream, and a [`Shape`] is a tuple of
//! the nodes it holds, whose `parse` takes one field after another.
//!
//! # What the generated file leans on
//!
//! Every item is generic over one stream `I`, bounded by
//! [`GeneratedInput`](super::GeneratedInput), and every shortcut it takes is a
//! type the backend already implements as syntax: [`Option`] for an operand that
//! may be absent, [`Vec`] for a run of them, and a tuple for operands taken back
//! to back with nothing between, which is what `concat` asks for. No combinator
//! is invented here, so nothing in the generated file has to be written before
//! the language it describes is.
//!
//! # Trivia
//!
//! A generated token keeps only the text it matched, never the runs around it,
//! so a field reached over a run of trivia carries a separator of its own — a
//! run of the whitespace type, held as a `Vec` because two tokens may well be
//! written with nothing between them. The separator is a field like any other,
//! which is what keeps the tree concrete: what was between two tokens is part
//! of the tree and not something the parser threw away.
//!
//! A rule of nodes that writes its own separator out as an operand is given no
//! field for the mention, since the run around it is the same trivia taken the
//! same way (see the `Whitespace` section of [`super::shape`]).
//!
//! # Recursion
//!
//! A node whose type is in the `pointed` set of the shape being written is held
//! behind a [`Box`], since naming one's self — or naming one's way back to one's
//! self — is the one thing a field of its own cannot do. The backend implements
//! [`Syntax`](parserc::syntax::Syntax) for [`Box`], so the pointer is invisible
//! wherever the field is taken.

use std::collections::{HashMap, HashSet};

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote};

use super::Paths;
use super::error::CodegenError;
use super::name::type_ident;
use super::scan::Emitter;
use super::shape::{Assert, Field, Shape, ShapeKind, Type, Variant};
use super::token::{Pred, Scanner};

/// The stream every generated type is written over, and the name its `parse`
/// takes it by.
const STREAM: &str = "input";

/// Writes the items of a generated file.
///
/// The renderer holds what is the same for every type in the file — where the
/// crates are, how visible the types are, what trivia looks like — so that the
/// two halves of an item can be written from the plan alone.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Renderer<'a> {
    /// The paths the generated file reaches the crates by.
    paths: &'a Paths,
    /// The visibility of the types and their fields.
    visibility: &'a TokenStream,
    /// The derives every type is given.
    derives: &'a [String],
    /// The type of the file's trivia, which is what every separator names, or
    /// nothing when the file declares none.
    whitespace: Option<&'a str>,
}

impl<'a> Renderer<'a> {
    /// Prepares a renderer for a file with the settings `codegen` was given,
    /// whose separators name the whitespace type `whitespace`.
    pub(crate) fn new(
        paths: &'a Paths,
        visibility: &'a TokenStream,
        derives: &'a [String],
        whitespace: Option<&'a str>,
    ) -> Self {
        Self {
            paths,
            visibility,
            derives,
            whitespace,
        }
    }

    /// The item of one type that reads text: the text it took, and the program
    /// that took it.
    pub(crate) fn scanner(
        &self,
        scanner: &Scanner,
        classes: &HashMap<String, Pred>,
    ) -> Result<TokenStream, CodegenError> {
        let ty = type_ident(&scanner.name);
        let body = Emitter::named(self.paths, classes, &scanner.name).scanner(&scanner.inst)?;
        let (parserc, unsyn) = (&self.paths.parserc, &self.paths.unsyn);
        let input = format_ident!("{STREAM}");
        let attrs = self.attrs();
        let uses = self.uses();
        let visibility = self.visibility;

        Ok(quote! {
            #attrs
            #visibility struct #ty<I>(#visibility I);

            impl<I> #parserc::syntax::Syntax<I> for #ty<I>
            where
                I: #unsyn::codegen::GeneratedInput,
            {
                fn parse(#input: &mut I) -> ::core::result::Result<Self, <I as #parserc::Input>::Error> {
                    #uses

                    #body
                }

                fn to_span(&self) -> #parserc::Span {
                    #uses

                    self.0.to_span()
                }
            }
        })
    }

    /// The item of one type that holds nodes.
    ///
    /// `pointed` is the set of node types that reach the type being written,
    /// which are the ones it has to hold behind a pointer.
    pub(crate) fn node(&self, shape: &Shape, pointed: &HashSet<String>) -> TokenStream {
        let ty = type_ident(&shape.name);
        let input = format_ident!("{STREAM}");
        let fail = self.fail(&shape.name, &input);
        let (parserc, unsyn) = (&self.paths.parserc, &self.paths.unsyn);
        let visibility = self.visibility;

        let writing = Writing {
            pointed,
            input: &input,
            fail: &fail,
        };
        let (declaration, body, spanning) = match &shape.kind {
            ShapeKind::Struct { fields } => {
                let parts = self.parts(fields, &writing);
                let tys = &parts.tys;
                let vals = &parts.vals;

                (
                    self.tuple_of(&ty, tys),
                    quote!(::core::result::Result::Ok(Self(#(#vals),*))),
                    self.union(&self.places(tys.len())),
                )
            }
            ShapeKind::Enum { variants } => {
                let written = variants
                    .iter()
                    .map(|variant| self.variant(variant, &writing))
                    .collect::<Vec<_>>();
                let names = written
                    .iter()
                    .map(|written| {
                        let name = &written.name;
                        let tys = &written.tys;

                        quote!(#name(#(#tys),*))
                    })
                    .collect::<Vec<_>>();
                let attempts = written
                    .iter()
                    .map(|written| written.attempt.clone())
                    .collect::<Vec<_>>();
                let arms = written
                    .iter()
                    .map(|written| {
                        let name = &written.name;
                        let binds = &written.binds;
                        let spanning =
                            self.union(&binds.iter().map(|bind| quote!(#bind)).collect::<Vec<_>>());

                        quote!(Self::#name(#(#binds),*) => #spanning,)
                    })
                    .collect::<Vec<_>>();

                (
                    quote!(#visibility enum #ty<I> { #(#names),* }),
                    quote! {
                        #(#attempts)*

                        ::core::result::Result::Err(#fail)
                    },
                    quote! {
                        match self {
                            #(#arms)*
                        }
                    },
                )
            }
            ShapeKind::Bounds {
                item,
                min,
                max,
                separated,
            } => {
                let (bounds, body, spanning) = self.bounds(item, *min, *max, *separated, &writing);

                (self.tuple_of(&ty, &[bounds]), body, spanning)
            }
            ShapeKind::Asserted { item, assert } => {
                let (taken, body, spanning) = self.asserted(item, assert, &writing);

                (self.tuple_of(&ty, &[taken]), body, spanning)
            }
        };

        let attrs = self.attrs();
        let uses = self.uses();

        quote! {
            #attrs
            #declaration

            impl<I> #parserc::syntax::Syntax<I> for #ty<I>
            where
                I: #unsyn::codegen::GeneratedInput,
            {
                fn parse(#input: &mut I) -> ::core::result::Result<Self, <I as #parserc::Input>::Error> {
                    #uses

                    #body
                }

                fn to_span(&self) -> #parserc::Span {
                    #uses

                    #spanning
                }
            }
        }
    }

    /// The type of a counted run, the `parse` that takes it, and the `to_span`
    /// of the one field it holds.
    ///
    /// A counted run is the one shape whose `parse` has to keep a count: the
    /// operand is taken as often as it fits, up to the most the rule stated, and
    /// a run shorter than the least fails where it stands, which is recovable
    /// and so is rewound by whoever tried the rule.
    fn bounds(
        &self,
        item: &Type,
        min: usize,
        max: Option<usize>,
        separated: bool,
        writing: &Writing<'_>,
    ) -> (TokenStream, TokenStream, TokenStream) {
        let input = writing.input;
        let fail = writing.fail;

        let elem = self.element(item, separated, writing.pointed);
        let tys = quote!(::std::vec::Vec<#elem>);
        let least = Literal::usize_unsuffixed(min);
        let most: Vec<TokenStream> = max
            .map(|max| {
                let most = Literal::usize_unsuffixed(max);

                quote!(if __items.len() >= #most { break; })
            })
            .into_iter()
            .collect();

        (
            tys,
            quote! {
                let mut __items = ::std::vec::Vec::new();

                loop {
                    #(#most)*

                    let ::core::option::Option::Some(__next) =
                        #input.parse::<::core::option::Option<#elem>>()?
                    else {
                        break;
                    };

                    __items.push(__next);
                }

                if __items.len() < #least {
                    return ::core::result::Result::Err(#fail);
                }

                ::core::result::Result::Ok(Self(__items))
            },
            self.union(&[self.at(0)]),
        )
    }

    /// The type of an asserted operand, the `parse` that asks about the text
    /// around it, and the `to_span` of the one field it holds.
    fn asserted(
        &self,
        item: &Type,
        assert: &Assert,
        writing: &Writing<'_>,
    ) -> (TokenStream, TokenStream, TokenStream) {
        let input = writing.input;

        let tys = self.ty(item, writing.pointed);
        let (asked, before) = match assert {
            Assert::Except(removed) => (removed, true),
            Assert::Followed(next) => (next, false),
        };
        let probe = self.probe(asked, before, writing);
        let (asked, taken) = if before {
            (probe, quote!(let __taken = #input.parse::<#tys>()?;))
        } else {
            (quote!(let __taken = #input.parse::<#tys>()?;), probe)
        };

        (
            tys.clone(),
            quote! {
                #asked

                #taken

                ::core::result::Result::Ok(Self(__taken))
            },
            self.union(&[self.at(0)]),
        )
    }

    /// The name, the fields, the attempt and the bindings of one alternative of
    /// a rule that has several.
    fn variant(&self, variant: &Variant, writing: &Writing<'_>) -> Written {
        let input = writing.input;

        let name = type_ident(&variant.name);
        let parts = self.parts(&variant.fields, writing);
        let vals = &parts.vals;
        let parserc = &self.paths.parserc;

        // An alternative is tried over a copy of the stream that `ok` rewinds
        // when the alternative gives up partway, which is what lets the next
        // one of the rule be tried at the same character.
        let attempt = quote! {
            if let ::core::option::Option::Some(__alt) =
                (|#input: &mut I| -> ::core::result::Result<Self, <I as #parserc::Input>::Error> {
                    ::core::result::Result::Ok(Self::#name(#(#vals),*))
                })
                .ok()
                .parse(#input)?
            {
                return ::core::result::Result::Ok(__alt);
            }
        };

        Written {
            name,
            tys: parts.tys.clone(),
            binds: self.binds(parts.tys.len()),
            attempt,
        }
    }

    /// The declaration parts of a sequence of fields: the type of each one, the
    /// expression that takes it, and the name it is bound to in `to_span`.
    ///
    /// A separator is a part like any other, so the three line up and a field
    /// reached over trivia takes two of them.
    fn parts(&self, fields: &[Field], writing: &Writing<'_>) -> Parts {
        let input = writing.input;

        let mut parts = Parts::default();

        for field in fields {
            if field.separator {
                let ws = self.separator();

                parts.push(ws.clone(), quote!(#input.parse::<#ws>()?));
            }

            let (ty, read) = self.read(&field.ty, writing);

            parts.push(ty, read);
        }

        parts
    }

    /// The type of a field and the expression that takes it, which is one call
    /// of `parse` whatever the field holds — except for a run of one or more,
    /// which is a `Vec` that has to be found non-empty once it was taken.
    fn read(&self, ty: &Type, writing: &Writing<'_>) -> (TokenStream, TokenStream) {
        let input = writing.input;
        let fail = writing.fail;

        let of = self.ty(ty, writing.pointed);

        match ty {
            Type::OneOrMore { .. } => (
                of.clone(),
                quote! {
                    {
                        let __run = #input.parse::<#of>()?;

                        if __run.is_empty() {
                            return ::core::result::Result::Err(#fail);
                        }

                        __run
                    }
                },
            ),
            _ => (of.clone(), quote!(#input.parse::<#of>()?)),
        }
    }

    /// The type a field holds, with a pointer wherever one is needed and the
    /// trivia of a run woven into its elements.
    fn ty(&self, ty: &Type, pointed: &HashSet<String>) -> TokenStream {
        match ty {
            Type::Named(name) => {
                let named = type_ident(name);

                if pointed.contains(name) {
                    quote!(::std::boxed::Box<#named<I>>)
                } else {
                    quote!(#named<I>)
                }
            }
            Type::Opt(item) => {
                let item = self.ty(item, pointed);

                quote!(::core::option::Option<#item>)
            }
            Type::Many { item, separated } | Type::OneOrMore { item, separated } => {
                let item = self.element(item, *separated, pointed);

                quote!(::std::vec::Vec<#item>)
            }
            Type::Concat { first, second } => {
                let (first, second) = (self.ty(first, pointed), self.ty(second, pointed));

                quote!((#first, #second))
            }
        }
    }

    /// The type of one element of a run, which carries the trivia before it
    /// when the run is separated, since the tokens of a run are reached over
    /// the same kind of gap as the tokens of a sequence.
    fn element(&self, item: &Type, separated: bool, pointed: &HashSet<String>) -> TokenStream {
        let item = self.ty(item, pointed);

        if separated {
            let ws = self.separator();

            quote!((#ws, #item))
        } else {
            item
        }
    }

    /// The type of the trivia before a token: a run of the file's whitespace
    /// type, which a file with no trivia of its own has no need of.
    ///
    /// A `whitespace` rule names one character of trivia, as any other token
    /// does, and what lies between two tokens is a run of them. `Vec` is what
    /// the backend already takes as often as the text offers, so the run may be
    /// nothing at all where two tokens are written together.
    fn separator(&self) -> TokenStream {
        match self.whitespace {
            Some(ws) => {
                let ws = type_ident(ws);

                quote!(::std::vec::Vec<#ws<I>>)
            }
            None => quote!(()),
        }
    }

    /// The statement that asks a question about the text and takes nothing of
    /// it: `except` asked before the operand, which fails where what it removes
    /// fits, and `followed` asked where the operand left off, which fails where
    /// what it requires does not fit.
    fn probe(&self, asked: &Type, before: bool, writing: &Writing<'_>) -> TokenStream {
        let input = writing.input;
        let fail = writing.fail;

        let tys = self.ty(asked, writing.pointed);
        let parserc = &self.paths.parserc;
        let probe = format_ident!("__probe");
        let fits = format_ident!("__fits");
        // The question is asked over a copy of the stream, so whatever the
        // operand it asks about took is dropped with the copy.
        let test = if before {
            format_ident!("is_some")
        } else {
            format_ident!("is_none")
        };

        quote! {
            {
                let mut #probe = #input.clone();

                let #fits = (|#probe: &mut I| -> ::core::result::Result<(), <I as #parserc::Input>::Error> {
                    #probe.parse::<#tys>()?;

                    ::core::result::Result::Ok(())
                })
                .ok()
                .parse(&mut #probe)?;

                if #fits.#test() {
                    return ::core::result::Result::Err(#fail);
                }
            }
        }
    }

    /// The span of a type, as the union of the spans of the fields it holds.
    ///
    /// The parts are the expressions the fields are reached by, which is a name
    /// bound in a match arm for one alternative of a rule and a place in the
    /// type for a struct. Either way a node reaches over the trivia it holds,
    /// since what was between two tokens is a field like any other.
    fn union(&self, parts: &[TokenStream]) -> TokenStream {
        let parserc = &self.paths.parserc;

        parts.iter().fold(
            quote!(#parserc::Span::None),
            |acc, part| quote!(#acc.union(&#part.to_span())),
        )
    }

    /// The place in the type the field at `index` is kept at, which is how a
    /// tuple struct is read a field at a time.
    fn at(&self, index: usize) -> TokenStream {
        let place = Literal::usize_unsuffixed(index);

        quote!(self . #place)
    }

    /// The places every field of a sequence is kept at, in the order they are
    /// taken.
    fn places(&self, count: usize) -> Vec<TokenStream> {
        (0..count).map(|index| self.at(index)).collect()
    }

    /// The names one match arm binds the fields of an alternative to.
    fn binds(&self, count: usize) -> Vec<Ident> {
        (0..count).map(|index| format_ident!("__{index}")).collect()
    }

    /// The declaration of a tuple struct of the types it holds, each field as
    /// visible as the type itself.
    fn tuple_of(&self, ty: &Ident, tys: &[TokenStream]) -> TokenStream {
        let visibility = self.visibility;

        quote!(#visibility struct #ty<I>(#(#visibility #tys),*);)
    }

    /// The attributes every generated type carries.
    ///
    /// The case conventions are asked to leave the names alone because a type is
    /// called what the file called its rule, which is `UPPER_CASE` for a lexer
    /// and a number after the owner's name for a group.
    fn attrs(&self) -> TokenStream {
        let derives = self
            .derives
            .iter()
            .map(|derive| format_ident!("{}", derive))
            .collect::<Vec<_>>();

        quote! {
            #[allow(non_camel_case_types, non_snake_case)]
            #[derive(#(#derives),*)]
        }
    }

    /// The traits a generated body reaches by name, all of which the shortcuts
    /// of [`scan`] and the fields of a shape are written against.
    fn uses(&self) -> TokenStream {
        let parserc = &self.paths.parserc;

        quote! {
            #[allow(unused_imports)]
            use #parserc::{AsStr, Find, Input, Parser, StartWith, syntax::{Syntax, SyntaxInput}};
        }
    }

    /// The failure of a type that took nothing more: recovable, so that the
    /// caller may rewind and look elsewhere, and one character wide, since the
    /// character there is the one that did not fit.
    fn fail(&self, name: &str, input: &Ident) -> TokenStream {
        let parserc = &self.paths.parserc;
        let name = Literal::string(name);

        quote! {
            #parserc::Kind::Syntax(
                #name,
                #parserc::ControlFlow::Recovable,
                #input.to_span_at(1),
            )
            .into()
        }
    }
}

/// What a helper that writes one type needs beside the type itself: the names
/// that have to be held behind a pointer, the stream to read over, and the
/// failure to report when the type takes nothing more.
struct Writing<'a> {
    /// The node types that reach the type being written.
    pointed: &'a HashSet<String>,
    /// The stream every field is taken from.
    input: &'a Ident,
    /// The failure of the type being written.
    fail: &'a TokenStream,
}

/// The declaration parts of a sequence, in the order they are written.
#[derive(Default)]
struct Parts {
    /// The type of each field, separators included.
    tys: Vec<TokenStream>,
    /// The expression that takes each field, separators included.
    vals: Vec<TokenStream>,
}

impl Parts {
    /// Adds one field to the two, in the order the sequence takes it.
    fn push(&mut self, ty: TokenStream, val: TokenStream) {
        self.tys.push(ty);
        self.vals.push(val);
    }
}

/// One alternative of a rule that has several, written out.
struct Written {
    /// The name of the variant.
    name: Ident,
    /// The types of its fields, separators included.
    tys: Vec<TokenStream>,
    /// The names its fields are bound to in `to_span`.
    binds: Vec<Ident>,
    /// The statement that tries it and returns what it made.
    attempt: TokenStream,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders `shape` the way the file would be written, with every run of
    /// whitespace collapsed to nothing, so that a test says what the generator
    /// means without spelling out how `proc_macro2` separates its tokens.
    fn render(shape: &Shape, pointed: &[&str], whitespace: Option<&str>) -> String {
        let paths = Paths::default();
        let visibility = quote!(pub);
        let derives = ["Debug"].map(String::from).to_vec();

        let written = Renderer::new(&paths, &visibility, &derives, whitespace)
            .node(shape, &pointed.iter().copied().map(String::from).collect());

        written.to_string().split_whitespace().collect()
    }

    /// The shape of a rule of one alternative over the named operands.
    fn struct_of(names: &[&str]) -> Shape {
        Shape {
            name: "Owner".to_owned(),
            kind: ShapeKind::Struct {
                fields: names
                    .iter()
                    .map(|name| Field {
                        ty: Type::Named((*name).to_owned()),
                        separator: false,
                    })
                    .collect(),
            },
        }
    }

    /// The same shape with the field at `index` reached over a run of trivia.
    fn separated(mut shape: Shape, index: usize) -> Shape {
        if let ShapeKind::Struct { fields } = &mut shape.kind {
            fields[index].separator = true;
        }

        shape
    }

    #[test]
    fn a_rule_of_one_alternative_is_a_tuple_struct_of_its_fields() {
        let written = render(&struct_of(&["Abc", "Def"]), &[], None);

        assert!(
            written.contains("pubstructOwner<I>(pubAbc<I>,pubDef<I>);"),
            "{written}"
        );
        assert!(
            written.contains("Ok(Self(input.parse::<Abc<I>>()?,input.parse::<Def<I>>()?))"),
            "{written}"
        );
    }

    #[test]
    fn a_field_reached_over_trivia_carries_the_run_as_a_field_of_its_own() {
        let written = render(&separated(struct_of(&["Abc", "Def"]), 1), &[], Some("WS"));

        assert!(written.contains("pub::std::vec::Vec<WS<I>>"), "{written}");
        // The run is taken before the token it reaches, and both are fields.
        assert!(
            written.contains("parse::<::std::vec::Vec<WS<I>>>()?,input.parse::<Def<I>>()?"),
            "{written}"
        );
    }

    #[test]
    fn a_file_with_no_trivia_declares_no_separators() {
        // With nothing between its tokens, a sequence of two is two fields and
        // not four, which is what the shape of the rule says.
        let written = render(&separated(struct_of(&["Abc", "Def"]), 1), &[], None);

        assert!(!written.contains("Option<"), "{written}");
    }

    #[test]
    fn a_node_that_reaches_its_owner_is_held_behind_a_pointer() {
        let written = render(&struct_of(&["Owner", "Else"]), &["Owner"], None);

        assert!(
            written.contains("pub::std::boxed::Box<Owner<I>>"),
            "{written}"
        );
        assert!(!written.contains("Box<Else"), "{written}");
    }

    #[test]
    fn a_run_of_one_or_more_is_a_vec_that_has_to_have_something_in_it() {
        let shape = Shape {
            name: "Owner".to_owned(),
            kind: ShapeKind::Struct {
                fields: vec![Field {
                    ty: Type::OneOrMore {
                        item: Box::new(Type::Named("Abc".to_owned())),
                        separated: true,
                    },
                    separator: false,
                }],
            },
        };

        let written = render(&shape, &[], Some("WS"));

        assert!(
            written.contains("Vec<(::std::vec::Vec<WS<I>>,Abc<I>)>"),
            "{written}"
        );
        assert!(written.contains("if__run.is_empty()"), "{written}");
    }

    #[test]
    fn a_counted_run_is_taken_as_greedily_as_it_fits_and_then_checked() {
        let shape = Shape {
            name: "Owner1".to_owned(),
            kind: ShapeKind::Bounds {
                item: Type::Named("Abc".to_owned()),
                min: 2,
                max: Some(4),
                separated: false,
            },
        };

        let written = render(&shape, &[], None);

        // The run is taken up to the most the rule stated, and a run shorter
        // than the least is the failure, in that order.
        assert!(written.contains("__items.len()"), "{written}");
        assert!(written.contains(">=4"), "{written}");
        assert!(written.contains("<2"), "{written}");
        assert!(
            written.find(">=4").expect("the most") < written.find("<2").expect("the least"),
            "{written}"
        );
    }

    #[test]
    fn an_unbounded_count_asks_the_text_and_nothing_else() {
        let shape = Shape {
            name: "Owner1".to_owned(),
            kind: ShapeKind::Bounds {
                item: Type::Named("Abc".to_owned()),
                min: 1,
                max: None,
                separated: false,
            },
        };

        let written = render(&shape, &[], None);

        assert!(!written.contains("len()>="), "{written}");
        assert!(written.contains("if__items.len()<1"), "{written}");
    }

    #[test]
    fn except_is_asked_where_the_operand_starts() {
        let shape = Shape {
            name: "Owner1".to_owned(),
            kind: ShapeKind::Asserted {
                item: Type::Named("Abc".to_owned()),
                assert: Assert::Except(Type::Named("Def".to_owned())),
            },
        };

        let written = render(&shape, &[], None);

        // The question is asked before the operand is taken, and it is the
        // answer "it fits" that loses.
        assert!(written.contains("__fits.is_some()"), "{written}");
        assert!(
            written.find("__fits").expect("a probe") < written.find("__taken").expect("an operand"),
            "{written}"
        );
    }

    #[test]
    fn followed_is_asked_where_the_operand_left_off() {
        let shape = Shape {
            name: "Owner1".to_owned(),
            kind: ShapeKind::Asserted {
                item: Type::Named("Abc".to_owned()),
                assert: Assert::Followed(Type::Named("Def".to_owned())),
            },
        };

        let written = render(&shape, &[], None);

        assert!(written.contains("__fits.is_none()"), "{written}");
        assert!(
            written.find("__taken").expect("an operand") < written.find("__fits").expect("a probe"),
            "{written}"
        );
    }

    #[test]
    fn every_alternative_is_tried_over_a_stream_that_is_rewound() {
        let shape = Shape {
            name: "Owner".to_owned(),
            kind: ShapeKind::Enum {
                variants: vec![
                    Variant {
                        name: "OwnerAbc".to_owned(),
                        fields: vec![Field {
                            ty: Type::Named("Abc".to_owned()),
                            separator: false,
                        }],
                    },
                    Variant {
                        name: "OwnerDef".to_owned(),
                        fields: vec![Field {
                            ty: Type::Named("Def".to_owned()),
                            separator: false,
                        }],
                    },
                ],
            },
        };

        let written = render(&shape, &[], None);

        assert!(written.contains("pubenumOwner<I>{"), "{written}");
        assert!(
            written.contains("OwnerAbc(Abc<I>)") && written.contains("OwnerDef(Def<I>)"),
            "{written}"
        );
        // Each attempt is run through `ok`, which is what rewinds a recovable
        // failure, and the last resort is the rule's own failure.
        assert_eq!(
            written.matches(".ok().parse(input)?").count(),
            2,
            "{written}"
        );
        assert!(written.contains("Kind::Syntax"), "{written}");
        assert!(written.contains("Self::OwnerAbc(__0)=>"), "{written}");
    }

    #[test]
    fn a_span_is_every_field_of_the_type_unioned() {
        let written = render(&struct_of(&["Abc", "Def"]), &[], None);

        assert!(
            written.contains("Span::None.union(&self.0.to_span()).union(&self.1.to_span())"),
            "{written}"
        );
    }

    #[test]
    fn a_literal_taken_back_to_back_with_another_is_one_tuple_field() {
        let shape = Shape {
            name: "Owner".to_owned(),
            kind: ShapeKind::Struct {
                fields: vec![Field {
                    ty: Type::Concat {
                        first: Box::new(Type::Named("Uplus".to_owned())),
                        second: Box::new(Type::Many {
                            item: Box::new(Type::Named("Hex".to_owned())),
                            separated: false,
                        }),
                    },
                    separator: false,
                }],
            },
        };

        let written = render(&shape, &[], None);

        assert!(
            written.contains("pub(Uplus<I>,::std::vec::Vec<Hex<I>>)"),
            "{written}"
        );
    }
}
