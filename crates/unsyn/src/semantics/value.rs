//! Evaluation of the literals used by node definitions.
//!
//! The lexical layer accepts literal tokens whose *content* may still be
//! invalid — a `\u{…}` escape outside the Unicode range is a well-formed token
//! but a meaningless pattern. Computing the value of a literal is therefore a
//! semantic task and lives here.
//!
//! Only one of these functions descends a tree, the one that reads a string
//! literal, and it descends by [`Visit`]. The rest decode
//! the text of a single token, which is a question about that token's shape
//! rather than about where it stands: `U+0041` denotes `A` whether it is a set
//! member or the end of a range, so there is nothing for a walk to add.

use parserc::syntax::Syntax;

use crate::{
    errors::{SemanticsKind, UnsynError},
    input::UnsynInput,
    lexical::lit::{AsciiEscape, LitDec, LitStr, LitUnicode, QuoteEscape, UnicodeEscape},
    visit::Visit,
};

/// Decodes the character content of a string literal.
///
/// The value is always returned; the second element of the tuple reports the
/// first malformed escape found in it, if any.
///
/// The segments are read through [`Visit`], which is what knows the four shapes
/// a segment takes; the pass enters at them rather than at the whole literal,
/// because the two quotes are how a literal was written and no character of
/// what it denotes.
///
/// Each of the three escapes stops the descent where it stands, so the only text
/// this pass ever sees is a run of plain characters.
pub fn str_value<I: UnsynInput>(lit: &LitStr<I>) -> (String, Option<UnsynError>) {
    let mut decoder = Decoder::default();

    for segment in &lit.content {
        decoder.visit_str_segment(segment);
    }

    (decoder.value, decoder.error)
}

/// The pass that spells out what a string literal denotes.
#[derive(Default)]
struct Decoder {
    value: String,
    error: Option<UnsynError>,
}

impl<'ast, I: UnsynInput> Visit<'ast, I> for Decoder {
    /// Plain text of a literal is the characters it denotes, escape-free.
    fn visit_input(&mut self, input: &'ast I) {
        self.value.push_str(input.as_str());
    }

    /// An escaped quote is a quote, whatever shape the escape was written in.
    fn visit_quote_escape(&mut self, _node: &'ast QuoteEscape<I>) {
        self.value.push('\'');
    }

    fn visit_ascii_escape(&mut self, node: &'ast AsciiEscape<I>) {
        self.value.push(ascii_escape_value(node));
    }

    fn visit_unicode_escape(&mut self, node: &'ast UnicodeEscape<I>) {
        match unicode_escape_value(node) {
            Some(c) => self.value.push(c),
            None => {
                self.error.get_or_insert_with(|| {
                    UnsynError::Semantics(SemanticsKind::InvalidCodePoint, node.to_span())
                });

                self.value.push(char::REPLACEMENT_CHARACTER);
            }
        }
    }
}

/// Decodes a string literal, replacing malformed escapes with `U+FFFD`.
///
/// Use [`str_value`] when the decoded value has to be validated.
#[inline]
pub fn str_chars<I: UnsynInput>(lit: &LitStr<I>) -> Vec<char> {
    str_value(lit).0.chars().collect()
}

/// Decodes a unicode code point literal.
///
/// Returns `None` when the literal does not denote a Unicode scalar value.
/// Use [`unicode_value`] when the result has to be validated.
pub fn unicode_char<I: UnsynInput>(lit: &LitUnicode<I>) -> Option<char> {
    let text = lit.0.as_str();
    let digits = text.strip_prefix("U+")?;

    u32::from_str_radix(digits, 16)
        .ok()
        .and_then(char::from_u32)
}

/// Decodes a unicode code point literal, validating its range.
pub fn unicode_value<I: UnsynInput>(lit: &LitUnicode<I>) -> Result<char, UnsynError> {
    unicode_char(lit)
        .ok_or_else(|| UnsynError::Semantics(SemanticsKind::InvalidCodePoint, lit.to_span()))
}

/// Evaluates a decimal literal, ignoring the `_` separators it may contain.
///
/// Returns `None` when the value does not fit into a [`usize`].
pub fn dec_usize<I: UnsynInput>(lit: &LitDec<I>) -> Option<usize> {
    lit.0.as_str().replace('_', "").parse::<usize>().ok()
}

