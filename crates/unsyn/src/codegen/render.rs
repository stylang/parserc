//! The Rust one type is written as: its declaration, its `parse` and its
//! `to_span`.
//!
//! [`Renderer`] is the last step of the generator and the only one that knows
//! anything about Rust syntax. It reads the two IRs and writes items in each
//! one's own terms: a [`TokenPattern`] is a tuple of the text it took, whose `parse`
//! is the pattern [`scan`] ran over the stream, and a [`TypeDef`] is a tuple of
//! the nodes it holds, whose `parse` takes one field after another.
//!
//! # What the generated file leans on
//!
//! Every item is generic over one stream `I`, bounded by
//! [`GeneratedInput`](crate::runtime::GeneratedInput), and every shortcut it takes is a
//! type the backend already implements as syntax: [`Option`] for an operand that
//! may be absent, [`Vec`] for a run of them, and a tuple for operands taken back
//! to back with nothing between, which is what `concat` asks for. No combinator
//! is invented here, so nothing in the generated file has to be written before
//! the language it describes is.
//!
//! # Trivia
//!
//! Where the trivia of a token goes is settled once for the whole file, and the
//! renderer only reads what was settled. A terminal that owns its trivia holds
//! the run on each side of the text it matched, so a node reached over such a
//! token finds the run already taken and needs no field of its own; a gap
//! stands as a field only between two operands that reach nothing, and it holds
//! `Option` of the whitespace type, since two tokens may well be written with
//! nothing between them (see the `Trivia` section of [`super::types`]).
//!
//! What the settled fields add up to is what the backend already has names for:
//! a run of an operand and its separator, with the operand optional at the end,
//! is one [`Punctuated`](parserc::syntax::Punctuated), and operands held between
//! two terminals that own their trivia are one
//! [`Delimiter`](parserc::syntax::Delimiter). The types of the backend are
//! named through their crate rather than by a `use`, since a rule of the
//! language being described is free to be called `Or` or `Comma` itself and
//! generate a type of that name into the same module.
//!
//! # Recursion
//!
//! A node whose type is in the `pointed` set of the type being written is held
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
use super::pattern::{CharClass, TokenPattern};
use super::scan::Emitter;
use super::types::{Assert, Field, FieldType, Trivia, TypeDef, TypeDefKind, Variant};

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
    /// Where a `<NAME>` the generator's table settles is called, by name.
    externals: &'a HashMap<String, TokenStream>,
    /// The visibility of the types and their fields.
    visibility: &'a TokenStream,
    /// The derives every type is given.
    derives: &'a [String],
    /// What was settled about the file's trivia: which terminals own the runs
    /// around them, and which type a gap holds a run of.
    trivia: &'a Trivia,
}

impl<'a> Renderer<'a> {
    /// Prepares a renderer for a file with the settings `codegen` was given and
    /// the trivia table the plan was settled with.
    pub(crate) fn new(
        paths: &'a Paths,
        externals: &'a HashMap<String, TokenStream>,
        visibility: &'a TokenStream,
        derives: &'a [String],
        trivia: &'a Trivia,
    ) -> Self {
        Self {
            paths,
            externals,
            visibility,
            derives,
            trivia,
        }
    }

