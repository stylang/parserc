//! The fixed point: the generator run on the output of its own parser.
//!
//! `unsyn.us` is read by the parser a build wrote for it. The tree that parser
//! built is then handed to the crate's own analysis and the crate's own
//! generator — the same sources `src/` compiles, reached here by path — and the
//! code that comes out is compared with the file whose parser went in. Equal
//! output is what "the two trees are one tree" means when it means anything;
//! [`tests/two_trees.rs`](../../tests/two_trees.rs) can only show that they
//! read the same text to the same spans, which a behaviour can share while its
//! types differ.
//!
//! This binary is behind the `bootstrap` feature and does not compile today.
//! That is not an accident of scheduling: the compiler's errors are the list of
//! what a build must learn to write, and [`BOOTSTRAP_LOG.md`](../../BOOTSTRAP_LOG.md)
//! counts them down stage by stage.
//!
//! ```text
//! cargo check -p unsyn --features bootstrap --example bootstrap   # the distance
//! cargo run   -p unsyn --features bootstrap --example bootstrap   # the loop, once it builds
//! ```

/// The scan the generated file reaches for as `crate::scan_ident_or_keyword`.
/// `<IdentOrKeyword>` is an external, and a generated file's externals are the
/// caller's to bind — here, to the same scan the crate's own tests bind, so the
/// two halves of the loop read a name the same way.
#[path = "../../tests/common/scan.rs"]
mod scan;

pub use scan::scan_ident_or_keyword;

mod compiler;

/// The spec, in the language it describes.
const SPEC: &str = include_str!("../../unsyn.us");

/// The code a build last wrote for it, kept in the crate.
const KEPT: &str = include_str!("../../tests/generated/unsyn_us.rs");

fn main() {
    use compiler::codegen::Builder;
    use compiler::semantic::analyze;
    use compiler::syntax::Crate;
    use parserc::{Input, syntax::SyntaxInput as _};

    // The first half: the parser a build wrote, on the file that made it. This
    // half already holds — it is what `unsyn_us.rs` proves today.
    let mut input = parserc::chars::TokenStream::<compiler::errors::UnsynError>::from(SPEC);

    let tree = input
        .parse::<Crate<_>>()
        .expect("the generated parser reads the spec it was generated from");

    assert!(input.is_empty(), "the generated parser left text unread");

    // The second half: the analysis and the generation, handed that tree and
    // nothing else — no re-parsing, no conversion, no switch in either file.
    let whole = parserc::chars::TokenStream::<compiler::errors::UnsynError>::from(SPEC);

    let report = analyze(&whole, &tree);
    assert!(
        !report.has_errors(),
        "the spec, read by its own parser, denotes nothing: {report:#?}"
    );

    let mut builder = Builder::new(&whole, &tree);
    builder.external(
        "IdentOrKeyword",
        quote::quote! { crate::scan_ident_or_keyword },
    );

    let code = builder
        .build()
        .expect("the spec, read by its own parser, builds");

    // The comparison. Whitespace only: a build writes tokens, and the kept file
    // is those tokens as `rustfmt` laid them out.
    let same = KEPT
        .split_whitespace()
        .eq(code.to_string().split_whitespace());

    println!(
        "bootstrap fixed point: {}",
        if same { "holds" } else { "differs" }
    );

    assert!(
        same,
        "the generator did not write back the file it was read from"
    );
}
