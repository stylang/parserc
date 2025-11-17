//! Input type for regex engine.

use std::fmt::Debug;

use parserc::{AsBytes, AsStr, Find, Input, StartWith, chars};

use crate::errors::RegexError;

/// Input for regex engine.
pub trait PatternInput:
    Input<Item = char, Error = RegexError>
    + AsBytes
    + AsStr
    + StartWith<&'static str>
    + Find<&'static str>
    + Clone
    + Debug
    + PartialEq
{
}

/// `Input` for pattern parsers.
pub type TokenStream<'a> = chars::TokenStream<'a, RegexError>;

impl<'a> PatternInput for TokenStream<'a> {}
