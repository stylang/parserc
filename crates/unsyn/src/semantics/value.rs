//! Evaluation of the literals used by node definitions.
//!
//! The lexical layer accepts literal tokens whose *content* may still be
//! invalid — a `\u{…}` escape outside the Unicode range is a well-formed token
//! but a meaningless pattern. Computing the value of a literal is therefore a
//! semantic task and lives here.

use parserc::syntax::Syntax;

use crate::{
    errors::{SemanticsKind, UnsynError},
    input::UnsynInput,
    lexical::lit::{LitDec, LitStr, LitUnicode},
    visit::{code_point_literal, count, text_literal},
};

/// Decodes the character content of a string literal.
///
/// The value is always returned; the second element of the tuple reports the
/// first malformed escape found in it, if any.
///
/// The decoding itself belongs to the [vocabulary](crate::visit::TextLiteral),
/// which is where the pieces of a literal say what they denote; this function
/// only hands the literal to that reading and turns the place of a bad escape
/// into a diagnostic.
pub fn str_value<I: UnsynInput>(lit: &LitStr<I>) -> (String, Option<UnsynError>) {
    let (value, invalid) = text_literal(lit).value();

    (
        value,
        invalid.map(|span| UnsynError::Semantics(SemanticsKind::InvalidCodePoint, span)),
    )
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
    code_point_literal(lit).value()
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
    count(lit).value()
}

/// Evaluates a decimal literal, validating its value.
pub fn dec_value<I: UnsynInput>(lit: &LitDec<I>) -> Result<usize, UnsynError> {
    dec_usize(lit).ok_or_else(|| UnsynError::Semantics(SemanticsKind::InvalidRepeat, lit.to_span()))
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
