//! Rewrites the code of `unsyn.us` into the files under
//! `tests/generated/unsyn_us/`.
//!
//! This is the fallback for `the_generated_files_of_the_spec`, which is the same
//! act done from inside the test binary. That cannot be the only way: the kept
//! files are compiled *by* that binary, so a change to the generator that makes
//! what it writes stop being Rust makes the test that would fix it impossible to
//! build. This writes them from outside, where only the library has to compile.
//!
//! The parts are taken with [`modules`](unsyn::codegen::Builder::modules) rather
//! than written with [`to_dir`](unsyn::codegen::Builder::to_dir), because each
//! file's header has to be read back before the file is replaced: the header —
//! its document lines and its `#![allow(..)]` list — belongs to the keeping and
//! not to the code, and restating it here would let it silently part company
//! with the copy in the test. A file written for the first time gets none.
//!
//! Run it, then `cargo fmt --all`, then the suite.

use std::path::Path;
use std::str::FromStr;

use parserc::syntax::SyntaxInput;
use unsyn::{codegen::Builder, input::TokenStream, semantic, syntax};

/// The header of a kept file: the document lines and the `#![allow(..)]` list
/// that opens the file, up to and including the line that closes the list. Where
/// a file carries no such list, its leading comment lines alone.
fn header_of(kept: &str) -> String {
    if let Some(at) = kept.find("\n)]\n") {
        return kept[..at + 4].to_owned();
    }

    kept.lines()
        .take_while(|line| line.starts_with("//!"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
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

    // The parts a build would write into that directory, taken without the
    // filesystem so that the header of each kept file can be read back first.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/generated/unsyn_us");
    let modules = Builder::new(whole.clone(), file.clone())
        .external(
            "IdentOrKeyword",
            proc_macro2::TokenStream::from_str("crate::scan_ident_or_keyword").unwrap(),
        )
        // The kept files stand inside this crate, beside the analysis that
        // reports what a token denotes wrongly and what no code answers, so they
        // reach the crate's own error rather than writing a second one — which
        // is also why no `error.rs` is among them.
        .error_at(
            "UnsynError",
            proc_macro2::TokenStream::from_str("::unsyn::errors").unwrap(),
        )
        .modules()
        .expect("unsyn.us should be generated whole");

    std::fs::create_dir_all(&dir).expect("the directory should be made");

    for module in modules {
        let path = dir.join(module.file);

        // The code as the generator wrote it, under the header the keeping gave.
        let kept = std::fs::read_to_string(&path).unwrap_or_default();
        let header = if kept.is_empty() {
            String::new()
        } else {
            header_of(&kept)
        };

        std::fs::write(&path, format!("{header}{}\n", module.code)).expect("writing fails");

        println!("wrote {}", path.display());
    }
}
