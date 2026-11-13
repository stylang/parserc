//! The matchers a generated grammar can call without the backend providing
//! anything.
//!
//! `<NAME>` in a pattern is a call into the backend: it stands for a piece of
//! text this file's rules do not describe, of which [`Module::matchers`] is the
//! list. The generator emits those calls as
//!
//! ```text
//! unsyn::codegen::matcher::NAME(input)?
//! ```
//!
//! under a path [`Codegen`](super::Codegen) can point anywhere, so a project may
//! keep its own implementations beside its generated file. What is written
//! here — the two character classes of [`unicode_ident`] — is what lets a file
//! that only needs an identifier be generated and compiled without the backend
//! being written first, which is the case for most grammars, including
//! `bootstrap.uns`.
//!
//! # The contract
//!
//! A matcher consumes at least one item and returns the part of the stream it
//! took, leaving the rest in place:
//!
//! ```ignore
//! fn NAME<I: unsyn::codegen::GeneratedInput>(input: &mut I) -> Result<I, I::Error>
//! ```
//!
//! That is [`Parser::Output = I`](parserc::Parser) with a function name, so a
//! matcher is written the same way the combinators of [`parserc`] are: it fails
//! with a [`ControlFlow::Recovable`](parserc::ControlFlow::Recovable) error when
//! the text here is not what it recognises, and never consumes on a failure.
//! Because a matcher's length is its own business, the generator never folds one
//! into a character class: `<XID_Start>` is run as a step, one call per
//! repetition, and is never scanned ahead.
//!
//! [`Module::matchers`]: crate::semantics::Module::matchers

use parserc::{Input, Parser};

use super::input::GeneratedInput;

/// Returns `true` when the matcher `name` is one of the two this module gives,
/// which is what lets a grammar that only needs an identifier be generated and
/// compiled before its backend is written.
#[inline]
pub(crate) fn is_builtin(name: &str) -> bool {
    matches!(name, "XID_Start" | "XID_Continue")
}

/// Matches one character that may start an identifier of the Unicode standard.
///
/// The matcher `<XID_Start>` calls this.
//
// The name is the spelling of the matcher call in a `.uns` file rather than the
// name of a function, so the case convention is asked to leave it alone.
#[allow(non_snake_case)]
pub fn XID_Start<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: GeneratedInput,
{
    parserc::next_if(unicode_ident::is_xid_start).parse(input)
}

/// Matches one character that may continue an identifier of the Unicode
/// standard.
///
/// The matcher `<XID_Continue>` calls this.
#[allow(non_snake_case)]
pub fn XID_Continue<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: GeneratedInput,
{
    parserc::next_if(unicode_ident::is_xid_continue).parse(input)
}

#[cfg(test)]
mod tests {
    use parserc::{AsStr, ControlFlow, ParseError};

    use super::*;
    use crate::input::TokenStream;

    #[test]
    fn a_matcher_takes_one_character_and_leaves_the_rest() {
        let mut input = TokenStream::from("aB c");

        let taken = XID_Start(&mut input).expect("an xid start");

        assert_eq!(taken.as_str(), "a");
        assert_eq!(input.as_str(), "B c");
    }

    #[test]
    fn a_matcher_that_does_not_fit_says_so_without_consuming() {
        let mut input = TokenStream::from(" 1");

        let error = XID_Start(&mut input).expect_err("a space is not an xid start");

        assert_eq!(error.control_flow(), ControlFlow::Recovable);
        assert_eq!(input.as_str(), " 1");

        // The two classes differ where a digit sits: it continues an identifier
        // but never starts one.
        let mut input = TokenStream::from("1");

        assert!(XID_Start(&mut input).is_err());
        assert!(XID_Continue(&mut input).is_ok());
    }

    #[test]
    fn a_matcher_at_the_end_of_the_input_is_incomplete() {
        let mut input = TokenStream::from("");

        assert_eq!(
            XID_Continue(&mut input).err().map(|err| err.control_flow()),
            Some(ControlFlow::Incomplete)
        );
    }

    #[test]
    fn a_matcher_is_a_step_a_pattern_can_repeat() {
        // The contract is `Parser<Output = I>`: a generated pattern calls a
        // matcher through `?` and carries on from wherever it landed, which is
        // how a repetition of `<XID_Continue>` walks along an identifier.
        let mut input = TokenStream::from("abc 3");

        let mut taken = String::new();

        while let Ok(step) = XID_Continue(&mut input) {
            taken.push_str(step.as_str());
        }

        assert_eq!(taken, "abc");
        assert_eq!(input.as_str(), " 3");
    }
}
