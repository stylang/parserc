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
//! A choice written inside a group belongs to no left side, and the
//! [analyzer](super::semantic) says so of every file that writes one. Where the
//! choice is what a *pattern* has to answer — which of the quantity operators
//! stood on an operand, which of the three forms spelled a bound — the group is
//! a rule of the spec and a node of the tree, because a reader of a pattern is
//! matching on it either way. Where it is only how a token is spelled, the group
//! earns no name and no node either: the token is read as one run of text and
//! [`decode`] says what the run holds. What the spec answers from outside
//! itself — `<IdentOrKeyword>` and its like — is read whole for the same
//! reason.
//!
//! [`decode`]: super::decode
//!
//! # Where the trivia goes
//!
//! No node of this module keeps a run of the trivia. A `lexer` token is read
//! out of the text it is handed, and a run taken there would end up inside that
//! text. The trivia on each side of such a token is therefore the sequence's
//! own: the `syntax` rule that reaches the token holds the run in a field laid
//! beside it, which is why the terminals a `syntax` rule is written with — the
//! puncts and the fixed words — live in [`super::syntax`] instead. See the
//! [crate documentation](crate) for how a run is filed away exactly once.
//!
//! The one consequence worth stating is the order of the alternatives of
//! [`Ident`]. A semantics error is fatal, so once a rule has refused text for
//! meaning nothing no enclosing choice may try another way — and `NonKeywordIdent`
//! reads `r#foo` as the name `r`, which is a name and is not fatal; [`RawIdent`]
//! therefore comes first, and takes the word whole.

use parserc::syntax::{Char, Syntax};
use parserc::{ControlFlow, Input, Kind, Parser, keyword, next, next_if, take_while};
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
// `lexer IdentOrKeyword -> <IdentOrKeyword>;`
// ---------------------------------------------------------------------------

/// A name before it is known whether it is a keyword.
///
/// `<IdentOrKeyword>` is a link the caller of a generated parser binds; the
/// hand tree binds it to the same scan the name has always been: a start —
/// [`is_xid_start`] or `_` — and then the run of characters a name continues
/// with, in one greedy piece. The rule was once written a character at a time,
/// over externals of the Unicode properties; the spec says the name is one
/// token and the scan reads it whole, which is what a name is worth: the word
/// it spells and nothing below it.
///
/// [`is_xid_start`]: unicode_ident::is_xid_start
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdentOrKeyword<I>(#[parserc(parser = read_ident_or_keyword)] pub I)
where
    I: UnsynInput;

/// The scan the external of [`IdentOrKeyword`] is bound to: the start of a
/// name, of either of its two shapes, and then the run that continues it.
fn read_ident_or_keyword<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    next_if(|c: char| is_xid_start(c) || c == '_')
        .parse(input)
        .map_err(SyntaxKind::Ident.map())?;

    take_while(is_xid_continue).parse(input)?;

    Ok(whole.split_to(before - input.len()))
}

impl<I: UnsynInput> IdentOrKeyword<I> {
    /// The whole of the name.
    #[inline]
    pub fn text(&self) -> String {
        self.0.as_str().to_owned()
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
/// A name written with the `r#` sigil, which is how a word of the language is
/// named as a name.
///
/// The sigil and the name are one token of text, which is what the generated
/// code reads the same production as: the name is asked for its text and never
/// for its parts, and a name read in pieces would put a node in the tree that
/// no rule of the language names.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawIdent<I>(#[parserc(parser = read_raw_ident)] pub I)
where
    I: UnsynInput;

/// Reads `'r#' IdentOrKeyword` as one token.
///
/// The sigil is what makes the difference between the two forms of a name, so
/// its refusal is fatal: what stands where it was written is not this name and
/// no other reading of the rule begins there either.
fn read_raw_ident<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    keyword("r#")
        .parse(input)
        .map_err(SyntaxKind::Ident.map())?;

    <IdentOrKeyword<I>>::into_parser()
        .map(|node| node.0)
        .parse(input)?;

    Ok(whole.split_to(before - input.len()))
}

impl<I: UnsynInput> RawIdent<I> {
    /// The whole of the name, sigil and all.
    #[inline]
    pub fn text(&self) -> String {
        self.0.as_str().to_owned()
    }
}

/// A name that is not a word of the language.
///
/// The name is one token of text, as the generated code reads it: what the
/// `except` asks is answered while the name is read and leaves nothing of its
/// own in the tree.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NonKeywordIdent<I>(#[parserc(parser = read_non_keyword_ident)] pub I)
where
    I: UnsynInput;

/// Reads the `IdentOrKeyword except Keywords` of [`NonKeywordIdent`] as one
/// token of text.
///
/// `except` asks its question of the text the operand to its left read and of
/// nothing else: the removal runs over that text on its own, and only a removal
/// that takes all of it refuses the name. `lexerX` is therefore a name of its
/// own, although `lexer` fits the head of it.
fn read_non_keyword_ident<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let name = IdentOrKeyword::into_parser().parse(input)?;

