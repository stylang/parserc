use std::{fs, path::PathBuf};

use parserc::{Input, syntax::Syntax};
use unsyn::{
    input::TokenStream,
    semantics::{NodeKind, analyze},
    syntax::{Crate, Item},
};

/// Parses `crates/unsyn/bootstrap.uns`.
///
/// The spec of `unsyn` written in `unsyn` is the toughest input the analyzer
/// has: it uses forward references, mutual recursion between the comment rules,
/// and the backend matchers `XID_Start` and `XID_Continue`.
fn parse_bootstrap() -> Crate<TokenStream<'static>> {
    let filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap.uns")
        .canonicalize()
        .unwrap();

    // The analyzed module borrows its source, so the text is kept alive here.
    let content: &'static str = Box::leak(fs::read_to_string(&filepath).unwrap().into_boxed_str());

    let mut token_stream = TokenStream::from(content);

    let c = Crate::parse(&mut token_stream).unwrap_or_else(|e| panic!("parse {filepath:?}: {e}"));

    assert_eq!(token_stream.len(), 0, "the spec was not consumed");

    c
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

    let statements = c
        .items
        .iter()
        .filter(|item| matches!(item, Item::Stmt(_)))
        .count();

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
