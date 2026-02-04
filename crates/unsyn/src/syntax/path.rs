use parserc::syntax::Syntax;

use crate::{
    input::UnsynInput,
    lexical::{
        ident::Ident,
        keyword::{Crate, Super, This},
        punct::PathSep,
    },
};

/// A path is a sequence of one ore more path segements separated by `::` tokens;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Path<I>
where
    I: UnsynInput,
{
    /// leading optional path separator
    pub leading_sep: Option<PathSep<I>>,
    /// first segment.
    pub first: PathSegment<I>,
    /// rest segments.
    pub rest: Vec<(PathSep<I>, PathSegment<I>)>,
}

/// Segment of path.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PathSegment<I>
where
    I: UnsynInput,
{
    This(This<I>),
    Super(Super<I>),
    Crate(Crate<I>),
    Ident(Ident<I>),
}
