//! The input types for `opgen` parsing.

use std::fmt::Debug;

use parserc::{AsBytes, AsStr, Find, Input, StartWith};

use crate::errors::OpgenError;

/// The `Input` trait for `unsyn` parsing.
pub trait UnsynInput:
    Input<Item = char, Error = OpgenError>
    + AsBytes
    + AsStr
    + StartWith<&'static str>
    + StartWith<&'static [u8]>
    + Find<&'static str>
    + Find<&'static [u8]>
    + Clone
    + Debug
    + PartialEq
{
}

/// A implementation of `CSTInput`.
pub type TokenStream<'a> = parserc::chars::TokenStream<'a, OpgenError>;

impl<'a> UnsynInput for TokenStream<'a> {}
