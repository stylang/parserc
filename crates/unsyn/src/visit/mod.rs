//! Reading a CST without naming its node types.
//!
//! The analyses of this crate — the [checks](crate::semantics) of a
//! definition's body, the [recursion](crate::semantics::left_recursive) of the
//! graph its definitions draw, the [codegen](crate::codegen) that turns a definition into
//! Rust — are all about the language `uns` describes, and a tree is only one way
//! of writing that language down. Today the tree is the hand-written types of
//! [`crate::syntax`]; when this crate has generated its own parser from the
//! grammar `bootstrap.uns` describes, the tree will be whatever that generator
//! emitted, and the two trees will share nothing but the grammar they
//! both describe.
//!
//! This module is what they can share. A node reports itself to a
//! [`Visitor`][trait@Visitor] as the *shape* it has, told in a vocabulary of
//! spans, text, operator names and children as opaque handles — and no node
//! type. The pass answers for the shapes it cares about and lets the walk carry
//! it down to the rest. Because the vocabulary is the same for both trees, one
//! implementation of a pass serves both, and the only
//! thing a second tree has to supply is the mapping from its nodes to the shapes
//! they are, which is what the private `decompose` is for the hand-written one.
//!
//! ## Why the node reports itself
//!
//! The alternative is a pass that matches on node types and descends with its
//! own recursion, which is what every pass here did before this module. That
//! works for one tree and cannot move. Here the direction is reversed: the node
//! knows its own shape and the pass knows what to do with a shape, so neither
//! knows the other's types, and the walk that joins them is this module's
//! [`descend`], which is written once.

mod decompose;
mod trace;
mod vocab;

/// Reads the bounds of a repetition of this tree as the vocabulary spells them.
///
/// The recursion analysis asks only this one question of a node it has already
/// reached by name, and the answer is the same number the walk would report.
pub(crate) use decompose::{code_point_literal, count, repeat_bounds, text_literal};
pub use trace::{Line, Trace};

pub use vocab::{
    Alternatives, AsciiForm, Bounds, BoundsForm, Call, CharRange, CodePoint, Comment, Count,
    Declaration, DefKind, Element, Excepted, Followed, Group, Literal, LiteralValue, Name, Piece,
    Production, Quantity, QuantityOp, RangeSide, Reference, Sequence, Set, SetKind, SetMember,
    Shape, TextLiteral, Token,
};

use parserc::Span;

/// A node that can report its own shape to a [`Visitor`].
///
/// Implemented one node at a time, and only for the nodes a parent can hand
/// over as a child: the members of a [`Set`] and the bounds of a [`Quantity`]
/// are values rather than places a pass can walk into, so they travel as the
/// data of the shape that holds them.
pub trait Visit {
    /// Reports this node, one level deep, to `v`, and then — unless `v` has
    /// taken the children away — descends into them.
    fn visit(&self, v: &mut dyn Visitor);

    /// The place this node occupies in the file.
    ///
    /// A pass gets this without dispatching, which is what lets it report on a
    /// child it was handed rather than only on the node it is standing in.
    fn to_span(&self) -> Span;
}

/// What a pass does with the shape a node reports.
///
/// Every method answers for one shape and does nothing by default, so a pass
/// implements the shapes it asks about and no others; the walk still reaches
/// everything, because it is driven by the nodes rather than by the methods the
/// pass chose. A method takes its shape by `&mut` where the shape holds
/// children, which is how a pass that must not see part of a tree clears the
/// children it is not interested in before the walk reaches them.
pub trait Visitor {
    /// A whole source file: its documents and its definitions.
    fn production(&mut self, _: &mut Production<'_>) {}

    /// One definition of a file, `lexer TEXT -> …;`.
    fn declaration(&mut self, _: &mut Declaration<'_>) {}

    /// A document comment.
    fn comment(&mut self, _: &Comment) {}

    /// The `|` of a rule.
    fn alternatives(&mut self, _: &mut Alternatives<'_>) {}

    /// Operands written one after another.
    fn sequence(&mut self, _: &mut Sequence<'_>) {}

    /// An operand with `*`, `?`, `+` or `{…}` on it.
    fn quantity(&mut self, _: &mut Quantity<'_>) {}

    /// `a followed b`.
    fn followed(&mut self, _: &mut Followed<'_>) {}

    /// `a except b`.
    fn excepted(&mut self, _: &mut Excepted<'_>) {}

    /// `( … )`.
    fn group(&mut self, _: &mut Group<'_>) {}

    /// `[ … ]`, or `~[ … ]`.
    fn set(&mut self, _: &Set) {}

    /// The name of another definition, standing as an operand of its own.
    fn reference(&mut self, _: &Reference) {}

    /// `< … >`, a call of an algorithm the caller provides.
    fn call(&mut self, _: &Call) {}

    /// `'…'` or `U+…`, standing as an operand of its own.
    fn literal(&mut self, _: &Literal) {}

    /// A punct, a keyword, an identifier or a run of whitespace.
    fn token(&mut self, _: &Token) {}
}

/// Walks into whatever children `shape` still holds.
///
/// The private `decompose` calls this after the pass has answered for the
/// shape, so the
/// pass has had its chance to keep, replace or clear the children.
pub fn descend<'a, S: Shape<'a>>(v: &mut dyn Visitor, shape: &mut S) {
    for child in shape.take_children() {
        if let Some(node) = child.node {
            node.visit(v);
        }
    }
}

/// The name a node refers to, where the node is nothing but that reference.
///
/// `A` is a bare reference; `A*`, `[A]` and `(A)` are not. A pass that has to
/// tell the two apart while it stands *above* a node rather than inside it —
/// which operands of a sequence were only separators, for one — asks this of the
/// handle, and the answer comes from the shape the node reports, so it is the
/// same answer for every tree.
pub fn bare_reference(node: &dyn Visit) -> Option<Name> {
    #[derive(Default)]
    struct Peek {
        /// The one name the node reported, if it reported exactly one.
        name: Option<Name>,
        /// Whether the node reported anything that is not a bare reference.
        other: bool,
    }

    impl Visitor for Peek {
        fn reference(&mut self, reference: &Reference) {
            if self.name.is_some() {
                self.other = true;
            } else {
                self.name = Some(reference.name.clone());
            }
        }

        // Anything else the node may report is a structure of its own, which a
        // bare reference by definition is not.
        fn production(&mut self, _: &mut Production<'_>) {
            self.other = true;
        }
        fn declaration(&mut self, _: &mut Declaration<'_>) {
            self.other = true;
        }
        fn comment(&mut self, _: &Comment) {
            self.other = true;
        }
        fn alternatives(&mut self, _: &mut Alternatives<'_>) {
            self.other = true;
        }
        fn sequence(&mut self, _: &mut Sequence<'_>) {
            self.other = true;
        }
        fn quantity(&mut self, _: &mut Quantity<'_>) {
            self.other = true;
        }
        fn followed(&mut self, _: &mut Followed<'_>) {
            self.other = true;
        }
        fn excepted(&mut self, _: &mut Excepted<'_>) {
            self.other = true;
        }
        fn group(&mut self, _: &mut Group<'_>) {
            self.other = true;
        }
        fn set(&mut self, _: &Set) {
            self.other = true;
        }
        fn call(&mut self, _: &Call) {
            self.other = true;
        }
        fn literal(&mut self, _: &Literal) {
            self.other = true;
        }
        fn token(&mut self, _: &Token) {
            self.other = true;
        }
    }

    let mut peek = Peek::default();

    node.visit(&mut peek);

    if peek.other { None } else { peek.name }
}
