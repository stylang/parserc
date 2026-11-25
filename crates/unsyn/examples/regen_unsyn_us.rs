//! Rewrites the code of `unsyn.us` into `tests/generated/unsyn_us.rs`.
//!
//! This is the fallback for `the_generated_file_of_the_spec`, which is the same
//! act done from inside the test binary. That cannot be the only way: the kept
//! file is compiled *by* that binary, so a change to the generator that makes
//! the kept file stop being Rust makes the test that would fix it impossible to
//! build. This writes the file from outside, where only the library has to
//! compile.
//!
//! The header of the kept file — its document lines and its `#![allow(..)]`
//! list — belongs to the keeping and not to the code, so it is read back off
//! the file and written again rather than restated here, where it would
//! silently part company with the copy in the test.
//!
//! Run it, then `cargo fmt --all`, then the suite.

use std::path::Path;
use std::str::FromStr;

use parserc::syntax::SyntaxInput;
use unsyn::{codegen::Builder, input::TokenStream, semantic, syntax};

/// The header of the kept file: the document lines and the `#![allow(..)]` list
/// that opens the file, up to and including the line that closes the list.
fn header_of(kept: &str) -> &str {
    let close = "\n)]\n";
    let end = kept
        .find(close)
        .map(|at| at + close.len())
        .expect("the kept file opens with a list of allows");

    &kept[..end]
}

fn main() {
    let source = include_str!("../unsyn.us");

    let whole = TokenStream::from(source);

    let mut input = TokenStream::from(source);
    let file = input
        .parse::<syntax::Crate<_>>()
        .expect("unsyn.us should be a file of the language");

    let report = semantic::analyze(&whole, &file);
    assert!(report.errors.is_empty(), "unsyn.us: {:#?}", report.errors);
    assert!(
        report.lints.is_empty(),
        "unsyn.us names nothing in {} place(s): {:?}",
        report.lints.len(),
        report.lints
    );

    let code = Builder::new(whole.clone(), file.clone())
        .external(
            "IdentOrKeyword",
            proc_macro2::TokenStream::from_str("crate::scan_ident_or_keyword").unwrap(),
        )
        .build()
        .expect("unsyn.us should be generated whole")
        .to_string();

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/generated/unsyn_us.rs");
    let kept = std::fs::read_to_string(&path).expect("the kept file should be readable");

    std::fs::write(&path, format!("{}{}\n", header_of(&kept), code)).expect("writing fails");

    println!("wrote {}", path.display());
}
