//! The code the generator writes for `unsyn.us`, kept in the crate as files of
//! their own so that it is compiled rather than only printed.
//!
//! [`generated/unsyn_us.rs`](generated) gathers the three files a build writes —
//! `input.rs`, `lexer.rs` and `syntax.rs`, under `generated/unsyn_us/` — and the
//! tests here rebuild them and ask whether the two are the same code, file for
//! file. It is the only place generated code is checked to *mean* what the
//! generator was asked for: a file that does not compile, or that reads a rule
//! the spec did not write, is caught here and not in whoever comes to depend on
//! it.
//!
//! Reading the files is one test; running the code they hold is another, and
//! [`and_it_reads`] does that.
//!
//! No `error.rs` is among them. This crate's analysis reports what a well formed
//! token still denotes wrongly and what no code answers, and those families are
//! the DSL's own rather than anything the spec declares, so the build was told
//! where the error stands instead of asked to write one.

use std::path::Path;
use std::str::FromStr;

use parserc::ToSpan;
use quote::ToTokens;
use unsyn::{
    codegen::{Builder, Module},
    input::TokenStream,
    semantic, syntax,
};

/// The code of the spec, kept beside this test so that it is compiled with it.
#[path = "generated/unsyn_us.rs"]
mod generated;

/// The directory the files of the spec are kept in.
const GENERATED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/generated/unsyn_us");

/// The scan the generated file binds `<IdentOrKeyword>` to, re-exported at
/// this crate's root, which is the path the binding reaches.
#[path = "common/scan.rs"]
mod scan;

pub use scan::scan_ident_or_keyword;

