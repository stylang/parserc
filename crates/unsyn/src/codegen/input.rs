//! The stream the generated grammars read their text from.
//!
//! [`crate::input::UnsynInput`] is the input of *this* language: it fixes the
//! error type to [`UnsynError`](crate::errors::UnsynError), which a generated
//! grammar must not do — the language it describes reports its own errors, under
//! its own type. What a generated grammar actually needs of a stream is narrower
//! and error-agnostic:
//!
//! * items that are [`char`]s, because a set, a range and a repetition bound all
//!   count characters;
//! * [`AsStr`], so that a scan for a sequence of characters can search the rest
//!   of the text in one go;
//! * [`Start`], so that a literal can be probed without consuming it, which
//!   is what `~` and `except` ask for;
//! * [`Find`], so that `~ […]*` can stop at the first occurrence of a sequence
//!   instead of stepping over it one character at a time;
//! * [`Clone`], because every alternative, every optional operand and every
//!   repetition has to be able to rewind.
//!
//! [`crate::input::TokenStream`] satisfies all of them, which is what lets the
//! tests of this module run the code it generates against the same stream the
//! `unsyn` source itself is read from.

use std::fmt::Debug;

use parserc::{AsStr, Find, Input, StartWith};

/// A character stream a generated grammar can be run over.
///
/// The error type is left to the implementor: a generated node reports failures
/// through [`parserc::Kind`], which every [`ParseError`](parserc::ParseError)
/// is required to be buildable from.
pub trait GeneratedInput:
    Input<Item = char>
    + AsStr
    + StartWith<&'static str>
    + Find<&'static str>
    + Clone
    + Debug
    + PartialEq
{
}

impl<T> GeneratedInput for T where
    T: Input<Item = char>
        + AsStr
        + StartWith<&'static str>
        + Find<&'static str>
        + Clone
        + Debug
        + PartialEq
{
}

#[cfg(test)]
mod tests {
    use parserc::{ControlFlow, ParseError, Parser, keyword};

    use super::*;
    use crate::input::TokenStream;

    #[test]
    fn the_streams_unsyn_reads_are_generated_inputs() {
        // The bound has to hold for the stream the tests of this module parse
        // generated code with.
        fn takes<T: GeneratedInput>(input: T) -> usize {
            input.len()
        }

        assert_eq!(takes(TokenStream::from("abc")), 3);
    }

    #[test]
    fn a_generated_input_can_probe_rewind_and_search() {
        let mut input = TokenStream::from("/* body */ rest");

        assert_eq!(input.starts_with("/*"), Some(2));
        assert_eq!(input.find("*/"), Some(8));
        assert_eq!(input.as_str(), "/* body */ rest");

        // A probe that does not fit is rewound by `ok`, which is how every
        // optional operand of a generated pattern leaves the stream alone.
        let missed = keyword("*/").ok().parse(&mut input).unwrap();

        assert_eq!(missed, None);
        assert_eq!(input.as_str(), "/* body */ rest");

        // `take_until` stops before what it looks for, and says nothing when the
        // text never holds it.
        let skipped = parserc::take_until("*/").parse(&mut input);

        assert_eq!(skipped.unwrap().as_str(), "/* body ");

        let mut absent = TokenStream::from("nothing here");

        assert_eq!(
            parserc::take_until::<TokenStream, _>("zzz")
                .parse(&mut absent)
                .err()
                .map(|err| err.control_flow()),
            Some(ControlFlow::Recovable)
        );
    }
}
