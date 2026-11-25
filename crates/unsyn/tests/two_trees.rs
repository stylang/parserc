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
//! There is no third thing the two trees settle differently *in what they
//! record*. The hand tree once kept parts — the digits of a number, the
//! escapes of a string, the nesting of a block comment — where the generated
//! code reads one token of text, and every one of those has now been given up,
//! because no rule of the language names them and no pass asks them.
//!
//! What the two trees do settle differently is named, and counted, by
//! [`the_parity_reaches_every_rule_of_either_tree`]. The spec now gives ten
//! parts of a pattern a left side of their own — `NegatedSet`, `Call`, `Paren`,
//! `Suffixed`, `RangeTo`, `RangeFrom`, `UnicodeRange`, `StrRange`,
//! `AsciiHexEscape` and `BlockCommentBody` — so the generated tree has a node,
//! and so a visitor hook, where the hand tree still writes the same shape
//! inline as the fields of a variant. The recorder below can only compare what
//! it is told to record, and the generated half of the table has no hand hook
//! to answer it yet; those ten are left out of the comparison and pinned by
//! that test so that the gap is a number in the file rather than a silence in
//! the result. Closing it is a matter of giving the hand tree the ten rules as
//! rules, which is the step this test will then stop needing.

use parserc::{Span, ToSpan};

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
        $(#[$meta])*#[derive(Default)]
        struct $name {
            events: Vec<Event>,
        }

        impl $name {
            /// The hooks this recorder records, which is the whole of what the
            /// parity it reaches covers.
            const HOOKS: &'static [&'static str] = &[$(stringify!($hook)),*];
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
        fn visit_negated_set:
            unsyn::compiler::syntax::NegatedSet<I> => unsyn::compiler::visit::walk::walk_negated_set,
        fn visit_call: unsyn::compiler::syntax::Call<I> => unsyn::compiler::visit::walk::walk_call,
        fn visit_paren: unsyn::compiler::syntax::Paren<I> => unsyn::compiler::visit::walk::walk_paren,
        fn visit_set: unsyn::compiler::syntax::Set<I> => unsyn::compiler::visit::walk::walk_set,
        fn visit_suffix: unsyn::compiler::syntax::Suffix<I> => unsyn::compiler::visit::walk::walk_suffix,
        fn visit_star: unsyn::compiler::syntax::Star<I> => unsyn::compiler::visit::walk::walk_star,
        fn visit_question: unsyn::compiler::syntax::Question<I> => unsyn::compiler::visit::walk::walk_question,
        fn visit_plus: unsyn::compiler::syntax::Plus<I> => unsyn::compiler::visit::walk::walk_plus,
        fn visit_expr_with_suffix:
            unsyn::compiler::syntax::ExprWithSuffix<I> => unsyn::compiler::visit::walk::walk_expr_with_suffix,
        fn visit_suffixed:
            unsyn::compiler::syntax::Suffixed<I> => unsyn::compiler::visit::walk::walk_suffixed,
        fn visit_repeat: unsyn::compiler::syntax::Repeat<I> => unsyn::compiler::visit::walk::walk_repeat,
        fn visit_repeat_bound:
            unsyn::compiler::syntax::RepeatBound<I> => unsyn::compiler::visit::walk::walk_repeat_bound,
        fn visit_range_to:
            unsyn::compiler::syntax::RangeTo<I> => unsyn::compiler::visit::walk::walk_range_to,
        fn visit_range_from:
            unsyn::compiler::syntax::RangeFrom<I> => unsyn::compiler::visit::walk::walk_range_from,
        fn visit_range: unsyn::compiler::syntax::Range<I> => unsyn::compiler::visit::walk::walk_range,
        fn visit_unicode_range:
            unsyn::compiler::syntax::UnicodeRange<I> => unsyn::compiler::visit::walk::walk_unicode_range,
        fn visit_str_range:
            unsyn::compiler::syntax::StrRange<I> => unsyn::compiler::visit::walk::walk_str_range,
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
        fn visit_operator_keywords:
            unsyn::compiler::lexer::OperatorKeywords<I> => unsyn::compiler::visit::walk::walk_operator_keywords,
        fn visit_lit_str: unsyn::compiler::lexer::LitStr<I> => unsyn::compiler::visit::walk::walk_lit_str,
        fn visit_quote_escape: unsyn::compiler::lexer::QuoteEscape<I> => unsyn::compiler::visit::walk::walk_quote_escape,
        fn visit_ascii_escape: unsyn::compiler::lexer::AsciiEscape<I> => unsyn::compiler::visit::walk::walk_ascii_escape,
        fn visit_ascii_hex_escape:
            unsyn::compiler::lexer::AsciiHexEscape<I> => unsyn::compiler::visit::walk::walk_ascii_hex_escape,
        fn visit_ascii_newline:
            unsyn::compiler::lexer::AsciiNewline<I> => unsyn::compiler::visit::walk::walk_ascii_newline,
        fn visit_ascii_carriage_return:
            unsyn::compiler::lexer::AsciiCarriageReturn<I> =>
                unsyn::compiler::visit::walk::walk_ascii_carriage_return,
        fn visit_ascii_tab:
            unsyn::compiler::lexer::AsciiTab<I> => unsyn::compiler::visit::walk::walk_ascii_tab,
        fn visit_ascii_backslash:
            unsyn::compiler::lexer::AsciiBackslash<I> => unsyn::compiler::visit::walk::walk_ascii_backslash,
        fn visit_ascii_null:
            unsyn::compiler::lexer::AsciiNull<I> => unsyn::compiler::visit::walk::walk_ascii_null,
        fn visit_unicode_escape: unsyn::compiler::lexer::UnicodeEscape<I> => unsyn::compiler::visit::walk::walk_unicode_escape,
        fn visit_oct_digit: unsyn::compiler::lexer::OctDigit<I> => unsyn::compiler::visit::walk::walk_oct_digit,
        fn visit_dec_digit: unsyn::compiler::lexer::DecDigit<I> => unsyn::compiler::visit::walk::walk_dec_digit,
        fn visit_hex_digit: unsyn::compiler::lexer::HexDigit<I> => unsyn::compiler::visit::walk::walk_hex_digit,
        fn visit_lit_dec: unsyn::compiler::lexer::LitDec<I> => unsyn::compiler::visit::walk::walk_lit_dec,
        fn visit_lit_unicode: unsyn::compiler::lexer::LitUnicode<I> => unsyn::compiler::visit::walk::walk_lit_unicode,
        fn visit_block_comment: unsyn::compiler::lexer::BlockComment<I> => unsyn::compiler::visit::walk::walk_block_comment,
        fn visit_block_comment_body:
            unsyn::compiler::lexer::BlockCommentBody<I> =>
                unsyn::compiler::visit::walk::walk_block_comment_body,
        fn visit_empty_block_comment:
            unsyn::compiler::lexer::EmptyBlockComment<I> =>
                unsyn::compiler::visit::walk::walk_empty_block_comment,
        fn visit_starred_empty_block_comment:
            unsyn::compiler::lexer::StarredEmptyBlockComment<I> =>
                unsyn::compiler::visit::walk::walk_starred_empty_block_comment,
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
    Generated: crate::generated::Visit<'ast, I>, crate::generated::UnsynInput,
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
        fn visit_negated_set:
            crate::generated::NegatedSet<I> => crate::generated::walk::walk_negated_set,
        fn visit_call: crate::generated::Call<I> => crate::generated::walk::walk_call,
        fn visit_paren: crate::generated::Paren<I> => crate::generated::walk::walk_paren,
        fn visit_set: crate::generated::Set<I> => crate::generated::walk::walk_set,
        fn visit_suffix: crate::generated::Suffix<I> => crate::generated::walk::walk_suffix,
        fn visit_star: crate::generated::Star<I> => crate::generated::walk::walk_star,
        fn visit_question: crate::generated::Question<I> => crate::generated::walk::walk_question,
        fn visit_plus: crate::generated::Plus<I> => crate::generated::walk::walk_plus,
        fn visit_expr_with_suffix:
            crate::generated::ExprWithSuffix<I> => crate::generated::walk::walk_expr_with_suffix,
        fn visit_suffixed: crate::generated::Suffixed<I> => crate::generated::walk::walk_suffixed,
        fn visit_repeat: crate::generated::Repeat<I> => crate::generated::walk::walk_repeat,
        fn visit_repeat_bound:
            crate::generated::RepeatBound<I> => crate::generated::walk::walk_repeat_bound,
        fn visit_range_to: crate::generated::RangeTo<I> => crate::generated::walk::walk_range_to,
        fn visit_range_from:
            crate::generated::RangeFrom<I> => crate::generated::walk::walk_range_from,
        fn visit_range: crate::generated::Range<I> => crate::generated::walk::walk_range,
        fn visit_unicode_range:
            crate::generated::UnicodeRange<I> => crate::generated::walk::walk_unicode_range,
        fn visit_str_range: crate::generated::StrRange<I> => crate::generated::walk::walk_str_range,
        fn visit_set_item: crate::generated::SetItem<I> => crate::generated::walk::walk_set_item,
        fn visit_ident_or_keyword: crate::generated::IdentOrKeyword<I> => crate::generated::walk::walk_ident_or_keyword,
        fn visit_raw_ident: crate::generated::RawIdent<I> => crate::generated::walk::walk_raw_ident,
        fn visit_non_keyword_ident:
            crate::generated::NonKeywordIdent<I> => crate::generated::walk::walk_non_keyword_ident,
        fn visit_ident: crate::generated::Ident<I> => crate::generated::walk::walk_ident,
        fn visit_keywords: crate::generated::Keywords<I> => crate::generated::walk::walk_keywords,
        fn visit_leading_keywords: crate::generated::LeadingKeywords<I> => crate::generated::walk::walk_leading_keywords,
        fn visit_operator_keywords:
            crate::generated::OperatorKeywords<I> => crate::generated::walk::walk_operator_keywords,
        fn visit_lit_str: crate::generated::LitStr<I> => crate::generated::walk::walk_lit_str,
        fn visit_quote_escape: crate::generated::QuoteEscape<I> => crate::generated::walk::walk_quote_escape,
        fn visit_ascii_escape: crate::generated::AsciiEscape<I> => crate::generated::walk::walk_ascii_escape,
        fn visit_ascii_hex_escape:
            crate::generated::AsciiHexEscape<I> => crate::generated::walk::walk_ascii_hex_escape,
        fn visit_ascii_newline:
            crate::generated::AsciiNewline<I> => crate::generated::walk::walk_ascii_newline,
        fn visit_ascii_carriage_return:
            crate::generated::AsciiCarriageReturn<I> => crate::generated::walk::walk_ascii_carriage_return,
        fn visit_ascii_tab: crate::generated::AsciiTab<I> => crate::generated::walk::walk_ascii_tab,
        fn visit_ascii_backslash:
            crate::generated::AsciiBackslash<I> => crate::generated::walk::walk_ascii_backslash,
        fn visit_ascii_null: crate::generated::AsciiNull<I> => crate::generated::walk::walk_ascii_null,
        fn visit_unicode_escape: crate::generated::UnicodeEscape<I> => crate::generated::walk::walk_unicode_escape,
        fn visit_oct_digit: crate::generated::OctDigit<I> => crate::generated::walk::walk_oct_digit,
        fn visit_dec_digit: crate::generated::DecDigit<I> => crate::generated::walk::walk_dec_digit,
        fn visit_hex_digit: crate::generated::HexDigit<I> => crate::generated::walk::walk_hex_digit,
        fn visit_lit_dec: crate::generated::LitDec<I> => crate::generated::walk::walk_lit_dec,
        fn visit_lit_unicode: crate::generated::LitUnicode<I> => crate::generated::walk::walk_lit_unicode,
        fn visit_block_comment: crate::generated::BlockComment<I> => crate::generated::walk::walk_block_comment,
        fn visit_block_comment_body:
            crate::generated::BlockCommentBody<I> => crate::generated::walk::walk_block_comment_body,
        fn visit_empty_block_comment:
            crate::generated::EmptyBlockComment<I> => crate::generated::walk::walk_empty_block_comment,
        fn visit_starred_empty_block_comment:
            crate::generated::StarredEmptyBlockComment<I> =>
                crate::generated::walk::walk_starred_empty_block_comment,
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

/// Reads `source` with both parsers, walks both trees with both visitors, and
/// returns the events each one said, as they came out of its visitor.
///
/// Nothing else is done to either sequence. The spans are the nodes' own text,
/// trivia and all, because both trees file a run of trivia in the same place —
/// beside the operands of the sequence that reads them.
fn both_trees(source: &str) -> (Vec<Event>, Vec<Event>) {
    use parserc::{Input, syntax::SyntaxInput};

    let mut hand_input = unsyn::compiler::input::TokenStream::from(source);
    let hand_tree = hand_input
        .parse::<unsyn::compiler::syntax::Crate<_>>()
        .unwrap_or_else(|error| panic!("the hand parser reads {source:?}: {error}"));
    assert!(
        hand_input.is_empty(),
        "the hand parser left text of {source:?}"
    );

    // The generated front end names its own stream, as the hand front end
    // names its own: the two are the same concrete stream over the same error,
    // so the spans the two trees report are measured against one source.
    let mut generated_input = generated::TokenStream::from(source);
    let generated_tree = generated_input
        .parse::<generated::Crate<_>>()
        .unwrap_or_else(|error| panic!("the generated parser reads {source:?}: {error}"));
    assert!(
        generated_input.is_empty(),
        "the generated parser left text of {source:?}"
    );

    let mut hand_recorder = Hand::default();
    unsyn::compiler::visit::Visit::visit_crate(&mut hand_recorder, &hand_tree);

    let mut generated_recorder = Generated::default();
    generated::Visit::visit_crate(&mut generated_recorder, &generated_tree);

    (generated_recorder.events, hand_recorder.events)
}

/// Says the two event sequences are one sequence, and points at the first
/// event where they part.
fn assert_one_sequence(generated: &[Event], hand: &[Event], source: &str) {
    for (i, (g, h)) in generated.iter().zip(hand.iter()).enumerate() {
        if g != h {
            println!("G: {:?}", &generated[i.saturating_sub(3)..i + 5]);
            println!("H: {:?}", &hand[i.saturating_sub(3)..i + 5]);
            panic!("first disagreement at event {i} of {source:?}");
        }
    }

    assert_eq!(
        generated.len(),
        hand.len(),
        "the two trees said a different number of rules over {source:?}"
    );
}

/// The two trees over `unsyn.us` say the same rules over the same text.
#[test]
fn the_two_trees_say_the_same_rules_over_the_same_text() {
    let (generated, hand) = both_trees(include_str!("../unsyn.us"));

    assert!(!hand.is_empty());

    assert_one_sequence(&generated, &hand, "unsyn.us");

    // A rule the spec names and both trees therefore walk has to show up in
    // what they said, or the comparison of it would be vacuous. The three
    // groups the spec took the trouble to name are the ones this says.
    for reached in ["visit_set", "visit_suffix", "visit_repeat_bound"] {
        assert!(
            generated.iter().any(|(hook, _)| *hook == reached),
            "the generated tree never reached {reached}"
        );
        assert!(
            hand.iter().any(|(hook, _)| *hook == reached),
            "the hand tree never reached {reached}"
        );
    }
}

/// A count of more than one digit is a token `unsyn.us` never writes, and the
/// one place the two trees had agreed only because the file was too small to
/// show otherwise: the hand tree used to hold the digits of a number one by
/// one, where the generated code takes the run after the first digit as text.
///
/// No bound in `unsyn.us` is above a single digit (`{1, 6}`), so the spec
/// itself cannot tell the two readings apart. This file can.
#[test]
fn a_count_above_one_digit_is_one_token_in_both_trees() {
    let source = r#"whitespace S -> [' '];
syntax A -> 'b'{12, 340};
syntax C -> 'd'{1_000};
syntax E -> 'f'{60, 7};
syntax G -> 'h'{,99};
"#;

    let (generated, hand) = both_trees(source);

    // The counts are read: every bound is reached, and a bound of two numbers
    // leaves the first digit of each — six across the four bounds written,
    // because two of them spell a lower and an upper.
    let digits = generated
        .iter()
        .filter(|(hook, _)| *hook == "visit_dec_digit")
        .count();
    assert_eq!(digits, 6, "the first digit of each count written");

    let bounds = generated
        .iter()
        .filter(|(hook, _)| *hook == "visit_repeat_bound")
        .count();
    assert_eq!(bounds, 4, "each of the four bounds written");

    assert_one_sequence(&generated, &hand, source);
}

/// A block comment is one token of text in both trees, and the nesting a
/// reading has to count to find its own closing leaves nothing behind.
///
/// `unsyn.us` cannot show this: it is written with line documents, so the
/// parity check over it never reaches the comment family. This file does, and
/// the two trees say the same rules over it with nothing done to either
/// sequence — which is the point, since the hand tree used to break a comment
/// into a head, a tail and a nested group and needed a projection to be
/// compared at all.
#[test]
fn a_block_comment_is_one_token_of_text_in_both_trees() {
    let source = r#"/*! inner /* x */ doc */
/** outer /* nested */ doc */
syntax A -> 'b';
"#;

    let (generated, hand) = both_trees(source);

    assert_one_sequence(&generated, &hand, source);
}

/// A name written raw is a name of two parts, and both trees say so.
///
/// `unsyn.us` never writes `r#`, so the parity check over it cannot tell
/// whether the two trees agree on the raw form of a name — and they did not:
/// one held the sigil and the name behind it as the two sub-expressions the
/// rule writes, the other held the whole of it as one run of text.
#[test]
fn a_name_written_raw_is_two_parts_in_both_trees() {
    let source = "syntax r#where -> 'a';\nlexer r#type -> 'b';\n";

    let (generated, hand) = both_trees(source);

    // The name behind the sigil is a node of its own, and both trees reach it:
    // the rule the sigil reaches is the one that refuses a word of the language,
    // so the raw form holds a name and not a bare run of characters.
    assert!(
        generated.iter().any(|(hook, _)| *hook == "visit_raw_ident"),
        "the generated tree never reached the raw form of a name"
    );
    assert!(
        generated
            .iter()
            .any(|(hook, _)| *hook == "visit_non_keyword_ident"),
        "the generated tree never reached the name behind the sigil"
    );

    assert_one_sequence(&generated, &hand, source);
}

/// The rules the two trees settle differently, in the one thing the parity
/// above can see: which hooks its recorders reach.
///
/// A recorder compares only what it is told to record, so a node the generated
/// half does not record is a node the comparison says nothing about — and the
/// silence would read as agreement. This names the silence.
///
/// Every rule the spec names is now a node of both trees and a hook of both
/// recorders, so what is left here is the pair that was never a rule: the text a
/// walk ends with, which is the leaf, and the trivia `S`, which both trees walk
/// and neither records. A third entry arriving would be a rule the parity had
/// silently stopped covering.
#[test]
fn the_parity_reaches_every_rule_of_either_tree() {
    let kept = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/generated/unsyn_us.rs"
    ))
    .expect("the kept file should be readable");

    let file = syn::parse_str::<syn::File>(&kept).expect("the kept file is Rust");

    // Every hook the visitor the generated file ships declares.
    let trait_hooks = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Trait(each) if each.ident == "Visit" => Some(each),
            _ => None,
        })
        .expect("the generated file ships a Visit trait")
        .items
        .iter()
        .filter_map(|entry| match entry {
            syn::TraitItem::Fn(each) => Some(each.sig.ident.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    let unrecorded = trait_hooks
        .iter()
        .filter(|hook| !Generated::HOOKS.contains(&hook.as_str()))
        .collect::<Vec<_>>();

    let expected = ["visit_text", "visit_s"];

    for hook in expected {
        assert!(
            unrecorded.contains(&&hook.to_owned()),
            "{} should be a hook the parity does not reach: {unrecorded:?}",
            hook
        );
    }

    assert_eq!(
        unrecorded.len(),
        expected.len(),
        "the parity leaves a rule out that this test does not name: {unrecorded:?}"
    );

    // None of the ten is a hook the hand tree has and the table leaves out,
    // which would be a plain fault of the table rather than a difference
    // between the trees.
    for hook in expected {
        assert!(
            !Hand::HOOKS.contains(&hook),
            "the hand tree reaches {hook}, so the parity should compare it"
        );
    }
}
