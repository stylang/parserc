//! The terminals of the DSL: the `whitespace` rule and the `lexer` rules that
//! `unsyn.us` beside this crate writes.
//!
//! One production, one type: every type here is named for the left side of its
//! production, and its right side is spelled out with the nodes and combinators
//! of [`parserc`], so that the declaration of a type is the declaration of the
//! rule. Where a right side is a sequence of other rules, the type reads them in
//! that order and keeps their extent rather than their parts.
//!
//! # A lexer node is the text it read
//!
//! A `lexer` rule denotes a set of strings rather than a region of the tree, so
//! its node keeps the whole text as one [`UnsynInput`] stream and none of the
//! parts that read it: `IdentOrKeyword` holds the name, and the digits inside
//! [`UnicodeEscape`] hold no list of their own. What those parts are worth is
//! their extent, which the text already records.
//!
//! The exception is a rule whose whole right side is a choice of other rules by
//! name — [`Ident`], [`Keywords`] and [`BlockCommentOrDoc`] — where keeping the
//! choice says *which* rule was read, and the text alone does not.
//!
//! # Where the trivia goes
//!
//! No node of this module owns a run of the trivia. A `lexer` token is read out
//! of the text it is handed, and a run taken there would end up inside that text;
//! the whitespace on each side of a token belongs to the `syntax` rule that
//! reaches it, which is why the terminals a `syntax` rule is written with — the
//! puncts and the fixed words — live in [`super::syntax`] instead. See the [crate
//! documentation](crate) for how a run is divided between two neighbours.
//!
//! The one consequence worth stating is the order of the alternatives of
//! [`Ident`]. A semantics error is [`ControlFlow::Fatal`], so once a rule has
//! refused text for meaning nothing no enclosing choice may try another way
//! `NonKeywordIdent` reads `r#foo` as the name `r`, which is a name and is not
//! fatal; [`RawIdent`] therefore comes first, and takes the word whole.

use parserc::{
    ControlFlow, Parser, keyword, next, next_if,
    syntax::{Char, Or, Syntax},
    take_while,
};
use unicode_ident::{is_xid_continue, is_xid_start};

use crate::codegen::errors::{KeywordKind, PunctKind, SemanticsKind, SyntaxKind, UnsynError};
use crate::codegen::input::UnsynInput;

/// Matches one fixed word of the language, refusing the tail of a longer name.
///
/// A keyword is matched as a literal prefix, which is not enough on its own:
/// `lexerX` is one name, not the keyword `lexer` and an `X`. A word that leaves
/// characters behind did not take the word, so the match is refused and the
/// caller reads the text as a name instead.
pub(crate) fn keyword_word<'a, I>(
    word: &'static str,
    kind: KeywordKind,
) -> impl Parser<I, Output = I> + 'a
where
    I: UnsynInput + 'a,
{
    move |input: &mut I| {
        let whole = input.clone();
        let text = keyword(word).parse(input).map_err(kind.map())?;

        if let Some(c) = input.iter().next()
            && continues_a_name(c)
        {
            // The word is only the head of the name written here, and a name is
            // one token, so the report reaches over the whole of it.
            let tail = take_while(continues_a_name)
                .parse(&mut input.clone())?
                .len();

            return Err(UnsynError::Keyword(
                kind,
                ControlFlow::Recovable,
                whole.to_span_at(text.len() + tail),
            ));
        }

        Ok(text)
    }
}

/// The words a definition opens with, in the order the rule lists them.
const LEADING_WORDS: [(&str, KeywordKind); 3] = [
    ("lexer", KeywordKind::Lexer),
    ("syntax", KeywordKind::Syntax),
    ("whitespace", KeywordKind::Whitespace),
];

/// The words that join two operands of a pattern, in the order listed.
const FOLLOWING_WORDS: [(&str, KeywordKind); 3] = [
    ("concat", KeywordKind::Concat),
    ("except", KeywordKind::Except),
    ("followed", KeywordKind::Followed),
];

