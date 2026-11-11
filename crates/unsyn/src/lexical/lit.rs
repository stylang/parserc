//! The literal tokens: the escapes a `char` or a `lexer` rule may contain, the
//! string and unicode literals an expression may name, and the decimal literal
//! that bounds a repetition.
//!
//! An escape is written the way Rust writes it, and each form is a token of its
//! own here: [`ASCIIEscape`], [`UnicodeEscape`] and [`QuoteEscape`] are the three
//! that can appear inside a [`LitStr`], where they are what lets a `'` or a `\`
//! be part of the text rather than the end of it.

use parserc::{
    ControlFlow, ParseError, Parser, keyword, next_if, syntax::Syntax, take_while, take_while_with,
};

use crate::{
    errors::{PunctKind, SemanticsKind, SyntaxKind, UnsynError},
    input::UnsynInput,
};

/// An escape for one of the characters ASCII writes with a backslash, see [`The Rust Reference`]
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
    /// Horizontal tab
    Tab(#[parserc(keyword = "\\t")] I),
    /// A backslash
    BlackSlash(#[parserc(keyword = "\\\\")] I),
    /// The null character
    Null(#[parserc(keyword = "\\0")] I),
    /// A 7-bit character code: `\x` and exactly two hex digits, so up to `0x7F`
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
            input.to_span_at(1),
        ));
    }

    if !matches!(buf[0], b'0'..=b'7') {
        return Err(UnsynError::Semantics(
            SemanticsKind::Char7BitEscapeOutOfRange,
            input.to_span_at(1),
        ));
    }

    if !buf[1].is_ascii_hexdigit() {
        return Err(UnsynError::Semantics(
            SemanticsKind::HexDigit,
            input.to_span_at(2),
        ));
    }

    input.split_to(2);

    Ok(content.split_to(4))
}

/// A Unicode escape, see [`The Rust Reference`]
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
    /// The code point: one to six hex digits
    ///
    /// Whether they name a character assigned by Unicode is not decided here,
    /// only that there are digits to read.
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
    take_while_with(1..7, |c: char| c.is_ascii_hexdigit())
        .parse(input)
        .map_err(SemanticsKind::UnicodeEscape.map())
}

