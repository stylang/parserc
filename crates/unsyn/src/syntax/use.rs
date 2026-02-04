use parserc::syntax::{Punctuated, Syntax};

use crate::{
    errors::SyntaxKind,
    input::UnsynInput,
    lexical::{
        delimiter::Brace,
        ident::Ident,
        keyword::{As, Mod, Use},
        punct::{Comma, PathSep, Star},
    },
    syntax::Path,
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
    use parserc::syntax::{Delimiter, SyntaxInput};

    use super::*;
    use crate::{
        input::TokenStream,
        lexical::{
            S,
            keyword::This,
            punct::{BraceEnd, BraceStart},
        },
        syntax::{PathSegment, UseDeclaration},
    };

    #[test]
    fn test_use_declaration() {
        assert_eq!(
            TokenStream::from("use a::b::{c, d, e::f, g::h::*}").parse::<UseDeclaration<_>>(),
            Ok(UseDeclaration {
                keyword: Use(
                    TokenStream::from((0, "use")),
                    Some(S(TokenStream::from((3, " "))))
                ),
                use_tree: UseTree::Group {
                    prefix: Some((
                        Some(Path {
                            leading_sep: None,
                            first: PathSegment::Ident(Ident(TokenStream::from((4, "a")))),
                            rest: vec![(
                                PathSep(None, TokenStream::from((5, "::")), None),
                                PathSegment::Ident(Ident(TokenStream::from((7, "b"))))
                            )]
                        }),
                        PathSep(None, TokenStream::from((8, "::")), None)
                    )),
                    group: Delimiter {
                        start: BraceStart(None, TokenStream::from((10, "{")), None),
                        end: BraceEnd(None, TokenStream::from((30, "}")), None),
                        body: Punctuated {
                            pairs: vec![
                                (
                                    UseTree::Path(
                                        Path {
                                            leading_sep: None,
                                            first: PathSegment::Ident(Ident(TokenStream::from((
                                                11, "c"
                                            )))),
                                            rest: vec![]
                                        },
                                        None
                                    ),
                                    Comma(
                                        None,
                                        TokenStream::from((12, ",")),
                                        Some(S(TokenStream::from((13, " "))))
                                    )
                                ),
                                (
                                    UseTree::Path(
                                        Path {
                                            leading_sep: None,
                                            first: PathSegment::Ident(Ident(TokenStream::from((
                                                14, "d"
                                            )))),
                                            rest: vec![]
                                        },
                                        None
                                    ),
                                    Comma(
                                        None,
                                        TokenStream::from((15, ",")),
                                        Some(S(TokenStream::from((16, " "))))
                                    )
                                ),
                                (
                                    UseTree::Path(
                                        Path {
                                            leading_sep: None,
                                            first: PathSegment::Ident(Ident(TokenStream::from((
                                                17, "e"
                                            )))),
                                            rest: vec![(
                                                PathSep(None, TokenStream::from((18, "::")), None),
                                                PathSegment::Ident(Ident(TokenStream::from((
                                                    20, "f"
                                                ))))
                                            )]
                                        },
                                        None
                                    ),
                                    Comma(
                                        None,
                                        TokenStream::from((21, ",")),
                                        Some(S(TokenStream::from((22, " "))))
                                    )
                                )
                            ],
                            tail: Some(Box::new(UseTree::Star {
                                prefix: Some((
                                    Some(Path {
                                        leading_sep: None,
                                        first: PathSegment::Ident(Ident(TokenStream::from((
                                            23, "g"
                                        )))),
                                        rest: vec![(
                                            PathSep(None, TokenStream::from((24, "::")), None),
                                            PathSegment::Ident(Ident(TokenStream::from((26, "h"))))
                                        )]
                                    }),
                                    PathSep(None, TokenStream::from((27, "::")), None)
                                )),
                                star: Star(None, TokenStream::from((29, "*")), None)
                            }))
                        }
                    }
                }
            })
        );

        assert_eq!(
            TokenStream::from("use a::b::{this as ab, c, d::{*, e::f}}")
                .parse::<UseDeclaration<_>>(),
            Ok(UseDeclaration {
                keyword: Use(
                    TokenStream::from((0, "use")),
                    Some(S(TokenStream::from((3, " "))))
                ),
                use_tree: UseTree::Group {
                    prefix: Some((
                        Some(Path {
                            leading_sep: None,
                            first: PathSegment::Ident(Ident(TokenStream::from((4, "a")))),
                            rest: vec![(
                                PathSep(None, TokenStream::from((5, "::")), None),
                                PathSegment::Ident(Ident(TokenStream::from((7, "b"))))
                            )]
                        }),
                        PathSep(None, TokenStream::from((8, "::")), None)
                    )),
                    group: Delimiter {
                        start: BraceStart(None, TokenStream::from((10, "{")), None),
                        end: BraceEnd(None, TokenStream::from((38, "}")), None),
                        body: Punctuated {
                            pairs: vec![
                                (
                                    UseTree::Path(
                                        Path {
                                            leading_sep: None,
                                            first: PathSegment::This(This(
                                                TokenStream::from((11, "this")),
                                                Some(S(TokenStream::from((15, " "))))
                                            )),
                                            rest: vec![]
                                        },
                                        Some((
                                            As(
                                                TokenStream::from((16, "as")),
                                                Some(S(TokenStream::from((18, " "))))
                                            ),
                                            Ident(TokenStream::from((19, "ab")))
                                        ))
                                    ),
                                    Comma(
                                        None,
                                        TokenStream::from((21, ",")),
                                        Some(S(TokenStream::from((22, " "))))
                                    )
                                ),
                                (
                                    UseTree::Path(
                                        Path {
                                            leading_sep: None,
                                            first: PathSegment::Ident(Ident(TokenStream::from((
                                                23, "c"
                                            )))),
                                            rest: vec![]
                                        },
                                        None
                                    ),
                                    Comma(
                                        None,
                                        TokenStream::from((24, ",")),
                                        Some(S(TokenStream::from((25, " "))))
                                    )
                                )
                            ],
                            tail: Some(Box::new(UseTree::Group {
                                prefix: Some((
                                    Some(Path {
                                        leading_sep: None,
                                        first: PathSegment::Ident(Ident(TokenStream::from((
                                            26, "d"
                                        )))),
                                        rest: vec![]
                                    }),
                                    PathSep(None, TokenStream::from((27, "::")), None)
                                )),
                                group: Delimiter {
                                    start: BraceStart(None, TokenStream::from((29, "{")), None),
                                    end: BraceEnd(None, TokenStream::from((37, "}")), None),
                                    body: Punctuated {
                                        pairs: vec![(
                                            UseTree::Star {
                                                prefix: None,
                                                star: Star(
                                                    None,
                                                    TokenStream::from((30, "*")),
                                                    None
                                                )
                                            },
                                            Comma(
                                                None,
                                                TokenStream::from((31, ",")),
                                                Some(S(TokenStream::from((32, " "))))
                                            )
                                        )],
                                        tail: Some(Box::new(UseTree::Path(
                                            Path {
                                                leading_sep: None,
                                                first: PathSegment::Ident(Ident(
                                                    TokenStream::from((33, "e"))
                                                )),
                                                rest: vec![(
                                                    PathSep(
                                                        None,
                                                        TokenStream::from((34, "::")),
                                                        None
                                                    ),
                                                    PathSegment::Ident(Ident(TokenStream::from((
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
