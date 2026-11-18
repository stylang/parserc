use std::{fs, path::PathBuf};

use parserc::{AsStr, syntax::Syntax};
use quote::quote;
use unsyn::{
    codegen::Codegen,
    input::TokenStream,
    semantics::{NodeKind, analyze},
    syntax::{Crate, Item},
};

/// Reads `crates/unsyn/bootstrap.uns`, kept alive for the modules built from it.
fn read_bootstrap() -> &'static str {
    let filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap.uns")
        .canonicalize()
        .unwrap();

    // The analyzed module borrows its source, so the text is kept alive here.
    Box::leak(fs::read_to_string(&filepath).unwrap().into_boxed_str())
}

/// Parses `crates/unsyn/bootstrap.uns`.
///
/// The spec of `unsyn` written in `unsyn` is the toughest input the analyzer
/// has: it uses forward references, mutual recursion between the comment rules,
/// and the two externals its caller is asked to supply.
fn parse_bootstrap() -> Crate<TokenStream<'static>> {
    let content = read_bootstrap();

    let mut token_stream = TokenStream::from(content);

    // `Crate` is read to its end, so a spec with text no item can read fails to
    // parse rather than being a file that was quietly cut short.
    Crate::parse(&mut token_stream).unwrap_or_else(|e| panic!("parse the spec: {e}"))
}

/// Counts the statements of a parsed spec.
fn statements(file: &Crate<TokenStream<'static>>) -> usize {
    file.items
        .iter()
        .filter(|item| matches!(item, Item::Stmt(_)))
        .count()
}

#[test]
fn bootstrap() {
    let c = parse_bootstrap();

    for item in c.items.iter() {
        println!("{:?}", item.to_span());
    }
}

#[test]
fn bootstrap_semantics() {
    let c = parse_bootstrap();

    let statements = statements(&c);

    let module = match analyze(&c) {
        Ok(module) => module,
        Err(errors) => {
            for error in errors.iter() {
                println!("{error}");
            }

            panic!("bootstrap.uns is not well-formed");
        }
    };

    // Every node of the spec is declared, and declared exactly once.
    assert!(statements > 20, "the spec looks truncated");
    assert_eq!(module.symbols().len(), statements);

    // The trivia of the language is declared by one `whitespace` node.
    assert_eq!(module.definition("S").unwrap().kind(), NodeKind::Whitespace);

    // A node of the tree is not a terminal pattern.
    assert_eq!(module.definition("Expr").unwrap().kind(), NodeKind::Syntax);
    assert_eq!(module.definition("LF").unwrap().kind(), NodeKind::Lexer);

    // The only externals the spec expects from outside the file. A spec may
    // name them as it likes; nothing in unsyn knows either name.
    assert_eq!(module.externals(), ["XID_Start", "XID_Continue"]);

    // The spec says nothing twice: every member of every set it enumerates
    // denotes a value no earlier member of that set already denotes.
    assert!(
        module.lints().is_empty(),
        "the spec has redundant members: {:#?}",
        module.lints()
    );
}

#[test]
fn a_spec_written_with_crlf_line_endings_is_the_same_spec() {
    // A block body keeps the `CR` of its line endings, so a spec edited on
    // Windows is read as the very same program rather than as broken documents.
    let crlf: &'static str = Box::leak(read_bootstrap().replace('\n', "\r\n").into_boxed_str());

    let mut token_stream = TokenStream::from(crlf);

    let file = match Crate::parse(&mut token_stream) {
        Ok(file) => file,
        Err(error) => panic!("a CRLF spec must parse: {error}"),
    };

    let module = match analyze(&file) {
        Ok(module) => module,
        Err(errors) => {
            for error in errors.iter() {
                println!("{error}");
            }

            panic!("a CRLF spec must be well-formed");
        }
    };

    // The same spec written with `LF` declares exactly these nodes.
    assert_eq!(
        module.symbols().len(),
        statements(&parse_bootstrap()),
        "a CRLF spec declares something else"
    );
}