/// Reads one of the words of a set, in the order the rule lists them.
///
/// The whole of the name written here decides which word of the set it can be,
/// since no word of a set begins another one: a name that only begins with a
/// word is that name and not the word, and the refusal is the word's own rather
/// than the last word the set happened to try. Where no word of the set begins
/// at all, it is the set that the position is missing.
fn keyword_set<I>(
    input: &mut I,
    words: &[(&str, KeywordKind)],
    missing: SyntaxKind,
) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut rest = input.clone();
    let name = take_while(continues_a_name).parse(&mut rest)?;
    let reached = name.len();
    let text = name.as_str();

    for (word, kind) in words {
        if *word == text {
            return Ok(input.split_to(word.len()));
        }

        if text.starts_with(word) {
            return Err(UnsynError::Keyword(
                *kind,
                ControlFlow::Recovable,
                input.to_span_at(reached),
            ));
        }
    }

    Err(UnsynError::Syntax(
        missing,
        ControlFlow::Recovable,
        input.to_span_at(reached),
    ))
}

/// Whether `c` is what an identifier continues with, which is also what makes a
/// keyword only the head of a longer word.
#[inline]
fn continues_a_name(c: char) -> bool {
    is_xid_continue(c) || c == '#'
}

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

/// A name before it is known whether it is a keyword: a start, then any run of
/// continuing characters.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdentOrKeyword<I>(#[parserc(parser = read_ident_or_keyword)] pub I)
where
    I: UnsynInput;

/// Reads the `( XID_Start | '_' ) XID_Continue*` of [`IdentOrKeyword`].
///
/// The repetition of a rule that keeps only its text is read through that rule,
/// so that the character class of the spec is stated once; what the run is worth
/// here is how far it reached, which is the length of the text it took.
fn read_ident_or_keyword<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    Or::<XID_Start<I>, Char<I, '_'>>::into_parser().parse(input)?;

    while XID_Continue::into_parser().ok().parse(input)?.is_some() {}

    Ok(whole.split_to(before - input.len()))
}

// ---------------------------------------------------------------------------
// `lexer RawIdent -> 'r#' IdentOrKeyword;`
// `lexer NonKeywordIdent -> IdentOrKeyword except Keywords;`
// `lexer Ident -> NonKeywordIdent | RawIdent;`
// ---------------------------------------------------------------------------

/// A name written out of the keywords of the language, as `r#where` is.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawIdent<I>(#[parserc(parser = read_raw_ident)] pub I)
where
    I: UnsynInput;

/// Reads the `'r#' IdentOrKeyword` of [`RawIdent`].
///
/// The name after the sigil is read by [`IdentOrKeyword`] and not by
/// [`NonKeywordIdent`], which is what lets a raw name be a word of the language.
fn read_raw_ident<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    keyword("r#")
        .parse(input)
        .map_err(SyntaxKind::Ident.map())?;
    IdentOrKeyword::into_parser().parse(input)?;

    Ok(whole.split_to(before - input.len()))
}

/// A name that is not a word of the language.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NonKeywordIdent<I>(#[parserc(parser = read_non_keyword_ident)] pub I)
where
    I: UnsynInput;

