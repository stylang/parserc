//! The terminals of the DSL: the `whitespace` rule and the `lexer` rules that
//! `unsyn.us` beside this crate writes.
//!
//! One production, one type: every type here is named for the left side of its
//! production, and its right side is spelled out with the nodes and combinators
//! of [`parserc`], so that the declaration of a type is the declaration of the
//! rule. A sequence of the right side is a field apiece of the struct; a choice
//! that is the whole of the right side is an enum whose variants are the
//! alternatives.
//!
//! A choice written inside a group — `( XID_Start | '_' )` within
//! [`IdentOrKeyword`], the body of [`LitStr`] — belongs to no left side, and the
//! [analyzer](super::semantic) says so of every file that writes one. The hand
//! tree cannot leave such a choice unmodelled either, so it names these groups
//! on the pass's behalf — [`IdentStart`] and its like — and models each of them
//! as an enum, the same shape a named rule would have earned.
//!
//! # Where the trivia goes
//!
//! No node of this module owns a run of the trivia. A `lexer` token is read out
//! of the text it is handed, and a run taken there would end up inside that
//! text; the whitespace on each side of a token belongs to the `syntax` rule
//! that reaches it, which is why the terminals a `syntax` rule is written with —
//! the puncts and the fixed words — live in [`super::syntax`] instead. See the
//! [crate documentation](crate) for how a run is divided between two neighbours.
//!
//! The one consequence worth stating is the order of the alternatives of
//! [`Ident`]. A semantics error is fatal, so once a rule has refused text for
//! meaning nothing no enclosing choice may try another way — and `NonKeywordIdent`
//! reads `r#foo` as the name `r`, which is a name and is not fatal; [`RawIdent`]
//! therefore comes first, and takes the word whole.

use parserc::syntax::{Char, Syntax};
use parserc::{ControlFlow, Input, Kind, Parser, Span, keyword, next, next_if};
use unicode_ident::{is_xid_continue, is_xid_start};

use crate::compiler::errors::{KeywordKind, PunctKind, SemanticsKind, SyntaxKind, UnsynError};
use crate::compiler::input::UnsynInput;

// ---------------------------------------------------------------------------
// `whitespace S -> [ U+0009-U+000D, U+0020, U+0085, U+00A0, U+1680,`
// `                  U+2000-U+200A, U+2028, U+2029, U+202F, U+205F, U+3000 ];`
// ---------------------------------------------------------------------------

/// The trivia of the language: one run of the characters the `whitespace` rule
/// lists, taken as greedily as the text offers.
///
/// A `syntax` rule is separated by this node and never names it, so a run of
/// trivia is owned by exactly one of the two terminals it lies between.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(take_while = is_trivia)]
pub struct S<I>(pub I)
where
    I: UnsynInput;

/// Whether a character is one of the eleven runs the `whitespace` rule counts
/// out.
#[inline]
fn is_trivia(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    )
}

// ---------------------------------------------------------------------------
// `lexer LF -> U+000A;`
// `lexer CR -> U+000D;`
// ---------------------------------------------------------------------------

/// A line feed, the end of a line of text.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(char = '\u{000A}')]
pub struct LF<I>(pub I)
where
    I: UnsynInput;

/// A carriage return, which a line document has to end at as much as a line feed.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(char = '\u{000D}')]
pub struct CR<I>(pub I)
where
    I: UnsynInput;

// ---------------------------------------------------------------------------
// `lexer IdentOrKeyword -> ( XID_Start | '_' ) XID_Continue*;`
// `lexer XID_Start -> <XID_Start>;`
// `lexer XID_Continue -> <XID_Continue>;`
// ---------------------------------------------------------------------------

