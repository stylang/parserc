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

#[cfg(test)]
mod tests {
    use parserc::{
        Span,
        syntax::{Syntax, SyntaxInput},
    };

    use crate::{
        errors::SemanticsKind,
        input::TokenStream,
        lexical::ident::Ident,
        syntax::{Crate, Item, Stmt},
    };

    use super::*;

    /// The kind, name token and statement of every definition of a parsed file.
    fn parts<I: UnsynInput>(file: &Crate<I>) -> Vec<(NodeKind, &Ident<I>, &Stmt<I>)> {
        file.items
            .iter()
            .filter_map(|item| match item {
                Item::Stmt(stmt) => Some(match stmt {
                    Stmt::Whitespace { ident, .. } => (NodeKind::Whitespace, ident, stmt),
                    Stmt::Lexer { ident, .. } => (NodeKind::Lexer, ident, stmt),
                    Stmt::Syntax { ident, .. } => (NodeKind::Syntax, ident, stmt),
                }),
                _ => None,
            })
            .collect()
    }

    /// Collects a file into a table, keeping the errors it reports.
    fn collect<'a, I: UnsynInput>(file: &'a Crate<I>) -> (SymbolTable<'a, I>, Vec<UnsynError>) {
        let mut table = SymbolTable::new();
        let mut errors = vec![];

        for (kind, name, stmt) in parts(file) {
            if let Err(error) = table.insert(Definition::from_stmt(kind, name, stmt)) {
                errors.push(error);
            }
        }

        (table, errors)
    }

    #[test]
    fn a_kind_is_named_by_the_keyword_that_introduces_it() {
        let kinds = [
            (NodeKind::Whitespace, "whitespace", true),
            (NodeKind::Lexer, "lexer", true),
            (NodeKind::Syntax, "syntax", false),
        ];

        for (kind, keyword, terminal) in kinds {
            assert_eq!(kind.keyword(), keyword);
            assert_eq!(kind.to_string(), keyword);
            assert_eq!(kind.is_terminal(), terminal);
        }
    }

    #[test]
    fn a_definition_is_the_statement_it_was_read_from() {
        let file = TokenStream::from("lexer A -> 'a';")
            .parse::<Crate<_>>()
            .expect("a definition should parse");

        let (table, errors) = collect(&file);

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());

        let def = table.get("A").expect("the name should be found");

        assert_eq!(def.kind(), NodeKind::Lexer);
        assert_eq!(def.name(), "A");
        assert_eq!(def.name_span(), &Span::Range(6..7));
        assert_eq!(def.stmt(), parts(&file)[0].2);
        assert_eq!(def.expr().to_span(), Span::Range(11..14));
    }

    /// A name is claimed by the definition that comes first, which is the one an
    /// error about a later one has to point back at.
    #[test]
    fn a_name_can_only_be_defined_once() {
        let file = TokenStream::from("lexer A -> 'a';\nsyntax A -> 'b';")
            .parse::<Crate<_>>()
            .expect("both definitions should parse");

        let (table, errors) = collect(&file);

        assert_eq!(table.len(), 1, "the second name is taken");
        assert_eq!(
            errors,
            vec![UnsynError::Semantics(
                SemanticsKind::NameCollision(Span::Range(6..7)),
                Span::Range(23..24),
            )],
            "{errors:?}"
        );

        // It keeps the kind it was first defined with.
        assert_eq!(table.get("A").map(Definition::kind), Some(NodeKind::Lexer));
    }

    /// A name answers to one kind only, so a reference that asks for the wrong
    /// one is as good as a reference to nothing.
    #[test]
    fn a_definition_is_found_by_its_name_and_its_kind() {
        let file = TokenStream::from("whitespace W -> ' '; lexer A -> 'a';")
            .parse::<Crate<_>>()
            .expect("both definitions should parse");

        let (table, _) = collect(&file);

        assert_eq!(
            table.get("W").map(Definition::kind),
            Some(NodeKind::Whitespace)
        );
        assert_eq!(
            table
                .get_of_kind("W", NodeKind::Whitespace)
                .map(Definition::name),
            Some("W"),
        );
        assert!(table.get_of_kind("W", NodeKind::Lexer).is_none());
        assert!(table.get_of_kind("A", NodeKind::Whitespace).is_none());
        assert!(table.get("no such node").is_none());

        // There is one trivia matcher per file, and the table knows it by kind
        // alone.
        assert_eq!(table.whitespace().map(Definition::name), Some("W"));

        let no_whitespace = TokenStream::from("lexer A -> 'a';")
            .parse::<Crate<_>>()
            .expect("a definition should parse");

        let (table, _) = collect(&no_whitespace);

        assert!(table.whitespace().is_none());
    }

    #[test]
    fn a_table_keeps_the_order_the_definitions_were_written_in() {
        let file = TokenStream::from("lexer B -> 'b'; syntax C -> 'c'; lexer A -> 'a';")
            .parse::<Crate<_>>()
            .expect("the definitions should parse");

        let (table, _) = collect(&file);

        let names: Vec<&str> = table.iter().map(Definition::name).collect();

        assert_eq!(names, vec!["B", "C", "A"]);
        assert_eq!(table.len(), names.len());
        assert!(
            table
                .iter()
                .all(|def| def.kind().is_terminal() || def.kind() == NodeKind::Syntax)
        );
    }

    #[test]
    fn an_empty_table_holds_nothing() {
        let file = TokenStream::from("//! nothing defined here\n")
            .parse::<Crate<_>>()
            .expect("a file of docs should parse");

        let (table, errors) = collect(&file);

        assert!(table.is_empty());
        assert!(errors.is_empty());
        assert!(table.get("nothing").is_none());
        assert!(table.whitespace().is_none());
    }
}
