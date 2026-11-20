//! The code the generator writes for `unsyn.us`, kept in the crate as a file of
//! its own so that it is compiled rather than only printed.
//!
//! [`generated/unsyn_us.rs`](generated) is the half of the spec a build writes
//! today — the `whitespace` rule and the `lexer` rules, whose nodes are the text
//! they read — and the test here rebuilds it and asks whether the two are the
//! same code. It is the only place a generated file is checked to *mean* what the
//! generator was asked for: a file that does not compile, or that reads a rule the
//! spec did not write, is caught here and not in whoever comes to depend on it.
//!
//! Reading the file is one test; running the code it holds is another, and
//! [`and_it_reads`] does that.
//!
//! Only the terminals are in it. A `syntax` production makes a node of the tree
//! rather than a run of the text, and no code is written for such a node yet, so a
//! build of the whole spec is refused rather than half-written; see the
//! [module documentation](unsyn::compiler::codegen) of the generator.

use std::path::Path;
use std::str::FromStr;

use parserc::syntax::SyntaxInput;
use quote::ToTokens;
use unsyn::compiler::{codegen::Builder, input::TokenStream, semantic, syntax};

/// The code of the spec, kept beside this test so that it is compiled with it.
#[path = "generated/unsyn_us.rs"]
mod generated;

/// The path of the file the code of the spec is kept in.
const GENERATED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/generated/unsyn_us.rs");

/// The scan the generated file binds `<IdentOrKeyword>` to, re-exported at
/// this crate's root, which is the path the binding reaches.
#[path = "common/scan.rs"]
mod scan;

pub use scan::scan_ident_or_keyword;

/// The code of the terminals `unsyn.us` declares.
///
/// The one external the spec calls is bound here rather than by anything the
/// generator knows: `IdentOrKeyword` is the name a file of this language asks
/// its caller for, and a build answers it with the scan above.
fn code_of_the_spec() -> String {
    let source = include_str!("../unsyn.us");

    let mut input = TokenStream::from(source);
    let file = input
        .parse::<syntax::Crate<_>>()
        .expect("unsyn.us should be a file of the language");

    let report = semantic::analyze(&file);
    assert!(report.errors.is_empty(), "unsyn.us: {:#?}", report.errors);

    let mut builder = Builder::new(&file);
    builder.external(
        "IdentOrKeyword",
        proc_macro2::TokenStream::from_str("crate::scan_ident_or_keyword").unwrap(),
    );

    builder
        .build()
        .expect("unsyn.us should be generated whole")
        .to_string()
}