    let mut rest = name.0.clone();

    if Keywords::into_parser().ok().parse(&mut rest)?.is_some() && rest.is_empty() {
        return Err(UnsynError::Semantics(
            SemanticsKind::Keyword,
            name.to_span(),
        ));
    }

    Ok(name.0)
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
            Ident::NonKeyword(ident) => ident.0.as_str().to_owned(),
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

/// A decimal number, which is how the bound of a repetition is written.
///
/// The first digit is a node because the rule names it; every character after
/// it is one run of text, which is how the generated code reads the same
/// production — `take_while` over the two characters the rule allows, and the
/// fastest reading of a number a grammar can name. An underscore is part of the
/// token and carries no value, so the count is taken from the text with the
/// separators dropped, which is what `count` does.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitDec<I>
where
    I: UnsynInput,
{
    /// The digit the number begins with.
    pub head: DecDigit<I>,
    /// Every digit and separator after it, as one run.
    #[parserc(parser = take_while(|c: char| matches!(c, '0'..='9' | '_')))]
    pub tail: I,
}

impl<I: UnsynInput> LitDec<I> {
    /// The number the digits spell, with the underscores the rule allows
    /// between them dropped.
    ///
    /// `None` says the number is past what a count of repetitions can be kept
    /// to, which no reading of the digits can turn into a count.
    pub fn count(&self) -> Option<u32> {
        let mut digits = String::from(self.head.0.as_str());

        digits.extend(self.tail.as_str().chars().filter(|c| *c != '_'));

        digits.parse::<u32>().ok()
    }

    /// The text of the literal, underscores and all.
    pub fn text(&self) -> String {
        let mut text = String::from(self.head.0.as_str());

        text.push_str(self.tail.as_str());

        text
    }
}

// ---------------------------------------------------------------------------
// `lexer LitUnicode -> 'U+' concat HexDigit{4};`
// ---------------------------------------------------------------------------

/// A code point written in a string, as `U+0009` is.
///
/// The literal is one token of text, read as the rule writes it: the `U+` the
/// spec puts there with `concat`, and then exactly four hex digits — four and
/// not five, because `concat` leaves nothing between the two operands for a
/// fifth digit to be read as anything else. The digits are not nodes of the
/// tree: this is the shape the generated code gives the same production, and a
/// code point is asked for its value and never for its parts.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitUnicode<I>(#[parserc(parser = read_lit_unicode)] pub I)
where
    I: UnsynInput;

/// Reads `'U+' concat HexDigit{4}`.
///
/// Four digits and no more are taken, exactly as the bound says: a fifth digit
/// is not part of this token and belongs to whatever operand comes next, which
/// is what the generated code does with the same production and the two
/// readings have to accept the same text for one spec to compile itself.
///
/// The digits are counted on a copy, so a refusal leaves them where they
/// stand: only the `U+` is gone, and the span of the refusal can point at
/// every digit that was read rather than at the one that stopped the count.
fn read_lit_unicode<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    let prefix = keyword("U+")
        .parse(input)
        .map_err(SyntaxKind::Unicode.map())?;

    let mut rest = input.clone();
    let digits = take_while(|c: char| c.is_ascii_hexdigit()).parse(&mut rest)?;

    if digits.len() < 4 {
        return Err(UnsynError::Semantics(
            SemanticsKind::Unicode,
            prefix.to_span().union(&digits.to_span()),
        ));
    }

    _ = input.split_to(4);

    Ok(whole.split_to(before - input.len()))
}

impl<I: UnsynInput> LitUnicode<I> {
    /// The code point the digits spell, when they spell a character.
    ///
    /// Four hex digits is the shape the reader counted, so the only thing left
    /// to answer is whether the number of them is a character, which `D800`
    /// through `DFFF` are not.
    pub fn value(&self) -> Option<char> {
        let digits = self.0.as_str().strip_prefix("U+")?;

        u32::from_str_radix(digits, 16)
            .ok()
            .and_then(char::from_u32)
    }

