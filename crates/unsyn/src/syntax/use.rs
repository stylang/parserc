use parserc::syntax::{Punctuated, Syntax};

use crate::{
    errors::SyntaxKind,
    input::UnsynInput,
    syntax::Path,
    token::{
        delimiter::Brace,
        ident::Ident,
        keyword::{As, Mod, Use},
        punct::{Comma, PathSep, Star},
    },
};

/// A use declaration creates one ore more local name bindings synonymous with some other path.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UseDeclaration<I>
where
    I: UnsynInput,
{
    /// required leading keyword `use`
    #[parserc(crucial)]
    pub keyword: Use<I>,
    /// Recursive use tree.
    pub use_tree: UseTree<I>,
}

/// Recursive use tree.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = SyntaxKind::UseTree.map())]
pub enum UseTree<I>
where
    I: UnsynInput,
{
    Star {
        /// optional path prefix.
        prefix: Option<(Option<Path<I>>, PathSep<I>)>,
        /// punct `*`.
        star: Star<I>,
    },
    Group {
        /// Optional path prefix.
        prefix: Option<(Option<Path<I>>, PathSep<I>)>,
        /// A set of subpaths.
        group: Brace<I, Punctuated<UseTree<I>, Comma<I>>>,
    },
    Path(
        /// from path
        Path<I>,
        /// Optional as branch.
        Option<(As<I>, Ident<I>)>,
    ),
}

/// Declare a module.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModuleDeclaration<I>
where
    I: UnsynInput,
{
    /// leading keyword `mod`
    pub keyword: Mod<I>,
    /// module name.
    pub ident: Ident<I>,
}

#[cfg(test)]
mod tests {
    use parserc::syntax::{Delimiter, SyntaxExt};

    use super::*;
    use crate::{
        input::Chars,
        syntax::{PathSegment, UseDeclaration},
        token::{
            S,
            keyword::This,
            punct::{BraceEnd, BraceStart},
        },
    };

