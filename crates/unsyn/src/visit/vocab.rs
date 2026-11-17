//! The vocabulary a [`Visitor`](trait@super::Visitor) speaks, which is the whole
//! point of this module: nothing in it is a node of any tree.
//!
//! A pass that reads a CST through Rust types is tied to the CST it reads. The
//! hand-written nodes of `crate::syntax` and a CST this crate generates from
//! `bootstrap.uns` are two different sets of types for one language, so the
//! analyses would have to be written twice — which is what this vocabulary
//! prevents. Every type here is made of the things both trees can say about
//! themselves: a [`Span`], the text a token holds, the operator a
//! group picked, and the children of a node as opaque handles.
//!
//! ## What a shape is
//!
//! Each struct is one *shape* a node can have, and says what the pass sees
//! without descending: a [`Set`] lists its members, a [`Quantity`] names its
//! operator and holds the operand it applies to, a [`Sequence`] holds its
//! operands in the order they were written. The leaves of the language are
//! shapes too, and carry no children at all: [`Reference`], [`Call`],
//! [`Literal`], [`Token`], [`Comment`].
//!
//! ## Children are handles, not types
//!
//! A child is an [`Element`]: the node itself as a `&dyn Visit`, plus the
//! [span](parserc::Span) of the separator that was written before it, if the
//! parent's shape has separators. A pass descends by handing the handle back
//! to [`Visitor`][super::Visitor], which dispatches it to the method for *its*
//! shape; what that method receives is again vocabulary, never a node.
//!
//! ## Values are text, and decoding is shared
//!
//! A shape never hands over a value it has already decided on. `'a'` is a
//! [`Piece::Text`], `'\u{D800}'` is a [`Piece::Unicode`] whose digits denote no
//! character, `'2'-'1'` is a [`CharRange`] whose bounds stand in the wrong
//! order. Decoding is a function of the text rather than of the tree, so it
//! lives beside these types ([`TextLiteral::value`], [`CodePoint::value`],
//! [`Count::value`]) and one tree's analysis is the next tree's analysis; only
//! the mapping from a node to the text it holds is per-tree, and that is the
//! private `decompose`.

use parserc::Span;

/// Which of the three kinds of definition a [`Declaration`] is.
///
/// The kind is the keyword that opened the statement, and it is the only thing
/// that tells a token from a node from the trivia of the language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DefKind {
    /// `whitespace`: the trivia a pattern may skip over. A file declares one.
    Whitespace,
    /// `lexer`: a token, matched against the source text.
    Lexer,
    /// `syntax`: a node, built from the tokens it refers to.
    Syntax,
}

/// One child of a node, as the parent's shape hands it over.
///
/// The node is `None` where the parent wrote a place for a child that is not
/// there: the tail of a `[a, b,]` list, an `a?` group with nothing after the
/// `?`. A childless place is still a child of the shape, because its separator
/// was written, and a pass that reports on *where* things stand needs the span
/// of the separator even when nothing follows it.
#[derive(Clone)]
pub struct Element<'a> {
    /// The child, as a handle the pass hands back to [`Visitor`](super::Visitor).
    pub node: Option<&'a dyn super::Visit>,
    /// Where the separator before this child was written, if the child is not
    /// the first of its parent.
    pub separator: Option<Span>,
}

// A child prints as the place it occupies, because that is all a pass can say
// about a handle it holds without dispatching it.
impl std::fmt::Debug for Element<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("span", &self.to_span())
            .field("separator", &self.separator)
            .finish()
    }
}

impl<'a> Element<'a> {
    /// A child that is a node of the tree.
    #[inline]
    pub fn new(node: &'a dyn super::Visit) -> Self {
        Self {
            node: Some(node),
            separator: None,
        }
    }

    /// A place the parent wrote without a node in it.
    #[inline]
    pub fn absent(span: Span) -> Self {
        Self {
            node: None,
            separator: Some(span),
        }
    }

