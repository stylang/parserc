//! How the hand-written tree of [`crate::syntax`] and [`crate::lexical`]
//! decomposes into the [`vocab`](super::vocab) shapes.
//!
//! This is the one place in the crate that knows both sides: the vocabulary on
//! one, the node types of one particular tree on the other. A second tree — the
//! one this crate will emit when it parses `bootstrap.uns` with a parser it
//! generated from it — adds its own module here and reuses every analysis that
//! speaks shapes. So the rule for what belongs in this file is narrow: read a
//! node's own fields, and nothing else. A shape that needed a grandchild to be
//! built is a shape the walk cannot hand over, and the pass reaches the
//! grandchild by walking instead.

use parserc::{
    Span,
    syntax::{Punctuated, Syntax},
};

use crate::{
    input::UnsynInput,
    lexical::{
        S,
        delimiter::{Angle, Bracket},
        ident::Ident,
        lit::{AsciiEscape, LitDec, LitStr, LitUnicode, StrSegment},
        punct::Comma,
    },
    syntax::{
        Crate, Expr, ExprNoTopAlt, ExprNoTopAlts, ExprWithSuffix, ExprWithoutSuffix, InnerDoc,
        Item, LeadingKey, OuterDoc, Range, Repeat, RepeatBound, SetItem, Stmt, SuffixOp,
    },
};

use super::{
    Alternatives, AsciiForm, Bounds, BoundsForm, Call, CharRange, CodePoint, Comment, Count,
    Declaration, DefKind, Element, Excepted, Followed as FollowedShape, Group, Literal,
    LiteralValue, Name, Piece, Production, Quantity, QuantityOp, RangeSide, Reference, Sequence,
    Set, SetKind, SetMember, TextLiteral, Token, Visit, Visitor, descend,
};

/// The place a node occupies, as the tree that holds it spells it.
///
/// Every node of this tree is a [`Syntax`], so every [`Visit`] impl writes
/// exactly this line for [`Visit::to_span`]; naming it here keeps the call from
/// reading like the method it implements.
#[inline]
fn span_of<I, N>(node: &N) -> Span
where
    I: UnsynInput,
    N: Syntax<I>,
{
    node.to_span()
}

impl<I: UnsynInput> Visit for Crate<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        let mut shape = Production {
            span: span_of(self),
            // The inner documents come first because a file writes them first;
            // an outer document is an item of its own, and belongs to the
            // definition that follows it.
            children: self
                .inner_docs
                .iter()
                .map(|doc| Element::new(doc))
                .chain(self.items.iter().map(|item| Element::new(item)))
                .collect(),
        };

        v.production(&mut shape);
        descend(v, &mut shape);
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

impl<I: UnsynInput> Visit for Item<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        match self {
            Item::S(s) => s.visit(v),
            Item::Stmt(stmt) => stmt.visit(v),
            Item::OuterDoc(doc) => doc.visit(v),
        }
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

impl<I: UnsynInput> Visit for S<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        v.token(&Token {
            text: self.0.as_str().to_owned(),
            span: span_of(self),
        });
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

impl<I: UnsynInput> Visit for InnerDoc<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        match self {
            InnerDoc::Block(_, doc) => v.comment(&comment(&doc.0, span_of(doc), true, true)),
            InnerDoc::Line(_, doc) => v.comment(&comment(&doc.0, span_of(doc), true, false)),
        }
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

impl<I: UnsynInput> Visit for OuterDoc<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        match self {
            OuterDoc::Block(_, doc) => v.comment(&comment(&doc.0, span_of(doc), false, true)),
            OuterDoc::Line(_, doc) => v.comment(&comment(&doc.0, span_of(doc), false, false)),
        }
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

/// The text and place of a document comment.
///
/// A document owns only its own text: the run of trivia before it is held by
/// the [`InnerDoc`] or [`OuterDoc`] that wraps it, and is not part of what the
/// document says.
fn comment<I: UnsynInput>(text: &I, span: Span, inner: bool, block: bool) -> Comment {
    Comment {
        text: text.as_str().to_owned(),
        block,
        inner,
        span,
    }
}

impl<I: UnsynInput> Visit for Ident<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        v.token(&Token {
            text: self.0.as_str().to_owned(),
            span: span_of(self),
        });
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

impl<I: UnsynInput> Visit for Stmt<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        let mut shape = Declaration {
            span: span_of(self),
            name: name(&self.ident),
            kind: match self.keyword {
                LeadingKey::Whitespace(_) => DefKind::Whitespace,
                LeadingKey::Lexer(_) => DefKind::Lexer,
                LeadingKey::Syntax(_) => DefKind::Syntax,
            },
            // The `->` and the `;` are punctuation of the statement rather than
            // anything a pass reads, so only the body is handed over.
            children: vec![Element::new(&self.definition.body)],
        };

        v.declaration(&mut shape);
        descend(v, &mut shape);
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

impl<I: UnsynInput> Visit for Expr<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        let mut shape = Alternatives {
            span: span_of(self),
            children: std::iter::once(Element::new(&self.first))
                .chain(self.rest.iter().map(|(bar, alts)| Element {
                    node: Some(alts),
                    separator: Some(span_of(bar)),
                }))
                .collect(),
        };

        v.alternatives(&mut shape);
        descend(v, &mut shape);
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

