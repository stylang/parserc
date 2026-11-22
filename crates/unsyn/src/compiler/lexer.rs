use parserc::{Parser, next_if, syntax::Syntax, take_while};
use unicode_ident::{is_xid_continue, is_xid_start};

use crate::compiler::{
    errors::{KeywordKind, SemanticsKind, SyntaxKind, UnsynError},
    input::UnsynInput,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Syntax)]
#[parserc(take_while = |c: char| is_whitespace(c))]
pub struct S<I>(pub I)
where
    I: UnsynInput;

#[inline]
fn is_whitespace(c: char) -> bool {
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Syntax)]
#[parserc(char = '\u{000A}')]
pub struct LF<I>(pub I)
where
    I: UnsynInput;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Syntax)]
#[parserc(char = '\u{000D}')]
pub struct CR<I>(pub I)
where
    I: UnsynInput;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Syntax)]
pub struct IdentOrKeyword<I>(#[parserc(parser = parse_ident_or_keyword)] pub I)
where
    I: UnsynInput;

#[inline]
fn parse_ident_or_keyword<I>(input: &mut I) -> Result<I, UnsynError>
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Syntax)]
pub struct RawIdent<I>
where
    I: UnsynInput,
{
    #[parserc(keyword = "r#", map_err = SyntaxKind::Ident.map())]
    pub f1: I,

    pub ident_or_keyword: IdentOrKeyword<I>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Syntax)]
pub struct NonKeywordIdent<I>(#[parserc(parser = parse_non_keyword_ident)] pub I)
where
    I: UnsynInput;

#[inline]
fn parse_non_keyword_ident<I>(input: &mut I) -> Result<I, UnsynError>
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

/// The word that joins two operands of a pattern into one construct.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(keyword = "\\'", map_err = SyntaxKind::QuoteEscape.map())]
pub struct QuoteEscape<I>(pub I)
where
    I: UnsynInput;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AsciiHexEscape<I>
where
    I: UnsynInput,
{
    #[parserc(keyword = "\\x")]
    pub f1: I,
    pub oct_digit: OctDigit<I>,
    pub hex_digit: HexDigit<I>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OctDigit<I>(#[parserc(parser = next_if(is_oct_digit))] pub I)
where
    I: UnsynInput;

#[inline]
fn is_oct_digit(c: char) -> bool {
    matches!(c, '0'..='7')
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DecDigit<I>(#[parserc(parser = next_if(is_dec_digit))] pub I)
where
    I: UnsynInput;

#[inline]
fn is_dec_digit(c: char) -> bool {
    matches!(c, '0'..='9')
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HexDigit<I>(#[parserc(parser = next_if(is_hex_digit))] pub I)
where
    I: UnsynInput;

#[inline]
fn is_hex_digit(c: char) -> bool {
    matches!(
        c,
        '0'..='9'
            | 'a'..='f'
            | 'A'..='F'
    )
}