/// Reads the `IdentOrKeyword except Keywords` of [`NonKeywordIdent`].
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
    pub fn text(&self) -> &I {
        match self {
            Ident::Raw(raw) => &raw.0,
            Ident::NonKeyword(ident) => &ident.0,
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
/// declaration from a `lexer` one and from a `syntax` one, and it is read from
/// the text of this node, since the three words are literals of the spec rather
/// than rules of it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LeadingKeywords<I>(#[parserc(parser = read_leading_keywords)] pub I)
where
    I: UnsynInput;

/// Reads the three words [`LeadingKeywords`] is written out of.
fn read_leading_keywords<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    keyword_set(input, &LEADING_WORDS, SyntaxKind::LeadingKeywords)
}

/// The word that joins two operands of a pattern into one construct.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FollowingKeywords<I>(#[parserc(parser = read_following_keywords)] pub I)
where
    I: UnsynInput;

/// Reads the three words [`FollowingKeywords`] is written out of.
fn read_following_keywords<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    keyword_set(input, &FOLLOWING_WORDS, SyntaxKind::FollowingKeywords)
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
/// An underscore is part of the token and carries no value; the pass that reads
/// the number drops them.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitDec<I>(#[parserc(parser = read_lit_dec)] pub I)
where
    I: UnsynInput;

/// Reads the `DecDigit ( DecDigit | '_' )*` of [`LitDec`].
fn read_lit_dec<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    DecDigit::into_parser().parse(input)?;

    while Or::<DecDigit<I>, Char<I, '_'>>::into_parser()
        .ok()
        .parse(input)?
        .is_some()
    {}

    Ok(whole.split_to(before - input.len()))
}

// ---------------------------------------------------------------------------
// `lexer LitUnicode -> 'U+' concat HexDigit{4};`
// ---------------------------------------------------------------------------

/// A code point written in a string, as `U+0009` is.
///
/// unsyn asks for exactly four hex digits after `U+`, where Rust allows four to
/// six, because the token it returns is always six characters long.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitUnicode<I>(#[parserc(parser = read_lit_unicode)] pub I)
where
    I: UnsynInput;

/// Reads the `'U+' concat HexDigit{4}` of [`LitUnicode`].
///
/// `concat` joins the two operands with nothing between them, so a fifth digit
/// cannot be the start of the next token: it is a bound written wrong, and the
/// literal is refused.
fn read_lit_unicode<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
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

    Ok(whole.split_to(prefix.len() + digits.len()))
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
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AsciiEscape<I>(
    #[parserc(
        parser = read_ascii_7bit_escape
            .or(keyword("\\n"))
            .or(keyword("\\r"))
            .or(keyword("\\t"))
            .or(keyword("\\\\"))
            .or(keyword("\\0"))
    )]
    pub I,
)
where
    I: UnsynInput;

/// Reads the `'\\x' OctDigit HexDigit` alternative: a code of seven bits, which
/// is why the first digit is refused above `7`.
fn read_ascii_7bit_escape<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();

    keyword("\\x")
        .parse(input)
        .map_err(SyntaxKind::AsciiEscape.map())?;

    for i in 0..2 {
        let Some(c) = input.iter().next() else {
            return Err(UnsynError::Semantics(
                SemanticsKind::Char7BitEscapeTooShort,
                input.to_span(),
            ));
        };

        if !c.is_ascii_hexdigit() {
            return Err(UnsynError::Semantics(
                SemanticsKind::HexDigit,
                input.to_span_at(1),
            ));
        }

        // The pair of digits is `OctDigit HexDigit`, so only the first of them
        // is bounded: `0x80` and above is not a character ASCII writes.
        if i == 0 && c > '7' {
            return Err(UnsynError::Semantics(
                SemanticsKind::Char7BitEscapeOutOfRange,
                input.to_span_at(1),
            ));
        }

        next(c).parse(input)?;
    }

    Ok(whole.split_to(4))
}

/// An escape of a code point, as `\u{1F600}` is.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnicodeEscape<I>(#[parserc(parser = read_unicode_escape)] pub I)
where
    I: UnsynInput;