    /// The place this child occupies: its own where it is there, its
    /// separator's where it is not.
    pub fn to_span(&self) -> Option<Span> {
        match self.node {
            Some(node) => Some(node.to_span()),
            None => self.separator.clone(),
        }
    }
}

/// A shape the walk descends into.
///
/// The private `decompose` builds a shape, reports it to the pass, and then
/// hands it to [`super::descend`], which walks whatever children the shape still
/// holds. A pass that must not see part of a tree therefore clears the children
/// it does not want before the walk reaches them.
pub trait Shape<'a> {
    /// Every child the shape holds, in the order they were written.
    fn take_children(&mut self) -> Vec<Element<'a>>;
}

/// A source file: the definitions it writes and the documents it opens with.
#[derive(Debug)]
pub struct Production<'a> {
    /// The whole file.
    pub span: Span,
    /// Its items, which are definitions, documents and the trivia between them.
    pub children: Vec<Element<'a>>,
}

impl<'a> Shape<'a> for Production<'a> {
    fn take_children(&mut self) -> Vec<Element<'a>> {
        std::mem::take(&mut self.children)
    }
}

/// One definition of a file: `lexer TEXT -> …;`.
#[derive(Debug)]
pub struct Declaration<'a> {
    /// The statement, keyword, name, [`->`](DefKind) and body included.
    pub span: Span,
    /// The name being defined, which is what the other definitions refer to it
    /// by.
    pub name: Name,
    /// Which keyword opened it.
    pub kind: DefKind,
    /// The body: the one [`Alternatives`] the definition assigns to the name.
    pub children: Vec<Element<'a>>,
}

impl<'a> Shape<'a> for Declaration<'a> {
    fn take_children(&mut self) -> Vec<Element<'a>> {
        std::mem::take(&mut self.children)
    }
}

/// A document comment, `///` `/**` `//!` or `/*!`.
///
/// A document is a shape rather than a string because the walk reaches it and
/// because two of the four forms say something about the tree: an inner
/// document describes the file it stands in, and a block document may hold
/// several lines where a line document holds one.
#[derive(Debug, Clone)]
pub struct Comment {
    /// The comment, markers included.
    pub text: String,
    /// Whether it is the `/*` form rather than the `//` form.
    pub block: bool,
    /// Whether it is the `!` form, which describes the file rather than the
    /// definition after it.
    pub inner: bool,
    /// Where it was written.
    pub span: Span,
}

/// The `|` of a rule: the alternatives a definition chooses between.
#[derive(Debug)]
pub struct Alternatives<'a> {
    /// The whole choice.
    pub span: Span,
    /// The alternatives, in the order they were written. Each one is a
    /// [`Sequence`], and each one after the first was introduced by `|`.
    pub children: Vec<Element<'a>>,
}

impl<'a> Shape<'a> for Alternatives<'a> {
    fn take_children(&mut self) -> Vec<Element<'a>> {
        std::mem::take(&mut self.children)
    }
}

/// Operands written one after another, which is how the language says *and*.
///
/// The separator each [`Element`] after the first carries is the whitespace
/// that divides the operands: a sequence has no operator of its own, so that
/// whitespace is what tells the two operands apart and is part of the tree.
#[derive(Debug)]
pub struct Sequence<'a> {
    /// The whole sequence.
    pub span: Span,
    /// The operands, in the order they were written.
    pub children: Vec<Element<'a>>,
}

impl<'a> Shape<'a> for Sequence<'a> {
    fn take_children(&mut self) -> Vec<Element<'a>> {
        std::mem::take(&mut self.children)
    }
}

/// An operand with one of the four quantity operators on it: `a*`, `a?`, `a+`,
/// `a{2,3}`, and the same for a group, as in `(a | b)*`.
#[derive(Debug)]
pub struct Quantity<'a> {
    /// The operand and the operator together.
    pub span: Span,
    /// Which operator was written.
    pub op: QuantityOp,
    /// How many times the operand may match, where the operator is `{…}`.
    pub bounds: Option<Bounds>,
    /// The operand the operator applies to.
    pub target: Element<'a>,
    /// Where the operator itself was written.
    pub operator: Span,
}