/// The code of the terminals and the nodes `unsyn.us` declares, as the files a
/// build writes them into.
///
/// The one external the spec calls is bound here rather than by anything the
/// generator knows: `IdentOrKeyword` is the name a file of this language asks
/// its caller for, and a build answers it with the scan above.
fn modules_of_the_spec() -> Vec<Module> {
    let source = include_str!("../unsyn.us");

    // Reading the spec is the builder's own work now: the stream, the tree and
    // the analysis all happen inside `from_source`.
    Builder::from_source(source)
        .expect("unsyn.us should be a file of the language")
        // `IdentOrKeyword` is the name a file of this language asks its caller
        // for, and a build answers it with the scan above.
        .external(
            "IdentOrKeyword",
            proc_macro2::TokenStream::from_str("crate::scan_ident_or_keyword").unwrap(),
        )
        // The kept files stand inside this crate, beside the analysis that
        // reports what a token denotes wrongly and what no code answers. A
        // second error written here would be a different enum from the one the
        // stream of the crate reports, so the build reaches the crate's own.
        .error_at(
            "UnsynError",
            proc_macro2::TokenStream::from_str("::unsyn::errors").unwrap(),
        )
        .modules()
        .expect("unsyn.us should be generated whole")
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

/// The code of a kept file, without the words that say what the file is.
///
/// The two files are compared as tokens rather than as text, and only their
/// items are compared: the header belongs to the keeping of a file and not to
/// the code, and how the code is laid out is the business of `cargo fmt`.
fn items_of(code: &str) -> proc_macro2::TokenStream {
    let mut file = syn::parse_file(code).expect("the code should be Rust");

    file.attrs.clear();
    file.shebang = None;

    file.to_token_stream()
}

/// Whether the files the code is kept in are the code the generator writes,
/// each against its own.
#[test]
fn the_kept_file_is_the_code_of_the_spec() {
    let modules = modules_of_the_spec();
    let kept = ["input.rs", "lexer.rs", "syntax.rs", "visit.rs"];

    assert_eq!(
        modules.iter().map(|module| module.file).collect::<Vec<_>>(),
        kept,
        "which files a build of the spec writes is what the keeping is made of",
    );

    for module in modules {
        let path = Path::new(GENERATED).join(module.file);
        let kept = std::fs::read_to_string(&path).expect("the generated file should be kept");

        assert_eq!(
            as_code(items_of(&kept)),
            as_code(items_of(&module.code.to_string())),
            "the code kept in {} is not the code of the spec; run \
             `cargo test -p unsyn --test unsyn_us the_generated_files_of_the_spec -- --ignored` \
             and then `cargo fmt` to rewrite it",
            path.display(),
        );
    }
}

/// Reading with the code the generator wrote, rather than only writing it.
///
/// [`the_kept_file_is_the_code_of_the_spec`] asks whether the file kept is the
/// code that was asked for. The tests here ask whether that code reads: a rule
/// that compiles and then takes the wrong span, or takes nothing, or takes what
/// the rule after it should have, is caught only by running it.
///
/// Two streams are read, one per row of the table the two phases make. The
/// stream of this crate is what a build is asked for by default: the name of
/// the input trait it writes is `UnsynInput`, so the file it writes is the
/// parser this crate would bootstrap itself with, refusals and all — what it
/// answers with is an [`UnsynError`] rather than a [`Kind`].
///
/// The other half is that this is a *default* and not a fact about the code:
/// `a_build_aimed_at_another_crate_names_that_crate`, at the foot of this
/// file, configures the same generator with a name of its own and gets a file
/// that reaches nothing of unsyn at all, which is what lets a grammar live in
/// a crate that never depends on unsyn.
mod and_it_reads {
    use super::generated::{
        BlockComment, Ident, LitStr, LitUnicode, S, TokenStream, UnicodeEscape,
    };
    use parserc::{
        AsStr,
        syntax::{Syntax, SyntaxInput},
    };
    use unsyn::errors::UnsynError;

    /// Reads the node `T` from the start of `source`.
    fn parse<'a, T>(source: &'a str) -> Result<T, UnsynError>
    where
        T: Syntax<TokenStream<'a>>,
    {
        TokenStream::from(source).parse::<T>()
    }

    /// Which of the two forms of [`Ident`] was read, and what it read.
    ///
    /// The raw form is a name of two parts — the sigil and, behind it, a name
    /// read again by the rule that names no word — and the plain form is the one
    /// text the `except` left standing. Both answer with the whole of the name,
    /// sigil and all, which is what a definition's name is.
    fn name(node: &Ident<TokenStream<'_>>) -> (String, &'static str) {
        match node {
            Ident::RawIdent(raw) => (
                raw.f1.as_str().to_owned() + raw.non_keyword_ident.0.as_str(),
                "RawIdent",
            ),
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
        let raw = parse::<Ident<_>>("r#type").expect("a raw name");
        assert_eq!(name(&raw), ("r#type".to_owned(), "RawIdent"));

        let plain = parse::<Ident<_>>("lexer2").expect("a name that is no word");
        assert_eq!(name(&plain), ("lexer2".to_owned(), "NonKeywordIdent"));

        // The sigil borrows a word of Rust, not one of the language: the rule it
        // reaches is the one that refuses the words of the DSL, and its refusal
        // is fatal, so `r#lexer` is no name and no other form is tried.
        assert!(
            parse::<Ident<_>>("r#lexer").is_err(),
            "a word of the language under a sigil is still that word"
        );

        assert!(
            parse::<Ident<_>>("lexer").is_err(),
            "a word of the language"
        );
    }

    #[test]
    fn a_unicode_literal_wants_four_digits() {
        // The two sub-expressions of the rule are the two fields of the node:
        // the `U+` and the four digits behind it.
        let lit = parse::<LitUnicode<_>>("U+0041").expect("four digits");
        assert_eq!(lit.f1.as_str(), "U+");
        assert_eq!(lit.hex_digit.len(), 4, "one node per digit");

        assert!(parse::<LitUnicode<_>>("U+004").is_err(), "three digits");

        // Four digits and no more: a fifth is not part of the literal but the
        // text of whatever operand comes next.
        let mut input = TokenStream::from("U+00419");
        let lit = input
            .parse::<LitUnicode<_>>()
            .expect("the first four digits");
        assert_eq!(lit.hex_digit.len(), 4);
        assert_eq!(input.as_str(), "9");
    }

    #[test]
    fn an_escape_takes_one_to_six_digits_and_no_separator() {
        // The three sub-expressions of the rule are the three fields of the
        // node: the marker, the digits and the brace that closes them.
        let escape = parse::<UnicodeEscape<_>>(r"\u{1F600}").expect("an escape of five digits");
        assert_eq!(escape.f1.as_str(), r"\u{");
        assert_eq!(escape.hex_digit.len(), 5, "one node per digit");
        assert_eq!(escape.f2.as_str(), "}");

        assert!(
            parse::<UnicodeEscape<_>>(r"\u{}").is_err(),
            "no digits at all"
        );

        // The rule bounds the digits and writes nothing between them, so a
        // separator is no part of an escape: the six digits are taken and the
        // brace is then asked for where the `_` stands.
        let mut input = TokenStream::from(r"\u{1F_600}");
        assert!(
            input.parse::<UnicodeEscape<_>>().is_err(),
            "an underscore separates no digits"
        );
    }

    #[test]
    fn a_comment_closes_the_one_it_opened() {
        // A comment that nests is the rule's body, which is the four
        // sub-expressions the production writes: the opening, its head, the
        // runs after the head, and the closing that belongs to this opening.
        let BlockComment::BlockCommentBody(body) =
            parse::<BlockComment<_>>("/* a /* b */ c */").expect("a nested comment")
        else {
            panic!("a comment that nests is not the rule's body");
        };

        assert_eq!(
            format!(
                "{}{}{}{}",
                body.0.as_str(),
                body.1.as_str(),
                body.2.as_str(),
                body.3.as_str()
            ),
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

    use super::generated::{Ident, LitStr, TokenStream};
    use unsyn::errors::UnsynError;

    /// Reads the node `T` from the start of `source`, over the stream the
    /// generated front end names — which is the same concrete stream this
    /// crate reads, reached by the path the build wrote into the file.
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
                parse::<Ident<_>>("r#type").expect("a raw name"),
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
    use parserc::syntax::SyntaxInput as _;
    use parserc::{AsStr, Input};

    use crate::generated::{
        Crate as GeneratedCrate, Item as GeneratedItem, TokenStream as GeneratedStream,
    };
    use unsyn::input::TokenStream as UnsynStream;
    use unsyn::{syntax, syntax::Item};

    const SOURCE: &str = include_str!("../unsyn.us");

    let mut hand = UnsynStream::from(SOURCE);
    let hand = hand
        .parse::<syntax::Crate<_>>()
        .expect("the hand parser reads its own spec");

    // Each front end names its own stream, and the two are the same concrete
    // stream over the same error — which is what lets the two trees be
    // compared span for span. A build aimed elsewhere would have been
    // configured to name another. See [`a_build_aimed_at_another_crate_names_that_crate`].
    let mut generated = GeneratedStream::from(SOURCE);
    let tree = generated
        .parse::<GeneratedCrate<_>>()
        .expect("the generated parser reads the spec it was generated from");

    assert!(
        generated.is_empty(),
        "the generated parser stopped at: {:?}",
        &generated.as_str()[..generated.as_str().len().min(200)]
    );

    // The generated `Crate` is the struct its pattern writes: two repeated
    // operands, each called by the rule it holds.
    let defined = |name: &unsyn::lexer::Ident<UnsynStream<'_>>| name.to_span();

    let hand_names: Vec<_> = hand
        .item
        .iter()
        .filter_map(|item| match item {
            Item::Stmt(stmt) => Some(defined(&stmt.ident)),
            _ => None,
        })
        .collect();

    let generated_names: Vec<_> = tree
        .item
        .iter()
        .filter_map(|item| match item {
            GeneratedItem::Stmt(stmt) => Some(stmt.ident.to_span()),
            _ => None,
        })
        .collect();

    assert_eq!(hand_names.len(), 60, "every definition of the spec");
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
        I: crate::generated::UnsynInput,
    {
        fn visit_text(&mut self, text: &'ast I) {
            self.0.push_str(text.as_str());
        }
    }

    let mut input = crate::generated::TokenStream::from(SOURCE);
    let tree = input
        .parse::<crate::generated::Crate<crate::generated::TokenStream<'_>>>()
        .expect("the generated parser reads the spec");

    let mut echo = Echo(String::new());
    echo.visit_crate(&tree);

    assert_eq!(echo.0, SOURCE, "every character, exactly once, in order");
}

/// A build aimed at another crate names that crate's stream, and nothing here.
///
/// The default input name is the compiler's own, which is what makes the file
/// a drop-in front end for this crate. A grammar of another language lives in
/// a crate that never depends on unsyn, and says so by giving the generator
/// the name to write and the path to find it at. Renaming is the whole of
/// aiming elsewhere: the file reaches its stream only through that name.
#[test]
fn a_build_aimed_at_another_crate_names_that_crate() {
    use parserc::syntax::SyntaxInput as _;
    use proc_macro2::TokenStream as Syntax;

    const SOURCE: &str = "whitespace W -> [' ']; lexer A -> 'a'; syntax B -> A;";

    let whole = TokenStream::from(SOURCE);
    let mut input = TokenStream::from(SOURCE);
    let file = input
        .parse::<syntax::Crate<_>>()
        .expect("a file of the language");

    let report = semantic::analyze(&whole, &file);
    assert!(report.errors.is_empty(), "{SOURCE}: {:#?}", report.errors);

    let code = Builder::from_source(SOURCE)
        .expect("a file of the language")
        .input_name("GrammarInput")
        .input_at(Syntax::from_str("::other::GrammarInput").unwrap())
        .build()
        .expect("the file should be generated")
        .to_string();

    // The name the builder was given is the name every bound wears, and it
    // comes from the path the builder was given.
    assert!(code.contains("use :: other :: GrammarInput ;"), "{code}");
    assert!(code.contains("I : GrammarInput"), "{code}");

    // Nothing of this crate survives the renaming, which is the whole of what
    // a crate that never depends on unsyn needs to be true.
    assert!(!code.contains("UnsynInput"), "{code}");
    assert!(!code.contains("unsyn"), "{code}");
}

/// Whether a build of the spec writes the four files, and each holds what its
/// kind of code is.
///
/// The kept file is one module, which is what a build hands back when it was
/// given no directory; the same code split into files has to say the same thing
/// and reach its neighbours for it. Both are asked here of the spec itself, and
/// not of a grammar of two rules, because the spec is the file whose parts a
/// crate is made of: the separator, the externals, and the rules that nest.
#[test]
fn a_build_of_the_spec_writes_one_file_per_kind_of_code() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("spec-split");
    let _ = std::fs::remove_dir_all(&dir);

    let source = include_str!("../unsyn.us");
    let flat = Builder::from_source(source)
        .expect("unsyn.us should be a file of the language")
        .project("unsyn")
        .external(
            "IdentOrKeyword",
            proc_macro2::TokenStream::from_str("crate::scan_ident_or_keyword").unwrap(),
        )
        .to_dir(dir.clone())
        .build()
        .expect("unsyn.us should be generated whole")
        .to_string();

    let file = |name: &str| {
        std::fs::read_to_string(dir.join(name)).unwrap_or_else(|_| panic!("{name} written"))
    };

    // Five files, in emit mode: no path was said for the stream or the error.
    let mut names = std::fs::read_dir(&dir)
        .expect("the directory should be readable")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();

    names.sort();
    assert_eq!(
        names,
        ["error.rs", "input.rs", "lexer.rs", "syntax.rs", "visit.rs"]
    );

    // The language names the two items that are no rule of it, and each file
    // reaches the neighbour that defines them.
    let input = file("input.rs");

    assert!(input.contains("pub trait UnsynInput :"), "{input}");
    assert!(
        input.contains("use super :: error :: UnsynError ;"),
        "{input}"
    );
    assert!(
        file("error.rs").contains("pub enum UnsynError"),
        "{}",
        file("error.rs")
    );

    // The terminals are one file and the nodes another. A file of a build
    // reaches nothing of the crate that ran it, since it is written to stand
    // wherever its own language stands.
    let lexer = file("lexer.rs");

    assert!(
        lexer.contains("use super :: input :: UnsynInput ;"),
        "{lexer}"
    );
    assert!(lexer.contains("pub struct LitStr"), "{lexer}");
    assert!(!lexer.contains("pub struct Stmt"), "{lexer}");

    let syntax = file("syntax.rs");

    assert!(syntax.contains("pub struct Stmt"), "{syntax}");
    // Every terminal a node is read out of is imported by name, and no more than
    // that: a name no node wears is no import of this file's.
    assert!(syntax.contains("use super :: lexer ::"), "{syntax}");

    // The visitor is a file of its own, since it descends through both kinds of
    // rule and belongs to neither of the files that holds them.
    let visit = file("visit.rs");

    assert!(visit.contains("pub trait Visit"), "{visit}");
    assert!(visit.contains("use super :: lexer ::"), "{visit}");
    assert!(visit.contains("use super :: syntax ::"), "{visit}");
    assert!(!syntax.contains("pub trait Visit"), "{syntax}");

    // The whole of the code is the files without their imports: the split moves
    // no item and writes no item twice. Compared as a set, because one stream
    // keeps the order the file declared its rules in, across both kinds, where
    // each file holds one kind in that order.
    let without_imports = |code: &str| {
        let mut items = syn::parse_file(code)
            .expect("the file should be Rust")
            .items
            .into_iter()
            .filter(|item| !matches!(item, syn::Item::Use(_)))
            .map(|item| quote::quote! { #item }.to_string())
            .collect::<Vec<_>>();

        items.sort();
        items
    };

    let mut split = Vec::new();

    for code in [&input, &file("error.rs"), &lexer, &syntax, &visit] {
        split.extend(without_imports(code));
    }

    split.sort();

    assert_eq!(split, without_imports(&flat));
}

/// A kept file: the words that say what the keeping is, then the code as the
/// generator wrote it, in one line.
///
/// `cargo fmt` lays the file out afterwards, which is why the code is written
/// as it comes from a build rather than as it is read.
fn kept_file(header: &str, code: &str) -> String {
    format!("{header}{code}\n")
}

/// The header of a kept file, read back so that the keeping says what it said
/// rather than what this test remembers of it: the document lines and the
/// `#![allow(..)]` list, up to and including the line that closes the list.
fn header_of(kept: &str) -> String {
    match kept.find("\n)]\n") {
        Some(at) => kept[..at + 4].to_owned(),
        None => {
            kept.lines()
                .take_while(|line| line.starts_with("//!"))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        }
    }
}

/// Rewrites the files the code is kept in, for when the generator changed.
///
/// The file written for the first time gets no header, which is what the
/// keeping — and not a build — is made of; run this twice, or write the header
/// once by hand, to get one.
#[test]
#[ignore = "rewrites files the repository keeps; run with --ignored"]
fn the_generated_files_of_the_spec() {
    for module in modules_of_the_spec() {
        let path = Path::new(GENERATED).join(module.file);
        let kept = std::fs::read_to_string(&path).unwrap_or_default();
        let header = if kept.is_empty() {
            String::new()
        } else {
            header_of(&kept)
        };

        std::fs::write(&path, kept_file(&header, &module.code.to_string()))
            .expect("the file should be written");
    }
}
