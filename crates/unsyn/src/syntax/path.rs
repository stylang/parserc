use parserc::syntax::Syntax;

use crate::{
    input::UnsynInput,
    token::{
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

#[cfg(test)]
mod tests {
    use parserc::{sourceinput::Source, syntax::SyntaxExt};

    use crate::{input::Chars, syntax::Path};

    use super::*;

    #[test]
    fn start_with_crate() {
        assert_eq!(
            Chars::begin("crate::a::b").parse::<Path<_>>(),
            Ok(Path {
                leading_sep: None,
                first: PathSegment::Crate(Crate(Source::offset(0, "crate"), None)),
                rest: vec![
                    (
                        PathSep(None, Source::offset(5, "::"), None),
                        PathSegment::Ident(Ident(Source::offset(7, "a")))
                    ),
                    (
                        PathSep(None, Source::offset(8, "::"), None),
                        PathSegment::Ident(Ident(Source::offset(10, "b")))
                    )
                ]
            })
        );
    }

    #[test]
    fn start_with_super() {
        assert_eq!(
            Chars::begin("super::a").parse::<Path<_>>(),
            Ok(Path {
                leading_sep: None,
                first: PathSegment::Super(Super(Source::offset(0, "super"), None)),
                rest: vec![(
                    PathSep(None, Source::offset(5, "::"), None),
                    PathSegment::Ident(Ident(Source::offset(7, "a")))
                )]
            })
        );
    }
}