/// The code as text, of a file laid out however `cargo fmt` likes it.
///
/// Rust lets a list broken over lines end with a comma that the same list written
/// on one line does not carry, and `cargo fmt` breaks them wherever it can, so the
/// comma with nothing after it inside its own group is dropped. What is left is
/// what the code is rather than how it is written down.
fn as_code(tokens: proc_macro2::TokenStream) -> String {
    use proc_macro2::{Delimiter, TokenTree};

    let mut trees: Vec<TokenTree> = tokens.into_iter().collect();

    while let Some(TokenTree::Punct(punct)) = trees.last()
        && punct.as_char() == ','
    {
        trees.pop();
    }

    // `cargo fmt` also ends a broken list of generic arguments with a comma
    // that the same list on one line does not carry, and the closer of a
    // generic list is a plain `>` punct rather than a group: the comma that
    // stands before one drops as surely as the comma at a group's end.
    let mut dropped: Vec<bool> = vec![false; trees.len()];

    for i in 0..trees.len().saturating_sub(1) {
        if let (TokenTree::Punct(comma), TokenTree::Punct(closer)) = (&trees[i], &trees[i + 1])
            && comma.as_char() == ','
            && closer.as_char() == '>'
        {
            dropped[i] = true;
        }
    }

    trees
        .iter()
        .zip(dropped)
        .filter(|(_, drop)| !*drop)
        .map(|(tree, _)| match tree {
            TokenTree::Group(group) => {
                let inner = as_code(group.stream());

                let (open, close) = match group.delimiter() {
                    Delimiter::Parenthesis => ("(", ")"),
                    Delimiter::Brace => ("{", "}"),
                    Delimiter::Bracket => ("[", "]"),
                    Delimiter::None => ("", ""),
                };

                format!("{open}{inner}{close}")
            }
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether the file the code is kept in is the code the generator writes.
///
/// The two are compared as tokens rather than as text, and only their items are
/// compared: the words that say what the file is belong to the keeping of it and
/// not to the code. How it is laid out is the business of `cargo fmt`.
#[test]
fn the_kept_file_is_the_code_of_the_spec() {
    let kept = std::fs::read_to_string(GENERATED).expect("the generated file should be kept");

    let items = |code: &str| {
        let mut file = syn::parse_file(code).expect("the code should be Rust");

        // What the file says it is is not part of the code it is.
        file.attrs.clear();
        file.shebang = None;

        file.to_token_stream()
    };

    assert_eq!(
        as_code(items(&kept)),
        as_code(items(&code_of_the_spec())),
        "the code kept in {} is not the code of the spec; \
         run `cargo test -p unsyn --all-features --test unsyn_us the_generated_file_of_the_spec \
         -- --ignored` and then `cargo fmt` to rewrite it",
        GENERATED
    );
}

/// Reading with the code the generator wrote, rather than only writing it.
///
/// [`the_kept_file_is_the_code_of_the_spec`] asks whether the file kept is the
/// code that was asked for. The tests here ask whether that code reads: a rule
/// that compiles and then takes the wrong span, or takes nothing, or takes what
/// the rule after it should have, is caught only by running it.
///
/// Two streams are read, one per row of the table the two phases make. The plain
/// [`parserc`] char stream says that the code a build writes names nothing of
/// this crate, so a grammar can live in a crate that never depends on unsyn.
/// The stream of this crate says the other half, which is that the same code is
/// the parser this crate would bootstrap itself with, refusals and all: what it
/// answers with is an [`UnsynError`] rather than a [`Kind`], and the generated
/// code is none the wiser for which of the two it is handing back.
mod and_it_reads {
    use parserc::{
        AsStr, Kind,
        chars::TokenStream,
        syntax::{Syntax, SyntaxInput},
    };

    use super::generated::{BlockComment, Ident, LitStr, LitUnicode, S, UnicodeEscape};

    /// Reads the node `T` from the start of `source`.
    fn parse<'a, T>(source: &'a str) -> Result<T, Kind>
    where
        T: Syntax<TokenStream<'a>>,
    {
        TokenStream::from(source).parse::<T>()
    }

    /// Which of the two forms of [`Ident`] was read, and what it read.
    ///
    /// The raw form is a name of two parts — the sigil and, behind it, a name
    /// read again by the rule that names no word — and the plain form is the
    /// one text the `except` left standing.
    fn name(node: &Ident<TokenStream<'_>>) -> (String, &'static str) {
        match node {
            Ident::RawIdent(raw) => (raw.0.as_str().to_owned(), "RawIdent"),
            Ident::NonKeywordIdent(plain) => (plain.0.as_str().to_owned(), "NonKeywordIdent"),
        }
    }

    #[test]
    fn a_character_literal_is_one_token() {
        for (source, literal) in [
            ("'a'", "'a'"),
            (r"'\''", r"'\''"),
            (r"'\n'", r"'\n'"),
            (r"'\u{41}'", r"'\u{41}'"),
            ("'lexer'", "'lexer'"),
            ("'x' 'y'", "'x'"),
        ] {
            let read = parse::<LitStr<_>>(source).expect(source);

            // The literal is its two quotes and the text the body read — a
            // tuple of them, which is what a sequence of nothing but parts
            // with no name of their own comes out as.
            let text = format!("{}{}{}", read.0.as_str(), read.1.as_str(), read.2.as_str());

            assert_eq!(text, literal, "{source}");
        }
    }

    #[test]
    fn a_name_says_which_of_its_two_forms_it_is() {
        let raw = parse::<Ident<_>>("r#lexer").expect("a raw name");
        assert_eq!(name(&raw), ("r#lexer".to_owned(), "RawIdent"));

        let plain = parse::<Ident<_>>("lexer2").expect("a name that is no word");
        assert_eq!(name(&plain), ("lexer2".to_owned(), "NonKeywordIdent"));

        assert!(
            parse::<Ident<_>>("lexer").is_err(),
            "a word of the language"
        );
    }

    #[test]
    fn a_unicode_literal_wants_four_digits() {
        assert_eq!(
            parse::<LitUnicode<_>>("U+0041")
                .expect("four digits")
                .0
                .as_str(),
            "U+0041"
        );
        assert!(parse::<LitUnicode<_>>("U+004").is_err(), "three digits");
    }

    #[test]
    fn an_escape_counts_its_groups() {
        assert_eq!(
            parse::<UnicodeEscape<_>>(r"\u{1F_600}")
                .expect("an escape with its underscores")
                .0
                .as_str(),
            r"\u{1F_600}"
        );
        assert!(
            parse::<UnicodeEscape<_>>(r"\u{}").is_err(),
            "no digits at all"
        );
    }

    #[test]
    fn a_comment_closes_the_one_it_opened() {
        assert_eq!(
            parse::<BlockComment<_>>("/* a /* b */ c */")
                .expect("a nested comment")
                .0
                .as_str(),
            "/* a /* b */ c */"
        );
    }

    #[test]
    fn the_whitespace_is_taken_greedily() {
        assert_eq!(
            parse::<S<_>>(" \u{a0}\t\n ")
                .expect("a run of whitespace")
                .0
                .as_str(),
            " \u{a0}\t\n "
        );
    }
}

/// The same code read over the stream this crate reads its own language with.
///
/// This is the shape the bootstrap takes: the terminals of `unsyn.us`, generated
/// into a parser of `unsyn.us`. What differs from [`and_it_reads`] is only the
/// stream, and with it the type a refusal answers with.
mod and_it_reads_this_crate {
    use parserc::{
        AsStr,
        syntax::{Syntax, SyntaxInput},
    };

    use super::generated::{Ident, LitStr};
    use unsyn::compiler::{errors::UnsynError, input::TokenStream};

    /// Reads the node `T` from the start of `source`, over this crate's input.
    fn parse<'a, T>(source: &'a str) -> Result<T, UnsynError>
    where
        T: Syntax<TokenStream<'a>>,
    {
        TokenStream::from(source).parse::<T>()
    }

    #[test]
    fn the_generated_terminals_read_the_input_of_this_crate() {
        let lit = parse::<LitStr<_>>("'a'").expect("a literal");

        assert_eq!(
            format!("{}{}{}", lit.0.as_str(), lit.1.as_str(), lit.2.as_str()),
            "'a'"
        );

        assert!(
            matches!(
                parse::<Ident<_>>("r#lexer").expect("a raw name"),
                Ident::RawIdent(_)
            ),
            "a name read over this crate's own stream should still say which form"
        );

        assert!(
            parse::<Ident<_>>("lexer").is_err(),
            "a word of the language"
        );
    }
}

/// The whole generated file reading the whole spec it was generated from.
///
/// This is the bootstrap's first closed circle: the code `unsyn.us` writes
/// reads `unsyn.us`, and what it reads is the file the hand tree reads —
/// definition for definition, name for name, span for span. A generation
/// that parses its own spec to a different tree is wrong wherever the two
/// disagree.
#[test]
fn the_generated_parser_reads_the_spec_it_was_generated_from() {
    use parserc::chars::TokenStream as Chars;
    use parserc::syntax::{Syntax, SyntaxInput as _};
    use parserc::{AsStr, Input};

    use crate::generated::{Crate as GeneratedCrate, Item as GeneratedItem};
    use unsyn::compiler::input::TokenStream as UnsynStream;
    use unsyn::compiler::{syntax, syntax::Item};

    const SOURCE: &str = include_str!("../unsyn.us");

    let mut hand = UnsynStream::from(SOURCE);
    let hand = hand
        .parse::<syntax::Crate<_>>()
        .expect("the hand parser reads its own spec");

    let mut generated = Chars::<parserc::Kind>::from(SOURCE);
    let tree = generated
        .parse::<GeneratedCrate<_>>()
        .expect("the generated parser reads the spec it was generated from");

    assert!(
        generated.is_empty(),
        "the generated parser stopped at: {:?}",
        &generated.as_str()[..generated.as_str().len().min(200)]
    );

    // The generated `Crate` is the struct its pattern writes: two repeated
    // operands, nameless and numbered, the items being the second.
    let defined = |name: &unsyn::compiler::lexer::Ident<UnsynStream<'_>>| name.to_span();

    let hand_names: Vec<_> = hand
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Stmt(stmt) => Some(defined(&stmt.ident)),
            _ => None,
        })
        .collect();

    let generated_names: Vec<_> = tree
        .1
        .iter()
        .filter_map(|item| match item.as_ref() {
            GeneratedItem::Stmt(stmt) => Some(stmt.ident.to_span()),
            _ => None,
        })
        .collect();

    assert_eq!(hand_names.len(), 37, "every definition of the spec");
    assert_eq!(
        generated_names, hand_names,
        "the same definitions, where they were written"
    );
}

