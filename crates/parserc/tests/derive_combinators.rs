//! The contract of the `Syntax` derive: every combinator of `parserc` can be
//! written at every position a parser is needed, under the name of the function
//! it calls, and reads what that function reads.
//!
//! The four positions are a `struct` that is one run of the input, an `enum`
//! that is one, a variant that is one inside a choice, and a field. The seven
//! combinators are the seven of `parserc::c`; `parser` is checked alongside them
//! because it is the escape hatch they are all shortcuts for.

use parserc::chars::CharsInput;
use parserc::syntax::{Syntax, SyntaxInput};
use parserc::{AsStr, Input, Kind, Span};

type Stream<'a> = parserc::chars::TokenStream<'a, Kind>;

/// Reads `source` as one `T`, asks that `T` leave nothing of it unread, and
/// says what the read took.
///
/// Reaching the end of the input is the half of each assertion that says the
/// combinator was used at all: a position that quietly fell back to the type's
/// own parser, or skipped its field, would stop short. Two combinators end on a
/// word they do not take and are asserted elsewhere, on their leftover.
fn took<T>(source: &str) -> String
where
    T: Syntax<Stream<'static>>,
{
    // The input is copied rather than borrowed from `source`, so the lifetime
    // `T` is asked about is one the caller never sees and `'static` stands for
    // all of them.
    let owned: &'static str = Box::leak(source.to_owned().into_boxed_str());
    let mut input = Stream::from(owned);
    let node = input.parse::<T>().expect(source);

    assert!(input.is_empty(), "{source:?} left text unread");

    let Span::Range(range) = node.to_span() else {
        panic!("a leaf reads a range of the input");
    };

    owned[range].to_owned()
}

// -- one combinator on a `struct` -------------------------------------------

#[derive(Debug, Syntax)]
#[parserc(next = '!')]
struct OneChar<I>(pub I)
where
    I: CharsInput;

#[derive(Debug, Syntax)]
#[parserc(next_if = |c: char| c.is_ascii_digit())]
struct OneClass<I>(pub I)
where
    I: CharsInput;

#[derive(Debug, Syntax)]
#[parserc(keyword = "if")]
struct OneWord<I>(pub I)
where
    I: CharsInput;

#[derive(Debug, Syntax)]
#[parserc(take_until = ";")]
struct UpToSemi<I>(pub I)
where
    I: CharsInput;

#[derive(Debug, Syntax)]
#[parserc(take_while = |c: char| c != ';')]
struct ToSemi<I>(pub I)
where
    I: CharsInput;

#[derive(Debug, Syntax)]
#[parserc(take_while_with = (2..=4, |c: char| c == 'a'))]
struct SomeAs<I>(pub I)
where
    I: CharsInput;

#[derive(Debug, Syntax)]
#[parserc(take_till = |c: char| c == ' ')]
struct OneWordByTill<I>(pub I)
where
    I: CharsInput;

#[derive(Debug, Syntax)]
#[parserc(parser = parserc::keyword("if"))]
struct WordByParser<I>(pub I)
where
    I: CharsInput;

#[test]
fn every_combinator_reads_a_struct() {
    assert_eq!(took::<OneChar<_>>("!"), "!");
    assert_eq!(took::<OneClass<_>>("7"), "7");
    assert_eq!(took::<OneWord<_>>("if"), "if");
    assert_eq!(took::<WordByParser<_>>("if"), "if");
}

#[test]
fn a_combinator_that_stops_short_of_a_word_leaves_it_unread() {
    // `take_until`, `take_while` and `take_till` are the three that end on
    // something they do not take, so the span they answer with is the run before
    // it and the delimiter is still there for the parser after. Asserting the
    // leftover rather than its absence is what tells the three apart from a
    // combinator that ate the delimiter by mistake.
    let mut input = Stream::from("ab;");
    let up_to = input.parse::<UpToSemi<_>>().unwrap();

    assert_eq!(up_to.to_span(), Span::Range(0..2));
    assert_eq!(input.as_str(), ";", "`take_until` left its delimiter");

    let mut input = Stream::from("ab;");
    let to = input.parse::<ToSemi<_>>().unwrap();

    assert_eq!(to.to_span(), Span::Range(0..2));
    assert_eq!(input.as_str(), ";", "`take_while` left what it refused");

    let mut input = Stream::from("ab ");
    let till = input.parse::<OneWordByTill<_>>().unwrap();

    assert_eq!(till.to_span(), Span::Range(0..2));
    assert_eq!(input.as_str(), " ", "`take_till` left what it stopped at");

    // A bound is greedy up to its top and stops there, taking none of what it
    // stopped on: `aaa!` is three of the four the rule allows, and the `!` was
    // never asked for.
    let mut input = Stream::from("aaaa!");
    let some = input.parse::<SomeAs<_>>().unwrap();

    assert_eq!(
        some.to_span(),
        Span::Range(0..4),
        "the bound's top is what it takes"
    );
    assert_eq!(input.as_str(), "!");
}

#[test]
fn a_struct_read_as_a_run_spans_just_that_run() {
    // The span is the whole of what the combinator took and no more, which is
    // what `to_span` of a leaf node can promise without looking at parts it
    // does not have.
    let mut input = Stream::from("ab;");
    let taken = input.parse::<ToSemi<_>>().unwrap();

    assert_eq!(taken.to_span(), Span::Range(0..2));
}

// -- one combinator on an `enum`, and on one variant of a choice -------------

#[derive(Debug, Syntax)]
#[parserc(keyword = "fn")]
struct KwOnly<I>(pub I)
where
    I: CharsInput;

#[derive(Debug, Syntax)]
enum Punctuation<I>
where
    I: CharsInput,
{
    #[parserc(next = ',')]
    Comma(I),
    #[parserc(next = ';')]
    Semicolon(I),
    /// An arm read from parts rather than as one run: the only shape a variant
    /// had before the combinator was accepted here.
    Pair(OneChar<I>, OneWord<I>),
}

#[test]
fn every_combinator_reads_an_enum_and_a_variant() {
    // An `enum` of one variant that is one run reads exactly as that run.
    assert_eq!(took::<KwOnly<_>>("fn"), "fn");

    // A variant of a choice reads its own combinator, and the arm beside it is
    // still reached when its own word is not there.
    assert_eq!(took::<Punctuation<_>>(","), ",");
    assert_eq!(took::<Punctuation<_>>(";"), ";");
    assert_eq!(took::<Punctuation<_>>("!if"), "!if");
}

// -- one combinator on a field ----------------------------------------------

#[derive(Debug, Syntax)]
struct Word<I>
where
    I: CharsInput,
{
    #[parserc(keyword = "let")]
    pub head: I,
    #[parserc(take_while_with = (1.., |c: char| c.is_ascii_whitespace()))]
    pub gap: I,
    #[parserc(next_if = |c: char| c.is_ascii_alphabetic())]
    pub name: I,
}

#[test]
fn every_combinator_reads_a_field() {
    assert_eq!(took::<Word<_>>("let  x"), "let  x");
}

#[test]
fn a_field_combinator_asks_no_trivia_of_its_own() {
    // Each field is the text its combinator took, read one after the other: the
    // run between `let` and `x` is a field here because the rule asked for it,
    // and nothing else slips between the three.
    let mut input = Stream::from("let\tx");
    let word = input.parse::<Word<_>>().unwrap();

    assert_eq!(
        (word.head.as_str(), word.gap.as_str(), word.name.as_str()),
        ("let", "\t", "x")
    );
}