/// Reads the `'\\u{' ( HexDigit '_'* ){1, 6} '}'` of [`UnicodeEscape`].
///
/// The bound is of the digits and not of the text, since an underscore may
/// separate them; it is counted here rather than read as a repetition because
/// `{1,6}` bounds the number of digits, which is what says how long the escape
/// may be.
fn read_unicode_escape<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    keyword("\\u{")
        .parse(input)
        .map_err(SyntaxKind::UnicodeEscape.map())?;

    let mut digits = 0;

    while digits < 6 {
        if HexDigit::into_parser().ok().parse(input)?.is_none() {
            break;
        }

        digits += 1;

        // `'_'*`: the separators of a digit belong to the digit they follow.
        Vec::<Char<I, '_'>>::into_parser().parse(input)?;
    }

    // Either no digit at all, which the bound refuses, or a seventh, which no
    // code point needs.
    if digits == 0 || matches!(input.iter().next(), Some(c) if c.is_ascii_hexdigit()) {
        return Err(UnsynError::Semantics(
            SemanticsKind::UnicodeEscape,
            input.to_span_at(1),
        ));
    }

    keyword("}")
        .parse(input)
        .map_err(PunctKind::BraceEnd.map())?;

    Ok(whole.split_to(before - input.len()))
}

// ---------------------------------------------------------------------------
// `lexer LitStr -> '\'' ( ~['"','\\',CR] | QuoteEscape | AsciiEscape`
// `                   | UnicodeEscape )* '\'';`
// ---------------------------------------------------------------------------

/// A string literal: any run of characters between two `'`, in which a `'`, a
/// `\` and a `CR` can only be written by an escape.
///
/// The content stops at the first `'` that is not part of an escape, so a
/// literal can never hold a bare apostrophe.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitStr<I>(#[parserc(parser = read_lit_str)] pub I)
where
    I: UnsynInput;

/// Reads the `'\''. ( ~[…]|QuoteEscape|AsciiEscape|UnicodeEscape )* '\''` of
/// [`LitStr`].
fn read_lit_str<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    keyword("'")
        .parse(input)
        .map_err(PunctKind::SingleQuote.map())?;

    loop {
        // `~['\'','\\',CR]`: the plain text of the body, taken in one piece up
        // to the first character that carries a meaning of its own.
        _ = take_while(|c: char| c != '\'' && c != '\\' && c != '\r').parse(input)?;

        match input.iter().next() {
            // The body ends where the closing quote begins.
            Some('\'') => break,
            // A `CR` is refused by the set, and no escape writes it either.
            Some('\r') => {
                return Err(UnsynError::Semantics(
                    SemanticsKind::StrContent,
                    input.to_span_at(1),
                ));
            }
            Some('\\') => {
                if Or::<QuoteEscape<I>, Or<AsciiEscape<I>, UnicodeEscape<I>>>::into_parser()
                    .ok()
                    .parse(input)?
                    .is_none()
                {
                    // A `\` that opens none of the three escapes belongs to no
                    // operand of the body; the missing quote is reported there.
                    break;
                }
            }
            // Nothing is left to read, and no quote closed the literal.
            None => {
                return Err(UnsynError::Syntax(
                    SyntaxKind::StrContent,
                    ControlFlow::Recovable,
                    input.to_span(),
                ));
            }
            Some(_) => unreachable!("the plain text of a body is taken greedily"),
        }
    }

    keyword("'")
        .parse(input)
        .map_err(PunctKind::SingleQuote.map())?;

    Ok(whole.split_to(before - input.len()))
}

// ---------------------------------------------------------------------------
// `lexer LineComment -> '//' (~['/','!',LF] | '//') ~[LF]*;`
// `lexer InnerLineDoc -> '//!' ~[LF,CR]*;`
// `lexer OuterLineDoc -> '///' ( ~['/'] ~[LF,CR]* )?;`
// ---------------------------------------------------------------------------

/// A line comment, which reaches to the end of its line and holds none of it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineComment<I>(#[parserc(parser = read_line_comment)] pub I)
where
    I: UnsynInput;

