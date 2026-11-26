//! The scans the generated front end reaches for.
//!
//! `unsyn.us` is written with one external, `<IdentOrKeyword>`, and an external
//! is the caller's to say the meaning of: a build writes the path given to
//! [`external`](crate::codegen::Builder::external) and nothing else, so this
//! module is where the crate that *is* its own caller answers that call.
//!
//! The answer is a scan rather than a walk over smaller rules. The spec says a
//! name is one token, and a token that is read one character at a time through
//! externals of the Unicode properties would put those characters in the tree
//! for no reader to ask. So the start is tested, the run that continues it is
//! taken greedily, and the whole is returned: what a definition's name is worth
//! is the word it spells and nothing below it.
//!
//! The bound is the whole of what the scan uses — one item at a time, a position
//! that can be snapshotted, and a length in the same units `split_to` counts — so
//! the same function serves whatever stream a build is written against, whether
//! it reports its failures as a [`Kind`](parserc::Kind) or as an
//! [`UnsynError`](crate::errors::UnsynError).

use parserc::{Input, Parser};

/// Reads a name: one character that starts a name, then every character that
/// continues one.
///
/// A name may begin with an underscore, which is no XID_Start but is the one
/// punctuation a name wears at its head; the underscore is taken with the start
/// and not before it, so that a lone `_` is a name and not an empty one.
pub fn scan_ident_or_keyword<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: Input<Item = char> + Clone,
{
    let mut whole = input.clone();
    let before = input.len();

    parserc::next_if(|c: char| unicode_ident::is_xid_start(c) || c == '_').parse(input)?;

    parserc::take_while(unicode_ident::is_xid_continue).parse(input)?;

    Ok(whole.split_to(before - input.len()))
}

#[cfg(test)]
mod tests {
    use crate::input::TokenStream;
    use parserc::AsStr;
    use parserc::syntax::SyntaxInput as _;

    /// The scan reads a name whole and leaves what follows for the rule above.
    #[test]
    fn a_name_is_read_whole_and_its_tail_left() {
        let mut input = TokenStream::from("lexer x");

        let taken = input
            .parse::<crate::lexer::IdentOrKeyword<_>>()
            .expect("a name begins here");

        assert_eq!(taken.0.as_str(), "lexer");
        assert_eq!(input.as_str(), " x");
    }

    /// An underscore starts a name, and a lone underscore is a name of one.
    #[test]
    fn an_underscore_starts_a_name() {
        let mut input = TokenStream::from("_gen");

        let taken = input
            .parse::<crate::lexer::IdentOrKeyword<_>>()
            .expect("an underscore begins a name");

        assert_eq!(taken.0.as_str(), "_gen");
    }

    /// A digit starts nothing: the name the scan asks for has a start to find.
    #[test]
    fn a_digit_starts_no_name() {
        let mut input = TokenStream::from("9lives");

        assert!(
            input.parse::<crate::lexer::IdentOrKeyword<_>>().is_err(),
            "no name begins with a digit"
        );
    }
}
