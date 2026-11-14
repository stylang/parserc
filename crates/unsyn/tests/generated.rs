//! Runs the Rust the generator wrote, against real text.
//!
//! `data/mini.rs` is what [`Codegen`] emits for the spec below, which `tests/dump.rs`
//! prints. It is kept as a file and compiled here because that is the only way
//! to say the emitted types are not merely valid Rust but *working* Rust: the
//! separators are fields of the tree they belong to, a rule that names itself is
//! held behind a pointer, and the shortcuts [`scan`] takes agree with the loops
//! they stand for.
//!
//! The first test is the one that keeps the rest honest: it regenerates the file
//! and says so when the generator moved without the fixture.
//!
//! [`Codegen`]: unsyn::codegen::Codegen
//! [`scan`]: unsyn::codegen

use std::{fs, path::PathBuf};

use parserc::{
    AsStr, Span,
    syntax::{Syntax, SyntaxInput},
};
use quote::{ToTokens, quote};
use unsyn::{codegen::Codegen, input::TokenStream, semantics::analyze, syntax::Crate};

#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code)]
#[path = "data/mini.rs"]
mod mini;

//  The spec `data/mini.rs` was generated from, which `tests/dump.rs` prints too.
//  It covers every construct `bootstrap.uns` is written with: sets, ranges, the
//  wildcard, complements, `concat`, `except`, `followed`, counted runs, groups
//  that need a type of their own, and rules that reach themselves.
include!("data/spec.rs");

/// Reads all of `text` as one node of the generated tree, and says so if the
/// rule left anything of it unread.
fn taken<T: Syntax<TokenStream<'static>>>(text: &'static str) -> T {
    let (taken, rest) = part(text);

    assert_eq!(rest, "", "`{text}` leaves `{rest}` of the rule unread");

    taken
}

/// Reads as much of `text` as the rule takes, and gives back what was left.
fn part<T: Syntax<TokenStream<'static>>>(text: &'static str) -> (T, String) {
    let mut input = TokenStream::from(text);

    let taken = match <T as Syntax<TokenStream<'static>>>::parse(&mut input) {
        Ok(taken) => taken,
        Err(error) => panic!(
            "`{text}` is not read by {}: {error}",
            std::any::type_name::<T>()
        ),
    };

    (taken, input.as_str().to_owned())
}

/// The range of the source a node was read from, which is all the tests here
/// need of a [`Span`].
fn range_of<T: Syntax<TokenStream<'static>>>(taken: &T) -> (usize, usize) {
    match taken.to_span() {
        Span::Range(range) => (range.start, range.end),
        other => panic!("the span of the node taken is not a range: {other:?}"),
    }
}

/// The same module `data/mini.rs` holds, generated now.
///
/// The table is the one `tests/dump.rs` writes the fixture with, since a name
/// is only what the caller says it is and the two have to agree about what the
/// spec's `<NAME>`s call.
fn regenerated() -> String {
    let mut tokens = TokenStream::from(SPEC);
    let file = Crate::parse(&mut tokens).expect("the spec should parse");
    let module = analyze(&file).expect("the spec should be well-formed");

    let codegen = Codegen::new().with_externals([
        (
            "XID_Start".to_owned(),
            quote!(::unsyn::runtime::external::XID_Start),
        ),
        (
            "XID_Continue".to_owned(),
            quote!(::unsyn::runtime::external::XID_Continue),
        ),
    ]);

    match codegen.generate(&module) {
        Ok(generated) => generated.to_string(),
        Err(errors) => panic!("the spec should be writable: {errors:?}"),
    }
}

#[test]
fn the_fixture_is_what_the_generator_writes() {
    let filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/mini.rs");
    let kept = fs::read_to_string(&filepath).expect("the fixture should be there");

    let kept = declared(&kept);
    let now = declared(&regenerated());

    assert_eq!(
        kept,
        now,
        "`{}` is behind the generator; run `cargo test -p unsyn --test dump -- --nocapture` and copy what it writes",
        filepath.display()
    );
}