/// Reads the `'//' (~['/','!',LF] | '//') ~[LF]*` of [`LineComment`].
///
/// The two forms the rule separates by their first character after the `//` are
/// read by the length of the run of `/`: three of them open an outer document,
/// and a `!` opens an inner one, so neither is a comment.
fn read_line_comment<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();

    let header = take_while(|c: char| c == '/').parse(input)?;

    match header.len() {
        // Not a comment at all: `~['/','!',LF]` has nothing to take.
        0 | 1 | 3 => {
            return Err(UnsynError::Syntax(
                SyntaxKind::LineComment,
                ControlFlow::Recovable,
                header.to_span(),
            ));
        }
        // `//!` is an inner document, and `//` with no text after it is the
        // `'//'` alternative with no `~[LF]*` to follow it.
        2 => match input.iter().next() {
            Some('!') | Some('\n') => {
                return Err(UnsynError::Syntax(
                    SyntaxKind::LineComment,
                    ControlFlow::Recovable,
                    whole.to_span_at(3),
                ));
            }
            _ => {}
        },
        // Four or more: the `'//'` alternative took a pair, and the rest of the
        // line is `~[LF]*`.
        _ => {}
    }

    let body = take_while(|c: char| c != '\n').parse(input)?;

    Ok(whole.split_to(header.len() + body.len()))
}

/// A line document that describes the file it is written in.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InnerLineDoc<I>(#[parserc(parser = read_inner_line_doc)] pub I)
where
    I: UnsynInput;

/// Reads the `'//!' ~[LF,CR]*` of [`InnerLineDoc`].
fn read_inner_line_doc<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();

    keyword("//!")
        .parse(input)
        .map_err(SyntaxKind::InnerLineDoc.map())?;

    let body = take_while(|c: char| c != '\n' && c != '\r').parse(input)?;

    Ok(whole.split_to(3 + body.len()))
}

/// A line document that describes the definition after it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OuterLineDoc<I>(#[parserc(parser = read_outer_line_doc)] pub I)
where
    I: UnsynInput;

/// Reads the `'///' ( ~['/'] ~[LF,CR]* )?` of [`OuterLineDoc`].
///
/// The body is optional, so `////` is a comment rather than an empty document, as
/// in Rust: the fourth `/` is refused by `~['/']`, which leaves the option
/// unfilled only when there is nothing to fill it with.
fn read_outer_line_doc<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();

    keyword("///")
        .parse(input)
        .map_err(SyntaxKind::OuterLineDoc.map())?;

    if let Some('/') = input.iter().next() {
        return Err(UnsynError::Syntax(
            SyntaxKind::OuterLineDoc,
            ControlFlow::Recovable,
            whole.to_span_at(4),
        ));
    }

    let body = take_while(|c: char| c != '\n' && c != '\r').parse(input)?;

    Ok(whole.split_to(3 + body.len()))
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

/// Scans the body of a block group whose opening was already taken.
///
/// The three rules that need it say the same thing their own way: a `/*` in the
/// text of a body can only be read by the recursive alternative of the rule, so
/// every opening needs a closing of its own. An opening whose closing never
/// arrives leaves the whole group unterminated even when a `*/` comes later, so
/// `/*/*/ */` is a broken comment rather than one that ends at that `* /`, which
/// is how Rust reads them. A `/` that is not part of an opening is only text, so
/// `/*/ */` does close.
///
/// `opened` is the length of the prefix already taken, which is also where the
/// count starts, and the result is the length of the whole group, its closing
/// `*/` included. `unterminated` builds the report for a group that ran out of
/// input, over the text it had to swallow to find nothing.
fn scan_block<I, F>(input: &mut I, opened: usize, unterminated: F) -> Result<usize, UnsynError>
where
    I: UnsynInput,
    F: FnOnce(usize) -> UnsynError,
{
    let mut offset = opened;
    let mut depth = 1usize;

    loop {
        // Anything that is neither pair is text, and skipping it in one piece is
        // what leaves the interesting decisions to the three branches below.
        offset += take_while(|c: char| c != '*' && c != '/')
            .parse(input)?
            .len();

        let Some(current) = input.iter().next() else {
            return Err(unterminated(offset));
        };

        if keyword("*/").ok().parse(input)?.is_some() {
            offset += 2;
            depth -= 1;

            if depth == 0 {
                return Ok(offset);
            }
        } else if keyword("/*").ok().parse(input)?.is_some() {
            // A group opened here has to close before this one does: nothing
            // else in this loop takes a `/*`, so an opening is a commitment
            // rather than one alternative among several.
            offset += 2;
            depth += 1;
        } else {
            // A lone `*` or `/` is text. It is also the only progress this
            // position can make, which is why taking it matters. Both are one
            // byte long, which is what lets the offset count by one.
            next(current).parse(input)?;
            offset += 1;
        }
    }
}