/// The whole generated file walked by the whole generated visitor.
///
/// The visitor the generator writes descends through the tree the generated
/// parser reads, and every character of the source is handed to exactly one
/// `visit_text` — the same invariant the hand tree pins, asserted of the
/// generated pair: nothing doubled, nothing dropped.
#[test]
fn the_generated_visitor_hands_over_every_character_once() {
    use parserc::syntax::SyntaxInput as _;

    use crate::generated::Visit as _;

    const SOURCE: &str = include_str!("../unsyn.us");

    struct Echo(String);

    impl<'ast, I> crate::generated::Visit<'ast, I> for Echo
    where
        I: parserc::chars::CharsInput,
    {
        fn visit_text(&mut self, text: &'ast I) {
            self.0.push_str(text.as_str());
        }
    }

    let mut input = parserc::chars::TokenStream::<parserc::Kind>::from(SOURCE);
    let tree = input
        .parse::<crate::generated::Crate<parserc::chars::TokenStream<parserc::Kind>>>()
        .expect("the generated parser reads the spec");

    let mut echo = Echo(String::new());
    echo.visit_crate(&tree);

    assert_eq!(echo.0, SOURCE, "every character, exactly once, in order");
}

/// Rewrites the file the code is kept in, for when the generator changed.
///
/// The file is written as the code comes out of the generator, in one line; run
/// `cargo fmt` afterwards to lay it out.
///
/// The header the file is kept in belongs to the keeping and not to the code, so
/// it says what the code will not: that a set the spec spells as a run of
/// characters is written as that run, which is what the lint about ASCII ranges
/// asks for instead, and that not every rule of the spec is read by anything here
/// yet.
#[test]
#[ignore = "rewrites a file the repository keeps; run with --ignored"]
fn the_generated_file_of_the_spec() {
    std::fs::write(
        Path::new(GENERATED),
        format!(
            "//! The code of `unsyn.us`, as the generator writes it.\n\
                 //!\n\
                 //! This file is generated: see `tests/unsyn_us.rs`. Do not edit it.\n\
                 #![allow(
    dead_code,
    clippy::manual_is_ascii_check,
    clippy::needless_borrow,
    clippy::type_complexity
)]\n\
                 {}\n",
            code_of_the_spec()
        ),
    )
    .expect("the file should be written");
}
