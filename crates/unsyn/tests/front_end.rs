//! Reading with this crate's own front end, which is code a build wrote.
//!
//! The front end is no longer kept beside these tests as a file to compare: it
//! is compiled into the crate by `build.rs`, from `unsyn.us`, by the released
//! generator. So there is nothing here about whether two trees agree — the
//! agreement is the build, and a mismatch in a name, a field or a bound is a
//! compile error rather than a test failure.
//!
//! What a build cannot tell is whether the parser it wrote *reads*. A rule that
//! compiles and then takes the wrong span, or one character where the production
//! bounds six, or a CST that drops the trivia between two tokens, is a correct
//! program and a wrong parser. The tests here are the parts of that which the
//! spec can be asked about one at a time, plus the one property the whole tree
//! owes: that a walk hands over every character of the file exactly once.

use parserc::{
    AsStr, Input as _,
    syntax::{Syntax, SyntaxInput as _},
};
use unsyn::{
    error::UnsynError,
    input::TokenStream,
    lexer::{BlockComment, Ident, LitStr, LitUnicode, S, UnicodeEscape},
    syntax::{Crate, Item},
    visit::Visit as _,
};

/// Reads the node `T` from the start of `source`.
///
/// What reading reports is the error a build wrote; the `UnsynError` every pass
/// returns wraps that together with the families of the analysis, so the reading
/// failure is carried into it here, the way a pass that reads and then judges a
/// file does with `?`.
fn parse<'a, T>(source: &'a str) -> Result<T, UnsynError>
where
    T: Syntax<TokenStream<'a>>,
{
    TokenStream::from(source).parse::<T>().map_err(Into::into)
}

/// The text one node covers, as its own rule writes it: the whole of the source
/// under the node's span, which is what a CST is for and what no field-by-field
/// reconstruction says as plainly.
fn text<T: parserc::ToSpan>(source: &str, node: &T) -> String {
    parserc::as_str(&TokenStream::from(source), node).to_owned()
}

/// Which of the two forms of [`Ident`] was read, and the name it spells.
///
/// The raw form is a name of two parts — the sigil and, behind it, a name read
/// again by the rule that refuses the words of the language — and the plain form
/// is the one run that `except` leaves standing. Both answer with the whole of the
/// name, sigil and all, which is what a definition's name is worth.
fn name(node: &Ident<TokenStream<'_>>) -> (String, &'static str) {
    match node {
        Ident::RawIdent(raw) => (
            raw.f1.as_str().to_owned() + raw.non_keyword_ident.0.as_str(),
            "RawIdent",
        ),
        Ident::NonKeywordIdent(plain) => (plain.0.as_str().to_owned(), "NonKeywordIdent"),
    }
}

#[test]
fn a_character_literal_is_one_token() {
    for (source, literal) in [
        ("'a'", "'a'"),
        (r"'\''", r"'\''"),
        (r"'\n'", r"'\n'"),
        (r"'\u{41}'", r"'\u{41}'"),
        ("'lexer'", "'lexer'"),
        // A literal stops at its own closer: what stands behind the quote belongs
        // to whatever operand comes next, not to this one.
        ("'x' 'y'", "'x'"),
    ] {
        let read = parse::<LitStr<_>>(source).expect(source);

        assert_eq!(text(source, &read), literal, "{source}");
    }
}

#[test]
fn a_name_says_which_of_its_two_forms_it_is() {
    let raw = parse::<Ident<_>>("r#type").expect("a raw name");
    assert_eq!(name(&raw), ("r#type".to_owned(), "RawIdent"));

    let plain = parse::<Ident<_>>("lexer2").expect("a name that is no word");
    assert_eq!(name(&plain), ("lexer2".to_owned(), "NonKeywordIdent"));

    // The sigil borrows a word of Rust, never one of the language: the rule it
    // reaches refuses the DSL's own words, and that refusal is fatal, so
    // `r#lexer` is no name and no alternative of the choice is tried after it.
    assert!(
        parse::<Ident<_>>("r#lexer").is_err(),
        "a word of the language under a sigil is still that word"
    );

    assert!(
        parse::<Ident<_>>("lexer").is_err(),
        "a word of the language"
    );
}