impl<'a> Shape<'a> for Quantity<'a> {
    fn take_children(&mut self) -> Vec<Element<'a>> {
        vec![std::mem::replace(
            &mut self.target,
            Element::absent(self.span.clone()),
        )]
    }
}

/// The four quantity operators, named by what they ask rather than by how they
/// are spelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QuantityOp {
    /// `*`: as many times as there is room for, none included.
    ZeroOrMore,
    /// `?`: once at most.
    ZeroOrOne,
    /// `+`: as many as there is room for, at least one.
    OneOrMore,
    /// `{…}`: a bounded number of times.
    Bounded,
}

/// `a followed b`: `b` must come after `a` in the source without being part of
/// what `a` matched.
///
/// This is one shape rather than a [`Sequence`] because the two operands do not
/// hold the same place in the match: only `a` consumes anything.
#[derive(Debug)]
pub struct Followed<'a> {
    /// The whole run, keyword included.
    pub span: Span,
    /// What the operand after the keyword must follow.
    pub target: Element<'a>,
    /// What must come next.
    pub operand: Element<'a>,
}

impl<'a> Shape<'a> for Followed<'a> {
    fn take_children(&mut self) -> Vec<Element<'a>> {
        let operand = std::mem::replace(&mut self.operand, Element::absent(self.span.clone()));
        vec![
            std::mem::replace(&mut self.target, Element::absent(self.span.clone())),
            operand,
        ]
    }
}

/// `a except b`: whatever `a` matched, minus what `b` matches.
///
/// The exclusion cannot itself carry a suffix, so its shape is one of the
/// operands that stand alone: a [`Set`], a [`Literal`], a [`Reference`], a
/// [`Group`].
#[derive(Debug)]
pub struct Excepted<'a> {
    /// The whole run, keyword included.
    pub span: Span,
    /// What the exclusion is taken out of.
    pub target: Element<'a>,
    /// What is taken out.
    pub exclusion: Element<'a>,
}

impl<'a> Shape<'a> for Excepted<'a> {
    fn take_children(&mut self) -> Vec<Element<'a>> {
        let exclusion = std::mem::replace(&mut self.exclusion, Element::absent(self.span.clone()));
        vec![
            std::mem::replace(&mut self.target, Element::absent(self.span.clone())),
            exclusion,
        ]
    }
}

/// An expression wrapped in parentheses, so that it may hold one place in a
/// sequence or take a suffix as one operand.
#[derive(Debug)]
pub struct Group<'a> {
    /// Both parentheses and what they hold.
    pub span: Span,
    /// Where the `(` was written.
    pub open: Span,
    /// Where the `)` was written, when the file wrote one.
    pub close: Option<Span>,
    /// The expression inside.
    pub children: Vec<Element<'a>>,
}

impl<'a> Shape<'a> for Group<'a> {
    fn take_children(&mut self) -> Vec<Element<'a>> {
        std::mem::take(&mut self.children)
    }
}

/// `[a, b, c]`: the union of the values its members denote.
///
/// The members are given as data rather than as children. Nothing below a
/// member is a place a pass can reach — a member holds one value and no
/// structure — and the question a set raises is *which* values its members
/// denote, which is a question about the members together rather than about any
/// one of them.
#[derive(Debug, Clone)]
pub struct Set {
    /// The brackets and the members between them.
    pub span: Span,
    /// Where the `[` was written.
    pub open: Span,
    /// Where the `]` was written, when the file wrote one.
    pub close: Option<Span>,
    /// Where the `~` of a complement was written; `None` for a union.
    pub tilde: Option<Span>,
    /// Whether the set is joined to its members or refused by them.
    pub kind: SetKind,
    /// What the set says, in the order it was written.
    pub members: Vec<SetMember>,
}

/// Whether a [`Set`] takes its members or refuses them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SetKind {
    /// `[a b]`: one of the members.
    Union,
    /// `~[a b]`: exactly one character, and none of the members.
    Complement,
}

