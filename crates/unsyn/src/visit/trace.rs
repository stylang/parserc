//! Writing down what a walk sees, so that two trees can be compared by what
//! they say about themselves rather than by what they are.
//!
//! The equivalence this crate will need at the end of its bootstrap is between
//! a hand-written tree and a generated one, and no `PartialEq` can state it: the
//! two are different types. What *can* be compared is the record a walk leaves
//! behind, because the record is written in the vocabulary alone — a shape's
//! name, the data it carries, and the places it was read from. Two trees that
//! decompose one file into one record accept one file in the same way, which is
//! the fact the equivalence test is after.
//!
//! The record is also the contract of [`decompose`](super::decompose): the tests
//! here say what one file's shapes are, in the order the walk reaches them, and a
//! second tree that reported the same shapes in the same order would pass the
//! same tests.

use std::fmt::Write;

use super::{
    Alternatives, Bounds, BoundsForm, Call, CharRange, CodePoint, Comment, Count, Declaration,
    DefKind, Element, Excepted, Followed, Group, Literal, LiteralValue, Name, Production, Quantity,
    QuantityOp, RangeSide, Reference, Sequence, Set, SetKind, SetMember, TextLiteral, Token, Visit,
    Visitor,
};

/// One line of a [`Trace`]: the shape it stands for, and what it said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The name of the shape, which is the method of [`Visitor`] that reported
    /// it.
    pub kind: &'static str,
    /// The data of the shape and the places it was read from, on one line.
    pub text: String,
}

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.kind, self.text)
    }
}

/// A [`Visitor`] that records every shape the walk reaches, in that order.
///
/// It answers for all of them, so a record states the whole shape of a tree and
/// not the part one pass was interested in.
#[derive(Debug, Default)]
pub struct Trace {
    lines: Vec<Line>,
}

impl Trace {
    /// The record of one walk over `node`.
    pub fn record(node: &dyn Visit) -> Self {
        let mut trace = Trace::default();

        node.visit(&mut trace);

        trace
    }

    /// The lines of the record, in the order the walk reached them.
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// The shapes of the record, in the order the walk reached them.
    ///
    /// A test that asks only which shapes a file yields, and not what each of
    /// them held, reads better against the shapes than the full record does.
    pub fn shapes(&self) -> Vec<&'static str> {
        self.lines.iter().map(|line| line.kind).collect()
    }

    /// The record, one line per shape.
    pub fn to_text(&self) -> String {
        let mut text = String::new();

        for line in &self.lines {
            writeln!(text, "{line}").expect("writing to a String cannot fail");
        }

        text
    }

    /// One line of the record.
    fn push(&mut self, kind: &'static str, text: impl Into<String>) {
        self.lines.push(Line {
            kind,
            text: text.into(),
        });
    }

    /// The children a shape holds, as the places they were read from and the
    /// separators that divide them.
    fn children(children: &[Element<'_>]) -> String {
        let mut text = String::from("(");

        for (index, child) in children.iter().enumerate() {
            if index > 0 {
                text.push_str(", ");
            }

            match child.to_span() {
                Some(span) => text.push_str(&at(&span)),
                // The parent wrote a place for a child and the file left it
                // empty: an `a,` in a list, a `b?` with nothing to repeat.
                None => text.push_str("absent"),
            }

            if let Some(separator) = &child.separator {
                let _ = write!(text, " sep {}", at(separator));
            }
        }

        text.push(')');
        text
    }
}

impl Visitor for Trace {
    fn production(&mut self, production: &mut Production<'_>) {
        self.push(
            "production",
            format!(
                "span={} children={}",
                at(&production.span),
                Trace::children(&production.children)
            ),
        );
    }

    fn declaration(&mut self, declaration: &mut Declaration<'_>) {
        self.push(
            "declaration",
            format!(
                "{} {} span={} body={}",
                match declaration.kind {
                    DefKind::Whitespace => "whitespace",
                    DefKind::Lexer => "lexer",
                    DefKind::Syntax => "syntax",
                },
                name_text(&declaration.name),
                at(&declaration.span),
                Trace::children(&declaration.children)
            ),
        );
    }