    /// The item of one type that reads text: the text it took, the runs of
    /// trivia around it where it owns them, and the pattern that took them.
    pub(crate) fn token_pattern(
        &self,
        pattern: &TokenPattern,
        classes: &HashMap<String, CharClass>,
    ) -> Result<TokenStream, CodegenError> {
        let ty = type_ident(&pattern.name);
        let emitter = Emitter::named(self.paths, self.externals, classes, &pattern.name);
        let (parserc, unsyn) = (&self.paths.parserc, &self.paths.unsyn);
        let input = format_ident!("{STREAM}");
        let attrs = self.attrs();
        let uses = self.uses();
        let visibility = self.visibility;

        // The type that *is* a run of trivia is declared over the same one
        // field as any other token, but it reads the run itself rather than one
        // copy of its pattern, and it owns nothing beside it: one run is all
        // there is to take at a time.
        if self.trivia.is_trivia(&pattern.name) {
            let body = emitter.parse_run_body(&pattern.step)?;

            return Ok(quote! {
                #attrs
                #visibility struct #ty<I>(#visibility I);

                impl<I> #parserc::syntax::Syntax<I> for #ty<I>
                where
                    I: #unsyn::runtime::GeneratedInput,
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
            });
        }

        // A terminal that owns its trivia holds the run on each side of the
        // text it matched, which is what lets a node reached over it say
        // nothing about the trivia at all. Its span stops with its own text,
        // as the trailing run belongs to whatever comes next.
        if self.trivia.owns(&pattern.name) {
            let ws = self.gap();
            let body = emitter.parse_owning_body(&pattern.step, &ws)?;
            let spanning = self.union(&[self.at(0), self.at(1)]);

            return Ok(quote! {
                #attrs
                #visibility struct #ty<I>(#visibility #ws, #visibility I, #visibility #ws);

                impl<I> #parserc::syntax::Syntax<I> for #ty<I>
                where
                    I: #unsyn::runtime::GeneratedInput,
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
            });
        }

        let body = emitter.parse_body(&pattern.step)?;

        Ok(quote! {
            #attrs
            #visibility struct #ty<I>(#visibility I);

            impl<I> #parserc::syntax::Syntax<I> for #ty<I>
            where
                I: #unsyn::runtime::GeneratedInput,
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
    pub(crate) fn node(&self, def: &TypeDef, pointed: &HashSet<String>) -> TokenStream {
        let ty = type_ident(&def.name);
        let input = format_ident!("{STREAM}");
        let fail = self.fail(&def.name, &input);
        let (parserc, unsyn) = (&self.paths.parserc, &self.paths.unsyn);
        let visibility = self.visibility;

        let writing = Writing {
            pointed,
            input: &input,
            fail: &fail,
        };
        let (declaration, body, spanning) = match &def.kind {
            TypeDefKind::Struct { fields } => {
                let parts = self.parts(fields, &writing);
                let tys = &parts.tys;
                let vals = &parts.vals;

                (
                    self.tuple_of(&ty, tys),
                    quote!(::core::result::Result::Ok(Self(#(#vals),*))),
                    self.union(&self.places(tys.len())),
                )
            }
            TypeDefKind::Enum { variants } => {
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
            TypeDefKind::Bounds { item, min, max } => {
                let (bounds, body, spanning) = self.bounds(item, *min, *max, &writing);

                (self.tuple_of(&ty, &[bounds]), body, spanning)
            }
            TypeDefKind::Asserted { item, assert } => {
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
                I: #unsyn::runtime::GeneratedInput,
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
    /// A counted run is the one type whose `parse` has to keep a count: the
    /// operand is taken as often as it fits, up to the most the rule stated, and
    /// a run shorter than the least fails where it stands, which is recovable
    /// and so is rewound by whoever tried the rule.
    fn bounds(
        &self,
        item: &FieldType,
        min: usize,
        max: Option<usize>,
        writing: &Writing<'_>,
    ) -> (TokenStream, TokenStream, TokenStream) {
        let input = writing.input;
        let fail = writing.fail;

        let elem = self.ty(item, writing.pointed);
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
        item: &FieldType,
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
    /// The fields are taken in the order they are written, which is also the
    /// order they are declared in, so a gap stands between the two operands it
    /// lies between rather than becoming part of either of them.
    fn parts(&self, fields: &[Field], writing: &Writing<'_>) -> Parts {
        let input = writing.input;

        let mut parts = Parts::default();

        for field in fields {
            let ty = self.ty(&field.ty, writing.pointed);

            parts.push(ty.clone(), quote!(#input.parse::<#ty>()?));
        }

        parts
    }

    /// The type a field holds, with a pointer wherever one is needed.
    ///
    /// Every shape here is a type the backend already takes as one operand, so
    /// taking a field of any of them is one call of `parse`.
    fn ty(&self, ty: &FieldType, pointed: &HashSet<String>) -> TokenStream {
        let parserc = &self.paths.parserc;

        match ty {
            FieldType::Named(name) => {
                let named = type_ident(name);

                if pointed.contains(name) {
                    quote!(::std::boxed::Box<#named<I>>)
                } else {
                    quote!(#named<I>)
                }
            }
            FieldType::Opt(item) => {
                let item = self.ty(item, pointed);

                quote!(::core::option::Option<#item>)
            }
            FieldType::Many { item } => {
                let item = self.ty(item, pointed);

                quote!(::std::vec::Vec<#item>)
            }
            FieldType::Concat { first, second } => {
                let (first, second) = (self.ty(first, pointed), self.ty(second, pointed));

                quote!((#first, #second))
            }
            // A group of operands that needed no type of its own is a tuple, so
            // it is taken as one field and holds its operands in the order the
            // rule wrote them.
            FieldType::Group(tys) => {
                let flat: Vec<TokenStream> = tys.iter().map(|ty| self.ty(ty, pointed)).collect();

                self.tuple(&flat)
            }
            FieldType::Gap => self.gap(),
            FieldType::Punctuated { item, sep } => {
                let (item, sep) = (self.ty(item, pointed), self.ty(sep, pointed));

                quote!(#parserc::syntax::Punctuated<#item, #sep>)
            }
            FieldType::Delimited { open, body, close } => {
                let (open, close) = (self.ty(open, pointed), self.ty(close, pointed));
                let flat: Vec<TokenStream> = body
                    .iter()
                    .map(|ty| self.ty(ty, pointed))
                    .collect::<Vec<_>>();
                let body = self.tuple(&flat);

                quote!(#parserc::syntax::Delimiter<#open, #close, #body>)
            }
        }
    }

    /// The type of a run of operands written beside each other: one tuple when
    /// there are several to hold, and the operand itself where there is only one
    /// to hold.
    fn tuple(&self, tys: &[TokenStream]) -> TokenStream {
        match tys {
            [] => quote!(()),
            [only] => only.clone(),
            tys => quote!(( #(#tys),* )),
        }
    }

    /// The type of a gap: one run of the file's trivia, which a file with no
    /// trivia of its own has no need of.
    ///
    /// A `whitespace` rule names one run of the characters that separate two
    /// tokens, so a gap is `Option` of it rather than a list of them: what the
    /// text offers between two tokens is one maximal run, and nothing at all is
    /// the other thing a gap may hold.
    fn gap(&self) -> TokenStream {
        match self.trivia.whitespace() {
            Some(ws) => {
                let ws = type_ident(ws);

                quote!(::core::option::Option<#ws<I>>)
            }
            None => quote!(()),
        }
    }

    /// The statement that asks a question about the text and takes nothing of
    /// it: `except` asked before the operand, which fails where what it removes
    /// fits, and `followed` asked where the operand left off, which fails where
    /// what it requires does not fit.
    fn probe(&self, asked: &FieldType, before: bool, writing: &Writing<'_>) -> TokenStream {
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
    /// of [`scan`] and the fields of a type are written against.
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
    /// The type of each field, gaps included.
    tys: Vec<TokenStream>,
    /// The expression that takes each field, gaps included.
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
    /// The types of its fields, gaps included.
    tys: Vec<TokenStream>,
    /// The names its fields are bound to in `to_span`.
    binds: Vec<Ident>,
    /// The statement that tries it and returns what it made.
    attempt: TokenStream,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders `def` the way the file would be written, so that a test says what
    /// the generator means without spelling out how `proc_macro2` separates its
    /// tokens.
    fn render(def: &TypeDef, pointed: &[&str], trivia: &Trivia) -> String {
        let paths = Paths::default();
        let externals = HashMap::new();
        let visibility = quote!(pub);
        let derives = ["Debug"].map(String::from).to_vec();

        let written = Renderer::new(&paths, &externals, &visibility, &derives, trivia)
            .node(def, &pointed.iter().copied().map(String::from).collect());

        written.to_string().split_whitespace().collect()
    }

    /// The table of a file whose trivia is of type `ws` and whose terminals own
    /// nothing, which is all that is left to know once the fields of a type were
    /// settled: only the name of the whitespace type is read from here on.
    fn trivia(ws: Option<&str>) -> Trivia {
        Trivia::new(ws.map(str::to_owned), HashMap::new())
    }

    /// The type of a rule of one alternative over the named operands.
    fn struct_of(names: &[&str]) -> TypeDef {
        TypeDef {
            name: "Owner".to_owned(),
            kind: TypeDefKind::Struct {
                fields: names
                    .iter()
                    .map(|name| Field {
                        ty: FieldType::Named((*name).to_owned()),
                    })
                    .collect(),
            },
        }
    }

    /// The same type with a gap let in before the operand at `index`.
    fn gapped(mut def: TypeDef, index: usize) -> TypeDef {
        if let TypeDefKind::Struct { fields } = &mut def.kind {
            fields.insert(index, Field { ty: FieldType::Gap });
        }

        def
    }

    #[test]
    fn a_rule_of_one_alternative_is_a_tuple_struct_of_its_fields() {
        let written = render(&struct_of(&["Abc", "Def"]), &[], &trivia(None));

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
    fn a_gap_is_one_run_of_the_whitespace_type_taken_as_a_field_of_its_own() {
        let written = render(
            &gapped(struct_of(&["Abc", "Def"]), 1),
            &[],
            &trivia(Some("WS")),
        );

        assert!(
            written.contains("pub::core::option::Option<WS<I>>"),
            "{written}"
        );
        // The run is taken before the token it reaches, and both are fields.
        assert!(
            written.contains("parse::<::core::option::Option<WS<I>>>()?,input.parse::<Def<I>>()?"),
            "{written}"
        );
    }

    #[test]
    fn a_file_with_no_trivia_holds_nothing_between_two_operands() {
        // With nothing between its tokens, a gap is a field that takes nothing,
        // and a sequence of two operands is two fields and not four.
        let written = render(&gapped(struct_of(&["Abc", "Def"]), 1), &[], &trivia(None));

        assert!(
            written.contains("pubstructOwner<I>(pubAbc<I>,pub(),pubDef<I>);"),
            "{written}"
        );
        assert!(!written.contains("Option<"), "{written}");
    }

    #[test]
    fn a_node_that_reaches_its_owner_is_held_behind_a_pointer() {
        let written = render(&struct_of(&["Owner", "Else"]), &["Owner"], &trivia(None));

        assert!(
            written.contains("pub::std::boxed::Box<Owner<I>>"),
            "{written}"
        );
        assert!(!written.contains("Box<Else"), "{written}");
    }

    #[test]
    fn a_run_holds_one_repetition_with_the_trivia_before_it() {
        // What each repetition of `Abc*` takes is settled by `Trivia` to a group
        // of the gap and the operand, so the run of them is a `Vec` of that.
        let def = TypeDef {
            name: "Owner".to_owned(),
            kind: TypeDefKind::Struct {
                fields: vec![Field {
                    ty: FieldType::Many {
                        item: Box::new(FieldType::Group(vec![
                            FieldType::Gap,
                            FieldType::Named("Abc".to_owned()),
                        ])),
                    },
                }],
            },
        };

        let written = render(&def, &[], &trivia(Some("WS")));

        assert!(
            written.contains("Vec<(::core::option::Option<WS<I>>,Abc<I>)>"),
            "{written}"
        );
        // A run of them may be nothing at all, so there is no length to check.
        assert!(!written.contains("is_empty"), "{written}");
    }

    #[test]
    fn an_item_and_its_separator_are_one_punctuated_list() {
        let def = TypeDef {
            name: "Owner".to_owned(),
            kind: TypeDefKind::Struct {
                fields: vec![Field {
                    ty: FieldType::Punctuated {
                        item: Box::new(FieldType::Named("Abc".to_owned())),
                        sep: Box::new(FieldType::Named("Comma".to_owned())),
                    },
                }],
            },
        };

        let written = render(&def, &[], &trivia(Some("WS")));

        // The type is named through its crate, since the language being
        // described is free to declare a rule of the same name itself.
        assert!(
            written.contains("pub::parserc::syntax::Punctuated<Abc<I>,Comma<I>>"),
            "{written}"
        );
        assert!(
            written.contains("input.parse::<::parserc::syntax::Punctuated<Abc<I>,Comma<I>>>()?"),
            "{written}"
        );
    }

    #[test]
    fn two_terminals_holding_operands_are_one_delimiter() {
        let def = TypeDef {
            name: "Owner".to_owned(),
            kind: TypeDefKind::Struct {
                fields: vec![Field {
                    ty: FieldType::Delimited {
                        open: Box::new(FieldType::Named("Open".to_owned())),
                        body: vec![
                            FieldType::Named("Abc".to_owned()),
                            FieldType::Named("Def".to_owned()),
                        ],
                        close: Box::new(FieldType::Named("Close".to_owned())),
                    },
                }],
            },
        };

        let written = render(&def, &[], &trivia(Some("WS")));

        // The backend names its delimiter open, then close, then body.
        assert!(
            written.contains("pub::parserc::syntax::Delimiter<Open<I>,Close<I>,(Abc<I>,Def<I>)>"),
            "{written}"
        );
    }

    #[test]
    fn a_delimiter_holding_one_operand_names_no_tuple() {
        let def = TypeDef {
            name: "Owner".to_owned(),
            kind: TypeDefKind::Struct {
                fields: vec![Field {
                    ty: FieldType::Delimited {
                        open: Box::new(FieldType::Named("Open".to_owned())),
                        body: vec![FieldType::Named("Abc".to_owned())],
                        close: Box::new(FieldType::Named("Close".to_owned())),
                    },
                }],
            },
        };

        let written = render(&def, &[], &trivia(Some("WS")));

        assert!(
            written.contains("Delimiter<Open<I>,Close<I>,Abc<I>>"),
            "{written}"
        );
    }

    #[test]
    fn a_counted_run_is_taken_as_greedily_as_it_fits_and_then_checked() {
        let def = TypeDef {
            name: "Owner1".to_owned(),
            kind: TypeDefKind::Bounds {
                item: FieldType::Named("Abc".to_owned()),
                min: 2,
                max: Some(4),
            },
        };

        let written = render(&def, &[], &trivia(None));

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
        let def = TypeDef {
            name: "Owner1".to_owned(),
            kind: TypeDefKind::Bounds {
                item: FieldType::Named("Abc".to_owned()),
                min: 1,
                max: None,
            },
        };

        let written = render(&def, &[], &trivia(None));

        assert!(!written.contains("len()>="), "{written}");
        assert!(written.contains("if__items.len()<1"), "{written}");
    }

    #[test]
    fn except_is_asked_where_the_operand_starts() {
        let def = TypeDef {
            name: "Owner1".to_owned(),
            kind: TypeDefKind::Asserted {
                item: FieldType::Named("Abc".to_owned()),
                assert: Assert::Except(FieldType::Named("Def".to_owned())),
            },
        };

        let written = render(&def, &[], &trivia(None));

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
        let def = TypeDef {
            name: "Owner1".to_owned(),
            kind: TypeDefKind::Asserted {
                item: FieldType::Named("Abc".to_owned()),
                assert: Assert::Followed(FieldType::Named("Def".to_owned())),
            },
        };

        let written = render(&def, &[], &trivia(None));

        assert!(written.contains("__fits.is_none()"), "{written}");
        assert!(
            written.find("__taken").expect("an operand") < written.find("__fits").expect("a probe"),
            "{written}"
        );
    }

    #[test]
    fn every_alternative_is_tried_over_a_stream_that_is_rewound() {
        let def = TypeDef {
            name: "Owner".to_owned(),
            kind: TypeDefKind::Enum {
                variants: vec![
                    Variant {
                        name: "OwnerAbc".to_owned(),
                        fields: vec![Field {
                            ty: FieldType::Named("Abc".to_owned()),
                        }],
                    },
                    Variant {
                        name: "OwnerDef".to_owned(),
                        fields: vec![Field {
                            ty: FieldType::Named("Def".to_owned()),
                        }],
                    },
                ],
            },
        };

        let written = render(&def, &[], &trivia(None));

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
        let written = render(&struct_of(&["Abc", "Def"]), &[], &trivia(None));

        assert!(
            written.contains("Span::None.union(&self.0.to_span()).union(&self.1.to_span())"),
            "{written}"
        );
    }

    #[test]
    fn a_literal_taken_back_to_back_with_another_is_one_tuple_field() {
        let def = TypeDef {
            name: "Owner".to_owned(),
            kind: TypeDefKind::Struct {
                fields: vec![Field {
                    ty: FieldType::Concat {
                        first: Box::new(FieldType::Named("Uplus".to_owned())),
                        second: Box::new(FieldType::Many {
                            item: Box::new(FieldType::Named("Hex".to_owned())),
                        }),
                    },
                }],
            },
        };

        let written = render(&def, &[], &trivia(None));

        assert!(
            written.contains("pub(Uplus<I>,::std::vec::Vec<Hex<I>>)"),
            "{written}"
        );
    }
}