/// Evaluates a decimal literal, validating its value.
pub fn dec_value<I: UnsynInput>(lit: &LitDec<I>) -> Result<usize, UnsynError> {
    dec_usize(lit).ok_or_else(|| UnsynError::Semantics(SemanticsKind::InvalidRepeat, lit.to_span()))
}

/// Returns the character denoted by an ASCII escape.
fn ascii_escape_value<I: UnsynInput>(escape: &AsciiEscape<I>) -> char {
    match escape {
        AsciiEscape::LF(_) => '\n',
        AsciiEscape::CR(_) => '\r',
        AsciiEscape::Tab(_) => '\t',
        AsciiEscape::BlackSlash(_) => '\\',
        AsciiEscape::Null(_) => '\0',
        AsciiEscape::Char(code) => {
            // The lexical pass guarantees the `\xNN` shape and its `0x00..=0x7F`
            // range, so a decoding failure here can only come from a handcrafted
            // tree; fall back to the replacement character.
            let text = code.as_str();

            text.get(text.len() - 2..)
                .and_then(|digits| u8::from_str_radix(digits, 16).ok())
                .map(char::from)
                .unwrap_or(char::REPLACEMENT_CHARACTER)
        }
    }
}

/// Returns the character denoted by a unicode escape, or `None` when the
/// literal is outside the Unicode range or denotes a surrogate.
fn unicode_escape_value<I: UnsynInput>(escape: &UnicodeEscape<I>) -> Option<char> {
    let digits = escape.digits.as_str();

    u32::from_str_radix(digits, 16)
        .ok()
        .and_then(char::from_u32)
}

#[cfg(test)]
mod tests {
    use parserc::syntax::SyntaxInput;

    use crate::input::TokenStream;

    use super::*;

    #[test]
    fn string_content_is_decoded() {
        let lit = TokenStream::from("'ab'").parse::<LitStr<_>>().unwrap();

        let (value, error) = str_value(&lit);

        assert_eq!(value, "ab");
        assert!(error.is_none());
        assert_eq!(str_chars(&lit), vec!['a', 'b']);
    }

    #[test]
    fn escapes_are_decoded() {
        let lit = TokenStream::from(r"'a\n\t\x7f\u{41}'")
            .parse::<LitStr<_>>()
            .unwrap();

        let (value, error) = str_value(&lit);

        assert_eq!(value, "a\n\t\u{7f}A");
        assert!(error.is_none());
    }

    #[test]
    fn a_bad_escape_is_reported_once_but_still_decodes() {
        let lit = TokenStream::from(r"'A\u{D800}B'")
            .parse::<LitStr<_>>()
            .unwrap();

        let (value, error) = str_value(&lit);

        assert_eq!(value, "A\u{FFFD}B");
        assert_eq!(
            error.map(|error| error.to_string()),
            Some("unexpect/invalid: not a valid unicode code point, Range(2..10)".into())
        );
    }

    #[test]
    fn code_points_are_decoded() {
        let lit = TokenStream::from("U+0041")
            .parse::<LitUnicode<_>>()
            .unwrap();

        assert_eq!(unicode_char(&lit), Some('A'));
        assert_eq!(unicode_value(&lit), Ok('A'));

        // A surrogate is well formed to the lexer but denotes no character.
        let lit = TokenStream::from("U+D800")
            .parse::<LitUnicode<_>>()
            .unwrap();

        assert_eq!(unicode_char(&lit), None);
        assert!(unicode_value(&lit).is_err());
    }

    #[test]
    fn decimals_ignore_digit_separators() {
        let lit = TokenStream::from("1_0").parse::<LitDec<_>>().unwrap();

        assert_eq!(dec_usize(&lit), Some(10));
        assert_eq!(dec_value(&lit), Ok(10));
    }

    #[test]
    fn a_decimal_that_does_not_fit_is_reported() {
        let lit = TokenStream::from("99999999999999999999999999")
            .parse::<LitDec<_>>()
            .unwrap();

        assert_eq!(dec_usize(&lit), None);
        assert!(dec_value(&lit).is_err());
    }
}
