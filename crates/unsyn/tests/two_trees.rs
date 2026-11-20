//! The hand tree and the generated tree, read from one file and told apart
//! by nothing a rule can tell.
//!
//! Both pairs — the hand parser with the visitor the hand code ships, and
//! the parser and visitor the generator writes for `unsyn.us` — walk the
//! same source and report one event per named node: the visitor hook the
//! node reached, and the span the node covers. The two sequences are then
//! compared, and they are one and the same sequence: every rule of the
//! file, in the same order, over the same text.
//!
//! The trivia between two nodes is a run of the rule `S`, filed in the
//! same place by both trees: beside the operands of a sequence, never
//! inside a terminal. An event is therefore a node whose span is its own
//! text on either side, and the comparison of two spans needs no
//! normalization at all.
//!
//! What the two trees cannot agree on is the interior of a rule the
//! generated code reads as text rather than as parts: the hand tree walks
//! such a rule into the nodes it spells, and the generated file has no
//! such nodes to walk. Those rules are the hooks `TEXT_INTERIORS` names,
//! and the projection drops every event that lies inside one. Nothing
//! else a rule says about a file do the two trees say differently —
//! which is what the passes that come after the parser need of either
//! tree, and what the next step of the bootstrap builds on.

use parserc::Span;

/// The scan the generated file binds `<IdentOrKeyword>` to, at this crate's
/// root, which is the path the binding reaches.
#[path = "common/scan.rs"]
mod scan;

pub use scan::scan_ident_or_keyword;