impl<I: UnsynInput> Visit for ExprNoTopAlts<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        let mut shape = Sequence {
            span: span_of(self),
            children: std::iter::once(Element::new(&self.first))
                .chain(self.rest.iter().map(|(s, expr)| Element {
                    node: Some(expr),
                    separator: s.as_ref().map(span_of),
                }))
                .collect(),
        };

        v.sequence(&mut shape);
        descend(v, &mut shape);
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

impl<I: UnsynInput> Visit for ExprNoTopAlt<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        match self {
            ExprNoTopAlt::WithSuffix(expr) => expr.visit(v),
            ExprNoTopAlt::WithoutSuffix(expr) => expr.visit(v),
        }
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

impl<I: UnsynInput> Visit for ExprWithSuffix<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        match self {
            // One of the four operators, applied to one operand.
            ExprWithSuffix::Grouped { target, suffix } => {
                let (op, bounds, operator) = match SuffixOp::of(suffix) {
                    SuffixOp::Star(op) => (QuantityOp::ZeroOrMore, None, span_of(op)),
                    SuffixOp::Question(op) => (QuantityOp::ZeroOrOne, None, span_of(op)),
                    SuffixOp::Plus(op) => (QuantityOp::OneOrMore, None, span_of(op)),
                    SuffixOp::Repeat(op) => {
                        (QuantityOp::Bounded, Some(repeat_bounds(op)), span_of(op))
                    }
                };

                let mut shape = Quantity {
                    span: span_of(self),
                    op,
                    bounds,
                    target: Element::new(target),
                    operator,
                };

                v.quantity(&mut shape);
                descend(v, &mut shape);
            }
            // `concat` joins two operands into one token, which is a sequence of
            // the two with the keyword between them.
            ExprWithSuffix::Concat {
                target,
                keyword,
                suffix,
                ..
            } => {
                let mut shape = Sequence {
                    span: span_of(self),
                    children: vec![
                        Element::new(target),
                        Element {
                            node: Some(&**suffix),
                            separator: Some(span_of(keyword)),
                        },
                    ],
                };

                v.sequence(&mut shape);
                descend(v, &mut shape);
            }
            // `followed` and `except` each hold two operands in two different
            // offices, which is why they are shapes of their own rather than
            // sequences.
            ExprWithSuffix::Followed { target, suffix, .. } => {
                let mut shape = FollowedShape {
                    span: span_of(self),
                    target: Element::new(target),
                    operand: Element::new(&**suffix),
                };

                v.followed(&mut shape);
                descend(v, &mut shape);
            }
            ExprWithSuffix::Except { target, tokens, .. } => {
                let mut shape = Excepted {
                    span: span_of(self),
                    target: Element::new(target),
                    exclusion: Element::new(tokens),
                };

                v.excepted(&mut shape);
                descend(v, &mut shape);
            }
        }
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

impl<I: UnsynInput> Visit for ExprWithoutSuffix<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        match self {
            ExprWithoutSuffix::Tilde(tilde, bracket) => {
                v.set(&set(bracket, Some(span_of(tilde)), SetKind::Complement));
            }
            ExprWithoutSuffix::Set(bracket) => v.set(&set(bracket, None, SetKind::Union)),
            ExprWithoutSuffix::Call(call) => v.call(&call_of(call)),
            ExprWithoutSuffix::Paren(group) => {
                let mut shape = Group {
                    span: span_of(group),
                    open: span_of(&group.start),
                    close: Some(span_of(&group.end)),
                    children: vec![Element::new(&*group.body)],
                };

                v.group(&mut shape);
                descend(v, &mut shape);
            }
            ExprWithoutSuffix::Str(lit) => v.literal(&Literal {
                span: span_of(lit),
                value: LiteralValue::Str(text_literal(lit)),
            }),
            ExprWithoutSuffix::Unicode(lit) => v.literal(&Literal {
                span: span_of(lit),
                value: LiteralValue::Char(code_point_literal(lit)),
            }),
            ExprWithoutSuffix::Ident(ident) => v.reference(&Reference { name: name(ident) }),
        }
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

impl<I: UnsynInput> Visit for LitStr<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        v.literal(&Literal {
            span: span_of(self),
            value: LiteralValue::Str(text_literal(self)),
        });
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

impl<I: UnsynInput> Visit for LitUnicode<I> {
    fn visit(&self, v: &mut dyn Visitor) {
        v.literal(&Literal {
            span: span_of(self),
            value: LiteralValue::Char(code_point_literal(self)),
        });
    }

    fn to_span(&self) -> Span {
        span_of(self)
    }
}

/// The name an identifier holds, as written.
fn name<I: UnsynInput>(ident: &Ident<I>) -> Name {
    Name {
        text: ident.0.as_str().to_owned(),
        span: span_of(ident),
    }
}

