//! The externals a generated grammar may be mapped onto.
//!
//! `<NAME>` in a pattern is a call into the backend: it stands for a piece of
//! text this file's rules do not describe, and the analysis of a spec collects
//! the names it expects from outside into the module's list of externals. The
//! generator writes a call of a name it was given as the path it was given, so a
//! project may keep its own implementations anywhere, and a name it was not
//! given is reported rather than guessed at.
//!
//! What lives here is therefore only a set of functions the caller may point at,
//! the two character classes of [`unicode_ident`], which are the externals a
//! grammar of identifiers needs and are what `bootstrap.uns` is generated
//! against.
//!
//! # The contract
//!
//! An external consumes at least one item and returns the part of the stream it
//! took, leaving the rest in place:
//!
//! ```ignore
//! fn NAME<I: unsyn::runtime::GeneratedInput>(input: &mut I) -> Result<I, I::Error>
//! ```
//!
//! That is [`Parser::Output = I`](parserc::Parser) with a function name, so a
//! external is written the same way the combinators of [`parserc`] are: it fails
//! with a [`ControlFlow::Recovable`](parserc::ControlFlow::Recovable) error when
//! the text here is not what it recognises, and never consumes on a failure.
//! Because an external's length is its own business, the generator never folds one
//! into a character class: `<XID_Start>` is run as a step, one call per
//! repetition, and is never scanned ahead.
//!
//! The table a generator starts with is empty, so whoever runs it says name by
//! name what a `<NAME>` calls, with the `with_externals` builder of
//! `codegen::Codegen`. Pointing both classes at this module is the whole of what
//! a grammar that only needs an identifier asks of a backend:
//!
//! ```text
//! Codegen::new().with_externals([
//!     ("XID_Start".to_owned(), quote!(::unsyn::runtime::external::XID_Start)),
//!     ("XID_Continue".to_owned(), quote!(::unsyn::runtime::external::XID_Continue)),
//! ])
//! ```
//!
//! The keyword rule of the DSL asks for one name of its own, `XID_Continue`, and
//! that ask is no different from any other: it is settled by the same table.

use parserc::{Input, Parser};

use super::input::GeneratedInput;

/// Matches one character that may start an identifier of the Unicode standard.
///
/// The external `<XID_Start>` calls this.
//
// The name is the spelling of the external call in a `.uns` file rather than the
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
/// The external `<XID_Continue>` calls this.
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

    /// The stream a generated grammar of this crate would be written over.
    type Stream<'a> = parserc::chars::TokenStream<'a, parserc::Kind>;

    #[test]
    fn an_external_takes_one_character_and_leaves_the_rest() {
        let mut input = Stream::from("aB c");

        let taken = XID_Start(&mut input).expect("an xid start");

        assert_eq!(taken.as_str(), "a");
        assert_eq!(input.as_str(), "B c");
    }

    #[test]
    fn an_external_that_does_not_fit_says_so_without_consuming() {
        let mut input = Stream::from(" 1");

        let error = XID_Start(&mut input).expect_err("a space is not an xid start");

        assert_eq!(error.control_flow(), ControlFlow::Recovable);
        assert_eq!(input.as_str(), " 1");

        // The two classes differ where a digit sits: it continues an identifier
        // but never starts one.
        let mut input = Stream::from("1");

        assert!(XID_Start(&mut input).is_err());
        assert!(XID_Continue(&mut input).is_ok());
    }

    #[test]
    fn an_external_at_the_end_of_the_input_is_incomplete() {
        let mut input = Stream::from("");

        assert_eq!(
            XID_Continue(&mut input).err().map(|err| err.control_flow()),
            Some(ControlFlow::Incomplete)
        );
    }

    #[test]
    fn an_external_is_a_step_a_pattern_can_repeat() {
        // The contract is `Parser<Output = I>`: a generated pattern calls a
        // external through `?` and carries on from wherever it landed, which is
        // how a repetition of `<XID_Continue>` walks along an identifier.
        let mut input = Stream::from("abc 3");

        let mut taken = String::new();

        while let Ok(step) = XID_Continue(&mut input) {
            taken.push_str(step.as_str());
        }

        assert_eq!(taken, "abc");
        assert_eq!(input.as_str(), " 3");
    }
}
