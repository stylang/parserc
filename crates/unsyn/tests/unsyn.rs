use std::{fs, path::PathBuf};

use parserc::{
    sourceinput::{AsStr, Length},
    syntax::Syntax,
};
use unsyn::{
    input::Chars,
    syntax::{Crate, Item},
};

#[test]
fn parse_unsyn() {
    let mut input = Chars::new(include_str!("../unsyn.syn"));

    let syn = Crate::parse(&mut input).unwrap();

    assert!(input.is_empty());

    let input = Chars::new(include_str!("../unsyn.syn"));

    let spec_root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("spec")
        .canonicalize()
        .unwrap();

    let spec_outer_doc_dir = spec_root_dir.join("outer_doc");

    fs::create_dir_all(&spec_outer_doc_dir).unwrap();

    let spec_use_dir = spec_root_dir.join("use");

    fs::create_dir_all(&spec_use_dir).unwrap();

    let spec_mod_dir = spec_root_dir.join("module");

    fs::create_dir_all(&spec_mod_dir).unwrap();

    let spec_stmt_dir = spec_root_dir.join("stmt");

    fs::create_dir_all(&spec_stmt_dir).unwrap();

    for item in syn.items {
        match item {
            Item::OuterDoc(outer_doc) => {
                let span = outer_doc.to_span();
                let filepath = spec_outer_doc_dir.join(format!(
                    "unsyn-{:?}-{:?}.unsyn",
                    span.start(),
                    span.end()
                ));

                fs::write(filepath, input.as_str_with(span).unwrap()).unwrap();
            }
            Item::S(_) => {}
            Item::Use(use_declaration, semi) => {
                let span = use_declaration.to_span() + semi.to_span();
                let filepath =
                    spec_use_dir.join(format!("unsyn-{:?}-{:?}.unsyn", span.start(), span.end()));

                fs::write(filepath, input.as_str_with(span).unwrap()).unwrap();
            }
            Item::Mod(module_declaration, semi) => {
                let span = module_declaration.to_span() + semi.to_span();
                let filepath =
                    spec_mod_dir.join(format!("unsyn-{:?}-{:?}.unsyn", span.start(), span.end()));

                fs::write(filepath, input.as_str_with(span).unwrap()).unwrap();
            }
            Item::Stmt(stmt) => {
                let span = stmt.to_span();

                let filepath =
                    spec_stmt_dir.join(format!("unsyn-{:?}-{:?}.unsyn", span.start(), span.end()));

                fs::write(filepath, input.as_str_with(span).unwrap()).unwrap();
            }
        }
    }
}