    fn comment(&mut self, comment: &Comment) {
        self.push(
            "comment",
            format!(
                "{} {} {} {:?}",
                if comment.inner { "inner" } else { "outer" },
                if comment.block { "block" } else { "line" },
                at(&comment.span),
                comment.text
            ),
        );
    }

    fn alternatives(&mut self, alternatives: &mut Alternatives<'_>) {
        self.push(
            "alternatives",
            format!(
                "span={} children={}",
                at(&alternatives.span),
                Trace::children(&alternatives.children)
            ),
        );
    }

    fn sequence(&mut self, sequence: &mut Sequence<'_>) {
        self.push(
            "sequence",
            format!(
                "span={} children={}",
                at(&sequence.span),
                Trace::children(&sequence.children)
            ),
        );
    }

    fn quantity(&mut self, quantity: &mut Quantity<'_>) {
        self.push(
            "quantity",
            format!(
                "{} operator={} target={} {}",
                match quantity.op {
                    QuantityOp::ZeroOrMore => "*",
                    QuantityOp::ZeroOrOne => "?",
                    QuantityOp::OneOrMore => "+",
                    QuantityOp::Bounded => "{{}}",
                },
                at(&quantity.operator),
                child_span(&quantity.target),
                bounds(quantity.bounds.as_ref()),
            ),
        );
    }

    fn followed(&mut self, followed: &mut Followed<'_>) {
        self.push(
            "followed",
            format!(
                "span={} target={} operand={}",
                at(&followed.span),
                child_span(&followed.target),
                child_span(&followed.operand)
            ),
        );
    }

    fn excepted(&mut self, excepted: &mut Excepted<'_>) {
        self.push(
            "excepted",
            format!(
                "span={} target={} exclusion={}",
                at(&excepted.span),
                child_span(&excepted.target),
                child_span(&excepted.exclusion)
            ),
        );
    }

    fn group(&mut self, group: &mut Group<'_>) {
        self.push(
            "group",
            format!(
                "open={} close={} children={}",
                at(&group.open),
                at(&group.close),
                Trace::children(&group.children)
            ),
        );
    }

