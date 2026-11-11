//! Node definitions collected from a source file.

use std::{collections::HashMap, fmt};

use parserc::Span;

use crate::{
    errors::{SemanticsKind, UnsynError},
    input::UnsynInput,
    lexical::ident::Ident,
    syntax::{Expr, Stmt},
};

/// The kind of a node introduced by a [`Stmt`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NodeKind {
    /// A `whitespace` node: the trivia matcher of the language.
    Whitespace,
    /// A `lexer` node: a terminal pattern.
    Lexer,
    /// A `syntax` node: a non-terminal of the concrete syntax tree.
    Syntax,
}

impl NodeKind {
    /// The keyword that introduces a definition of this kind.
    #[inline]
    pub fn keyword(&self) -> &'static str {
        match self {
            NodeKind::Whitespace => "whitespace",
            NodeKind::Lexer => "lexer",
            NodeKind::Syntax => "syntax",
        }
    }

    /// Returns `true` when nodes of this kind are terminal patterns.
    #[inline]
    pub fn is_terminal(&self) -> bool {
        matches!(self, NodeKind::Whitespace | NodeKind::Lexer)
    }
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.keyword())
    }
}

/// One `whitespace | lexer | syntax` definition of a source file.
#[derive(Debug, Clone)]
pub struct Definition<'a, I>
where
    I: UnsynInput,
{
    kind: NodeKind,
    name: &'a str,
    name_span: Span,
    stmt: &'a Stmt<I>,
}

impl<'a, I> Definition<'a, I>
where
    I: UnsynInput,
{
    /// Builds a definition from a statement, its name token and the kind of the
    /// introducing keyword.
    #[inline]
    pub fn from_stmt(kind: NodeKind, name: &'a Ident<I>, stmt: &'a Stmt<I>) -> Self {
        let name_span = name.0.to_span();

        Self {
            kind,
            name: name.0.as_str(),
            name_span,
            stmt,
        }
    }

    /// The kind of this node.
    #[inline]
    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    /// The name this node is defined by.
    #[inline]
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// The source region of the node name.
    #[inline]
    pub fn name_span(&self) -> &Span {
        &self.name_span
    }

    /// The statement that defines this node.
    #[inline]
    pub fn stmt(&self) -> &'a Stmt<I> {
        self.stmt
    }

    /// The body expression of this node.
    #[inline]
    pub fn expr(&self) -> &'a Expr<I> {
        stmt_expr(self.stmt)
    }
}

/// The body expression of a definition statement.
#[inline]
pub(crate) fn stmt_expr<I: UnsynInput>(stmt: &Stmt<I>) -> &Expr<I> {
    match stmt {
        Stmt::Whitespace { expr, .. } | Stmt::Lexer { expr, .. } | Stmt::Syntax { expr, .. } => {
            expr
        }
    }
}

/// The node definitions of one source file, indexed by name.
#[derive(Debug)]
pub struct SymbolTable<'a, I>
where
    I: UnsynInput,
{
    definitions: Vec<Definition<'a, I>>,
    index: HashMap<&'a str, usize>,
}

impl<'a, I> Default for SymbolTable<'a, I>
where
    I: UnsynInput,
{
    #[inline]
    fn default() -> Self {
        Self {
            definitions: Default::default(),
            index: Default::default(),
        }
    }
}

impl<'a, I> SymbolTable<'a, I>
where
    I: UnsynInput,
{
    /// Creates an empty symbol table.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a definition.
    ///
    /// Reports [`SemanticsKind::NameCollision`] when the name is already taken.
    pub fn insert(&mut self, def: Definition<'a, I>) -> Result<(), UnsynError> {
        if let Some(&previous) = self.index.get(def.name) {
            return Err(UnsynError::Semantics(
                SemanticsKind::NameCollision(self.definitions[previous].name_span.clone()),
                def.name_span.clone(),
            ));
        }

        self.index.insert(def.name, self.definitions.len());
        self.definitions.push(def);

        Ok(())
    }

    /// Returns the definition of `name`.
    #[inline]
    pub fn get(&self, name: &str) -> Option<&Definition<'a, I>> {
        self.index.get(name).map(|&i| &self.definitions[i])
    }

    /// Returns the definition of a node of the given kind, if there is exactly one.
    pub fn get_of_kind(&self, name: &str, kind: NodeKind) -> Option<&Definition<'a, I>> {
        self.get(name).filter(|def| def.kind() == kind)
    }

    /// Returns the `whitespace` declaration of this file.
    #[inline]
    pub fn whitespace(&self) -> Option<&Definition<'a, I>> {
        self.definitions
            .iter()
            .find(|def| def.kind() == NodeKind::Whitespace)
    }

    /// Iterates the definitions in declaration order.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Definition<'a, I>> {
        self.definitions.iter()
    }

    /// Returns the number of collected definitions.
    #[inline]
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Returns `true` when no definition has been collected.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }
}

impl<'a, I> Clone for SymbolTable<'a, I>
where
    I: UnsynInput,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            definitions: self.definitions.clone(),
            index: self.index.clone(),
        }
    }
}