/// What one place of a set says.
#[derive(Debug, Clone)]
pub enum SetMember {
    /// `U+0041`: one code point.
    Char(CodePoint),
    /// `'/*'`: a string of any number of characters, which a set of strings may
    /// hold and a range may not.
    Str(TextLiteral),
    /// `'a'-'z'`: every code point between two bounds, both included.
    Range(CharRange),
    /// `NAME`: the set that another definition denotes, whose values no single
    /// file knows.
    Reference(Name),
}

/// The two bounds of a [`SetMember::Range`].
#[derive(Debug, Clone)]
pub struct CharRange {
    /// Both bounds and the `-` between them.
    pub span: Span,
    /// Where the dash was written.
    pub dash: Span,
    /// The lower place of the interval, which need not be the lower value.
    pub start: RangeSide,
    /// The upper place of the interval.
    pub end: RangeSide,
}

/// One bound of a range, which has to be exactly one character for the range to
/// denote anything.
#[derive(Debug, Clone)]
pub enum RangeSide {
    /// `U+0041`.
    Char(CodePoint),
    /// `'A'`, which is a string that has to hold one character.
    Str(TextLiteral),
}

/// A reference to another definition, written as its bare name.
#[derive(Debug, Clone)]
pub struct Reference {
    /// The name.
    pub name: Name,
}

/// `<WORD_START>`: a call of an algorithm the caller of the generated parser
/// provides.
#[derive(Debug, Clone)]
pub struct Call {
    /// The name between the angle brackets.
    pub name: Name,
}

/// A name as it was written.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name {
    /// The identifier itself.
    pub text: String,
    /// Where it was written.
    pub span: Span,
}

/// A literal that stands as an operand of its own: `'a'` or `U+0041`.
///
/// What a set holds is the same two forms plus a range and a name, which is why
/// [`SetMember`] spells them out rather than holding one of these.
#[derive(Debug, Clone)]
pub struct Literal {
    /// The literal, quotes or `U+` included.
    pub span: Span,
    /// Which of the two forms was written.
    pub value: LiteralValue,
}

/// The two forms a literal operand takes.
#[derive(Debug, Clone)]
pub enum LiteralValue {
    /// `'…'`.
    Str(TextLiteral),
    /// `U+…`.
    Char(CodePoint),
}

/// The content of a `'…'` literal, as the pieces it was written in.
///
/// A string literal is not one string: it is whatever the writer put between the
/// quotes, and the value it denotes only follows from the pieces. So a pass can
/// ask both questions — what does this say, and where did it fail to say it —
/// without reaching into a tree.
#[derive(Debug, Clone)]
pub struct TextLiteral {
    /// The quotes and the content between them.
    pub span: Span,
    /// The content, in the order it was written.
    pub pieces: Vec<Piece>,
}

impl TextLiteral {
    /// The string the literal denotes.
    ///
    /// The [`String`] always holds something for every piece: where a piece
    /// denotes no character, the [replacement
    /// character](char::REPLACEMENT_CHARACTER) stands in for it, so that a
    /// length or a comparison remains meaningful. The second element of the
    /// tuple is where the first piece that denotes nothing was written.
    pub fn value(&self) -> (String, Option<Span>) {
        let mut value = String::new();
        let mut invalid: Option<Span> = None;

        for piece in &self.pieces {
            match piece {
                Piece::Text(text) => value.push_str(text),
                Piece::Quote(_) => value.push('\''),
                Piece::Ascii(form) => value.push(form.value()),
                Piece::Unicode { digits, span } => match code_point(digits) {
                    Some(c) => value.push(c),
                    None => {
                        invalid.get_or_insert_with(|| span.clone());
                        value.push(char::REPLACEMENT_CHARACTER);
                    }
                },
            }
        }

        (value, invalid)
    }

    /// The characters the literal denotes, malformed pieces included as the
    /// replacement character.
    pub fn chars(&self) -> Vec<char> {
        self.value().0.chars().collect()
    }
}