    /// The text of the literal, `U+` and all.
    pub fn text(&self) -> String {
        self.0.as_str().to_string()
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

/// A string literal: any run of characters between two `'`, in which a `'`, a
/// `\` and a `CR` can only be written by an escape.
///
/// The body is one run of text, which is the shape the generated code gives the
/// same production: the pieces are read as tokens only so that the run stops
/// where a piece does not begin, and what the tree keeps is the text they made
/// up. What the literal spells is read back from that text by [`decode`],
/// because a member of a set and the end of a range are both asked what a
/// literal *spells* and never how it was spelled.
///
/// [`decode`]: super::decode
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitStr<I>
where
    I: UnsynInput,
{
    /// The quote that opens the literal.
    #[parserc(keyword = "'", map_err = PunctKind::SingleQuote.map())]
    pub open: I,
    /// The body of the literal, pieces of plain text and escapes together.
    ///
    /// The repetition ends where no piece begins, which is the closing quote
    /// wherever one stands and whatever the text after the pieces otherwise
    /// is; the quote that has to come next says what was expected there.
    #[parserc(parser = read_lit_str_body)]
    pub body: I,
    /// The quote that closes the literal.
    #[parserc(keyword = "'", map_err = PunctKind::SingleQuote.map())]
    pub close: I,
}

/// Reads the body of a string literal as one run of text.
///
/// The four pieces the rule lists are tried one after another, and each is
/// thrown away once it has been read: they bound the run and say nothing the
/// tree does not already hold. A piece that does not begin is the end of the
/// body, so the loop stops there with the input untouched and the closing
/// quote — or whatever else stands there — is left for the rule above.
fn read_lit_str_body<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let mut taken = 0;

    loop {
        let here = input.len();

        let piece = next_if(|c: char| c != '\'' && c != '\\' && c != '\r')
            .map(|_| ())
            .or(<QuoteEscape<I>>::into_parser().map(|_| ()))
            .or(<AsciiEscape<I>>::into_parser().map(|_| ()))
            .or(<UnicodeEscape<I>>::into_parser().map(|_| ()))
            .ok();

        if piece.parse(input)?.is_none() {
            break;
        }

        taken += here - input.len();
    }

    Ok(whole.split_to(taken))
}

impl<I: UnsynInput> LitStr<I> {
    /// What the body spells: the characters of the literal, and the span of
    /// every escape among them that spells no character.
    ///
    /// The two quotes bound the literal and carry no value, so only the body is
    /// read, and the offsets the refusals carry are the source's because the
    /// body says where in the source it began.
    pub fn decoded(&self) -> super::decode::Body {
        super::decode::string_body(self.body.as_str(), self.body.start())
    }

    /// The characters the literal spells.
    ///
    /// An escape of no character yields no character: the refusal it is worth
    /// is in [`decoded`](LitStr::decoded), and the pass that reads a value is
    /// the one to say so from that span.
    pub fn chars(&self) -> Vec<char> {
        self.decoded().chars
    }

    /// The text of the literal, both quotes and all, as it was written.
    pub fn text(&self) -> String {
        let mut text = String::from(self.open.as_str());

        text.push_str(self.body.as_str());

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

/// A line document that describes the definition after it.
///
/// The body is one run of text, as the generated code reads it: the rule asks
/// for it at most once, and what it holds is a character that is no slash
/// followed by the rest of the line — a run the tree keeps whole, because a
/// document is trivia and nothing asks it for its pieces.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OuterLineDoc<I>
where
    I: UnsynInput,
{
    /// The `///` that opens the document.
    #[parserc(keyword = "///", map_err = SyntaxKind::OuterLineDoc.map())]
    pub open: I,
    /// The body, empty when the rule's one optional group is not filled: the
    /// fourth `/` is refused by what the group asks first, which leaves the
    /// body unfilled, and what stands after the document is for the file to
    /// say.
    #[parserc(parser = read_outer_line_doc_body)]
    pub body: I,
}

/// Reads the one optional group of [`OuterLineDoc`] as a run of text.
///
/// The group is optional as a whole, so a body that does not begin is no error
/// and the run it would have made is empty.
fn read_outer_line_doc_body<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    let body = |input: &mut I| {
        next_if(|c: char| c != '/').parse(input)?;
        take_while(stays_on_the_line).parse(input)?;
        Ok(())
    };

    _ = body.ok().parse(input)?;

    Ok(whole.split_to(before - input.len()))
}

impl<I: UnsynInput> OuterLineDoc<I> {
    /// The text of the document, as it was written.
    #[inline]
    pub fn text(&self) -> String {
        let mut text = String::from(self.open.as_str());

        text.push_str(self.body.as_str());

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

/// The whole of a block comment: the body form, or one of the two closed forms.
///
/// The alternatives come in the order the spec writes them. `/***/` is still
/// read whole, because the body form takes `/*`, the two stars of its head and
/// the closing that follows them, which is five characters — so the shorter
/// closed form never wins the text the longer one means.
fn read_block_comment<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    read_block_comment_body
        .or(keyword("/**/").map_err(SyntaxKind::BlockComment.map()))
        .or(keyword("/***/").map_err(SyntaxKind::BlockComment.map()))
        .parse(input)
}

/// Reads the first alternative of [`BlockComment`]: the opening, the head, the
/// body and the closing of one comment.
fn read_block_comment_body<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    keyword("/*")
        .parse(input)
        .map_err(SyntaxKind::BlockComment.map())?;

