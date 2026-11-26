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
//! One of the five files a build writes is absent, and that is deliberate:
//! [`input_at`] says the stream trait stands in `src/input.rs` rather than being
//! written again. The trait is the one thing a build cannot derive — it is bounded
//! by what the *passes over* the tree need (a prefix test, a look-ahead, a
//! snapshot), which no grammar of the language states — and it carries the tests
//! of the stream itself.
//!
//! The error is not exempt, and is written here: [`error`](unsyn's `error`) holds
//! what reading a file reports, a combinator's own failure and a refusal named
//! after a rule of the spec. What the analysis adds — a token well formed and
//! meaningless, a shape no code answers — is not derivable from a grammar, so
//! `src/errors.rs` wraps the generated error together with those families into the
//! one type every pass returns.
//!
//! [`input_at`]: unsyn::codegen::Builder::input_at

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
        // The stream trait is this crate's own, reached by path: the bounds the
        // passes below are written against are wider than any grammar states,
        // and the trait is documented and tested by hand.
        //
        // The error is not. A build writes it — the framework's kind and the
        // family of rules a refusal is named after, both from the spec — and
        // `src/errors.rs` wraps that together with the families of the analysis,
        // which no production states. It is named `ParseError` rather than after
        // the language because the wrapper is the crate's error, and this is the
        // phase of it that reading a file produces.
        .input_at(tokens("crate::input::UnsynInput"))
        .error_name("ParseError")
        .to_dir(dir)
        .build()
        .unwrap_or_else(|err| panic!("unsyn.us does not build: {err}"));
}

/// The path or expression an external is written as.
fn tokens(path: &str) -> proc_macro2::TokenStream {
    proc_macro2::TokenStream::from_str(path).expect("a path is a token stream")
}
