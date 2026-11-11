use std::{fs, path::PathBuf};

use parserc::syntax::Syntax;
use unsyn::{
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
/// and the backend matchers `XID_Start` and `XID_Continue`.
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
    assert_eq!(
        module.definition("WHITESPACE").unwrap().kind(),
        NodeKind::Whitespace
    );

    // A node of the tree is not a terminal pattern.
    assert_eq!(module.definition("Expr").unwrap().kind(), NodeKind::Syntax);
    assert_eq!(module.definition("LF").unwrap().kind(), NodeKind::Lexer);

    // The only matchers the spec expects from the backend.
    assert_eq!(module.matchers(), ["XID_Start", "XID_Continue"]);
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