/// One piece of the content of a string literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    /// Characters written as themselves.
    Text(String),
    /// `\'`, which is the one quote a string may hold.
    Quote(Span),
    /// One of the ASCII escapes, `\n` `\r` `\t` `\\` `\0` `\xNN`.
    Ascii(AsciiForm),
    /// `\u{…}`, the digits of which may name no character at all.
    Unicode {
        /// The hexadecimal digits between the braces.
        digits: String,
        /// The whole escape, `\u{` and `}` included.
        span: Span,
    },
}

/// The ASCII escape that was written, as the character it denotes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsciiForm {
    /// `\n`
    Newline,
    /// `\r`
    CarriageReturn,
    /// `\t`
    Tab,
    /// `\\`
    Backslash,
    /// `\0`
    Null,
    /// `\xNN`, the two digits of which the lexer has already checked.
    Hex {
        /// The two hexadecimal digits.
        digits: String,
        /// The escape, `\x` included.
        span: Span,
    },
}

impl AsciiForm {
    /// The character the escape denotes.
    ///
    /// A `\xNN` whose digits do not read as a number can only come from a tree
    /// that no parser built, so it denotes the replacement character rather than
    /// nothing at all.
    pub fn value(&self) -> char {
        match self {
            AsciiForm::Newline => '\n',
            AsciiForm::CarriageReturn => '\r',
            AsciiForm::Tab => '\t',
            AsciiForm::Backslash => '\\',
            AsciiForm::Null => '\0',
            AsciiForm::Hex { digits, .. } => u8::from_str_radix(digits, 16)
                .ok()
                .map_or(char::REPLACEMENT_CHARACTER, char::from),
        }
    }
}

/// A code point written as `U+…`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodePoint {
    /// The hexadecimal digits, `U+` left off.
    pub digits: String,
    /// The whole literal.
    pub span: Span,
}

impl CodePoint {
    /// The character the code point names, or `None` when the digits name no
    /// character: a number above the Unicode range, or a surrogate.
    pub fn value(&self) -> Option<char> {
        code_point(&self.digits)
    }
}

/// A number written as a decimal literal, as in `{2, 3}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Count {
    /// The digits, with the `_` separators a writer may put between them.
    pub digits: String,
    /// Where it was written.
    pub span: Span,
}

impl Count {
    /// The number the literal holds, or `None` when it does not fit.
    pub fn value(&self) -> Option<usize> {
        self.digits.replace('_', "").parse::<usize>().ok()
    }
}

/// How many times a [`Quantity`] lets its operand match.
#[derive(Debug, Clone)]
pub struct Bounds {
    /// The braces and the numbers between them.
    pub span: Span,
    /// Which of the three forms was written.
    pub form: BoundsForm,
}

/// The three forms a `{…}` bound takes.
#[derive(Debug, Clone)]
pub enum BoundsForm {
    /// `{n}`: exactly `n` times.
    Exactly(Count),
    /// `{,n}`: `n` times at most, which asks no lower bound.
    AtMost(Count),
    /// `{n,}`, or `{n,m}` with an upper bound that may be absent.
    AtLeast {
        /// The number of times the operand must at least match.
        from: Count,
        /// The number of times it may at most match, where one was written.
        to: Option<Count>,
    },
}

impl BoundsForm {
    /// The least number of times the operand may match.
    ///
    /// A number that does not fit is counted as zero, which keeps an analysis
    /// that only asks how much a repetition consumes from reporting the same
    /// broken literal twice.
    pub fn lower(&self) -> usize {
        match self {
            BoundsForm::Exactly(count) | BoundsForm::AtLeast { from: count, .. } => {
                count.value().unwrap_or(0)
            }
            BoundsForm::AtMost(_) => 0,
        }
    }
}

/// A token that holds text and nothing else: a punct, a keyword, an identifier
/// or a run of whitespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The text of the token.
    pub text: String,
    /// Where it was written.
    pub span: Span,
}

/// Reads the code point a run of hexadecimal digits names.
fn code_point(digits: &str) -> Option<char> {
    u32::from_str_radix(digits, 16)
        .ok()
        .and_then(char::from_u32)
}
