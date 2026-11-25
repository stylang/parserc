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
//! This binary is behind the `bootstrap` feature; `required-features` means an
//! explicit `--example bootstrap` without it errors rather than silently doing
//! nothing, though a bulk `--all-targets` will still skip it.
//!
//! ```text
//! cargo run   -p unsyn --features bootstrap --example bootstrap   # the loop
//! cargo check -p unsyn --features bootstrap --example bootstrap   # it compiles
//! ```
//!
//! # Why this target allows `dead_code`
//!
//! The point of the harness is to compile the crate's own `codegen`, `visit`,
//! `read` and `decode` a second time over a *different* front end, reached by
//! `#[path]`. That copies their public surface into a binary: the `Builder`
//! configuration methods, and every `walk_*` the visitor ships, are reachable
//! from outside the library but have no caller in this one program. rustc then
//! calls them dead.
//!
//! Nothing is being hidden. The lint fires because of where the code was
//! compiled, not because the API is unused — the same items are live in the lib
//! and are exercised by `src/`'s own tests and by `two_trees.rs`. The generated
//! file carries the same allowance in its own header for the same reason: a
//! build writes every rule's node and descent whether or not one particular
//! program asks for them.
#![allow(dead_code)]

/// The scan the generated file reaches for as `crate::scan_ident_or_keyword`.
/// `<IdentOrKeyword>` is an external, and a generated file's externals are the
/// caller's to bind — here, to the same scan the crate's own tests bind, so the
/// two halves of the loop read a name the same way.
#[path = "../../tests/common/scan.rs"]
mod scan;

pub use scan::scan_ident_or_keyword;

// The pipeline below is declared at this crate's **root**, not under a
// `compiler` module, because the library's modules live at its root too: the
// sources reached by path say `crate::input` and `crate::lexer`, and those
// resolve here only if this harness gives them the same depth they have there.
// Mirroring `src/` means mirroring its shape, not merely its file names.

/// The nodes, the parsers and the visitor a build wrote for `unsyn.us`.
///
/// One file, because a build writes one file. The slots below are what let that
/// one file stand in for the parts the hand front end was cut into.
#[path = "../../tests/generated/unsyn_us.rs"]
pub mod generated;

/// A terminal, in the generated telling. The split of terminals from nodes
/// carries nothing a tree can tell; the hand file is cut that way, so the
/// slots are cut that way.
pub mod lexer {
    pub use crate::generated::*;
}

/// A node of the tree, in the same telling.
pub mod syntax {
    pub use crate::generated::*;
}

/// The stream the generated file names: the trait it emitted, and its alias.
pub mod input {
    pub use crate::generated::*;
}

/// The reports say nothing about which tree they were handed, and must be
/// compiled exactly once: the stream the generated file names carries
/// `Error = ::unsyn::errors::UnsynError` written into it in absolute path, so a
/// second copy here would be a different enum that the generated `UnsynInput`
/// refuses to accept. `errors` therefore stays a re-export.
pub use ::unsyn::errors;

/// The three phases that name nodes of the front end are compiled again here,
/// so that `crate::syntax` and `crate::lexer` inside them mean the tree under
/// test rather than the hand one. The same is true of `visit`, whose every walk
/// names a node, and of `decode`, which names the literal whose digits it reads.
#[path = "../../src/read.rs"]
pub mod read;

#[path = "../../src/semantic.rs"]
pub mod semantic;

#[path = "../../src/codegen.rs"]
pub mod codegen;

#[path = "../../src/visit.rs"]
pub mod visit;

#[path = "../../src/decode.rs"]
pub mod decode;

/// The spec, in the language it describes.
const SPEC: &str = include_str!("../../unsyn.us");

/// The code a build last wrote for it, kept in the crate.
const KEPT: &str = include_str!("../../tests/generated/unsyn_us.rs");