    fn set(&mut self, set: &Set) {
        let tilde = match &set.tilde {
            Some(tilde) => format!(" tilde={}", at(tilde)),
            None => String::new(),
        };
        let close = match &set.close {
            Some(close) => at(close),
            // A set left unclosed is an error the file already reported, and the
            // record still says what the set holds.
            None => "-".to_owned(),
        };

        self.push(
            "set",
            format!(
                "{} open={} close={}{tilde} members=({})",
                match set.kind {
                    SetKind::Union => "union",
                    SetKind::Complement => "complement",
                },
                at(&set.open),
                close,
                set.members
                    .iter()
                    .map(member_text)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }

    fn reference(&mut self, reference: &Reference) {
        self.push("reference", name_text(&reference.name));
    }

    fn call(&mut self, call: &Call) {
        self.push("call", name_text(&call.name));
    }

    fn literal(&mut self, literal: &Literal) {
        match &literal.value {
            LiteralValue::Str(text) => self.push("literal", format!("str {}", value(text))),
            LiteralValue::Char(code) => self.push("literal", format!("char {}", code_point(code))),
        }
    }

    fn token(&mut self, token: &Token) {
        self.push("token", format!("{:?} {}", token.text, at(&token.span)));
    }
}

/// A place, written where it stands in the line.
fn at(span: &impl std::fmt::Debug) -> String {
    format!("{span:?}")
}

/// The place a child occupies, which is where it was read from or, when the
/// parent left the place empty, the word `absent`.
fn child_span(child: &Element<'_>) -> String {
    match child.to_span() {
        Some(span) => at(&span),
        None => "absent".to_owned(),
    }
}

/// How many times a repetition lets its operand match.
fn bounds(bounds: Option<&Bounds>) -> String {
    match bounds {
        None => String::new(),
        Some(bounds) => format!(
            "bounds {} {}",
            match &bounds.form {
                BoundsForm::Exactly(count) => format!("exactly {}", count_of(count)),
                BoundsForm::AtMost(count) => format!("at most {}", count_of(count)),
                BoundsForm::AtLeast { from, to } => format!(
                    "at least {}{}",
                    count_of(from),
                    match to {
                        Some(to) => format!(" at most {}", count_of(to)),
                        None => String::new(),
                    }
                ),
            },
            at(&bounds.span)
        ),
    }
}

fn count_of(count: &Count) -> String {
    format!("{:?}={:?} {}", count.digits, count.value(), at(&count.span))
}

fn code_point(code: &CodePoint) -> String {
    format!("U+{:?}={:?} {}", code.digits, code.value(), at(&code.span))
}

fn name_text(name: &Name) -> String {
    format!("{:?} {}", name.text, at(&name.span))
}

/// The string a literal denotes, with the place of the first escape that denotes
/// nothing.
fn value(text: &TextLiteral) -> String {
    let (value, invalid) = text.value();

    match invalid {
        Some(span) => format!(
            "{:?} {} pieces={} invalid={}",
            value,
            at(&text.span),
            text.pieces.len(),
            at(&span)
        ),
        None => format!(
            "{:?} {} pieces={}",
            value,
            at(&text.span),
            text.pieces.len()
        ),
    }
}

/// One member of a set, as the value it says and the place it was written.
fn member_text(member: &SetMember) -> String {
    match member {
        SetMember::Char(code) => format!("char {}", code_point(code)),
        SetMember::Str(text) => format!("str {}", value(text)),
        SetMember::Range(range) => range_text(range),
        SetMember::Reference(name) => format!("ref {}", name_text(name)),
    }
}

fn range_text(range: &CharRange) -> String {
    let side = |side: &RangeSide| match side {
        RangeSide::Char(code) => code_point(code),
        RangeSide::Str(text) => value(text),
    };

    format!(
        "range {} - {} dash={} {}",
        side(&range.start),
        side(&range.end),
        at(&range.dash),
        at(&range.span)
    )
}

#[cfg(test)]
mod tests {
    use parserc::syntax::SyntaxInput;

    use crate::{
        input::TokenStream,
        syntax::{Crate, Expr, Item},
    };

    use super::*;

    /// Records the shapes one body expression decomposes into.
    fn shapes(body: &str) -> Vec<&'static str> {
        let expr = TokenStream::from(body)
            .parse::<Expr<_>>()
            .expect("the test body should parse");

        Trace::record(&expr).shapes()
    }

    /// Records the whole of a body, spans and values included.
    fn record(body: &str) -> String {
        let expr = TokenStream::from(body)
            .parse::<Expr<_>>()
            .expect("the test body should parse");

        Trace::record(&expr).to_text()
    }

    #[test]
    fn a_walk_reaches_every_shape_a_body_holds() {
        // One of each shape, in the order the walk reaches them: the shape of a
        // body first, then the shapes of its operands from the first to the last.
        assert_eq!(
            shapes(
                r"['a', 'b'-'z', U+0041, A] | ~['x'] | <EXT> | ('a' | 'b')* | 'a'? | 'c'{1,2}
                   | 'd' except 'e' | 'f' followed 'g' | A | 'a' 'b'"
            ),
            [
                "alternatives",
                "sequence",
                // The members of a set are data of the set, so no shape below
                // one is reached: a set says all of its members at once.
                "set",
                "sequence",
                "set",
                "sequence",
                "call",
                "sequence",
                "quantity",
                "group",
                "alternatives",
                "sequence",
                "literal",
                "sequence",
                "literal",
                "sequence",
                "quantity",
                "literal",
                "sequence",
                "quantity",
                "literal",
                "sequence",
                "excepted",
                "literal",
                "literal",
                "sequence",
                "followed",
                "literal",
                "literal",
                "sequence",
                "reference",
                "sequence",
                "literal",
                "literal",
            ]
        );
    }

    #[test]
    fn a_body_records_what_it_said_and_where() {
        // The record is the whole contract a second tree has to meet: the shape,
        // the value it denotes, and the place it was read from.
        assert_eq!(
            record("U+0041 U+0042"),
            [
                "alternatives span=Range(0..13) children=(Range(0..13))",
                "sequence span=Range(0..13) children=(Range(0..6), Range(7..13) sep Range(6..7))",
                "literal char U+\"0041\"=Some('A') Range(0..6)",
                "literal char U+\"0042\"=Some('B') Range(7..13)",
            ]
            .join("\n")
                + "\n"
        );
    }

    #[test]
    fn a_broken_piece_is_recorded_where_it_stands() {
        // The replacement character is in the value, and the place of the escape
        // that failed is recorded beside it, so a record of two trees says the
        // same thing about the same broken file.
        let broken = char::REPLACEMENT_CHARACTER;

        assert_eq!(
            record(r"'A\u{D800}B'"),
            format!(
                "alternatives span=Range(0..12) children=(Range(0..12))\n\
                 sequence span=Range(0..12) children=(Range(0..12))\n\
                 literal str \"A{broken}B\" Range(0..12) pieces=3 invalid=Range(2..10)\n"
            )
        );
    }

    #[test]
    fn a_set_records_the_values_its_members_denote() {
        // Three spellings of one character, and the range that is one too, are
        // four members that say one thing: the record keeps all four, because
        // which of them to drop is the lint's business and not the walk's.
        assert_eq!(
            record("['a', U+0061, '\\u{61}', 'a'-'a']"),
            [
                "alternatives span=Range(0..32) children=(Range(0..32))",
                "sequence span=Range(0..32) children=(Range(0..32))",
                "set union open=Range(0..1) close=Range(31..32) members=(\
                 str \"a\" Range(1..4) pieces=1, \
                 char U+\"0061\"=Some('a') Range(6..12), \
                 str \"a\" Range(14..22) pieces=1, \
                 range \"a\" Range(24..27) pieces=1 - \"a\" Range(28..31) pieces=1 \
                 dash=Range(27..28) Range(24..31))",
            ]
            .join("\n")
                + "\n"
        );
    }

    #[test]
    fn a_file_records_its_definitions_and_the_documents_above_them() {
        // A file is a production whose items are documents, statements, and the
        // whitespace between them, so the record reaches a `token` for the lone
        // space that divides the inner document from the first statement. The
        // newline after the first `;` is not such an item: the `;` owns it, and
        // an owned place is not walked.
        let file = TokenStream::from(
            r"//! The inner document of a file.
lexer A -> 'a';
whitespace WS -> [' '];
",
        )
        .parse::<Crate<_>>()
        .expect("the test file should parse");

        assert_eq!(
            Trace::record(&file).shapes(),
            [
                "production",
                "comment",
                "token",
                "declaration",
                "alternatives",
                "sequence",
                "literal",
                "declaration",
                "alternatives",
                "sequence",
                "set",
            ]
        );
    }

    #[test]
    fn the_walk_of_a_real_grammar_reaches_every_definition_it_writes() {
        // `bootstrap.uns` describes this language in itself, so its tree is the
        // file a second tree must reproduce. A walk that stopped early, or that
        // never reached a definition, would miss part of what the equivalence is
        // about, so the record is checked against the items of the tree itself.
        let file = TokenStream::from(include_str!("../../bootstrap.uns"))
            .parse::<Crate<_>>()
            .expect("the spec of the language should parse");

        let trace = Trace::record(&file);

        let statements = trace
            .lines()
            .iter()
            .filter(|line| line.kind == "declaration")
            .count();

        let definitions = file
            .items
            .iter()
            .filter(|item| matches!(item, Item::Stmt(_)))
            .count();

        assert_eq!(statements, definitions, "the definitions the spec writes");

        // Thirteen of the fourteen shapes are reached by the spec's own rules,
        // which is what makes the record enough to compare two trees by. The
        // one exception is `followed`: the spec names the keyword in a set and
        // in a rule body, but never uses it between two operands, so no
        // followed expression exists to walk. `except` does survive the trip,
        // because `NonKeywordIdent` subtracts the keywords from an identifier.
        // The shapes the spec leaves out of its own text are the ones a
        // hand-made body reaches, as `a_walk_reaches_every_shape_a_body_holds`
        // shows.
        let mut reached = trace.shapes();
        reached.sort_unstable();
        reached.dedup();

        assert_eq!(
            reached,
            [
                "alternatives",
                "call",
                "comment",
                "declaration",
                "excepted",
                "group",
                "literal",
                "production",
                "quantity",
                "reference",
                "sequence",
                "set",
                "token",
            ]
        );
    }
}