/// A block comment, which nests and is closed by the `*/` of its own opening.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockComment<I>(#[parserc(parser = read_block_comment)] pub I)
where
    I: UnsynInput;

/// Reads the three alternatives of [`BlockComment`].
///
/// `/**/` and `/***/` are written out by the rule because the body it asks for
/// cannot be empty, and both are the whole of their own token: the empty comment
/// is the second character of the opening closing the first, and the second `*`
/// of the four is text.
fn read_block_comment<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();

    keyword("/*")
        .parse(input)
        .map_err(SyntaxKind::BlockComment.map())?;

    let rest = input.clone();

    match rest.iter().next() {
        // `/*!` opens an inner document, which the first set of the rule keeps
        // out of a comment.
        Some('!') => {
            return Err(UnsynError::Syntax(
                SyntaxKind::BlockComment,
                ControlFlow::Recovable,
                whole.to_span_at(3),
            ));
        }
        Some('*') => {
            let mut after = input.clone();
            _ = next_if(|c: char| c == '*').parse(&mut after)?;

            match after.iter().next() {
                // `/***/` is the third alternative of the rule, whose body is the
                // single `*` the opening cannot take.
                Some('*') => {}
                // `/**/` is the second: the empty body closes on the very `*` of
                // the opening, so the token is four characters and no more.
                Some('/') => return Ok(whole.split_to(4)),
                _ => {
                    return Err(UnsynError::Syntax(
                        SyntaxKind::BlockComment,
                        ControlFlow::Recovable,
                        whole.to_span_at(3),
                    ));
                }
            }
        }
        _ => {}
    }

    let offset = scan_block(input, 2, |offset| {
        UnsynError::Syntax(
            SyntaxKind::BlockComment,
            ControlFlow::Recovable,
            whole.to_span_at(offset),
        )
    })?;

    Ok(whole.split_to(offset))
}

/// A block document that describes the file it is written in.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InnerBlockDoc<I>(#[parserc(parser = read_inner_block_doc)] pub I)
where
    I: UnsynInput;

/// Reads the `'/*!' ( BlockCommentOrDoc | ~['/*','*/'] )* '*/'` of
/// [`InnerBlockDoc`].
fn read_inner_block_doc<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();

    keyword("/*!")
        .parse(input)
        .map_err(SyntaxKind::InnerBlockDoc.map())?;

    let offset = scan_block(input, 3, |offset| {
        UnsynError::Syntax(
            SyntaxKind::InnerBlockDoc,
            ControlFlow::Recovable,
            whole.to_span_at(offset),
        )
    })?;

    Ok(whole.split_to(offset))
}

/// A block document that describes the definition after it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OuterBlockDoc<I>(#[parserc(parser = read_outer_block_doc)] pub I)
where
    I: UnsynInput;

/// Reads the `'/**' ( ~['*','/*'] | BlockCommentOrDoc ) … '*/'` of
/// [`OuterBlockDoc`].
///
/// The first operand of the body refuses a third `*`, which is what leaves
/// `/**/` and `/***/` to the block comment: an opening of three is a comment that
/// closes at once, and not an empty document.
fn read_outer_block_doc<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();

    keyword("/**")
        .parse(input)
        .map_err(SyntaxKind::OuterBlockDoc.map())?;

    if let Some('*') = input.iter().next() {
        return Err(UnsynError::Syntax(
            SyntaxKind::OuterBlockDoc,
            ControlFlow::Recovable,
            whole.to_span_at(3),
        ));
    }

    let offset = scan_block(input, 3, |offset| {
        UnsynError::Syntax(
            SyntaxKind::OuterBlockDoc,
            ControlFlow::Recovable,
            whole.to_span_at(offset),
        )
    })?;

    Ok(whole.split_to(offset))
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