fn main() {
    use codegen::Builder;
    use parserc::{Input, syntax::SyntaxInput as _};
    use quote::ToTokens as _;
    use semantic::analyze;
    use syntax::Crate;

    // The first half: the parser a build wrote, on the file that made it. This
    // half already holds — it is what `unsyn_us.rs` proves today.
    let mut input = parserc::chars::TokenStream::<errors::UnsynError>::from(SPEC);

    let tree = input
        .parse::<Crate<_>>()
        .expect("the generated parser reads the spec it was generated from");

    assert!(input.is_empty(), "the generated parser left text unread");

    // The second half: the analysis and the generation, handed that tree and
    // nothing else — no re-parsing, no conversion, no switch in either file.
    let whole = parserc::chars::TokenStream::<errors::UnsynError>::from(SPEC);

    let report = analyze(&whole, &tree);
    assert!(
        !report.has_errors(),
        "the spec, read by its own parser, denotes nothing: {report:#?}"
    );

    // A build holds what it generates from, so this hands over a copy and keeps
    // the tree it parsed for the checks above.
    let code = Builder::new(whole.clone(), tree.clone())
        .external(
            "IdentOrKeyword",
            quote::quote! { crate::scan_ident_or_keyword },
        )
        .build()
        .expect("the spec, read by its own parser, builds");

    // The comparison. What the file says about itself is not part of the code
    // it is: the kept file carries a header of doc comments and `allow`s that
    // no build writes, so both sides are reduced to their items before they are
    // compared, exactly as `tests/unsyn_us.rs` does.
    let items = |code: &str| -> Vec<String> {
        let mut file = syn::parse_file(code).expect("the code should be Rust");
        file.attrs.clear();
        file.shebang = None;
        // A trailing comma before a closer is layout, not code: `rustfmt` adds
        // one to every list it breaks over lines, and a build writes none. It
        // is dropped here by walking the token *tree* — cutting it out of the
        // stringified form cannot work, because the printer joins the last token
        // to the closer (`I)`) so the comma is not a separable word.
        fn norm(ts: proc_macro2::TokenStream) -> Vec<String> {
            use proc_macro2::{Delimiter, TokenTree};
            let mut out: Vec<String> = Vec::new();
            for tt in ts {
                match tt {
                    TokenTree::Group(g) => {
                        let mut inner = norm(g.stream());
                        while inner.last().map(|s| s == ",") == Some(true) {
                            inner.pop();
                        }
                        let (o, c) = match g.delimiter() {
                            Delimiter::Parenthesis => ("(", ")"),
                            Delimiter::Bracket => ("[", "]"),
                            Delimiter::Brace => ("{", "}"),
                            Delimiter::None => ("", ""),
                        };
                        out.push(format!("{o}{}{c}", inner.join(" ")));
                    }
                    other => out.push(other.to_string()),
                }
            }
            // a generic-argument list ends with a plain `>` punct, not a group,
            // so the comma before one drops the same way
            let mut i = 0;
            while i + 1 < out.len() {
                if out[i] == "," && out[i + 1] == ">" {
                    out.remove(i);
                } else {
                    i += 1;
                }
            }
            out
        }
        norm(file.to_token_stream())
    };

    let same = items(KEPT) == items(&code.to_string());

    if !same {
        // Print the first divergence rather than only the verdict: "differs"
        // without a location says nothing an edit can act on.
        let a = items(KEPT);
        let b = items(&code.to_string());
        let at = a
            .iter()
            .zip(&b)
            .position(|(x, y)| x != y)
            .unwrap_or_else(|| a.len().min(b.len()));
        println!(
            "first difference at token {at} (kept has {}, built has {})",
            a.len(),
            b.len()
        );
        println!(
            "kept : {:?}",
            &a[at.saturating_sub(3)..(at + 6).min(a.len())]
        );
        println!(
            "built: {:?}",
            &b[at.saturating_sub(3)..(at + 6).min(b.len())]
        );
    }

    println!(
        "bootstrap fixed point: {}",
        if same { "holds" } else { "differs" }
    );

    assert!(
        same,
        "the generator did not write back the file it was read from"
    );
}