    #[test]
    fn test_use_declaration() {
        assert_eq!(
            Chars::begin("use a::b::{c, d, e::f, g::h::*}").parse::<UseDeclaration<_>>(),
            Ok(UseDeclaration {
                keyword: Use(Chars::from((0, "use")), Some(S(Chars::from((3, " "))))),
                use_tree: UseTree::Group {
                    prefix: Some((
                        Some(Path {
                            leading_sep: None,
                            first: PathSegment::Ident(Ident(Chars::from((4, "a")))),
                            rest: vec![(
                                PathSep(None, Chars::from((5, "::")), None),
                                PathSegment::Ident(Ident(Chars::from((7, "b"))))
                            )]
                        }),
                        PathSep(None, Chars::from((8, "::")), None)
                    )),
                    group: Delimiter {
                        start: BraceStart(None, Chars::from((10, "{")), None),
                        end: BraceEnd(None, Chars::from((30, "}")), None),
                        body: Punctuated {
                            pairs: vec![
                                (
                                    UseTree::Path(
                                        Path {
                                            leading_sep: None,
                                            first: PathSegment::Ident(Ident(Chars::from((
                                                11, "c"
                                            )))),
                                            rest: vec![]
                                        },
                                        None
                                    ),
                                    Comma(
                                        None,
                                        Chars::from((12, ",")),
                                        Some(S(Chars::from((13, " "))))
                                    )
                                ),
                                (
                                    UseTree::Path(
                                        Path {
                                            leading_sep: None,
                                            first: PathSegment::Ident(Ident(Chars::from((
                                                14, "d"
                                            )))),
                                            rest: vec![]
                                        },
                                        None
                                    ),
                                    Comma(
                                        None,
                                        Chars::from((15, ",")),
                                        Some(S(Chars::from((16, " "))))
                                    )
                                ),
                                (
                                    UseTree::Path(
                                        Path {
                                            leading_sep: None,
                                            first: PathSegment::Ident(Ident(Chars::from((
                                                17, "e"
                                            )))),
                                            rest: vec![(
                                                PathSep(None, Chars::from((18, "::")), None),
                                                PathSegment::Ident(Ident(Chars::from((20, "f"))))
                                            )]
                                        },
                                        None
                                    ),
                                    Comma(
                                        None,
                                        Chars::from((21, ",")),
                                        Some(S(Chars::from((22, " "))))
                                    )
                                )
                            ],
                            tail: Some(Box::new(UseTree::Star {
                                prefix: Some((
                                    Some(Path {
                                        leading_sep: None,
                                        first: PathSegment::Ident(Ident(Chars::from((23, "g")))),
                                        rest: vec![(
                                            PathSep(None, Chars::from((24, "::")), None),
                                            PathSegment::Ident(Ident(Chars::from((26, "h"))))
                                        )]
                                    }),
                                    PathSep(None, Chars::from((27, "::")), None)
                                )),
                                star: Star(None, Chars::from((29, "*")), None)
                            }))
                        }
                    }
                }
            })
        );

        assert_eq!(
            Chars::begin("use a::b::{self as ab, c, d::{*, e::f}}").parse::<UseDeclaration<_>>(),
            Ok(UseDeclaration {
                keyword: Use(Chars::from((0, "use")), Some(S(Chars::from((3, " "))))),
                use_tree: UseTree::Group {
                    prefix: Some((
                        Some(Path {
                            leading_sep: None,
                            first: PathSegment::Ident(Ident(Chars::from((4, "a")))),
                            rest: vec![(
                                PathSep(None, Chars::from((5, "::")), None),
                                PathSegment::Ident(Ident(Chars::from((7, "b"))))
                            )]
                        }),
                        PathSep(None, Chars::from((8, "::")), None)
                    )),
                    group: Delimiter {
                        start: BraceStart(None, Chars::from((10, "{")), None),
                        end: BraceEnd(None, Chars::from((38, "}")), None),
                        body: Punctuated {
                            pairs: vec![
                                (
                                    UseTree::Path(
                                        Path {
                                            leading_sep: None,
                                            first: PathSegment::This(This(
                                                Chars::from((11, "self")),
                                                Some(S(Chars::from((15, " "))))
                                            )),
                                            rest: vec![]
                                        },
                                        Some((
                                            As(
                                                Chars::from((16, "as")),
                                                Some(S(Chars::from((18, " "))))
                                            ),
                                            Ident(Chars::from((19, "ab")))
                                        ))
                                    ),
                                    Comma(
                                        None,
                                        Chars::from((21, ",")),
                                        Some(S(Chars::from((22, " "))))
                                    )
                                ),
                                (
                                    UseTree::Path(
                                        Path {
                                            leading_sep: None,
                                            first: PathSegment::Ident(Ident(Chars::from((
                                                23, "c"
                                            )))),
                                            rest: vec![]
                                        },
                                        None
                                    ),
                                    Comma(
                                        None,
                                        Chars::from((24, ",")),
                                        Some(S(Chars::from((25, " "))))
                                    )
                                )
                            ],
                            tail: Some(Box::new(UseTree::Group {
                                prefix: Some((
                                    Some(Path {
                                        leading_sep: None,
                                        first: PathSegment::Ident(Ident(Chars::from((26, "d")))),
                                        rest: vec![]
                                    }),
                                    PathSep(None, Chars::from((27, "::")), None)
                                )),
                                group: Delimiter {
                                    start: BraceStart(None, Chars::from((29, "{")), None),
                                    end: BraceEnd(None, Chars::from((37, "}")), None),
                                    body: Punctuated {
                                        pairs: vec![(
                                            UseTree::Star {
                                                prefix: None,
                                                star: Star(None, Chars::from((30, "*")), None)
                                            },
                                            Comma(
                                                None,
                                                Chars::from((31, ",")),
                                                Some(S(Chars::from((32, " "))))
                                            )
                                        )],
                                        tail: Some(Box::new(UseTree::Path(
                                            Path {
                                                leading_sep: None,
                                                first: PathSegment::Ident(Ident(Chars::from((
                                                    33, "e"
                                                )))),
                                                rest: vec![(
                                                    PathSep(None, Chars::from((34, "::")), None),
                                                    PathSegment::Ident(Ident(Chars::from((
                                                        36, "f"
                                                    ))))
                                                )]
                                            },
                                            None
                                        )))
                                    }
                                }
                            }))
                        }
                    }
                }
            })
        );
    }
}
