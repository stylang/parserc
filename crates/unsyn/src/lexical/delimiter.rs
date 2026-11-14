//! The delimiter pairs that group tokens.
//!
//! A delimiter is `(open, body, close)`: the body is parsed between them, and
//! the closing token is matched *fatally* — once a group has been opened, a
//! missing close is a broken file rather than a rule that does not fit, so the
//! parser reports it instead of backtracking over the typo.
//!
//! [`Angle`] is the pair a node uses to call an external of the backend, as in
//! `<XID_Start>`; the other three group a pattern, as Rust does.

use parserc::syntax::Delimiter;

use super::punct::*;

/// Curly braces `{…}`, the suffix of a repetition.
pub type Brace<I, T> = Delimiter<BraceStart<I>, BraceEnd<I>, T>;
/// Square brackets `[…]`, a set of characters or strings.
pub type Bracket<I, T> = Delimiter<BracketStart<I>, BracketEnd<I>, T>;
/// Parentheses `(…)`, a group inside a pattern.
pub type Paren<I, T> = Delimiter<ParenStart<I>, ParenEnd<I>, T>;
/// Angle brackets `<…>`, a call of an external the backend provides.
pub type Angle<I, T> = Delimiter<Lt<I>, Gt<I>, T>;

#[cfg(test)]
mod tests {
    use parserc::{
        AsStr, ControlFlow, Span,
        syntax::{Syntax, SyntaxInput},
    };

    use crate::{
        errors::{PunctKind, UnsynError},
        input::TokenStream,
        lexical::{S, ident::Ident, punct::Semi},
        syntax::Repeat,
    };

    use super::*;

    #[test]
    fn a_group_keeps_the_tokens_it_wraps() {
        let mut input = TokenStream::from("[; ]");

        let group = input
            .parse::<Bracket<_, Semi<_>>>()
            .expect("the group should parse");

        assert_eq!(group.start.1.as_str(), "[");
        assert_eq!(group.body.1.as_str(), ";");
        // The separator inside the brackets belongs to the body, not to the
        // closing token.
        assert_eq!(group.body.2.expect("a trailing run").0.as_str(), " ");
        assert_eq!(group.end.1.as_str(), "]");
        assert_eq!(input.as_str(), "");
    }

    #[test]
    fn whitespace_is_allowed_on_the_inside_of_a_group() {
        let mut input = TokenStream::from("< XID_Start >");

        let call = input
            .parse::<Angle<_, Ident<_>>>()
            .expect("the call should parse");

        assert_eq!(call.body.0.as_str(), "XID_Start");
        assert_eq!(input.as_str(), "");
    }

    #[test]
    fn a_group_that_is_not_closed_is_fatal() {
        let mut input = TokenStream::from("[;");

        let error = input
            .parse::<Bracket<_, Semi<_>>>()
            .expect_err("the group is not closed");

        // Backtracking over an unclosed bracket would hide the typo, so the
        // close is matched fatally and the file stops here.
        assert!(
            matches!(
                error,
                UnsynError::Punct(PunctKind::BracketEnd, ControlFlow::Fatal, _)
            ),
            "{error}"
        );
    }

    #[test]
    fn a_group_of_an_empty_body_is_still_a_group() {
        let mut input = TokenStream::from("()");

        let group = input
            .parse::<Paren<_, Option<S<_>>>>()
            .expect("the empty group should parse");

        assert_eq!(group.start.1.as_str(), "(");
        assert_eq!(group.end.1.as_str(), ")");
    }

    #[test]
    fn the_span_of_a_group_reaches_its_closing_token() {
        let mut input = TokenStream::from("{2, 3}");

        let group = input
            .parse::<Brace<_, Repeat<_>>>()
            .expect("the repetition should parse");

        assert_eq!(group.to_span(), Span::Range(0..6));
    }
}