/// An escaped quote `\'`, see [`The Rust Reference`]
///
/// A `char` literal is bounded by `'`, so this is how the character itself is
/// written as its content.
///
/// [`The Rust Reference`]: https://doc.rust-lang.org/reference/tokens.html#quote-escapes
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = SyntaxKind::QuoteEscape.map())]
pub struct QuoteEscape<I>(#[parserc(keyword = "\\'")] pub I)
where
    I: UnsynInput;

/// One item of the content of a [`LitStr`]: either an escape or plain text.
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
            input.to_span_at(1),
        ));
    }

    let mut iter = input.iter_indices();

    loop {
        match iter.next() {
            Some((offset, '\r')) => {
                input.split_to(offset);
                return Err(UnsynError::Semantics(
                    SemanticsKind::StrContent,
                    input.to_span_at(1),
                ));
            }
            Some((offset, '\'' | '\\')) => {
                if offset == 0 {
                    return Err(UnsynError::Syntax(
                        SyntaxKind::StrContent,
                        ControlFlow::Recovable,
                        input.to_span_at(1),
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

/// A string literal is a sequence of any Unicode characters enclosed within two
/// U+0027 (apostrophe) characters, with the exception of U+0027 itself, which
/// must be escaped by a preceding U+005C character (\\), see [`The Rust Reference`]
///
/// The content stops at the first `'` that is not part of an escape, so a string
/// can never contain a bare apostrophe.
///
/// [`The Rust Reference`]: https://doc.rust-lang.org/reference/tokens.html#string-literals
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

/// A Unicode code point written in a string, as in `U+0009`.
///
/// unsyn asks for exactly four hex digits after `U+`, where Rust allows four to
/// six, because the token it returns is always six characters long.
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
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        let mut content = input.clone();

        let prefix = keyword("U+")
            .parse(input)
            .map_err(SyntaxKind::Unicode.map())?;

        take_while_with(4..5, |c: char| c.is_ascii_hexdigit())
            .parse(input)
            .map_err(|err| {
                UnsynError::Semantics(SemanticsKind::Unicode, prefix.to_span() + err.to_span())
            })?;

        if let Some(c) = input.iter().next()
            && c.is_ascii_hexdigit()
        {
            return Err(UnsynError::Semantics(
                SemanticsKind::Unicode,
                prefix.to_span() + input.to_span_at(1),
            ));
        }

        Ok(Self(content.split_to(6)))
    }

    #[inline]
    fn to_span(&self) -> parserc::Span {
        self.0.to_span()
    }
}

/// A decimal literal starts with a decimal digit and continues with any mixture
/// of decimal digits and underscores, see [`The Rust Reference`]
///
/// Underscores are part of the token but carry no value; the semantic analysis
/// drops them when it reads the number.
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
    fn parse(input: &mut I) -> Result<Self, <I as parserc::Input>::Error> {
        let mut content = input.clone();

        next_if(|c: char| c.is_ascii_digit())
            .parse(input)
            .map_err(SyntaxKind::Dec.map())?;

        let rest = take_while(|c: char| c.is_ascii_digit() || c == '_').parse(input)?;

        Ok(Self(content.split_to(rest.len() + 1)))
    }

    fn to_span(&self) -> parserc::Span {
        self.0.to_span()
    }
}

#[cfg(test)]
mod tests {
    use parserc::{Span, syntax::SyntaxInput};

    use crate::{
        errors::{PunctKind, SemanticsKind, SyntaxKind, UnsynError},
        input::TokenStream,
    };

    use super::*;

    #[test]
    fn test_unicode() {
        assert_eq!(
            TokenStream::from("U+2029").parse::<LitUnicode<_>>(),
            Ok(LitUnicode(TokenStream::from((0, "U+2029"))))
        );

        assert_eq!(
            TokenStream::from("U+1").parse::<LitUnicode<_>>(),
            Err(UnsynError::Semantics(
                SemanticsKind::Unicode,
                Span::Range(0..3)
            ))
        );
        assert_eq!(
            TokenStream::from("U+11234").parse::<LitUnicode<_>>(),
            Err(UnsynError::Semantics(
                SemanticsKind::Unicode,
                Span::Range(0..7)
            ))
        );
    }

    #[test]
    fn test_lit_str() {
        assert_eq!(
            TokenStream::from("'\\'static'").parse::<LitStr<_>>(),
            Ok(LitStr {
                delimiter_start: TokenStream::from((0, "'")),
                content: vec![
                    StrSegment::QuoteEscape(QuoteEscape(TokenStream::from((1, "\\'")))),
                    StrSegment::CharWithException(TokenStream::from((3, "static")))
                ],
                delimiter_end: TokenStream::from((9, "'"))
            })
        );

        // A `\` then a `\` is one backslash, and the `'` that follows is the end.
        assert_eq!(
            TokenStream::from(r#"'\\\''"#).parse::<LitStr<_>>(),
            Ok(LitStr {
                delimiter_start: TokenStream::from((0, "'")),
                content: vec![
                    StrSegment::ASCIIEscape(ASCIIEscape::BlackSlash(TokenStream::from((
                        1, "\\\\"
                    )))),
                    StrSegment::QuoteEscape(QuoteEscape(TokenStream::from((3, "\\'")))),
                ],
                delimiter_end: TokenStream::from((5, "'"))
            })
        );
    }

    /// A string ends at the first `'` that an escape did not already consume.
    #[test]
    fn a_string_is_bounded_by_the_quotes_it_contains() {
        assert_eq!(
            TokenStream::from("''").parse::<LitStr<_>>(),
            Ok(LitStr {
                delimiter_start: TokenStream::from((0, "'")),
                content: vec![],
                delimiter_end: TokenStream::from((1, "'")),
            })
        );

        // The second `'` closes the literal, and the `b` that follows is a
        // separate token rather than part of it.
        assert_eq!(
            TokenStream::from("'a''b'").parse::<LitStr<_>>(),
            Ok(LitStr {
                delimiter_start: TokenStream::from((0, "'")),
                content: vec![StrSegment::CharWithException(TokenStream::from((1, "a")))],
                delimiter_end: TokenStream::from((2, "'")),
            })
        );

        // Without that closing `'` there is no literal to report.
        assert_eq!(
            TokenStream::from("'a").parse::<LitStr<_>>(),
            Err(UnsynError::Punct(
                PunctKind::SingleQuote,
                ControlFlow::Fatal,
                Span::Range(2..2),
            ))
        );
    }

    /// A raw carriage return is not text: the line ending is what the DSL writes
    /// with `\r`, so an actual one in a literal is a mistake worth pointing at.
    #[test]
    fn a_carriage_return_is_not_string_content() {
        assert_eq!(
            TokenStream::from("'\r hello'").parse::<LitStr<_>>(),
            Err(UnsynError::Semantics(
                SemanticsKind::StrContent,
                Span::Range(1..2)
            ))
        );
    }

    #[test]
    fn every_ascii_escape_form_is_recognised() {
        let escapes = [
            ("\\n", ASCIIEscape::LF(TokenStream::from((0, "\\n")))),
            ("\\r", ASCIIEscape::CR(TokenStream::from((0, "\\r")))),
            ("\\t", ASCIIEscape::Tab(TokenStream::from((0, "\\t")))),
            (
                "\\\\",
                ASCIIEscape::BlackSlash(TokenStream::from((0, "\\\\"))),
            ),
            ("\\0", ASCIIEscape::Null(TokenStream::from((0, "\\0")))),
            ("\\x7F", ASCIIEscape::Char(TokenStream::from((0, "\\x7F")))),
        ];

        for (input, expect) in escapes {
            assert_eq!(
                TokenStream::from(input).parse::<ASCIIEscape<_>>(),
                Ok(expect),
                "{input}"
            );
        }
    }

    /// `\x` takes exactly two hex digits, and the pair must be `00` to `7F`.
    #[test]
    fn a_seven_bit_escape_is_two_hex_digits() {
        assert_eq!(
            TokenStream::from("\\x7f").parse::<ASCIIEscape<_>>(),
            Ok(ASCIIEscape::Char(TokenStream::from((0, "\\x7f"))))
        );

        // The first digit is out of range for a 7-bit character.
        assert_eq!(
            TokenStream::from("\\x8F").parse::<ASCIIEscape<_>>(),
            Err(UnsynError::Semantics(
                SemanticsKind::Char7BitEscapeOutOfRange,
                Span::Range(2..3)
            ))
        );

        // Neither digit is a hex digit at all.
        assert_eq!(
            TokenStream::from("\\xZF").parse::<ASCIIEscape<_>>(),
            Err(UnsynError::Semantics(
                SemanticsKind::HexDigit,
                Span::Range(2..3)
            ))
        );

        // The second digit is missing.
        assert_eq!(
            TokenStream::from("\\x7").parse::<ASCIIEscape<_>>(),
            Err(UnsynError::Semantics(
                SemanticsKind::Char7BitEscapeTooShort,
                Span::Range(2..3)
            ))
        );
    }

    /// `\u{` is bounded by a `}` that the DSL asks for fatally, so a content
    /// error inside it stops the parse rather than only ruling this form out.
    #[test]
    fn a_unicode_escape_reads_its_digits_until_the_brace() {
        assert_eq!(
            TokenStream::from("\\u{10FFFF}").parse::<UnicodeEscape<_>>(),
            Ok(UnicodeEscape {
                delimiter_start: TokenStream::from((0, "\\u{")),
                digits: TokenStream::from((3, "10FFFF")),
                delimiter_end: TokenStream::from((9, "}")),
            })
        );

        // One hex digit is enough to be read here, even before anyone asks
        // whether it names a character; no digits at all is not.
        assert!(
            TokenStream::from("\\u{1}")
                .parse::<UnicodeEscape<_>>()
                .is_ok()
        );
        assert_eq!(
            TokenStream::from("\\u{)").parse::<UnicodeEscape<_>>(),
            Err(UnsynError::Semantics(
                SemanticsKind::UnicodeEscape,
                Span::Range(3..4)
            ))
        );

        // Whatever stands in for the closing brace is fatal.
        assert_eq!(
            TokenStream::from("\\u{A_}").parse::<UnicodeEscape<_>>(),
            Err(UnsynError::Punct(
                PunctKind::BraceEnd,
                ControlFlow::Fatal,
                Span::Range(4..5)
            ))
        );
    }

    /// A `U+` literal is the prefix and exactly four hex digits.
    #[test]
    fn a_unicode_literal_needs_four_hex_digits() {
        assert_eq!(
            TokenStream::from("U+0009").parse::<LitUnicode<_>>(),
            Ok(LitUnicode(TokenStream::from((0, "U+0009"))))
        );

        // Too few, and too many: both are reported over the digits that were read.
        for (input, end) in [("U+009", 3), ("U+00099", 7)] {
            assert_eq!(
                TokenStream::from(input).parse::<LitUnicode<_>>(),
                Err(UnsynError::Semantics(
                    SemanticsKind::Unicode,
                    Span::Range(0..end)
                )),
                "{input}"
            );
        }
    }

    /// An underscore may separate digits anywhere but the first one.
    #[test]
    fn a_decimal_is_digits_and_underscores() {
        let decimals = ["0", "1_0", "12_34__56"];

        for input in decimals {
            assert_eq!(
                TokenStream::from(input).parse::<LitDec<_>>(),
                Ok(LitDec(TokenStream::from((0, input)))),
                "{input}"
            );
        }

        assert_eq!(
            TokenStream::from("_1").parse::<LitDec<_>>(),
            Err(UnsynError::Syntax(
                SyntaxKind::Dec,
                ControlFlow::Recovable,
                Span::Range(0..1)
            ))
        );
    }
}
