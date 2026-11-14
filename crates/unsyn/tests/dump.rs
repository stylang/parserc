//! Writes the file `tests/data/mini.rs` holds, from the spec in
//! `tests/data/spec.rs`.
//!
//! This is the tool that keeps the compiled fixture in step with the generator:
//! run it with `--nocapture`, format what it writes with `rustfmt --edition
//! 2024`, and copy it over the fixture. `tests/generated.rs` is the test that
//! says when that has not been done, since it regenerates the same file and
//! compares the two whole.
//!
//! The spec is the one `bootstrap.uns` is written against in everything but
//! scale: sets, ranges, the wildcard, complements, `concat`, `except`,
//! `followed`, counted runs, groups that need a type of their own, and rules
//! that reach themselves. A spec that cannot be generated is reported, with the
//! span of what the generator could not write, rather than half-written.

use std::{fs, path::PathBuf};

use parserc::syntax::Syntax;
use quote::quote;
use unsyn::{codegen::Codegen, input::TokenStream, semantics::analyze, syntax::Crate};

include!("data/spec.rs");

/// A generator told what the one name the spec writes calls.
///
/// The spec writes `<WORD_START>`, and a name is only ever what the caller of
/// the generator says it is, so this is where the caller says it. The path leads
/// to `tests/generated.rs`, which is the crate that compiles what this writes and
/// so is where the function behind the name lives. Nothing else is said here:
/// a generated file takes the bound of `GeneratedInput` from unsyn by name and
/// takes nothing else, which is why a spec of keywords needs no larger table
/// than this one.
fn codegen() -> Codegen {
    Codegen::new().with_externals([("WORD_START".to_owned(), quote!(crate::backend::word_start))])
}

#[test]
fn dump() {
    let mut tokens = TokenStream::from(SPEC);
    let file = Crate::parse(&mut tokens).expect("the spec should parse");
    let module = analyze(&file).expect("the spec should be well formed");

    let generated = match codegen().generate(&module) {
        Ok(generated) => generated.to_string(),
        Err(errors) => {
            for error in errors.iter() {
                println!("CANNOT: {error}");
            }

            panic!("the spec should be writable");
        }
    };

    // The tokens are read back as Rust here, so a run of this test says at once
    // whether the generator wrote a file a compiler would take.
    if let Err(error) = syn::parse_file(&generated) {
        panic!("the generated file is not valid Rust: {error}");
    }

    // The header comes from the same file as the spec, so that copying what
    // this writes over the fixture is the whole of what updating it takes.
    let filepath = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("mini.rs");

    fs::write(&filepath, format!("{HEADER}{generated}")).expect("the file should be written");

    println!("WROTE {}", filepath.display());
}
