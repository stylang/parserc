//! A run of whitespace between two tokens.

use parserc::{syntax::Syntax, take_while_with};

use crate::input::UnsynInput;

/// One or more whitespace characters, as the separator between two tokens.
///
/// `unsyn` is whitespace-insensitive, so every token that can be separated from
/// its neighbour owns an `Option<S>` on each side of it; `S` itself only ever
/// matches a *non-empty* run, which is what makes the `Option` meaningful.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct S<I>(#[parserc(parser = take_while_with(1..,|c: char| c.is_whitespace()) )] pub I)
where
    I: UnsynInput;

#[cfg(test)]
mod tests {
    use parserc::{
        AsStr,
        syntax::{Syntax, SyntaxInput},
    };

    use crate::input::TokenStream;

    use super::*;

    #[test]
    fn a_run_takes_every_whitespace_character_it_sees() {
        let mut input = TokenStream::from(" \t\r\nnext");

        let run = S::parse(&mut input).expect("the run should parse");

        assert_eq!(run.0.as_str(), " \t\r\n");
        assert_eq!(input.as_str(), "next");

        // A run stops at the first character that is not whitespace.
        let mut input = TokenStream::from("  ;");

        assert_eq!(S::parse(&mut input).unwrap().0.as_str(), "  ");
        assert_eq!(input.as_str(), ";");
    }

    #[test]
    fn a_run_of_nothing_is_not_a_run() {
        // The callers hold `Option<S>`, so this failure is how they say "no
        // separator here" without consuming anything.
        assert!(TokenStream::from("").parse::<S<_>>().is_err());
        assert!(TokenStream::from("lexer").parse::<S<_>>().is_err());
    }
}
