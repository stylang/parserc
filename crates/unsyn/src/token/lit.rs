//! literal tokens.

use parserc::{
    ControlFlow, ParseError, Parser,
    combinators::{keyword, next_if, take_while, take_while_range},
    sourceinput::{Span, ToSpan},
    syntax::Syntax,
};

use crate::{
    errors::{PunctKind, SemanticsKind, SyntaxKind, UnsynError},
    input::UnsynInput,
};

/// ASCII escape, more information see [`The Rust Reference`]
///
/// [`The Rust Reference`]: https://doc.rust-lang.org/reference/tokens.html#railroad-ASCII_ESCAPE
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ASCIIEscape<I>
where
    I: UnsynInput,
{
    /// New line break
    LF(#[parserc(keyword = "\\n")] I),
    /// Carriage return
    CR(#[parserc(keyword = "\\r")] I),
    /// Table
    Tab(#[parserc(keyword = "\\t")] I),
    /// Black slash
    BlackSlash(#[parserc(keyword = "\\\\")] I),
    /// Null
    Null(#[parserc(keyword = "\\0")] I),
    /// 7-bit character code (exactly 2 digits, up to 0x7F)
    Char(#[parserc(parser = parse_7bit_char)] I),
}

#[inline]
fn parse_7bit_char<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut content = input.clone();

    keyword("\\x")
        .parse(input)
        .map_err(SyntaxKind::ASCIIEscape.map())?;

    let buf = input.as_bytes();

    if buf.len() < 2 {
        return Err(UnsynError::Semantics(
            SemanticsKind::Char7BitEscapeTooShort,
            input.to_span(),
        ));
    }

    if !buf[0].is_ascii_hexdigit() {
        return Err(UnsynError::Semantics(
            SemanticsKind::HexDigit,
            input.to_span_with(1),
        ));
    }

    if !matches!(buf[0], b'0'..=b'7') {
        return Err(UnsynError::Semantics(
            SemanticsKind::Char7BitEscapeOutOfRange,
            input.to_span_with(1),
        ));
    }

    if !buf[1].is_ascii_hexdigit() {
        return Err(UnsynError::Semantics(
            SemanticsKind::HexDigit,
            input.to_span_with(2),
        ));
    }

    input.split_to(2);

    Ok(content.split_to(4))
}

/// Unicode escape, more information see [`The Rust Reference`]
///
/// [`The Rust Reference`]: https://doc.rust-lang.org/reference/tokens.html#railroad-UNICODE_ESCAPE
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnicodeEscape<I>
where
    I: UnsynInput,
{
    /// leading chars `\u{`
    #[parserc(keyword = "\\u{", crucial, map_err = SyntaxKind::UnicodeEscape.map())]
    pub delimiter_start: I,
    /// 24-bits hex-digits up to `10ffff`
    #[parserc(parser = parse_unicode_hex_digits)]
    pub digits: I,
    /// tailing punct `}`
    #[parserc(keyword = "}", map_err = PunctKind::BraceEnd.map())]
    pub delimiter_end: I,
}

#[inline]
fn parse_unicode_hex_digits<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    take_while_range(1..=6, |c: char| c.is_ascii_hexdigit())
        .parse(input)
        .map_err(SemanticsKind::UnicodeEscape.map())
}

/// Quote ``' escapes, more information see [`The Rust Reference`]
///
/// [`The Rust Reference`]: https://doc.rust-lang.org/reference/tokens.html#quote-escapes
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = SyntaxKind::QuoteEscape.map())]
pub struct QuoteEscape<I>(#[parserc(keyword = "\\'")] pub I)
where
    I: UnsynInput;

/// Content item of [`LitStr`]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StrSegment<I>
where
    I: UnsynInput,
{
    QuoteEscape(QuoteEscape<I>),
    ASCIIEscape(ASCIIEscape<I>),
    UnicodeEscape(UnicodeEscape<I>),
    CharWithException(#[parserc(parser = parse_str_item_with_exception)] I),
}

#[inline]
fn parse_str_item_with_exception<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    if input.is_empty() {
        return Err(UnsynError::Syntax(
            SyntaxKind::StrContent,
            ControlFlow::Recovable,
            input.to_span_with(1),
        ));
    }

    let mut iter = input.iter_indices();

    loop {
        match iter.next() {
            Some((offset, '\r')) => {
                input.split_to(offset);
                return Err(UnsynError::Semantics(
                    SemanticsKind::StrContent,
                    input.to_span_with(1),
                ));
            }
            Some((offset, '\'' | '\\')) => {
                if offset == 0 {
                    return Err(UnsynError::Syntax(
                        SyntaxKind::StrContent,
                        ControlFlow::Recovable,
                        input.to_span_with(1),
                    ));
                }

                return Ok(input.split_to(offset));
            }
            Some((_, _)) => {
                continue;
            }
            None => return Ok(input.split_to(input.len())),
        }
    }
}

/// A string literal is a sequence of any Unicode characters enclosed within two U+0027 (double-quote) characters,
/// with the exception of U+0027 itself, which must be escaped by a preceding U+005C character (\).
/// see [`The Rust Reference`]
///
/// [`The Rust Reference`]:https://doc.rust-lang.org/reference/tokens.html#string-literals
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitStr<I>
where
    I: UnsynInput,
{
    #[parserc(keyword = "'", map_err = PunctKind::SingleQuote.map(), crucial)]
    pub delimiter_start: I,
    /// sequence of content item of literal string.
    pub content: Vec<StrSegment<I>>,
    #[parserc(keyword = "'", map_err = PunctKind::SingleQuote.map())]
    pub delimiter_end: I,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitUnicode<I>(pub I)
where
    I: UnsynInput;

impl<I> Syntax<I> for LitUnicode<I>
where
    I: UnsynInput,
{
    #[inline]
    fn parse(input: &mut I) -> Result<Self, <I as parserc::sourceinput::Input>::Error> {
        let mut content = input.clone();

        let prefix = keyword("U+")
            .parse(input)
            .map_err(SyntaxKind::Unicode.map())?;

        let digits = take_while_range(4..=6, |c: char| c.is_ascii_hexdigit())
            .parse(input)
            .map_err(|err| {
                UnsynError::Semantics(SemanticsKind::Unicode, prefix.to_span() + err.to_span())
            })?;

        if let Some(c) = input.iter().next() {
            if c.is_ascii_hexdigit() {
                return Err(UnsynError::Semantics(
                    SemanticsKind::Unicode,
                    prefix.to_span() + input.to_span_with(1),
                ));
            }
        }

        Ok(Self(content.split_to(2 + digits.len())))
    }

    #[inline]
    fn to_span(&self) -> Span {
        self.0.to_span()
    }
}

/// A decimal literal starts with a decimal digit and continues with any mixture of decimal digits and underscores.
///
///  see [`The Rust Reference`]
///
/// [`The Rust Reference`]: https://doc.rust-lang.org/reference/tokens.html#railroad-DEC_LITERAL
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitDec<I>(pub I)
where
    I: UnsynInput;

impl<I> Syntax<I> for LitDec<I>
where
    I: UnsynInput,
{
    fn parse(input: &mut I) -> Result<Self, <I as parserc::sourceinput::Input>::Error> {
        let mut content = input.clone();

        next_if(|c: char| c.is_ascii_digit())
            .parse(input)
            .map_err(SyntaxKind::Dec.map())?;

        let rest = take_while(|c: char| c.is_ascii_digit() || c == '_').parse(input)?;

        Ok(Self(content.split_to(rest.len() + 1)))
    }

    fn to_span(&self) -> Span {
        self.0.to_span()
    }
}

#[cfg(test)]
mod tests {
    use parserc::syntax::SyntaxExt;

    use super::*;

    use crate::{
        errors::{SemanticsKind, UnsynError},
        input::Chars,
    };

    #[test]
    fn test_unicode() {
        assert_eq!(
            Chars::new("U+2029").parse::<LitUnicode<_>>(),
            Ok(LitUnicode(Chars::from((0, "U+2029"))))
        );

        assert_eq!(
            Chars::new("U+20290").parse::<LitUnicode<_>>(),
            Ok(LitUnicode(Chars::from((0, "U+20290"))))
        );

        assert_eq!(
            Chars::new("U+202900").parse::<LitUnicode<_>>(),
            Ok(LitUnicode(Chars::from((0, "U+202900"))))
        );

        assert_eq!(
            Chars::new("U+1").parse::<LitUnicode<_>>(),
            Err(UnsynError::Semantics(
                SemanticsKind::Unicode,
                Span::from(0..3)
            ))
        );
        assert_eq!(
            Chars::new("U+1123411").parse::<LitUnicode<_>>(),
            Err(UnsynError::Semantics(
                SemanticsKind::Unicode,
                Span::from(0..9)
            ))
        );
    }

    #[test]
    fn test_lit_str() {
        assert_eq!(
            Chars::new("'\\'static'").parse::<LitStr<_>>(),
            Ok(LitStr {
                delimiter_start: Chars::from((0, "'")),
                content: vec![
                    StrSegment::QuoteEscape(QuoteEscape(Chars::from((1, "\\'")))),
                    StrSegment::CharWithException(Chars::from((3, "static")))
                ],
                delimiter_end: Chars::from((9, "'"))
            })
        );

        println!("{:?}", Chars::new(r#"'\\\''"#).parse::<LitStr<_>>());
    }
}