/// The types a Rust file declares, each with the fields it declares: the
/// inventory two files are the same when they share.
///
/// A formatter may part the tokens of a generated file from the tokens the
/// generator wrote — it puts a block around a long closure body and takes it
/// away again from a short one — so the fixture is compared with what it stands
/// for by what it declares, which is what a generated file is for.
fn declared(text: &str) -> Vec<String> {
    let file = syn::parse_file(text).expect("the file should be Rust");

    let mut types = Vec::new();

    for item in &file.items {
        match item {
            syn::Item::Struct(given) => {
                let fields = given
                    .fields
                    .iter()
                    .map(|field| field.ty.to_token_stream().to_string())
                    .collect::<Vec<_>>()
                    .join("| ");

                types.push(format!("struct {}({fields})", given.ident));
            }
            syn::Item::Enum(given) => {
                let variants = given
                    .variants
                    .iter()
                    .map(|variant| {
                        let fields = variant
                            .fields
                            .iter()
                            .map(|field| field.ty.to_token_stream().to_string())
                            .collect::<Vec<_>>()
                            .join("| ");

                        format!("{}({fields})", variant.ident)
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");

                types.push(format!("enum {}{{{variants}}}", given.ident));
            }
            _ => {}
        }
    }

    types
}

#[test]
fn a_lexer_rule_keeps_the_text_it_took() {
    let (word, rest) = part::<mini::WORD<_>>("abc_1 rest");

    assert_eq!(word.0.as_str(), "abc_1");
    assert_eq!(rest, " rest");
    assert_eq!(range_of(&word), (0, 5));

    // A keyword of the language ends where a word would carry on, so `letx` is
    // one word and not the keyword `let` with `x` after it.
    assert!(mini::LET::parse(&mut TokenStream::from("letx")).is_err());
    assert_eq!(part::<mini::WORD<_>>("letx").0.0.as_str(), "letx");
}

#[test]
fn a_name_is_a_word_that_is_not_a_keyword() {
    let (name, rest) = part::<mini::IDENT<_>>("lett x");

    assert_eq!(name.0.as_str(), "lett");
    assert_eq!(rest, " x");

    // The operand `except` removes is asked about where the operand it removes
    // something from starts, and it takes nothing itself.
    assert!(mini::IDENT::parse(&mut TokenStream::from("let x")).is_err());
    assert!(mini::IDENT::parse(&mut TokenStream::from("fn x")).is_err());
}

#[test]
fn operands_joined_by_concat_are_one_field_with_nothing_between() {
    // `DIGIT concat UNDER DIGIT` asks the backend to read the first two as one
    // token, so they are one field of a tuple type and no separator is taken
    // between them; the third operand is reached over the trivia, which is
    // where the rule ends: `1_2` is the whole of it.
    let joined = taken::<mini::Joined<_>>("1_2");

    let (head, tail) = (&joined.0.0, &joined.0.1);

    assert_eq!(head.0.as_str(), "1");
    assert_eq!(tail.0.as_str(), "_");
    assert_eq!(joined.2.0.as_str(), "2");

    // The pair is taken back to back, so a run of trivia between its two
    // operands is nothing the rule can read.
    assert!(mini::Joined::parse(&mut TokenStream::from("1 _2")).is_err());
}

#[test]
fn a_followed_operand_is_asked_where_the_operand_left_off() {
    // `IDENT followed '-'` takes the name and then asks about what comes after
    // it, taking nothing of what it asked about.
    let (guarded, rest) = part::<mini::Guarded<_>>("ab-c");

    assert_eq!(guarded.0.0.0.as_str(), "ab");
    // The assertion took nothing, so the text it asked about is still there.
    assert_eq!(rest, "-c");

    // Without the `-` after the name the assertion fails, and it fails after
    // the name was read, which is what the probe rewinds.
    assert!(mini::Guarded::parse(&mut TokenStream::from("ab c")).is_err());
}

#[test]
fn the_wildcard_stands_for_any_one_character() {
    // `(MATCHER | _) (MATCHER | _)*` is a run of one character each, where the
    // underscore is any character at all: it is the wildcard of the language
    // being described, not the trivia between two tokens.
    let any = taken::<mini::ANYONE<_>>("a-b");

    assert_eq!(any.1.len(), 2, "{any:?}");

    // The run is of a group of two alternatives, so each of its elements says
    // which of them it was read as: a character the external takes, or any one
    // character at all.
    assert!(
        matches!(
            any.1[0].1,
            mini::ANYONE2::ANYONE2MATCHER(_) | mini::ANYONE2::ANYONE2Any(_)
        ),
        "{any:?}"
    );
}

#[test]
fn a_comment_reads_over_the_text_it_is_asked_about() {
    // `SLASH STAR (~STAR | UNDER)* STAR SLASH`: the run takes one character at
    // a time, each of them asked about first, and stops at the `STAR` that
    // closes the comment.
    let commented = taken::<mini::Commented<_>>("/*ab*/");

    assert_eq!(range_of(&commented), (0, 6));
    assert_eq!(commented.3.len(), 2, "{commented:?}");

    // A comment with a star inside it is nothing the rule reads, since nothing
    // in the run can take the character it asks about.
    assert!(mini::Commented::parse(&mut TokenStream::from("/*a*b*/")).is_err());
}

#[test]
fn a_group_at_the_head_of_a_rule_is_a_field_of_its_own() {
    // `(IDENT | DIGIT) LEFT IDENT` cannot be a tuple of the rule, since the
    // group is a sum, so the rule is three fields and the first is the group.
    let either = taken::<mini::Either<_>>("a[b");

    assert_eq!(either.2.0.as_str(), "[");
    assert_eq!(either.4.0.as_str(), "b");
    assert!(
        matches!(
            either.0,
            mini::Either1::Either1IDENT(_) | mini::Either1::Either1DIGIT(_)
        ),
        "{either:?}"
    );
}

#[test]
fn a_counted_run_of_nodes_is_a_type_of_its_own() {
    let two = taken::<mini::Dotted<_>>("a b");

    assert_eq!(two.0.0.len(), 2, "{two:?}");

    // The bounds are of the count and not of the text, and a run too short
    // fails where it stands.
    assert!(mini::Dotted::parse(&mut TokenStream::from("a")).is_err());

    let three = taken::<mini::Dotted<_>>("a b c");

    assert_eq!(three.0.0.len(), 3, "{three:?}");
}

#[test]
fn the_trivia_rule_takes_one_character_of_what_it_lists() {
    let mut input = TokenStream::from(" \t\n x");

    let taken = input
        .parse::<mini::WS<_>>()
        .expect("one character of trivia");

    // The `whitespace` rule names one character of trivia, as any other token
    // does; a run of them is what a separator holds, and this is the one place
    // a single character of it is taken on its own.
    assert_eq!(taken.0.as_str(), " ");
    assert_eq!(input.as_str(), "\t\n x");

    // Where there is no trivia at all the rule says so rather than taking
    // nothing, which is what makes a separator a run that may be empty.
    assert!(mini::WS::parse(&mut TokenStream::from("x")).is_err());
}

#[test]
fn the_trivia_between_two_tokens_is_a_field_of_its_own() {
    // The separators are held, which is what makes the tree concrete: what was
    // between two tokens is part of the tree and not something thrown away.
    let tight = taken::<mini::ITEM<_>>("fn f -> a;");

    assert_eq!(tight.1.len(), 1, "one run between the keyword and the name");
    assert_eq!(tight.3[0].0.as_str(), " ");

    let wide = taken::<mini::ITEM<_>>("fn    f   -> a ;");

    // A separator is a run of the trivia rule, so the four spaces before the
    // name are four characters of it and one token of the run.
    assert_eq!(wide.1.len(), 4, "{wide:?}");
    assert_eq!(wide.7[0].0.as_str(), " ");

    // The last separator before the semicolon is held too, and the node ends
    // where the semicolon does.
    assert_eq!(range_of(&wide), (0, 16));
}

#[test]
fn a_rule_of_several_alternatives_says_which_one_it_took() {
    assert!(matches!(
        taken::<mini::BINOP<_>>("+"),
        mini::BINOP::BINOPPlus(_)
    ));
    assert!(matches!(
        taken::<mini::BINOP<_>>("->"),
        mini::BINOP::BINOPARROW(_)
    ));
    assert!(matches!(
        taken::<mini::PRIMARY<_>>("x"),
        mini::PRIMARY::PRIMARYIDENT(_)
    ));
    assert!(matches!(
        taken::<mini::PRIMARY<_>>("(x)"),
        mini::PRIMARY::PRIMARYGROUP(_)
    ));
    assert!(matches!(
        taken::<mini::LITERAL<_>>("1"),
        mini::LITERAL::LITERALNUMBER(_)
    ));
}

#[test]
fn a_rule_that_names_itself_is_taken_from_the_inside_out() {
    let nested = taken::<mini::EXPR<_>>("a + (b - (c))");

    // The rest of an expression is a run of the operator and the operand it
    // takes, and the operand is a node of the rule being read.
    assert_eq!(nested.1.len(), 1, "{nested:?}");

    let binop = &nested.1[0].1;

    assert!(matches!(&binop.0, mini::BINOP::BINOPPlus(_)));

    // The operand of the second run is a group, and the group holds the
    // expression the parentheses are around.
    let mini::PRIMARY::PRIMARYGROUP(group) = &*binop.2 else {
        panic!("the operand of `+` is not a group");
    };

    let first = &group.2;

    assert!(matches!(&*first.0, mini::PRIMARY::PRIMARYIDENT(_)));

    // The group reaches the same rule through the parentheses that hold it,
    // and so on in: `a + (b - (c))` is a group two deep.
    let innermost = &group.2.1[0].1.2;

    let mini::PRIMARY::PRIMARYGROUP(inner) = &**innermost else {
        panic!("the innermost operand is not a group: {innermost:?}");
    };

    assert_eq!(inner.0.0.as_str(), "(");
    assert_eq!(inner.4.0.as_str(), ")");
}

#[test]
fn an_optional_run_of_operands_may_be_taken_whole_or_not_at_all() {
    let none = taken::<mini::PARAMS<_>>("()");

    assert!(none.2.is_none(), "{none:?}");

    let three = taken::<mini::PARAMS<_>>("(a, b, c)");

    let run = three.2.as_ref().expect("the names between the parentheses");

    assert_eq!(run.0.0.as_str(), "a");
    assert_eq!(run.1.len(), 2, "{run:?}");
    assert_eq!(run.1[0].1.2.0.as_str(), "b");
    assert_eq!(run.1[1].1.2.0.as_str(), "c");
}

#[test]
fn a_run_of_one_or_more_takes_at_least_one() {
    // `(COMMA IDENT)+` asks for a name after every comma, so a tuple of one is
    // nothing the rule can read: the run after the first name must be taken.
    assert!(mini::TUPLE::parse(&mut TokenStream::from("(a)")).is_err());

    let one = taken::<mini::TUPLE<_>>("(a, b)");

    assert_eq!(one.3.len(), 1, "{one:?}");

    let three = taken::<mini::TUPLE<_>>("(a, b, c)");

    assert_eq!(three.3.len(), 2, "{three:?}");

    // And `(a,)` is nothing either: the run sticks at the comma it took, since
    // there is no name after it for the operand of the run to be.
    assert!(mini::TUPLE::parse(&mut TokenStream::from("(a,)")).is_err());
}

#[test]
fn a_counted_run_is_checked_after_it_was_taken() {
    let def = taken::<mini::SHAPE<_>>("#1234");

    assert_eq!(def.1.0.len(), 4, "{def:?}");

    // One character short of the count the rule stated is not a shape, and the
    // failure is one a caller may recover from.
    assert!(mini::SHAPE::<_>::parse(&mut TokenStream::from("#123")).is_err());

    // The bound is a count of the operand and not a length of the text, so
    // trivia between the characters of the run is taken as it comes between
    // two operands of a sequence.
    let spread = taken::<mini::SHAPE<_>>("# 1 2 3 4");

    assert_eq!(spread.1.0.len(), 4, "{spread:?}");
    assert_eq!(spread.1.0[1].0[0].0.as_str(), " ", "{spread:?}");
}

#[test]
fn a_group_inside_a_rule_is_a_type_named_for_the_rule_around_it() {
    let long = taken::<mini::NUMBER<_>>("1_234");

    assert_eq!(long.1.len(), 4, "{long:?}");

    // The group `(DIGIT | '_')` is an alternative of its own, so each run of
    // the repetition says which of its two members was taken.
    let mini::NUMBER1::NUMBER1Underscore(under) = &long.1[0].1 else {
        panic!(
            "the first run of the group is not an underscore: {:?}",
            long.1[0].1
        );
    };

    assert_eq!(under.0.as_str(), "_");
    assert!(matches!(long.1[1].1, mini::NUMBER1::NUMBER1DIGIT(_)));
}

#[test]
fn a_set_and_a_complement_written_in_a_rule_of_nodes_are_scanners_of_their_own() {
    // `~[BSLASH, QUOTE]` is one character that is neither of two, and the set
    // `['n', 't']` is one of two: both are token types, written for the rule.
    let escaped = taken::<mini::TEXT<_>>("'a\\nb'");

    assert_eq!(escaped.1.len(), 3, "{escaped:?}");
    assert_eq!(range_of(&escaped), (0, 6));

    let mini::TEXT1::TEXT1BSLASH(bslash, _, letter) = &escaped.1[1].1 else {
        panic!("the second run is not the escape");
    };

    assert_eq!(bslash.0.as_str(), "\\");
    assert_eq!(letter.0.as_str(), "n");

    // A quote ends the text where the run stops, since `~[BSLASH, QUOTE]` is
    // asked about the character here and does not fit for a quote.
    let (plain, rest) = part::<mini::TEXT<_>>("'a,b' rest");

    assert_eq!(rest, " rest");
    assert_eq!(plain.1.len(), 3, "{plain:?}");
    assert_eq!(plain.3.0.as_str(), "'");
}

#[test]
fn a_negated_operand_takes_the_one_character_it_asked_about() {
    let negated = taken::<mini::NEGATED<_>>("~x");

    assert_eq!(negated.0.0.as_str(), "~");

    let mini::PRIMARY::PRIMARYIDENT(name) = &*negated.2 else {
        panic!("the negation is not of a name");
    };

    assert_eq!(name.0.as_str(), "x");

    // `~` asks about the character here and takes it whatever the answer was,
    // so the node is two characters wide and no more.
    assert_eq!(range_of(&negated), (0, 2));
}

#[test]
fn a_node_reaches_over_the_trivia_it_holds() {
    let item = taken::<mini::ITEM<_>>("fn f -> a;");

    // The node begins where the keyword does and ends where the semicolon
    // does, which is what a span of a node is for.
    assert_eq!(range_of(&item), (0, 10));

    let group = taken::<mini::GROUP<_>>("( a )");

    // The node inside the parentheses is reached over the trivia on either
    // side of it, and the whole group is five characters wide.
    assert_eq!(range_of(&group), (0, 5));

    // The expression the group holds is itself a node of `EXPR`, which names
    // the rule that reaches back into it: the tree is a tree, not a token.
    let mini::PRIMARY::PRIMARYIDENT(name) = &*group.2.0 else {
        panic!(
            "the expression inside the group is not a name: {:?}",
            group.2
        );
    };

    assert_eq!(name.0.as_str(), "a");
}

#[test]
fn a_run_of_nodes_holds_the_trivia_before_each_of_them() {
    let list = taken::<mini::LIST<_>>("(a b c)");

    assert_eq!(list.1.len(), 3, "{list:?}");

    // The trivia before the first name of the run is empty, since the name
    // follows the parenthesis with nothing between them, and every name after
    // it is reached over the run that went before it.
    assert!(list.1[0].0.is_empty(), "{list:?}");
    assert_eq!(list.1[1].0[0].0.as_str(), " ");
    assert_eq!(list.1[1].1.0.as_str(), "b");

    let empty = taken::<mini::LIST<_>>("()");

    assert!(empty.1.is_empty(), "{empty:?}");
}