/// `[a, b, c]`, with the office of the brackets and the members between them.
fn set<I: UnsynInput>(
    bracket: &Bracket<I, Punctuated<SetItem<I>, Comma<I>>>,
    tilde: Option<Span>,
    kind: SetKind,
) -> Set {
    Set {
        span: span_of(bracket),
        open: span_of(&bracket.start),
        close: Some(span_of(&bracket.end)),
        tilde,
        kind,
        members: bracket
            .body
            .pairs
            .iter()
            .map(|(item, _)| item)
            .chain(bracket.body.tail.iter().map(|tail| &**tail))
            .map(member)
            .collect(),
    }
}

/// One member of a set, as the value it says.
fn member<I: UnsynInput>(item: &SetItem<I>) -> SetMember {
    match item {
        SetItem::Range(range) => SetMember::Range(range_of(range)),
        SetItem::Str(lit) => SetMember::Str(text_literal(lit)),
        SetItem::Unicode(lit) => SetMember::Char(code_point_literal(lit)),
        SetItem::Ident(ident) => SetMember::Reference(name(ident)),
    }
}

/// `a-b`, either of the two ways the language writes a range.
fn range_of<I: UnsynInput>(range: &Range<I>) -> CharRange {
    let (start, dash, end) = match range {
        Range::Unicode(start, dash, end) => (
            RangeSide::Char(code_point_literal(start)),
            dash,
            RangeSide::Char(code_point_literal(end)),
        ),
        Range::Str(start, dash, end) => (
            RangeSide::Str(text_literal(start)),
            dash,
            RangeSide::Str(text_literal(end)),
        ),
    };

    CharRange {
        span: span_of(range),
        dash: span_of(dash),
        start,
        end,
    }
}

/// `<NAME>`, the call of an algorithm the caller of the generated parser links.
fn call_of<I: UnsynInput>(call: &Angle<I, Ident<I>>) -> Call {
    Call {
        name: name(&call.body),
    }
}

/// `'…'`, the content of a string literal read as the pieces it was written in.
///
/// The three readers of a literal are also open to the passes that hold one by
/// name rather than by walking to it, which is the same reading the walk hands
/// to a [`Literal`] shape.
pub(crate) fn text_literal<I: UnsynInput>(lit: &LitStr<I>) -> TextLiteral {
    TextLiteral {
        span: span_of(lit),
        pieces: lit.content.iter().map(pieces).collect(),
    }
}

fn pieces<I: UnsynInput>(segment: &StrSegment<I>) -> Piece {
    match segment {
        StrSegment::QuoteEscape(quote) => Piece::Quote(span_of(quote)),
        StrSegment::AsciiEscape(escape) => Piece::Ascii(ascii(escape)),
        StrSegment::UnicodeEscape(escape) => Piece::Unicode {
            digits: escape.digits.as_str().to_owned(),
            span: span_of(escape),
        },
        StrSegment::CharWithException(text) => Piece::Text(text.as_str().to_owned()),
    }
}

/// One of the ASCII escapes. `\xNN` keeps its two digits, which the lexer has
/// already checked are hex; the rest denote their character by what they are.
fn ascii<I: UnsynInput>(escape: &AsciiEscape<I>) -> AsciiForm {
    match escape {
        AsciiEscape::LF(_) => AsciiForm::Newline,
        AsciiEscape::CR(_) => AsciiForm::CarriageReturn,
        AsciiEscape::Tab(_) => AsciiForm::Tab,
        AsciiEscape::BlackSlash(_) => AsciiForm::Backslash,
        AsciiEscape::Null(_) => AsciiForm::Null,
        AsciiEscape::Char(code) => {
            let text = code.as_str();

            // The lexical pass matched `\x` and then exactly two hex digits, so
            // the last two characters are the number and nothing else is.
            AsciiForm::Hex {
                digits: text.get(text.len() - 2..).unwrap_or(text).to_owned(),
                span: code.to_span(),
            }
        }
    }
}

/// `U+…`, the code point the literal names.
pub(crate) fn code_point_literal<I: UnsynInput>(lit: &LitUnicode<I>) -> CodePoint {
    CodePoint {
        digits: lit
            .0
            .as_str()
            .strip_prefix("U+")
            .unwrap_or(lit.0.as_str())
            .to_owned(),
        span: span_of(lit),
    }
}

/// `{…}`, how many times the operand of a [`Quantity`] may match.
pub(crate) fn repeat_bounds<I: UnsynInput>(repeat: &Repeat<I>) -> Bounds {
    Bounds {
        span: span_of(repeat),
        form: match RepeatBound::of(&repeat.body) {
            RepeatBound::RangeTo(_, end) => BoundsForm::AtMost(count(end)),
            RepeatBound::RangeFrom(start, _, end) => BoundsForm::AtLeast {
                from: count(start),
                to: end.map(count),
            },
            RepeatBound::Count(lit) => BoundsForm::Exactly(count(lit)),
        },
    }
}

/// A decimal literal, as the number it may hold.
pub(crate) fn count<I: UnsynInput>(lit: &LitDec<I>) -> Count {
    Count {
        digits: lit.0.as_str().to_owned(),
        span: span_of(lit),
    }
}