/// The start of a name, which the spec reaches outside itself.
///
/// `<XID_Start>` is a link the caller of a generated parser binds; here the
/// answer is the Unicode property of the same name, which `unicode-ident`
/// publishes.
///
/// The name is the one the spec writes, underscores and all, so that the node of
/// a bootstrap tree built from this file is the node this file declares.
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct XID_Start<I>(#[parserc(parser = next_if(|c: char| is_xid_start(c)))] pub I)
where
    I: UnsynInput;

/// A character a name continues with, which the spec likewise reaches outside
/// itself.
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct XID_Continue<I>(#[parserc(parser = next_if(|c: char| is_xid_continue(c)))] pub I)
where
    I: UnsynInput;

/// The `( XID_Start | '_' )` that begins a name.
///
/// The group names no rule of the spec, which is what the analyzer's
/// [`UnnamedAlternative`](super::errors::LintKind::UnnamedAlternative) lint says
/// of a file that writes one: the hand tree cannot leave the choice unmodelled,
/// so it names the group here and gives it the enum the choice deserves.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum IdentStart<I>
where
    I: UnsynInput,
{
    /// A start the external of the same name answers for.
    Xid(XID_Start<I>),
    /// The `_` that may also begin a name.
    Underscore(Char<I, '_'>),
}

/// A name before it is known whether it is a keyword: a start, then any run of
/// continuing characters.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdentOrKeyword<I>
where
    I: UnsynInput,
{
    /// The character the name begins with, of either of the two shapes.
    pub start: IdentStart<I>,
    /// The characters the name continues with, each of the external's own shape.
    pub tail: Vec<XID_Continue<I>>,
}

impl<I: UnsynInput> IdentStart<I> {
    /// The text of the one character the start read.
    #[inline]
    pub fn text(&self) -> &str {
        match self {
            IdentStart::Xid(start) => start.0.as_str(),
            IdentStart::Underscore(underscore) => underscore.0.as_str(),
        }
    }
}

impl<I: UnsynInput> IdentOrKeyword<I> {
    /// The whole of the name, its start and its run together.
    pub fn text(&self) -> String {
        let mut text = String::from(self.start.text());

        for continue_ in &self.tail {
            text.push_str(continue_.0.as_str());
        }

        text
    }
}

// ---------------------------------------------------------------------------
// `lexer RawIdent -> 'r#' IdentOrKeyword;`
// `lexer NonKeywordIdent -> IdentOrKeyword except Keywords;`
// `lexer Ident -> NonKeywordIdent | RawIdent;` — the hand tree reads the raw
// form first, which is what the order of a choice decides and not what the
// spec means to say about either form.
// ---------------------------------------------------------------------------

/// A name written out of the keywords of the language, as `r#where` is.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawIdent<I>
where
    I: UnsynInput,
{
    /// The `r#` that opens the name.
    #[parserc(keyword = "r#", map_err = SyntaxKind::Ident.map())]
    pub sigil: I,
    /// The name the sigil marks, read as a name and not as a word, which is what
    /// lets a raw name be a word of the language.
    pub name: IdentOrKeyword<I>,
}

impl<I: UnsynInput> RawIdent<I> {
    /// The whole of the name, sigil and all.
    #[inline]
    pub fn text(&self) -> String {
        self.sigil.as_str().to_owned() + &self.name.text()
    }
}

/// A name that is not a word of the language.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NonKeywordIdent<I>
where
    I: UnsynInput,
{
    /// The name the `except` leaves standing, which is all the node keeps of it.
    ///
    /// What the removal asks is not a part of the name, and nothing beyond the
    /// refusal it makes can say more than the node of [`IdentOrKeyword`] already
    /// says.
    #[parserc(parser = read_non_keyword_ident)]
    pub name: IdentOrKeyword<I>,
}

/// Reads the `IdentOrKeyword except Keywords` of [`NonKeywordIdent`].
///
/// `except` asks its question of the text the name read and of nothing else: the
/// removal runs over that text on its own, and only a removal that takes all of
/// it refuses the name. `lexerX` is therefore a name of its own, although
/// `lexer` fits the head of it.
fn read_non_keyword_ident<I>(input: &mut I) -> Result<IdentOrKeyword<I>, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let name = IdentOrKeyword::into_parser().parse(input)?;

    let mut rest = whole.split_to(whole.len() - input.len());

    if Keywords::into_parser().ok().parse(&mut rest)?.is_some() && rest.is_empty() {
        return Err(UnsynError::Semantics(
            SemanticsKind::Keyword,
            name.to_span(),
        ));
    }

    Ok(name)
}

/// A name of a node: either a word of the language written raw, or one that is
/// no word of it.
///
/// The spec writes the raw form second, where it has to stand first: a name is
/// read by the first alternative that fits, and `NonKeywordIdent` fits the `r` of
/// `r#foo` and leaves `#foo` behind, so the raw form would never be reached.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Ident<I>
where
    I: UnsynInput,
{
    /// A name written raw, as `r#where`.
    Raw(RawIdent<I>),
    /// A name that is no word of the language.
    NonKeyword(NonKeywordIdent<I>),
}

impl<I: UnsynInput> Ident<I> {
    /// The text of the name, sigil and all.
    #[inline]
    pub fn text(&self) -> String {
        match self {
            Ident::Raw(raw) => raw.text(),
            Ident::NonKeyword(ident) => ident.name.text(),
        }
    }
}

// ---------------------------------------------------------------------------
// `lexer Keywords -> LeadingKeywords | FollowingKeywords;`
// `lexer LeadingKeywords -> 'lexer' | 'syntax' | 'whitespace';`
// `lexer FollowingKeywords -> 'concat' | 'except' | 'followed';`
// ---------------------------------------------------------------------------

/// A word of the language, of either of the two places one may stand.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Keywords<I>
where
    I: UnsynInput,
{
    /// A word that opens a definition.
    Leading(LeadingKeywords<I>),
    /// A word that joins two operands of a pattern.
    Following(FollowingKeywords<I>),
}

/// The word a definition opens with.
///
/// Which of the three it was is the only thing that tells a `whitespace`
/// declaration from a `lexer` one and from a `syntax` one, and the enum says it
/// as a variant rather than as text to be read: the words are the alternatives
/// the rule writes, and the variant is the one that was read.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = map_leading_keywords_err)]
pub enum LeadingKeywords<I>
where
    I: UnsynInput,
{
    /// `lexer`, which defines a terminal.
    Lexer(#[parserc(keyword = "lexer", map_err = KeywordKind::Lexer.map())] I),
    /// `syntax`, which defines a node of the tree.
    Syntax(#[parserc(keyword = "syntax", map_err = KeywordKind::Syntax.map())] I),
    /// `whitespace`, which declares the trivia of the language.
    Whitespace(#[parserc(keyword = "whitespace", map_err = KeywordKind::Whitespace.map())] I),
}

/// Names the set the word belongs to, which is more than the bare combinator
/// error of the enum says.
///
/// The three words are mutually exclusive heads, so a position that begins none
/// of them can only be told as the set that is missing here.
#[inline]
fn map_leading_keywords_err(err: UnsynError) -> UnsynError {
    match err {
        UnsynError::Kind(Kind::Syntax("LeadingKeywords", control_flow, span)) => {
            UnsynError::Syntax(SyntaxKind::LeadingKeywords, control_flow, span)
        }
        err => err,
    }
}

impl<I: UnsynInput> LeadingKeywords<I> {
    /// The word that was read.
    #[inline]
    pub fn word(&self) -> &'static str {
        match self {
            LeadingKeywords::Lexer(_) => "lexer",
            LeadingKeywords::Syntax(_) => "syntax",
            LeadingKeywords::Whitespace(_) => "whitespace",
        }
    }
}

/// The word that joins two operands of a pattern into one construct.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = map_following_keywords_err)]
pub enum FollowingKeywords<I>
where
    I: UnsynInput,
{
    /// `concat`, which joins what two operands matched into one token.
    Concat(#[parserc(keyword = "concat", map_err = KeywordKind::Concat.map())] I),
    /// `except`, which removes what one operand matches from another.
    Except(#[parserc(keyword = "except", map_err = KeywordKind::Except.map())] I),
    /// `followed`, which asks that an operand come next without being taken.
    Followed(#[parserc(keyword = "followed", map_err = KeywordKind::Followed.map())] I),
}

/// Names the set the word belongs to, as the leading words are named.
#[inline]
fn map_following_keywords_err(err: UnsynError) -> UnsynError {
    match err {
        UnsynError::Kind(Kind::Syntax("FollowingKeywords", control_flow, span)) => {
            UnsynError::Syntax(SyntaxKind::FollowingKeywords, control_flow, span)
        }
        err => err,
    }
}

impl<I: UnsynInput> FollowingKeywords<I> {
    /// The word that was read.
    #[inline]
    pub fn word(&self) -> &'static str {
        match self {
            FollowingKeywords::Concat(_) => "concat",
            FollowingKeywords::Except(_) => "except",
            FollowingKeywords::Followed(_) => "followed",
        }
    }
}

// ---------------------------------------------------------------------------
// `lexer OctDigit -> ['0'-'7'];`
// `lexer DecDigit -> ['0'-'9'];`
// `lexer HexDigit -> ['0'-'9','a'-'f','A'-'F'];`
// ---------------------------------------------------------------------------

/// An octal digit, the first of the pair that spells a seven-bit escape.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OctDigit<I>(#[parserc(parser = next_if(|c: char| matches!(c, '0'..='7')))] pub I)
where
    I: UnsynInput;

/// A decimal digit.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DecDigit<I>(#[parserc(parser = next_if(|c: char| c.is_ascii_digit()))] pub I)
where
    I: UnsynInput;

/// A hexadecimal digit.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HexDigit<I>(#[parserc(parser = next_if(|c: char| c.is_ascii_hexdigit()))] pub I)
where
    I: UnsynInput;

// ---------------------------------------------------------------------------
// `lexer LitDec -> DecDigit ( DecDigit | '_' )*;`
// ---------------------------------------------------------------------------

/// The `( DecDigit | '_' )` that continues a decimal literal.
///
/// The group names no rule of the spec, which is what the analyzer's
/// [`UnnamedAlternative`](super::errors::LintKind::UnnamedAlternative) lint says
/// of a file that writes one; the hand tree names it here.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DigitOrUnderscore<I>
where
    I: UnsynInput,
{
    /// A digit of the number.
    Dec(DecDigit<I>),
    /// The `_` that separates digits and carries no value.
    Underscore(Char<I, '_'>),
}

/// A decimal number, which is how the bound of a repetition is written.
///
/// An underscore is part of the token and carries no value; the count of the
/// literal is read from its digits, which the tree holds one by one.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitDec<I>
where
    I: UnsynInput,
{
    /// The digit the number begins with.
    pub head: DecDigit<I>,
    /// Every digit and separator after it.
    pub tail: Vec<DigitOrUnderscore<I>>,
}

impl<I: UnsynInput> LitDec<I> {
    /// The number the digits spell, with the underscores the rule allows
    /// between them dropped.
    ///
    /// `None` says the number is past what a count of repetitions can be kept
    /// to, which no reading of the digits can turn into a count.
    pub fn count(&self) -> Option<u32> {
        let mut digits = String::new();

        digits.push_str(self.head.0.as_str());

        for part in &self.tail {
            if let DigitOrUnderscore::Dec(digit) = part {
                digits.push_str(digit.0.as_str());
            }
        }

        digits.parse::<u32>().ok()
    }

    /// The text of the literal, underscores and all.
    pub fn text(&self) -> String {
        let mut text = String::from(self.head.0.as_str());

        for part in &self.tail {
            text.push_str(match part {
                DigitOrUnderscore::Dec(digit) => digit.0.as_str(),
                DigitOrUnderscore::Underscore(underscore) => underscore.0.as_str(),
            });
        }

        text
    }
}

// ---------------------------------------------------------------------------
// `lexer LitUnicode -> 'U+' concat HexDigit{4};`
// ---------------------------------------------------------------------------

/// A code point written in a string, as `U+0009` is.
///
/// unsyn asks for exactly four hex digits after `U+`, where Rust allows four to
/// six, because the token it returns is always six characters long. The count
/// is a thing the grammar cannot say in fields, so the reading of the digits
/// keeps the check the rule asks for; what the node holds once it has passed
/// is the digits themselves.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitUnicode<I>
where
    I: UnsynInput,
{
    /// The `U+` that opens the literal.
    pub prefix: I,
    /// The four digits that spell the code point.
    pub digits: Vec<HexDigit<I>>,
}

impl<I: UnsynInput> LitUnicode<I> {
    /// The code point the digits spell, when they spell a character.
    ///
    /// Four hex digits is the shape the reader counted, so the only thing left
    /// to answer is whether the number of them is a character, which `D800`
    /// through `DFFF` are not.
    pub fn value(&self) -> Option<char> {
        let mut digits = String::new();

        for digit in &self.digits {
            digits.push_str(digit.0.as_str());
        }

        u32::from_str_radix(&digits, 16)
            .ok()
            .and_then(char::from_u32)
    }

    /// The text of the literal, `U+` and all.
    pub fn text(&self) -> String {
        let mut text = String::from(self.prefix.as_str());

        for digit in &self.digits {
            text.push_str(digit.0.as_str());
        }

        text
    }
}

impl<I> Syntax<I> for LitUnicode<I>
where
    I: UnsynInput,
{
    /// Reads the `'U+' concat HexDigit{4}` of [`LitUnicode`].
    ///
    /// `concat` joins the two operands with nothing between them, so a fifth
    /// digit cannot be the start of the next token: it is a bound written
    /// wrong, and the literal is refused.
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as Input>::Error> {
        let mut whole = input.clone();

        let prefix = keyword("U+")
            .parse(input)
            .map_err(SyntaxKind::Unicode.map())?;

        let digits = Vec::<HexDigit<I>>::into_parser().parse(input)?;

        if digits.len() != 4 {
            return Err(UnsynError::Semantics(
                SemanticsKind::Unicode,
                prefix.to_span().union(&digits.to_span()),
            ));
        }

        _ = whole.split_to(prefix.len() + digits.len());

        Ok(Self { prefix, digits })
    }

    #[inline]
    fn to_span(&self) -> Span {
        self.prefix.to_span().union(&self.digits.to_span())
    }
}

// ---------------------------------------------------------------------------
// `lexer QuoteEscape -> '\\'';`
// `lexer AsciiEscape -> '\\x' OctDigit HexDigit | '\\n' | '\\r' | '\\t'`
// `                   | '\\\\' | '\\0';`
// `lexer UnicodeEscape -> '\\u{' ( HexDigit '_'* ){1, 6} '}';`
// ---------------------------------------------------------------------------

/// An escaped quote, which is how a `'` is written inside a `'` delimited
/// literal.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(keyword = "\\'", map_err = SyntaxKind::QuoteEscape.map())]
pub struct QuoteEscape<I>(pub I)
where
    I: UnsynInput;

/// An escape for one of the characters ASCII writes with a backslash.
///
/// The rule is a choice, and the first of its alternatives is a sequence — the
/// marker of `\x` and the two digits of the seven-bit code it spells — which is
/// what the named fields of that variant hold.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AsciiEscape<I>
where
    I: UnsynInput,
{
    /// `\x` and the two digits of a code of seven bits, which is why the first
    /// of them is refused above `7`.
    Hex {
        /// The `\x` that opens the escape.
        ///
        /// What is read past this marker is a written escape going wrong, and
        /// no enclosing choice can put it right.
        #[parserc(crucial, keyword = "\\x", map_err = SyntaxKind::AsciiEscape.map())]
        marker: I,
        /// The leading digit, bounded by [`OctDigit`].
        oct: OctDigit<I>,
        /// The second digit of the pair.
        hex: HexDigit<I>,
    },
    /// `\n`, a line feed.
    N(#[parserc(keyword = "\\n")] I),
    /// `\r`, a carriage return.
    R(#[parserc(keyword = "\\r")] I),
    /// `\t`, a tab.
    T(#[parserc(keyword = "\\t")] I),
    /// `\\`, the backslash itself.
    Backslash(#[parserc(keyword = "\\\\")] I),
    /// `\0`, the nul character.
    Zero(#[parserc(keyword = "\\0")] I),
}

impl<I: UnsynInput> AsciiEscape<I> {
    /// The character the escape spells.
    ///
    /// Every shape the enum holds spells exactly one character, and the
    /// seven-bit pair of the first variant is two hex digits by the rule of
    /// [`HexDigit`], so there is nothing left for a refusal to say.
    pub fn value(&self) -> char {
        match self {
            AsciiEscape::Hex { oct, hex, .. } => {
                let digits = format!("{}{}", oct.0.as_str(), hex.0.as_str());

                u32::from_str_radix(&digits, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .expect("a pair of hex digits whose leader is below `8` is a character")
            }
            AsciiEscape::N(_) => '\n',
            AsciiEscape::R(_) => '\r',
            AsciiEscape::T(_) => '\t',
            AsciiEscape::Backslash(_) => '\\',
            AsciiEscape::Zero(_) => '\0',
        }
    }

    /// The text of the escape, as it was written.
    pub fn text(&self) -> String {
        match self {
            AsciiEscape::Hex { marker, oct, hex } => {
                format!("{}{}{}", marker.as_str(), oct.0.as_str(), hex.0.as_str())
            }
            AsciiEscape::N(word)
            | AsciiEscape::R(word)
            | AsciiEscape::T(word)
            | AsciiEscape::Backslash(word)
            | AsciiEscape::Zero(word) => word.as_str().to_owned(),
        }
    }
}

/// One digit of a code point escape, with the separators that follow it.
///
/// The group `( HexDigit '_'* )` names no rule of the spec, which is what the
/// analyzer's [`UnnamedAlternative`](super::errors::LintKind::UnnamedAlternative)
/// lint says of a file that writes one; the hand tree names it here, because
/// `{1, 6}` counts these and nothing else.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EscapeDigit<I>
where
    I: UnsynInput,
{
    /// The digit itself.
    pub digit: HexDigit<I>,
    /// The separators of the digit, which belong to the digit they follow.
    pub sep: Vec<Char<I, '_'>>,
}

/// An escape of a code point, as `\u{1F600}` is.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnicodeEscape<I>
where
    I: UnsynInput,
{
    /// The `\u{` that opens the escape.
    #[parserc(keyword = "\\u{", map_err = SyntaxKind::UnicodeEscape.map())]
    pub marker: I,
    /// The one to six digits the braces hold.
    #[parserc(parser = read_escape_digits)]
    pub digits: Vec<EscapeDigit<I>>,
    /// The `}` that closes the escape.
    #[parserc(keyword = "}", map_err = PunctKind::BraceEnd.map())]
    pub close: I,
}

/// Reads the `{1, 6}` digits of [`UnicodeEscape`].
///
/// The bound is of the digits and not of the text, since an underscore may
/// separate them; it is counted here rather than left to the grammar because
/// `{1, 6}` bounds the number of digits, which is what says how long the escape
/// may be. What a seventh asks is not the bound but the `}` after it, and the
/// closing of the escape is the one to say that it did not arrive.
fn read_escape_digits<I>(input: &mut I) -> Result<Vec<EscapeDigit<I>>, UnsynError>
where
    I: UnsynInput,
{
    let mut digits = Vec::new();

    while digits.len() < 6 {
        let Some(digit) = EscapeDigit::into_parser().ok().parse(input)? else {
            break;
        };

        digits.push(digit);
    }

    // No digit at all, which the lower bound of `{1, 6}` refuses.
    if digits.is_empty() {
        return Err(UnsynError::Semantics(
            SemanticsKind::UnicodeEscape,
            input.to_span_at(1),
        ));
    }

    Ok(digits)
}

impl<I: UnsynInput> UnicodeEscape<I> {
    /// The code point the digits spell, when they spell a character.
    ///
    /// Four to six hex digits is a number, and a number past `DFFF` or short
    /// of `D800`'s refusal is no character; the pass that reads a value is the
    /// one that says so, from the span of the escape it was reading.
    pub fn value(&self) -> Option<char> {
        let mut digits = String::new();

        for group in &self.digits {
            digits.push_str(group.digit.0.as_str());
        }

        u32::from_str_radix(&digits, 16)
            .ok()
            .and_then(char::from_u32)
    }

    /// The text of the escape, as it was written.
    pub fn text(&self) -> String {
        let mut text = String::from(self.marker.as_str());

        for group in &self.digits {
            text.push_str(group.digit.0.as_str());

            for _ in &group.sep {
                text.push('_');
            }
        }

        text.push_str(self.close.as_str());
        text
    }
}

// ---------------------------------------------------------------------------
// `lexer LitStr -> '\'' ( ~['"','\\',CR] | QuoteEscape | AsciiEscape`
// `                   | UnicodeEscape )* '\'';`
// ---------------------------------------------------------------------------

/// The `( ~['"','\\',CR] | QuoteEscape | AsciiEscape | UnicodeEscape )` that
/// fills a string.
///
/// The group names no rule of the spec, which is what the analyzer's
/// [`UnnamedAlternative`](super::errors::LintKind::UnnamedAlternative) lint says
/// of a file that writes one; the hand tree names it here, because the reading
/// of a string is a reading of its pieces one by one.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LitStrContent<I>
where
    I: UnsynInput,
{
    /// One character of the plain text of the body, which is none of the three
    /// the set refuses: the quote that would close the literal, the backslash
    /// that opens an escape, and the `CR` no escape writes.
    Plain(#[parserc(parser = next_if(|c: char| c != '\'' && c != '\\' && c != '\r'))] I),
    /// An escaped quote.
    Quote(QuoteEscape<I>),
    /// One of the escapes ASCII writes with a backslash.
    Ascii(AsciiEscape<I>),
    /// An escape of a code point.
    Unicode(UnicodeEscape<I>),
}

impl<I: UnsynInput> LitStrContent<I> {
    /// The characters the piece spells.
    ///
    /// An escape of no character — `\u{D800}` and its like, a shape the reader
    /// accepts because four to six hex digits is a number — spells none, and
    /// the pass that reads a value is the one to refuse it.
    pub fn chars(&self) -> Vec<char> {
        match self {
            LitStrContent::Plain(text) => text.as_str().chars().collect(),
            LitStrContent::Quote(_) => vec!['\''],
            LitStrContent::Ascii(escape) => vec![escape.value()],
            LitStrContent::Unicode(escape) => escape.value().into_iter().collect(),
        }
    }

    /// The text of the piece, as it was written.
    pub fn text(&self) -> String {
        match self {
            LitStrContent::Plain(text) => text.as_str().to_owned(),
            LitStrContent::Quote(quote) => quote.0.as_str().to_owned(),
            LitStrContent::Ascii(escape) => escape.text(),
            LitStrContent::Unicode(escape) => escape.text(),
        }
    }
}

/// A string literal: any run of characters between two `'`, in which a `'`, a
/// `\` and a `CR` can only be written by an escape.
///
/// The content stops at the first `'` that is not part of an escape, so a
/// literal can never hold a bare apostrophe.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitStr<I>
where
    I: UnsynInput,
{
    /// The quote that opens the literal.
    #[parserc(keyword = "'", map_err = PunctKind::SingleQuote.map())]
    pub open: I,
    /// The pieces of the body, each one character of plain text or one escape.
    ///
    /// The repetition ends where no piece begins, which is the closing quote
    /// wherever one stands and whatever the text after the pieces otherwise
    /// is; the quote that has to come next says what was expected there.
    pub body: Vec<LitStrContent<I>>,
    /// The quote that closes the literal.
    #[parserc(keyword = "'", map_err = PunctKind::SingleQuote.map())]
    pub close: I,
}

impl<I: UnsynInput> LitStr<I> {
    /// The characters the literal spells.
    ///
    /// The two quotes bound the literal and carry no value, so only the body
    /// is read. An escape of no character yields no character: the refusal it
    /// is worth belongs to the pass that reads a value, which reaches the
    /// escape itself and says so from its span.
    pub fn chars(&self) -> Vec<char> {
        self.body.iter().flat_map(LitStrContent::chars).collect()
    }

    /// The text of the literal, both quotes and all, as it was written.
    pub fn text(&self) -> String {
        let mut text = String::from(self.open.as_str());

        for piece in &self.body {
            text.push_str(&piece.text());
        }

        text.push_str(self.close.as_str());
        text
    }
}

// ---------------------------------------------------------------------------
// `lexer InnerLineDoc -> '//!' ~[LF,CR]*;`
// `lexer OuterLineDoc -> '///' ( ~['/'] ~[LF,CR]* )?;`
// ---------------------------------------------------------------------------

/// Whether a character leaves the line a document is written on.
#[inline]
fn stays_on_the_line(c: char) -> bool {
    c != '\n' && c != '\r'
}

/// A line document that describes the file it is written in.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InnerLineDoc<I>
where
    I: UnsynInput,
{
    /// The `//!` that opens the document.
    #[parserc(keyword = "//!", map_err = SyntaxKind::InnerLineDoc.map())]
    pub open: I,
    /// The text to the end of the line, which the set of the rule refuses to
    /// cross. The run is taken in one piece: that is what a repetition of one
    /// refused character arrives at, and nothing below it says more.
    #[parserc(take_while = stays_on_the_line)]
    pub body: I,
}

impl<I: UnsynInput> InnerLineDoc<I> {
    /// The text of the document, as it was written.
    #[inline]
    pub fn text(&self) -> String {
        self.open.as_str().to_owned() + self.body.as_str()
    }
}

/// The body of an [`OuterLineDoc`]: the one character its group asks for first
/// and the run that follows it, named by the hand tree for the group the spec
/// leaves unnamed.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineDocBody<I>
where
    I: UnsynInput,
{
    /// The `~['/']`: one character that is no slash.
    #[parserc(parser = next_if(|c: char| c != '/'))]
    pub first: I,
    /// The `~[LF,CR]*` after it, taken in one piece as the inner document's is.
    #[parserc(take_while = stays_on_the_line)]
    pub rest: I,
}

/// A line document that describes the definition after it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OuterLineDoc<I>
where
    I: UnsynInput,
{
    /// The `///` that opens the document.
    #[parserc(keyword = "///", map_err = SyntaxKind::OuterLineDoc.map())]
    pub open: I,
    /// The body, optional as the rule writes it: the fourth `/` is refused by
    /// `~['/']`, which leaves the option unfilled, and what stands after the
    /// document is for the file to say.
    pub body: Option<LineDocBody<I>>,
}

impl<I: UnsynInput> OuterLineDoc<I> {
    /// The text of the document, as it was written.
    #[inline]
    pub fn text(&self) -> String {
        let mut text = String::from(self.open.as_str());

        if let Some(body) = &self.body {
            text.push_str(body.first.as_str());
            text.push_str(body.rest.as_str());
        }

        text
    }
}

// ---------------------------------------------------------------------------
// `lexer BlockComment -> '/*' ( ~['*','!','/*'] | '**' | BlockCommentOrDoc )`
// `                         ( BlockCommentOrDoc | ~['/*','*/'] )* '*/'`
// `                       | '/**/' | '/***/';`
// `lexer InnerBlockDoc -> '/*!' ( BlockCommentOrDoc | ~['/*','*/'] )* '*/';`
// `lexer OuterBlockDoc -> '/**' ( ~['*','/*'] | BlockCommentOrDoc )`
// `                        ( BlockCommentOrDoc | ~['/*','*/'] )* '*/';`
// `lexer BlockCommentOrDoc -> BlockComment | OuterBlockDoc | InnerBlockDoc;`
// ---------------------------------------------------------------------------

/// Takes one character when it is none of `singles` and opens none of the two
/// character `pairs`.
///
/// This is what the negated set of a block rule asks of a character: the members
/// of the set are read as much from a `/*` as from a `*`, so the window of two
/// characters is what a plain character of a body is refused by. The refusal is
/// [`Recovable`](ControlFlow::Recovable): what stands where a plain character is
/// asked for is the alternative after it, or the end of the body.
fn plain_refusing<I>(
    input: &mut I,
    singles: &[char],
    pairs: &[&'static str],
) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let refused =
        |input: &mut I| UnsynError::Kind(Kind::NextIf(ControlFlow::Recovable, input.to_span_at(1)));

    let Some(c) = input.iter().next() else {
        return Err(UnsynError::Kind(Kind::NextIf(
            ControlFlow::Incomplete,
            input.to_span_at(1),
        )));
    };

    if singles.contains(&c) || pairs.iter().any(|pair| input.starts_with(*pair).is_some()) {
        return Err(refused(input));
    }

    next(c).parse(input)
}

/// The one character of the text of a [`BlockComment`] body, which is none of
/// `*` and `!` and opens no nested group.
fn head_plain<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    plain_refusing(input, &['*', '!'], &["/*"])
}

/// The one character of the text of an [`OuterBlockDoc`] body, which is no `*`
/// and opens no nested group.
fn outer_head_plain<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    plain_refusing(input, &['*'], &["/*"])
}

/// The one character of the text of a body that is only text, which opens no
/// nested group and closes no group either.
fn tail_plain<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    plain_refusing(input, &[], &["/*", "*/"])
}

/// The `( ~['*','!','/*'] | '**' | BlockCommentOrDoc )` that opens the body of a
/// [`BlockComment`].
///
/// The group names no rule of the spec, which is what the analyzer's
/// [`UnnamedAlternative`](super::errors::LintKind::UnnamedAlternative) lint says
/// of a file that writes one; the hand tree names it here.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BlockCommentHead<I>
where
    I: UnsynInput,
{
    /// One character of the text of the body.
    Plain(#[parserc(parser = head_plain)] I),
    /// `'**'`, which is the empty comment's second star and the nesting
    /// document's, taken as text of the comment because neither of the two
    /// other arms of the group can begin with a `*`.
    StarStar(#[parserc(keyword = "**", map_err = SyntaxKind::BlockComment.map())] I),
    /// A group nested here, which needs a closing of its own.
    Nested(Box<BlockCommentOrDoc<I>>),
}

/// The `( ~['*','/*'] | BlockCommentOrDoc )` that opens the body of an
/// [`OuterBlockDoc`], named by the hand tree as the comment's head is.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OuterBlockDocHead<I>
where
    I: UnsynInput,
{
    /// One character of the text of the body.
    Plain(#[parserc(parser = outer_head_plain)] I),
    /// A group nested here.
    Nested(Box<BlockCommentOrDoc<I>>),
}

/// The `( BlockCommentOrDoc | ~['/*','*/'] )` that fills the body of all three
/// block rules, named once for the three because the three write it the same.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BlockCommentTail<I>
where
    I: UnsynInput,
{
    /// A group nested here.
    Nested(Box<BlockCommentOrDoc<I>>),
    /// One character of the text of the body.
    Plain(#[parserc(parser = tail_plain)] I),
}

impl<I: UnsynInput> BlockCommentHead<I> {
    /// The text the head read.
    pub fn text(&self) -> String {
        match self {
            BlockCommentHead::Plain(text) => text.as_str().to_owned(),
            BlockCommentHead::StarStar(text) => text.as_str().to_owned(),
            BlockCommentHead::Nested(group) => group.text(),
        }
    }
}

impl<I: UnsynInput> OuterBlockDocHead<I> {
    /// The text the head read.
    pub fn text(&self) -> String {
        match self {
            OuterBlockDocHead::Plain(text) => text.as_str().to_owned(),
            OuterBlockDocHead::Nested(group) => group.text(),
        }
    }
}

impl<I: UnsynInput> BlockCommentTail<I> {
    /// The text the part of the body read.
    pub fn text(&self) -> String {
        match self {
            BlockCommentTail::Nested(group) => group.text(),
            BlockCommentTail::Plain(text) => text.as_str().to_owned(),
        }
    }
}

/// A block comment, which nests and is closed by the `*/` of its own opening.
///
/// The alternatives are tried in the order they are written, and the two closed
/// forms come in the order the reading needs rather than the order the spec
/// writes them: `/***/` opens as the body of two stars that never finds a
/// closing, so the five-character form has to be answered before the four of
/// `/**/`, which is its own head. The three write the same text at the front,
/// and the longest reading is the one that was meant.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = SyntaxKind::BlockComment.map())]
pub enum BlockComment<I>
where
    I: UnsynInput,
{
    /// The whole of the rule's first alternative: the body, its head and its
    /// closing, between the `/*` and the `*/`.
    Group {
        /// The `/*` that opens the comment.
        #[parserc(keyword = "/*", map_err = SyntaxKind::BlockComment.map())]
        open: I,
        /// What opens the body, of one of the three shapes the rule writes.
        head: BlockCommentHead<I>,
        /// The body after its head, every part of it a nested group or one
        /// character of text.
        tail: Vec<BlockCommentTail<I>>,
        /// The `*/` that closes the comment, which is asked for exactly where
        /// the body ends and says itself what was missing there.
        #[parserc(keyword = "*/", map_err = SyntaxKind::BlockComment.map())]
        close: I,
    },
    /// `'/***/'`, which the rule writes third.
    Star(#[parserc(keyword = "/***/", map_err = SyntaxKind::BlockComment.map())] I),
    /// `'/**/'`, which the rule writes second.
    Empty(#[parserc(keyword = "/**/", map_err = SyntaxKind::BlockComment.map())] I),
}

impl<I: UnsynInput> BlockComment<I> {
    /// The text of the comment, as it was written.
    pub fn text(&self) -> String {
        match self {
            BlockComment::Group {
                open,
                head,
                tail,
                close,
            } => {
                let mut text = String::from(open.as_str());

                text.push_str(&head.text());

                for part in tail {
                    text.push_str(&part.text());
                }

                text.push_str(close.as_str());
                text
            }
            BlockComment::Star(token) | BlockComment::Empty(token) => token.as_str().to_owned(),
        }
    }
}

/// A block document that describes the file it is written in.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InnerBlockDoc<I>
where
    I: UnsynInput,
{
    /// The `/*!` that opens the document.
    #[parserc(keyword = "/*!", map_err = SyntaxKind::InnerBlockDoc.map())]
    pub open: I,
    /// The body: nested groups and text, until the closing arrives.
    pub tail: Vec<BlockCommentTail<I>>,
    /// The `*/` that closes the document.
    #[parserc(keyword = "*/", map_err = SyntaxKind::InnerBlockDoc.map())]
    pub close: I,
}

impl<I: UnsynInput> InnerBlockDoc<I> {
    /// The text of the document, as it was written.
    pub fn text(&self) -> String {
        let mut text = String::from(self.open.as_str());

        for part in &self.tail {
            text.push_str(&part.text());
        }

        text.push_str(self.close.as_str());
        text
    }
}

/// A block document that describes the definition after it.
///
/// The opening of three refuses what the head set keeps out of the body, which
/// is what leaves `/**/` and `/***/` to the block comment: an opening of three
/// is a comment that closes at once, and not an empty document. The same set is
/// what keeps `/** a */` out of the comment, whose head can only open with a
/// text that is no star, two stars, or a group of its own.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OuterBlockDoc<I>
where
    I: UnsynInput,
{
    /// The `/**` that opens the document.
    #[parserc(keyword = "/**", map_err = SyntaxKind::OuterBlockDoc.map())]
    pub open: I,
    /// What opens the body, which is never a third star.
    pub head: OuterBlockDocHead<I>,
    /// The body after its head.
    pub tail: Vec<BlockCommentTail<I>>,
    /// The `*/` that closes the document.
    #[parserc(keyword = "*/", map_err = SyntaxKind::OuterBlockDoc.map())]
    pub close: I,
}

impl<I: UnsynInput> OuterBlockDoc<I> {
    /// The text of the document, as it was written.
    pub fn text(&self) -> String {
        let mut text = String::from(self.open.as_str());

        text.push_str(&self.head.text());

        for part in &self.tail {
            text.push_str(&part.text());
        }

        text.push_str(self.close.as_str());
        text
    }
}

/// A block group, which is the one rule that nests: a comment may hold a
/// document, and a document may hold a comment.
///
/// The order is the order of the spec. It matters only for `/***`, where a block
/// comment and an outer document both begin with the `/`, `*`, `*` and the fourth
/// character decides: the comment is read first, and refuses what an outer
/// document would have taken, so the choice settles on the comment.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BlockCommentOrDoc<I>
where
    I: UnsynInput,
{
    /// A `/*` group.
    Block(BlockComment<I>),
    /// A `/**` group.
    Outer(OuterBlockDoc<I>),
    /// A `/*!` group.
    Inner(InnerBlockDoc<I>),
}

impl<I: UnsynInput> BlockCommentOrDoc<I> {
    /// The text of the group, as it was written.
    pub fn text(&self) -> String {
        match self {
            BlockCommentOrDoc::Block(comment) => comment.text(),
            BlockCommentOrDoc::Outer(doc) => doc.text(),
            BlockCommentOrDoc::Inner(doc) => doc.text(),
        }
    }
}

#[cfg(test)]
mod tests {
    use parserc::{AsStr, Span, syntax::SyntaxInput};

    use super::*;
    use crate::compiler::input::TokenStream;

    /// Parses `source` as the node `T`.
    fn parse<'a, T>(source: &'a str) -> Result<T, UnsynError>
    where
        T: Syntax<TokenStream<'a>>,
    {
        TokenStream::from(source).parse::<T>()
    }

    /// The text a node of this module read.
    fn text<T>(node: &T) -> String
    where
        T: ToText,
    {
        node.text()
    }

    trait ToText {
        fn text(&self) -> String;
    }

    /// Pins the text of every node that is one run of text of its own.
    macro_rules! holds_its_own_text {
        ($($name:ident),* $(,)?) => {
            $(
                impl ToText for $name<TokenStream<'_>> {
                    #[inline]
                    fn text(&self) -> String {
                        self.0.as_str().to_owned()
                    }
                }
            )*
        };
    }

    holds_its_own_text!(
        S,
        LF,
        CR,
        XID_Start,
        XID_Continue,
        OctDigit,
        DecDigit,
        HexDigit,
        QuoteEscape,
    );

    // The nodes that are modelled as the shapes their right sides write answer
    // for their text from those shapes.

    impl ToText for IdentOrKeyword<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    impl ToText for RawIdent<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    impl ToText for NonKeywordIdent<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.name.text()
        }
    }

    impl ToText for LeadingKeywords<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.word().to_owned()
        }
    }

    impl ToText for FollowingKeywords<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.word().to_owned()
        }
    }

    impl ToText for LitDec<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    impl ToText for LitUnicode<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    impl ToText for AsciiEscape<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    impl ToText for UnicodeEscape<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    impl ToText for LitStr<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    impl ToText for BlockComment<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    impl ToText for InnerBlockDoc<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    impl ToText for OuterBlockDoc<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    impl ToText for InnerLineDoc<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    impl ToText for OuterLineDoc<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    #[test]
    fn the_trivia_is_the_set_of_the_rule() {
        for c in [
            '\u{0009}', '\u{000A}', '\u{000B}', '\u{000C}', '\u{000D}', '\u{0020}', '\u{0085}',
            '\u{00A0}', '\u{1680}', '\u{2000}', '\u{200A}', '\u{2028}', '\u{2029}', '\u{202F}',
            '\u{205F}', '\u{3000}',
        ] {
            assert!(is_trivia(c), "{c:?}");
        }

        for c in [
            '\u{0008}', '\u{000E}', '\u{0021}', '\u{00A1}', '\u{200B}', '\u{3001}',
        ] {
            assert!(!is_trivia(c), "{c:?}");
        }

        // The run is greedy, and it takes the whole of what the text offers.
        assert_eq!(text(&parse::<S<_>>(" \n\t ").unwrap()), " \n\t ");
        assert_eq!(text(&parse::<S<_>>("  x").unwrap()), "  ");

        // A run of nothing at all is not a run.
        assert!(parse::<S<_>>("x").is_err());
    }

    #[test]
    fn a_line_ending_is_its_own_token() {
        assert_eq!(text(&parse::<LF<_>>("\n").unwrap()), "\n");
        assert!(parse::<LF<_>>("\r").is_err());
        assert_eq!(text(&parse::<CR<_>>("\r").unwrap()), "\r");
    }

    #[test]
    fn a_name_is_a_start_and_a_run() {
        for source in ["x", "_", "XID_Continue", "a1", "café"] {
            assert_eq!(text(&parse::<IdentOrKeyword<_>>(source).unwrap()), source);
        }

        // A name starts where a continuing character cannot.
        assert_eq!(text(&parse::<IdentOrKeyword<_>>("_a-b").unwrap()), "_a");
        assert!(parse::<IdentOrKeyword<_>>("1a").is_err());
        assert!(parse::<IdentOrKeyword<_>>("-a").is_err());
    }

    #[test]
    fn a_keyword_is_a_word_whatever_stands_behind_it() {
        for word in ["lexer", "syntax", "whitespace"] {
            assert_eq!(text(&parse::<LeadingKeywords<_>>(word).unwrap()), word);
        }
        for word in ["concat", "except", "followed"] {
            assert_eq!(text(&parse::<FollowingKeywords<_>>(word).unwrap()), word);
        }

        // The rule writes a word and nothing else, so the word is read wherever
        // its text stands: `lexerX` begins with `lexer`, and it is the rules
        // about it — the `except` that keeps the words out of the names, and the
        // name that has to follow the word — that decide what the rest of the
        // text is.
        assert_eq!(
            text(&parse::<LeadingKeywords<_>>("lexerX").unwrap()),
            "lexer"
        );
        assert_eq!(
            text(&parse::<FollowingKeywords<_>>("excepting").unwrap()),
            "except"
        );

        // Where no word of the set begins at all, it is the set that the
        // position is missing.
        assert_eq!(
            parse::<LeadingKeywords<_>>("| 'a';"),
            Err(UnsynError::Syntax(
                SyntaxKind::LeadingKeywords,
                ControlFlow::Recovable,
                Span::Range(0..1)
            ))
        );
    }

    #[test]
    fn a_word_is_a_name_except_for_the_words_of_the_language() {
        // The `except` of the spec runs over the text the name read and over
        // nothing else, so a word that is only the head of a name leaves it a
        // name.
        for keyword in [
            "whitespace",
            "lexer",
            "syntax",
            "followed",
            "except",
            "concat",
        ] {
            assert_eq!(
                parse::<NonKeywordIdent<_>>(keyword),
                Err(UnsynError::Semantics(
                    SemanticsKind::Keyword,
                    Span::Range(0..keyword.len())
                )),
                "{keyword}"
            );
        }

        for source in ["WHITESPACE_", "_x", "lexerX", "xlet", "syntaxes"] {
            assert_eq!(text(&parse::<NonKeywordIdent<_>>(source).unwrap()), source);
        }
    }

    #[test]
    fn a_raw_name_may_be_a_word_of_the_language() {
        assert_eq!(text(&parse::<RawIdent<_>>("r#where").unwrap()), "r#where");
        assert_eq!(text(&parse::<RawIdent<_>>("r#lexer").unwrap()), "r#lexer");
        assert!(parse::<RawIdent<_>>("r#1").is_err());

        // The raw form is read first, because `NonKeywordIdent` would settle for
        // the `r` of `r#lexer` and leave `#lexer` behind.
        assert!(matches!(parse::<Ident<_>>("r#lexer"), Ok(Ident::Raw(_))));
        assert!(matches!(
            parse::<Ident<_>>("lexer2"),
            Ok(Ident::NonKeyword(_))
        ));
    }

    #[test]
    fn the_digits_of_the_sets_are_one_character() {
        assert_eq!(text(&parse::<OctDigit<_>>("7").unwrap()), "7");
        assert!(parse::<OctDigit<_>>("8").is_err());
        assert_eq!(text(&parse::<DecDigit<_>>("9").unwrap()), "9");
        assert_eq!(text(&parse::<HexDigit<_>>("f").unwrap()), "f");
        assert!(parse::<HexDigit<_>>("g").is_err());

        // A set is one character, so it takes no second one.
        assert_eq!(
            parse::<OctDigit<_>>("77").map(|d| d.0.as_str().to_string()),
            Ok("7".to_string())
        );
    }

    #[test]
    fn a_decimal_literal_separates_with_underscores() {
        for source in ["0", "12", "1_000", "1_2_3_4"] {
            assert_eq!(text(&parse::<LitDec<_>>(source).unwrap()), source);
        }

        // An underscore cannot open the literal, and a run of digits ends where
        // a name would continue.
        assert!(parse::<LitDec<_>>("_1").is_err());
        assert_eq!(text(&parse::<LitDec<_>>("12x").unwrap()), "12");
    }

    #[test]
    fn a_unicode_literal_is_exactly_four_digits() {
        assert_eq!(text(&parse::<LitUnicode<_>>("U+2029").unwrap()), "U+2029");

        // The span of the refusal covers the `U+` and the digits it read.
        for (source, span) in [
            ("U+1", 0..3),
            ("U+11234", 0..7),
            ("U+12_3", 0..4),
            ("U+0x29", 0..3),
        ] {
            assert_eq!(
                parse::<LitUnicode<_>>(source),
                Err(UnsynError::Semantics(
                    SemanticsKind::Unicode,
                    Span::Range(span)
                )),
                "{source}"
            );
        }
    }

    #[test]
    fn an_ascii_escape_is_seven_bits() {
        for source in ["\\n", "\\r", "\\t", "\\\\", "\\0"] {
            assert_eq!(text(&parse::<AsciiEscape<_>>(source).unwrap()), source);
        }

        assert_eq!(text(&parse::<AsciiEscape<_>>("\\x7F").unwrap()), "\\x7F");

        // The two digits of the escape are the escape's own once its marker has
        // been read: whatever stands where a digit must is a written escape
        // going wrong, and no enclosing choice can mean anything else there.
        for (source, span) in [("\\x80", 2..3), ("\\xg0", 2..3), ("\\x0", 3..3)] {
            assert_eq!(
                parse::<AsciiEscape<_>>(source),
                Err(UnsynError::Kind(Kind::NextIf(
                    ControlFlow::Fatal,
                    Span::Range(span)
                ))),
                "{source}"
            );
        }

        assert_eq!(text(&parse::<QuoteEscape<_>>(r"\'").unwrap()), r"\'");
    }

    #[test]
    fn a_unicode_escape_is_one_to_six_digits() {
        for source in ["\\u{1}", "\\u{1F600}", "\\u{1_0_0}", "\\u{abcdef}"] {
            assert_eq!(text(&parse::<UnicodeEscape<_>>(source).unwrap()), source);
        }

        // No digits at all is the lower bound of the escape refusing its own
        // reading.
        assert!(
            matches!(
                parse::<UnicodeEscape<_>>("\\u{}"),
                Err(UnsynError::Semantics(SemanticsKind::UnicodeEscape, _))
            ),
            "no digits at all"
        );

        // A seventh digit is not the bound speaking: six are taken and the
        // brace is then asked for where a digit stands. Text that is no digit
        // at all leaves the same question to the same place.
        for source in ["\\u{1234567}", "\\u{abcdefg}", "\\u{0x}"] {
            assert!(
                matches!(
                    parse::<UnicodeEscape<_>>(source),
                    Err(UnsynError::Punct(PunctKind::BraceEnd, _, _))
                ),
                "{source}"
            );
        }

        // The closing brace is not optional.
        assert_eq!(
            parse::<UnicodeEscape<_>>("\\u{1"),
            Err(UnsynError::Punct(
                PunctKind::BraceEnd,
                ControlFlow::Recovable,
                Span::Range(4..4)
            ))
        );
    }

    #[test]
    fn a_string_stops_at_the_quote_that_is_not_escaped() {
        for source in [
            "''",
            "'a'",
            "'\\''",
            "'\\\\x41'",
            "'\\u{41}'",
            "'a\nb'",
            "'U+0041'",
        ] {
            assert_eq!(text(&parse::<LitStr<_>>(source).unwrap()), source);
        }

        // A double quote is nothing special here, and a bare apostrophe closes
        // the literal wherever it stands.
        assert_eq!(text(&parse::<LitStr<_>>("'\"'").unwrap()), "'\"'");
        assert_eq!(text(&parse::<LitStr<_>>("'a'b'").unwrap()), "'a'");

        // A body that runs to the end of the input, and a `CR` that is no piece
        // of a body, both end the reading where they stand: what was asked for
        // there is the closing quote, and that is what the refusal says.
        assert_eq!(
            parse::<LitStr<_>>("'a"),
            Err(UnsynError::Punct(
                PunctKind::SingleQuote,
                ControlFlow::Recovable,
                Span::Range(2..2)
            ))
        );
        assert_eq!(
            parse::<LitStr<_>>("'a\rb'"),
            Err(UnsynError::Punct(
                PunctKind::SingleQuote,
                ControlFlow::Recovable,
                Span::Range(2..3)
            ))
        );
        // A `\` that opens no escape is no content, and it is where the closing
        // quote is then asked for and found missing.
        assert_eq!(
            parse::<LitStr<_>>("'\\q'"),
            Err(UnsynError::Punct(
                PunctKind::SingleQuote,
                ControlFlow::Recovable,
                Span::Range(1..2),
            ))
        );
    }

    #[test]
    fn a_line_document_ends_at_either_line_ending() {
        assert_eq!(
            text(&parse::<InnerLineDoc<_>>("//! about\nnext").unwrap()),
            "//! about"
        );
        assert_eq!(
            text(&parse::<InnerLineDoc<_>>("//!about\r\nnext").unwrap()),
            "//!about"
        );
        assert_eq!(
            text(&parse::<InnerLineDoc<_>>("//!").unwrap()),
            "//!",
            "the body of a line document is optional"
        );

        assert_eq!(
            text(&parse::<OuterLineDoc<_>>("/// about").unwrap()),
            "/// about"
        );
        assert_eq!(text(&parse::<OuterLineDoc<_>>("///").unwrap()), "///");

        // The fourth `/` is refused by `~['/']`, which leaves the body of the
        // document unfilled: the document is its three slashes, and whatever
        // stands after them belongs to the file to read.
        assert_eq!(text(&parse::<OuterLineDoc<_>>("////").unwrap()), "///");
    }

    /// The table of the forms a comment takes, which the third character
    /// decides.
    #[test]
    fn a_block_group_is_told_apart_by_how_it_opens() {
        let cases = &[
            ("/* a */", "/* a */"),
            ("/**/ ", "/**/"),
            ("/***/ ", "/***/"),
            ("/* a /* nested */ still */", "/* a /* nested */ still */"),
        ];

        for &(source, expected) in cases {
            assert_eq!(
                parse::<BlockComment<_>>(source).map(|node| text(&node)),
                Ok(expected.to_string()),
                "{source}"
            );
            assert!(parse::<BlockCommentOrDoc<_>>(source).is_ok(), "{source}");
        }

        // A document is not a comment, and a comment is not a document.
        assert!(parse::<BlockComment<_>>("/*! a */").is_err());
        assert!(parse::<OuterBlockDoc<_>>("/*! a */").is_err());
        assert!(parse::<InnerBlockDoc<_>>("/* a */").is_err());
        assert!(parse::<OuterBlockDoc<_>>("/**/").is_err());
        assert!(parse::<OuterBlockDoc<_>>("/***/").is_err());

        // `/*/*/ */` opens twice and closes once.
        assert!(parse::<BlockComment<_>>("/*/*/ */").is_err());
        // A lone `/` is only text, so this one does close.
        assert_eq!(text(&parse::<BlockComment<_>>("/*/ */").unwrap()), "/*/ */");
    }

    #[test]
    fn an_unterminated_group_is_answered_by_no_form_of_its_rule() {
        // The comment is a choice of three forms, and none of them is what the
        // text turns out to hold: the refusal comes back as the choice that
        // was never fulfilled, where the opening was read.
        assert_eq!(
            parse::<BlockComment<_>>("/* a "),
            Err(UnsynError::Syntax(
                SyntaxKind::BlockComment,
                ControlFlow::Recovable,
                Span::Range(0..1)
            ))
        );

        // The two documents are one form each, so what is missing is exactly
        // the closing the body ran out before.
        assert_eq!(
            parse::<InnerBlockDoc<_>>("/*! a "),
            Err(UnsynError::Syntax(
                SyntaxKind::InnerBlockDoc,
                ControlFlow::Recovable,
                Span::Range(6..6)
            ))
        );
        assert_eq!(
            parse::<OuterBlockDoc<_>>("/** a "),
            Err(UnsynError::Syntax(
                SyntaxKind::OuterBlockDoc,
                ControlFlow::Recovable,
                Span::Range(6..6)
            ))
        );
    }

    #[test]
    fn a_block_group_of_the_three_forms_is_a_block_comment_or_doc() {
        for source in ["/* a */", "/** a */", "/*! a */"] {
            assert!(parse::<BlockCommentOrDoc<_>>(source).is_ok(), "{source}");
        }

        assert!(parse::<BlockCommentOrDoc<_>>("// a").is_err());
    }
}