#[cfg(test)]
mod tests {
    use parserc::{AsStr, Span, syntax::SyntaxInput};

    use super::*;
    use crate::codegen::input::TokenStream;

    /// Parses `source` as the node `T`.
    fn parse<'a, T>(source: &'a str) -> Result<T, UnsynError>
    where
        T: Syntax<TokenStream<'a>>,
    {
        TokenStream::from(source).parse::<T>()
    }

    /// The text a node of this module read, which is all it keeps.
    fn text<T>(node: &T) -> String
    where
        T: ToText,
    {
        node.text().to_string()
    }

    trait ToText {
        fn text(&self) -> &str;
    }

    /// Pins the text of every node this module hands back to a `syntax` rule.
    macro_rules! holds_its_own_text {
        ($($name:ident),* $(,)?) => {
            $(
                impl ToText for $name<TokenStream<'_>> {
                    #[inline]
                    fn text(&self) -> &str {
                        self.0.as_str()
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
        XID_Start,
        XID_Continue,
        RawIdent,
        NonKeywordIdent,
        LeadingKeywords,
        FollowingKeywords,
        OctDigit,
        DecDigit,
        HexDigit,
        LitDec,
        LitUnicode,
        QuoteEscape,
        AsciiEscape,
        UnicodeEscape,
        LitStr,
        LineComment,
        InnerLineDoc,
        OuterLineDoc,
        BlockComment,
        InnerBlockDoc,
        OuterBlockDoc,
    );

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
    fn a_keyword_is_only_the_head_of_a_longer_word() {
        for word in ["lexer", "syntax", "whitespace"] {
            assert_eq!(text(&parse::<LeadingKeywords<_>>(word).unwrap()), word);
        }
        for word in ["concat", "except", "followed"] {
            assert_eq!(text(&parse::<FollowingKeywords<_>>(word).unwrap()), word);
        }

        // The head of a longer name is not a keyword: `lexerX` is one name, and
        // so is `excepting`. The word the name begins with is the one reported,
        // over the span of the whole name.
        for (source, kind, span) in [
            ("lexerX", KeywordKind::Lexer, 0..6),
            ("lexer_", KeywordKind::Lexer, 0..6),
            ("lexer9", KeywordKind::Lexer, 0..6),
            ("syntaxX", KeywordKind::Syntax, 0..7),
            ("syntax_", KeywordKind::Syntax, 0..7),
            ("whitespace_", KeywordKind::Whitespace, 0..11),
        ] {
            assert_eq!(
                parse::<LeadingKeywords<_>>(source),
                Err(UnsynError::Keyword(
                    kind,
                    ControlFlow::Recovable,
                    Span::Range(span)
                )),
                "{source}"
            );
        }

        for (source, kind, span) in [
            ("concatX", KeywordKind::Concat, 0..7),
            ("excepting", KeywordKind::Except, 0..9),
            ("followed_", KeywordKind::Followed, 0..9),
        ] {
            assert_eq!(
                parse::<FollowingKeywords<_>>(source),
                Err(UnsynError::Keyword(
                    kind,
                    ControlFlow::Recovable,
                    Span::Range(span)
                )),
                "{source}"
            );
        }

        // Where no word of a set begins at all, it is the set that is missing.
        assert_eq!(
            parse::<LeadingKeywords<_>>("| 'a';"),
            Err(UnsynError::Syntax(
                SyntaxKind::LeadingKeywords,
                ControlFlow::Recovable,
                Span::Range(0..0)
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
        assert!(matches!(
            parse::<Ident<_>>("r#lexer"),
            Ok(Ident::Raw(RawIdent(_)))
        ));
        assert!(matches!(
            parse::<Ident<_>>("lexer2"),
            Ok(Ident::NonKeyword(NonKeywordIdent(_)))
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

        assert_eq!(
            parse::<AsciiEscape<_>>("\\x80"),
            Err(UnsynError::Semantics(
                SemanticsKind::Char7BitEscapeOutOfRange,
                Span::Range(2..3)
            ))
        );
        assert_eq!(
            parse::<AsciiEscape<_>>("\\xg0"),
            Err(UnsynError::Semantics(
                SemanticsKind::HexDigit,
                Span::Range(2..3)
            ))
        );
        assert_eq!(
            parse::<AsciiEscape<_>>("\\x0"),
            Err(UnsynError::Semantics(
                SemanticsKind::Char7BitEscapeTooShort,
                Span::Range(3..3)
            ))
        );
        assert_eq!(text(&parse::<QuoteEscape<_>>(r"\'").unwrap()), r"\'");
    }

    #[test]
    fn a_unicode_escape_is_one_to_six_digits() {
        for source in ["\\u{1}", "\\u{1F600}", "\\u{1_0_0}", "\\u{abcdef}"] {
            assert_eq!(text(&parse::<UnicodeEscape<_>>(source).unwrap()), source);
        }

        // A run of digits that is too long, or of no digits at all, is refused by
        // the bound of the escape.
        for source in ["\\u{}", "\\u{1234567}"] {
            assert!(
                matches!(
                    parse::<UnicodeEscape<_>>(source),
                    Err(UnsynError::Semantics(SemanticsKind::UnicodeEscape, _))
                ),
                "{source}"
            );
        }

        // Text that is no digit at all leaves the closing brace to be asked for.
        for source in ["\\u{abcdefg}", "\\u{0x}"] {
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

        assert_eq!(
            parse::<LitStr<_>>("'a"),
            Err(UnsynError::Syntax(
                SyntaxKind::StrContent,
                ControlFlow::Recovable,
                Span::Range(2..2)
            ))
        );
        assert_eq!(
            parse::<LitStr<_>>("'a\rb'"),
            Err(UnsynError::Semantics(
                SemanticsKind::StrContent,
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
    fn a_line_comment_reaches_the_end_of_its_line() {
        assert_eq!(text(&parse::<LineComment<_>>("//x").unwrap()), "//x");
        assert_eq!(text(&parse::<LineComment<_>>("// x\ny").unwrap()), "// x");
        assert_eq!(text(&parse::<LineComment<_>>("//// x").unwrap()), "//// x");
        assert_eq!(text(&parse::<LineComment<_>>("//").unwrap()), "//");

        // The three other forms are told apart by their first character.
        assert!(parse::<LineComment<_>>("/// x").is_err());
        assert!(parse::<LineComment<_>>("//! x").is_err());
        assert!(parse::<LineComment<_>>("/x").is_err());
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
        assert!(parse::<OuterLineDoc<_>>("////").is_err());
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
    fn an_unterminated_group_reports_over_the_text_it_swallowed() {
        assert_eq!(
            parse::<BlockComment<_>>("/* a "),
            Err(UnsynError::Syntax(
                SyntaxKind::BlockComment,
                ControlFlow::Recovable,
                Span::Range(0..5)
            ))
        );
        assert_eq!(
            parse::<InnerBlockDoc<_>>("/*! a "),
            Err(UnsynError::Syntax(
                SyntaxKind::InnerBlockDoc,
                ControlFlow::Recovable,
                Span::Range(0..6)
            ))
        );
        assert_eq!(
            parse::<OuterBlockDoc<_>>("/** a "),
            Err(UnsynError::Syntax(
                SyntaxKind::OuterBlockDoc,
                ControlFlow::Recovable,
                Span::Range(0..6)
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