/// The event of one node: the visitor hook it reached, and the text it covers.
type Event = (&'static str, Span);

/// Writes one recorder: a visitor over one tree that records the hook of
/// every named node with its span and walks on — leaving `S` unrecorded and
/// merely walked, since a run of trivia names no node of the tree.
macro_rules! recorder {
    (
        $(#[$meta:meta])*
        $name:ident: $visit:path, $bound:path, $skip:path, $swalk:path,
        [ $(fn $hook:ident: $ty:path => $step:path),* $(,)? ]
    ) => {
        $(#[$meta])*
        #[derive(Default)]
        struct $name {
            events: Vec<Event>,
        }

        impl<'ast, I> $visit for $name
        where
            I: $bound,
        {
            fn visit_s(&mut self, node: &'ast $skip) {
                $swalk(self, node);
            }

            $(
                fn $hook(&mut self, node: &'ast $ty) {
                    use parserc::syntax::Syntax as _;

                    self.events.push((stringify!($hook), node.to_span()));

                    $step(self, node);
                }
            )*
        }
    };
}

recorder! {
    /// Records the hand tree.
    Hand: unsyn::compiler::visit::Visit<'ast, I>, unsyn::compiler::input::UnsynInput,
        unsyn::compiler::lexer::S<I>, unsyn::compiler::visit::walk::walk_s,
    [
        fn visit_crate: unsyn::compiler::syntax::Crate<I> => unsyn::compiler::visit::walk::walk_crate,
        fn visit_item: unsyn::compiler::syntax::Item<I> => unsyn::compiler::visit::walk::walk_item,
        fn visit_stmt: unsyn::compiler::syntax::Stmt<I> => unsyn::compiler::visit::walk::walk_stmt,
        fn visit_outer_doc: unsyn::compiler::syntax::OuterDoc<I> => unsyn::compiler::visit::walk::walk_outer_doc,
        fn visit_inner_doc: unsyn::compiler::syntax::InnerDoc<I> => unsyn::compiler::visit::walk::walk_inner_doc,
        fn visit_expr: unsyn::compiler::syntax::Expr<I> => unsyn::compiler::visit::walk::walk_expr,
        fn visit_expr_no_top_alt: unsyn::compiler::syntax::ExprNoTopAlt<I> => unsyn::compiler::visit::walk::walk_expr_no_top_alt,
        fn visit_expr_without_suffix:
            unsyn::compiler::syntax::ExprWithoutSuffix<I> => unsyn::compiler::visit::walk::walk_expr_without_suffix,
        fn visit_expr_with_suffix:
            unsyn::compiler::syntax::ExprWithSuffix<I> => unsyn::compiler::visit::walk::walk_expr_with_suffix,
        fn visit_repeat: unsyn::compiler::syntax::Repeat<I> => unsyn::compiler::visit::walk::walk_repeat,
        fn visit_range: unsyn::compiler::syntax::Range<I> => unsyn::compiler::visit::walk::walk_range,
        fn visit_set_item: unsyn::compiler::syntax::SetItem<I> => unsyn::compiler::visit::walk::walk_set_item,
        fn visit_ident_or_keyword:
            unsyn::compiler::lexer::IdentOrKeyword<I> => unsyn::compiler::visit::walk::walk_ident_or_keyword,
        fn visit_raw_ident: unsyn::compiler::lexer::RawIdent<I> => unsyn::compiler::visit::walk::walk_raw_ident,
        fn visit_non_keyword_ident:
            unsyn::compiler::lexer::NonKeywordIdent<I> => unsyn::compiler::visit::walk::walk_non_keyword_ident,
        fn visit_ident: unsyn::compiler::lexer::Ident<I> => unsyn::compiler::visit::walk::walk_ident,
        fn visit_keywords: unsyn::compiler::lexer::Keywords<I> => unsyn::compiler::visit::walk::walk_keywords,
        fn visit_leading_keywords:
            unsyn::compiler::lexer::LeadingKeywords<I> => unsyn::compiler::visit::walk::walk_leading_keywords,
        fn visit_following_keywords:
            unsyn::compiler::lexer::FollowingKeywords<I> => unsyn::compiler::visit::walk::walk_following_keywords,
        fn visit_lit_str: unsyn::compiler::lexer::LitStr<I> => unsyn::compiler::visit::walk::walk_lit_str,
        fn visit_quote_escape: unsyn::compiler::lexer::QuoteEscape<I> => unsyn::compiler::visit::walk::walk_quote_escape,
        fn visit_ascii_escape: unsyn::compiler::lexer::AsciiEscape<I> => unsyn::compiler::visit::walk::walk_ascii_escape,
        fn visit_unicode_escape: unsyn::compiler::lexer::UnicodeEscape<I> => unsyn::compiler::visit::walk::walk_unicode_escape,
        fn visit_oct_digit: unsyn::compiler::lexer::OctDigit<I> => unsyn::compiler::visit::walk::walk_oct_digit,
        fn visit_dec_digit: unsyn::compiler::lexer::DecDigit<I> => unsyn::compiler::visit::walk::walk_dec_digit,
        fn visit_hex_digit: unsyn::compiler::lexer::HexDigit<I> => unsyn::compiler::visit::walk::walk_hex_digit,
        fn visit_lit_dec: unsyn::compiler::lexer::LitDec<I> => unsyn::compiler::visit::walk::walk_lit_dec,
        fn visit_lit_unicode: unsyn::compiler::lexer::LitUnicode<I> => unsyn::compiler::visit::walk::walk_lit_unicode,
        fn visit_block_comment: unsyn::compiler::lexer::BlockComment<I> => unsyn::compiler::visit::walk::walk_block_comment,
        fn visit_block_comment_or_doc:
            unsyn::compiler::lexer::BlockCommentOrDoc<I> => unsyn::compiler::visit::walk::walk_block_comment_or_doc,
        fn visit_inner_line_doc: unsyn::compiler::lexer::InnerLineDoc<I> => unsyn::compiler::visit::walk::walk_inner_line_doc,
        fn visit_inner_block_doc:
            unsyn::compiler::lexer::InnerBlockDoc<I> => unsyn::compiler::visit::walk::walk_inner_block_doc,
        fn visit_outer_line_doc: unsyn::compiler::lexer::OuterLineDoc<I> => unsyn::compiler::visit::walk::walk_outer_line_doc,
        fn visit_outer_block_doc:
            unsyn::compiler::lexer::OuterBlockDoc<I> => unsyn::compiler::visit::walk::walk_outer_block_doc,
        fn visit_lf: unsyn::compiler::lexer::LF<I> => unsyn::compiler::visit::walk::walk_lf,
        fn visit_cr: unsyn::compiler::lexer::CR<I> => unsyn::compiler::visit::walk::walk_cr,
    ]
}

recorder! {
    /// Records the generated tree.
    Generated: crate::generated::Visit<'ast, I>, parserc::chars::CharsInput,
        crate::generated::S<I>, crate::generated::walk::walk_s,
    [
        fn visit_crate: crate::generated::Crate<I> => crate::generated::walk::walk_crate,
        fn visit_item: crate::generated::Item<I> => crate::generated::walk::walk_item,
        fn visit_stmt: crate::generated::Stmt<I> => crate::generated::walk::walk_stmt,
        fn visit_outer_doc: crate::generated::OuterDoc<I> => crate::generated::walk::walk_outer_doc,
        fn visit_inner_doc: crate::generated::InnerDoc<I> => crate::generated::walk::walk_inner_doc,
        fn visit_expr: crate::generated::Expr<I> => crate::generated::walk::walk_expr,
        fn visit_expr_no_top_alt: crate::generated::ExprNoTopAlt<I> => crate::generated::walk::walk_expr_no_top_alt,
        fn visit_expr_without_suffix:
            crate::generated::ExprWithoutSuffix<I> => crate::generated::walk::walk_expr_without_suffix,
        fn visit_expr_with_suffix:
            crate::generated::ExprWithSuffix<I> => crate::generated::walk::walk_expr_with_suffix,
        fn visit_repeat: crate::generated::Repeat<I> => crate::generated::walk::walk_repeat,
        fn visit_range: crate::generated::Range<I> => crate::generated::walk::walk_range,
        fn visit_set_item: crate::generated::SetItem<I> => crate::generated::walk::walk_set_item,
        fn visit_ident_or_keyword: crate::generated::IdentOrKeyword<I> => crate::generated::walk::walk_ident_or_keyword,
        fn visit_raw_ident: crate::generated::RawIdent<I> => crate::generated::walk::walk_raw_ident,
        fn visit_non_keyword_ident:
            crate::generated::NonKeywordIdent<I> => crate::generated::walk::walk_non_keyword_ident,
        fn visit_ident: crate::generated::Ident<I> => crate::generated::walk::walk_ident,
        fn visit_keywords: crate::generated::Keywords<I> => crate::generated::walk::walk_keywords,
        fn visit_leading_keywords: crate::generated::LeadingKeywords<I> => crate::generated::walk::walk_leading_keywords,
        fn visit_following_keywords:
            crate::generated::FollowingKeywords<I> => crate::generated::walk::walk_following_keywords,
        fn visit_lit_str: crate::generated::LitStr<I> => crate::generated::walk::walk_lit_str,
        fn visit_quote_escape: crate::generated::QuoteEscape<I> => crate::generated::walk::walk_quote_escape,
        fn visit_ascii_escape: crate::generated::AsciiEscape<I> => crate::generated::walk::walk_ascii_escape,
        fn visit_unicode_escape: crate::generated::UnicodeEscape<I> => crate::generated::walk::walk_unicode_escape,
        fn visit_oct_digit: crate::generated::OctDigit<I> => crate::generated::walk::walk_oct_digit,
        fn visit_dec_digit: crate::generated::DecDigit<I> => crate::generated::walk::walk_dec_digit,
        fn visit_hex_digit: crate::generated::HexDigit<I> => crate::generated::walk::walk_hex_digit,
        fn visit_lit_dec: crate::generated::LitDec<I> => crate::generated::walk::walk_lit_dec,
        fn visit_lit_unicode: crate::generated::LitUnicode<I> => crate::generated::walk::walk_lit_unicode,
        fn visit_block_comment: crate::generated::BlockComment<I> => crate::generated::walk::walk_block_comment,
        fn visit_block_comment_or_doc:
            crate::generated::BlockCommentOrDoc<I> => crate::generated::walk::walk_block_comment_or_doc,
        fn visit_inner_line_doc: crate::generated::InnerLineDoc<I> => crate::generated::walk::walk_inner_line_doc,
        fn visit_inner_block_doc: crate::generated::InnerBlockDoc<I> => crate::generated::walk::walk_inner_block_doc,
        fn visit_outer_line_doc: crate::generated::OuterLineDoc<I> => crate::generated::walk::walk_outer_line_doc,
        fn visit_outer_block_doc: crate::generated::OuterBlockDoc<I> => crate::generated::walk::walk_outer_block_doc,
        fn visit_lf: crate::generated::LF<I> => crate::generated::walk::walk_lf,
        fn visit_cr: crate::generated::CR<I> => crate::generated::walk::walk_cr,
    ]
}

/// The code of the spec, kept beside these tests so that it is compiled with them.
#[path = "generated/unsyn_us.rs"]
mod generated;

/// The two trees over `unsyn.us` say the same rules over the same text.
#[test]
fn the_two_trees_say_the_same_rules_over_the_same_text() {
    use parserc::{Input, syntax::SyntaxInput};

    const SOURCE: &str = include_str!("../unsyn.us");

    let mut hand_input = unsyn::compiler::input::TokenStream::from(SOURCE);
    let hand_tree = hand_input
        .parse::<unsyn::compiler::syntax::Crate<_>>()
        .expect("the hand parser reads the spec");
    assert!(hand_input.is_empty());

    let mut generated_input = parserc::chars::TokenStream::<parserc::Kind>::from(SOURCE);
    let generated_tree = generated_input
        .parse::<generated::Crate<_>>()
        .expect("the generated parser reads the spec");
    assert!(generated_input.is_empty());

    let mut hand_recorder = Hand::default();
    unsyn::compiler::visit::Visit::visit_crate(&mut hand_recorder, &hand_tree);

    let mut generated_recorder = Generated::default();
    generated::Visit::visit_crate(&mut generated_recorder, &generated_tree);

    assert!(!hand_recorder.events.is_empty());

    // Every event of every rule must agree exactly: a node at a different
    // place, of a different extent, or in a different order fails.
    //
    // The one projection the comparison needs is the other kind of
    // difference, a real one: a rule the generated code reads as text —
    // `NonKeywordIdent` beside the `except` it holds, the literals whose
    // digits and escapes are asked for their values rather than their
    // parts — has no interior of its own in the generated tree, and the
    // hand events inside one are the parts of a node the generated file
    // does not have. Everything else the two trees must say identically.
    //
    // The list is the set of rules the generated code reads as one piece of
    // text, not the subset today's file happens to reach: an escape lies
    // inside a `LitStr`, so it is covered by that interval already, and a
    // block comment appears nowhere in the spec — both are named so that the
    // projection says what the generated file is, and stays right when the
    // spec changes.
    const TEXT_INTERIORS: &[&str] = &[
        "visit_non_keyword_ident",
        "visit_lit_str",
        "visit_lit_unicode",
        "visit_unicode_escape",
        "visit_ascii_escape",
        "visit_block_comment",
        "visit_block_comment_or_doc",
        "visit_inner_block_doc",
        "visit_outer_block_doc",
    ];

    let mut hand_projected: Vec<Event> = Vec::new();
    let mut interiors: Vec<(usize, usize)> = Vec::new();

    for event in hand_recorder.events.clone() {
        let (start, end) = match &event.1 {
            Span::Range(e) => (e.start, e.end),
            _ => (0, 0),
        };

        // Events come in a depth-first order: an interval is open exactly
        // over the run of events whose spans it covers, so an event outside
        // the top interval closes it, and every interval beneath it too.
        while let Some(top) = interiors.last() {
            if start >= top.0 && end <= top.1 {
                break;
            }

            interiors.pop();
        }

        // The text rule's own event, when it is not inside another rule's
        // interior, is a node of both trees; what lies inside an open
        // interval — the parts of another partless node — is not.
        if !interiors.is_empty() {
            continue;
        }

        if TEXT_INTERIORS.contains(&event.0) {
            interiors.push((start, end));
        }

        hand_projected.push(event);
    }

    let generated = &generated_recorder.events;

    for (i, (g, h)) in generated.iter().zip(hand_projected.iter()).enumerate() {
        if g != h {
            println!("G: {:?}", generated[i.saturating_sub(3)..i + 5].to_vec());
            println!(
                "H: {:?}",
                hand_projected[i.saturating_sub(3)..i + 5].to_vec()
            );
            panic!("first disagreement at event {}", i);
        }
    }

    assert_eq!(
        generated.len(),
        hand_projected.len(),
        "the two trees said a different number of rules"
    );
}
