//! The bootstrap: this crate's own front end is the code a build writes for
//! `unsyn.us`.
//!
//! `unsyn.us` is the spec of the language — the terminals, the nodes, the
//! separator and the visitor of a file of it. The tree `src/read.rs`,
//! `src/semantic.rs` and `src/codegen.rs` are written against is therefore not
//! hand-written: it is generated, from the file that describes it, by the same
//! generator those sources ship. What the build fails to compile *is* the
//! mismatch: no test compares two trees by hand, because the compiler reads both
//! and says where they part.
//!
//! The generator run here is the released one, not the one in `src/`. A crate
//! cannot compile the code that decides its own shape, so the loop turns one
//! release late: the change to what a build *writes* lands in the release after
//! the one whose build ran the previous generator. `Cargo.toml` pins that
//! version with `=`, so the code shipped by a release is exactly the code its
//! predecessor wrote from the spec at its tag.
//!
//! Two of the five files a build writes are absent, and that is deliberate:
//! [`input_at`] and [`error_at`] say the stream trait and the error stand in
//! `src/input.rs` and `src/errors.rs` rather than being written again. The error
//! is the sharper case — its families of *analysis* (what a well formed token
//! still denotes wrongly, what no code answers) are rules of the DSL and of the
//! passes below, and no production of the spec states them, so a build told to
//! write one would invent what the file never said.
//!
//! [`input_at`]: unsyn::codegen::Builder::input_at
//! [`error_at`]: unsyn::codegen::Builder::error_at

use std::path::PathBuf;
use std::str::FromStr;

/// The one external `unsyn.us` calls, bound to a scan of this crate's own.
///
/// A generated file reaches it by the path written here, which is what an
/// external is: the caller of the build decides what a call means, and here the
/// caller is the crate the code lands in. The scan reads a name whole — the start
/// of one and then the greedy run that continues it — because a token that is
/// "whatever the Unicode property was" is still one token.
const IDENT_OR_KEYWORD: &str = "crate::scan::scan_ident_or_keyword";

/// The externals the Unicode properties are bound to, for a spec that asks for
/// them by name rather than by a scan of its own.
const XID_START: &str = "::parserc::next_if(::unicode_ident::is_xid_start)";
const XID_CONTINUE: &str = "::parserc::next_if(::unicode_ident::is_xid_continue)";

fn main() {
    // The spec is read by `include_str!`, which cargo cannot see a dependency of.
    println!("cargo:rerun-if-changed=unsyn.us");

    let source = include_str!("unsyn.us");
    let dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set for a build script"))
        .join("unsyn");

    let builder = unsyn::codegen::Builder::from_source(source)
        .unwrap_or_else(|err| panic!("unsyn.us is not a file of the language: {err}"));

    builder
        .external("IdentOrKeyword", tokens(IDENT_OR_KEYWORD))
        .external("XID_Start", tokens(XID_START))
        .external("XID_Continue", tokens(XID_CONTINUE))
        // The stream trait and the error are this crate's, reached by path: a
        // second pair written here would be a different trait and a different
        // error than the ones the analysis below is written against. The two
        // setters differ in what their path names — `input_at` is told the trait
        // itself, `error_at` the module the error is found in — which is what the
        // import of each is built from.
        .input_at(tokens("crate::input::UnsynInput"))
        .error_at("UnsynError", tokens("crate::errors"))
        .to_dir(dir)
        .build()
        .unwrap_or_else(|err| panic!("unsyn.us does not build: {err}"));
}

/// The path or expression an external is written as.
fn tokens(path: &str) -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::from_str(path).expect("a path is a token stream")
}