    // The head: one character that is no star, no bang and opens no group; the
    // two stars of a comment that closes at once; or a group of its own.
    head_plain
        .or(keyword("**").map_err(SyntaxKind::BlockComment.map()))
        .or(<BlockCommentOrDoc<I>>::into_parser().map(|node| node.0))
        .parse(input)?;

    // The body, until the group that closes here arrives.
    loop {
        let part = <BlockCommentOrDoc<I>>::into_parser()
            .map(|node| node.0)
            .or(tail_plain)
            .ok()
            .parse(input)?;

        if part.is_none() {
            break;
        }
    }

    keyword("*/")
        .parse(input)
        .map_err(SyntaxKind::BlockComment.map())?;

    Ok(whole.split_to(before - input.len()))
}

/// Reads [`InnerBlockDoc`]: its opening, a body of groups and text, and the
/// closing of its own.
fn read_inner_block_doc<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    keyword("/*!")
        .parse(input)
        .map_err(SyntaxKind::InnerBlockDoc.map())?;

    loop {
        let part = <BlockCommentOrDoc<I>>::into_parser()
            .map(|node| node.0)
            .or(tail_plain)
            .ok()
            .parse(input)?;

        if part.is_none() {
            break;
        }
    }

    keyword("*/")
        .parse(input)
        .map_err(SyntaxKind::InnerBlockDoc.map())?;

    Ok(whole.split_to(before - input.len()))
}

/// Reads [`OuterBlockDoc`]: its opening of three, the head that is never a third
/// star, the body, and the closing.
fn read_outer_block_doc<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    keyword("/**")
        .parse(input)
        .map_err(SyntaxKind::OuterBlockDoc.map())?;

    outer_head_plain
        .or(<BlockCommentOrDoc<I>>::into_parser().map(|node| node.0))
        .parse(input)?;

    loop {
        let part = <BlockCommentOrDoc<I>>::into_parser()
            .map(|node| node.0)
            .or(tail_plain)
            .ok()
            .parse(input)?;

        if part.is_none() {
            break;
        }
    }

    keyword("*/")
        .parse(input)
        .map_err(SyntaxKind::OuterBlockDoc.map())?;

    Ok(whole.split_to(before - input.len()))
}

/// Reads [`BlockCommentOrDoc`], the one rule that nests.
///
/// The order is the order of the spec, and it matters only for `/***`, where a
/// comment and an outer document both begin with the three characters and the
/// fourth decides: the comment is read first, and the text an outer document
/// would have taken is refused, so the choice settles on the comment.
fn read_block_comment_or_doc<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    <BlockComment<I>>::into_parser()
        .map(|node| node.0)
        .or(<OuterBlockDoc<I>>::into_parser().map(|node| node.0))
        .or(<InnerBlockDoc<I>>::into_parser().map(|node| node.0))
        .parse(input)
}

/// A block comment, which nests and is closed by the `*/` of its own opening.
///
/// The comment is one token of text, as the generated code reads it: what a
/// pass asks of a comment is nothing at all — it is trivia — and the parts the
/// hand tree used to keep were only how the reading found the closing that
/// belongs to this opening.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockComment<I>(#[parserc(parser = read_block_comment)] pub I)
where
    I: UnsynInput;

/// A block document that describes the file it is written in.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InnerBlockDoc<I>(#[parserc(parser = read_inner_block_doc)] pub I)
where
    I: UnsynInput;

/// A block document that describes the definition after it.
///
/// The opening of three refuses what the head set keeps out of the body, which
/// is what leaves `/**/` and `/***/` to the block comment: an opening of three
/// is a comment that closes at once, and not an empty document.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OuterBlockDoc<I>(#[parserc(parser = read_outer_block_doc)] pub I)
where
    I: UnsynInput;