/// The names a spec gives its nodes are the names of the Rust types the
/// generator emits, so the spec of `unsyn` writes them the way Rust writes its
/// own types: an uppercase first letter, and no word in capitals.
///
/// Two names keep their capitals on purpose. `LF` and `CR` are how the two
/// control characters are spelled everywhere, `lexical::AsciiEscape` among them,
/// and they name nothing a tree can hold: each appears only as a member of a
/// character set. The externals `XID_Start` and `XID_Continue` are keyed by the
/// spelling UAX #31 gives the two properties, which is the spelling the caller's
/// table must use too.
#[test]
fn the_nodes_are_named_as_rust_names_its_types() {
    for item in parse_bootstrap().items.iter() {
        let Item::Stmt(stmt) = item else {
            continue;
        };

        let name = stmt.ident.0.as_str();

        assert!(
            name.starts_with(|c: char| c.is_ascii_uppercase())
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "{name} is not written like a Rust type"
        );
        assert!(
            name.len() <= 2 || name.chars().any(|c| c.is_ascii_lowercase()),
            "{name} shouts"
        );
    }
}

/// Every type of [`unsyn::lexical`] and [`unsyn::syntax`] that a node of the
/// hand-written tree is made of has a production of the very same name.
///
/// This is the pin that keeps the two spellings from drifting apart: a parser
/// generated from this spec is only a drop-in for the hand-written one while the
/// names agree, and a rename on either side has to be a rename on the other.
///
/// [`unsyn::lexical`]: unsyn::lexical
/// [`unsyn::syntax`]: unsyn::syntax
#[test]
fn the_spec_names_the_types_the_crate_already_has() {
    let c = parse_bootstrap();

    let module = analyze(&c).expect("the spec should be well-formed");

    for name in [
        // The trivia and the tokens a node holds.
        "S",
        "Ident",
        "LitStr",
        "LitUnicode",
        "LitDec",
        "AsciiEscape",
        "QuoteEscape",
        "UnicodeEscape",
        "LineComment",
        "BlockComment",
        "InnerLineDoc",
        "InnerBlockDoc",
        "OuterLineDoc",
        "OuterBlockDoc",
        // The nodes of the tree itself, from the file down to one operand.
        "Crate",
        "Item",
        "InnerDoc",
        "OuterDoc",
        "Stmt",
        "Expr",
        "ExprNoTopAlt",
        "ExprWithoutSuffix",
        "ExprWithSuffix",
        "Repeat",
        "Range",
        "SetItem",
    ] {
        assert!(
            module.definition(name).is_some(),
            "the spec no longer declares {name}, which the crate has a type of"
        );
    }
}

/// A type of its own is only ever what the left side of a production turns out
/// to be, so the sums the generator writes for this spec are all names the spec
/// declares.
///
/// The rule of the generator is that nothing on the right side of a `->` is
/// named: a group of several alternatives is one field of the rule that holds
/// it, and a type written for such a group would be a name no file ever gave,
/// which is what a reader of the generated tree would have to keep straight.
/// The names are checked rather than the shapes, since a shape is what the
/// generator reaches by any number of routes and a name is what it declares.
#[test]
fn every_sum_type_of_the_generated_parser_is_a_production() {
    let c = parse_bootstrap();

    let module = analyze(&c).expect("the spec should be well-formed");

    let generated = Codegen::new()
        .with_externals([
            ("XID_Start".to_owned(), quote!(::my_backend::xid_start)),
            (
                "XID_Continue".to_owned(),
                quote!(::my_backend::xid_continue),
            ),
        ])
        .generate(&module)
        .expect("the spec should be writable")
        .to_string();

    let sums: Vec<String> = syn::parse_file(&generated)
        .expect("the generated file should be valid Rust")
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Enum(def) => Some(def.ident.to_string()),
            _ => None,
        })
        .collect();

    // The eight rules of the spec that are a choice of several ways are exactly
    // the types that say which way was read; nothing a rule only holds earns
    // one, which is why the comment rules and the escapes are not here. The last
    // three are the file itself: which of the two documents and the whitespace it
    // was written with came first is a thing the file says, and the rule above
    // them, `Crate`, holds two lists and so earns a struct instead.
    assert_eq!(
        sums,
        [
            "ExprNoTopAlt",
            "ExprWithoutSuffix",
            "ExprWithSuffix",
            "Range",
            "SetItem",
            "Item",
            "OuterDoc",
            "InnerDoc",
        ],
        "{sums:?}"
    );

    for name in sums {
        assert!(
            module.definition(&name).is_some(),
            "{name} is a type of its own but no production of the spec"
        );
    }
}