#[test]
fn a_unicode_literal_wants_exactly_four_digits() {
    // The production joins the marker and the digits with `concat`, which is why
    // the two are fields of the node and no run of the separator stands between.
    let lit = parse::<LitUnicode<_>>("U+0041").expect("four digits");
    assert_eq!(lit.f1.as_str(), "U+");
    assert_eq!(lit.hex_digit.len(), 4, "one node per digit");

    assert!(parse::<LitUnicode<_>>("U+004").is_err(), "three digits");

    // Four and no more: the fifth character is whatever comes next in the file.
    let mut input = TokenStream::from("U+00419");
    let lit = input
        .parse::<LitUnicode<_>>()
        .expect("the first four digits");
    assert_eq!(lit.hex_digit.len(), 4);
    assert_eq!(input.as_str(), "9");
}

#[test]
fn an_escape_takes_one_to_six_digits_and_no_separator() {
    let escape = parse::<UnicodeEscape<_>>(r"\u{1F600}").expect("an escape of five digits");
    assert_eq!(escape.f1.as_str(), r"\u{");
    assert_eq!(escape.hex_digit.len(), 5, "one node per digit");
    assert_eq!(escape.f2.as_str(), "}");

    assert!(
        parse::<UnicodeEscape<_>>(r"\u{}").is_err(),
        "no digits at all is no escape"
    );

    // The rule bounds the digits and writes nothing between them, so a separator
    // is no part of an escape: the digits are taken, and the brace is then asked
    // for where the underscore stands.
    assert!(
        parse::<UnicodeEscape<_>>(r"\u{1F_600}").is_err(),
        "an underscore separates no digits here"
    );
}

#[test]
fn a_comment_closes_the_one_it_opened() {
    // A comment that nests is read by the rule of four operands: the opening, the
    // head, the runs after the head, and the closer of this opening. The whole is
    // one token of text, nesting included.
    let BlockComment::BlockCommentBody(body) =
        parse::<BlockComment<_>>("/* a /* b */ c */").expect("a comment that nests")
    else {
        panic!("a comment that nests is the rule's body, not one of its two short forms");
    };

    assert_eq!(
        format!(
            "{}{}{}{}",
            body.0.as_str(),
            body.1.as_str(),
            body.2.as_str(),
            body.3.as_str()
        ),
        "/* a /* b */ c */"
    );

    // The two forms that close at once are the other two alternatives, and they
    // are told apart by how the group opens and no more.
    assert!(matches!(
        parse::<BlockComment<_>>("/**/").expect("a comment of nothing"),
        BlockComment::EmptyBlockComment(_)
    ));
    assert!(matches!(
        parse::<BlockComment<_>>("/***/").expect("a comment of two stars"),
        BlockComment::StarredEmptyBlockComment(_)
    ));
}

#[test]
fn the_whitespace_is_the_set_of_the_rule_and_taken_greedily() {
    // The declaration is a set of characters, and the separator is one greedy run
    // over it — including the newline, which is why a run reaches across lines.
    let run = parse::<S<_>>(" \u{a0}\t\n ")
        .expect("a run of the separator")
        .0;

    assert_eq!(run.as_str(), " \u{a0}\t\n ");
}

#[test]
fn the_front_end_reads_the_spec_it_was_generated_from() {
    // The circle the bootstrap closes: the parser this crate ships — which a
    // build wrote from `unsyn.us` — reads `unsyn.us`, and reads all of it.
    let source = include_str!("../unsyn.us");
    let mut input = TokenStream::from(source);

    let tree = input
        .parse::<Crate<_>>()
        .expect("the front end reads the spec it was generated from");

    assert!(input.is_empty(), "the spec was left with text unread");

    // The definitions are told in the order the file declares them, and every
    // one of them names a rule: no definition is read as trivia.
    let rules = tree
        .item
        .iter()
        .filter(|item| matches!(item, Item::Stmt(_)))
        .count();

    assert_eq!(rules, 60, "the spec declares sixty productions");
}

#[test]
fn the_visitor_hands_over_every_character_once() {
    // What a CST owes the file it came from: the leaves, in order, are the source
    // — the trivia, the documents and the words alike, none dropped and none
    // counted twice. This is the property a build cannot check.
    struct Echo(String);

    impl<'ast, I> unsyn::visit::Visit<'ast, I> for Echo
    where
        I: unsyn::input::UnsynInput,
    {
        fn visit_text(&mut self, text: &'ast I) {
            self.0.push_str(text.as_str());
        }
    }

    let source = include_str!("../unsyn.us");
    let mut input = TokenStream::from(source);
    let tree = input.parse::<Crate<_>>().expect("the spec reads");

    let mut echo = Echo(String::new());
    echo.visit_crate(&tree);

    assert_eq!(echo.0, source, "every character, exactly once, in order");
}