/// A block group, which is the one rule that nests: a comment may hold a
/// document, and a document may hold a comment.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockCommentOrDoc<I>(#[parserc(parser = read_block_comment_or_doc)] pub I)
where
    I: UnsynInput;

/// The text of a block group, as it was written.
///
/// The four forms answer for their text the same way, so this is what each
/// `text` of the family is.
impl<I: UnsynInput> BlockComment<I> {
    /// The text of the comment, as it was written.
    pub fn text(&self) -> String {
        self.0.as_str().to_owned()
    }
}

impl<I: UnsynInput> InnerBlockDoc<I> {
    /// The text of the document, as it was written.
    pub fn text(&self) -> String {
        self.0.as_str().to_owned()
    }
}

impl<I: UnsynInput> OuterBlockDoc<I> {
    /// The text of the document, as it was written.
    pub fn text(&self) -> String {
        self.0.as_str().to_owned()
    }
}

impl<I: UnsynInput> BlockCommentOrDoc<I> {
    /// The text of the group, as it was written.
    pub fn text(&self) -> String {
        self.0.as_str().to_owned()
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
        IdentOrKeyword,
        OctDigit,
        DecDigit,
        HexDigit,
        QuoteEscape,
    );

    // The nodes that are modelled as the shapes their right sides write answer
    // for their text from those shapes.

    impl ToText for RawIdent<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.text()
        }
    }

    impl ToText for NonKeywordIdent<TokenStream<'_>> {
        #[inline]
        fn text(&self) -> String {
            self.0.as_str().to_owned()
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

    impl ToText for BlockCommentOrDoc<TokenStream<'_>> {
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

        // Four digits is the whole token. A fifth is not a miswritten literal
        // but the text that follows it, which is what the bound says and what
        // the generated reading of the same rule does — the two parsers have
        // to accept one language for a spec to compile itself.
        let mut input = TokenStream::from("U+11234");
        let lit = input.parse::<LitUnicode<_>>().unwrap();
        assert_eq!(lit.to_span(), Span::Range(0..6));
        assert_eq!(lit.text(), "U+1123");
        assert_eq!(input.as_str(), "4");

        // Fewer than four is the literal written short, and the span of the
        // refusal covers the `U+` and every digit that was read.
        for (source, span) in [("U+1", 0..3), ("U+12_3", 0..4), ("U+0x29", 0..3)] {
            assert_eq!(
                parse::<LitUnicode<_>>(source),
                Err(UnsynError::Semantics(
                    SemanticsKind::Unicode,
                    Span::Range(span)
                )),
                "{source}"
            );
        }

        // A refusal leaves the digits where they stand: the `U+` is gone, and
        // what the rule could not finish is still there to be read again.
        let mut input = TokenStream::from("U+12_3");
        assert!(input.parse::<LitUnicode<_>>().is_err());
        assert_eq!(input.as_str(), "12_3");
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
        // text turns out to hold: the refusal is the longest form the rule
        // could have been, which is what the generated reading of the same
        // rule answers too — the two now report the same place, and differ
        // only in that the hand file names what it expected.
        assert_eq!(
            parse::<BlockComment<_>>("/* a "),
            Err(UnsynError::Syntax(
                SyntaxKind::BlockComment,
                ControlFlow::Recovable,
                Span::Range(0..5)
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

    /// What the two document forms do with a group nested in them, pinned here
    /// before the reading of a block group collapses into text: the tree will
    /// not say these parts again, so this is where the language says what it
    /// accepts.
    #[test]
    fn a_document_holds_the_group_it_nests_and_closes_once() {
        for source in [
            "/** outer /* nested */ doc */",
            "/** two /* a */ /* b */ */",
            "/*! inner /* x */ doc */",
            "/*! inner /*! no */ */",
            "/** a /* b /* c */ d */ e */",
        ] {
            assert_eq!(
                parse::<BlockCommentOrDoc<_>>(source).map(|node| text(&node)),
                Ok(source.to_string()),
                "{source}"
            );
        }

        // The opening of three stars is the comment's, and the document of the
        // same opening never sees the fourth character.
        assert_eq!(text(&parse::<BlockComment<_>>("/***/").unwrap()), "/***/");
        assert!(parse::<OuterBlockDoc<_>>("/***/").is_err());

        // A nested group that closes the outer one leaves nothing to close the
        // rest with, so no form of the rule holds.
        assert!(parse::<BlockCommentOrDoc<_>>("/** a /* b */").is_err());
    }
}
