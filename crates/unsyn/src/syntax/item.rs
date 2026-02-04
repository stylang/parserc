use parserc::syntax::Syntax;

use crate::{
    input::UnsynInput,
    lexical::{
        S,
        comments::{InnerBlockDoc, InnerLineDoc, OuterBlockDoc, OuterLineDoc},
        punct::Semi,
    },
    syntax::{ModuleDeclaration, Stmt, UseDeclaration},
};

/// inner document for source file.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InnerDoc<I>
where
    I: UnsynInput,
{
    Block(Option<S<I>>, InnerBlockDoc<I>),
    Line(Option<S<I>>, InnerLineDoc<I>),
}

/// Outer document for stmts.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OuterDoc<I>
where
    I: UnsynInput,
{
    Block(Option<S<I>>, OuterBlockDoc<I>),
    Line(Option<S<I>>, OuterLineDoc<I>),
}

/// Item of one source file.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Item<I>
where
    I: UnsynInput,
{
    OuterDoc(OuterDoc<I>),
    S(S<I>),
    Use(UseDeclaration<I>, Semi<I>),
    Mod(ModuleDeclaration<I>, Semi<I>),
    Stmt(Stmt<I>),
}

/// The output of one source file.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Crate<I>
where
    I: UnsynInput,
{
    /// inner-doucments list of this crate.
    pub inner_docs: Vec<InnerDoc<I>>,
    /// child-items of this crate.
    pub items: Vec<Item<I>>,
}
