//! Semantic analyzer for the `unsyn` language.
//!
//! The grammar of `unsyn` is context-free, so the CST produced by
//! [`crate::syntax`] accepts programs that mean nothing. This module turns one
//! parsed source file into a [`Module`] and reports what is wrong with it:
//!
//! 1. **collection** — every `whitespace`, `lexer` and `syntax` statement is
//!    registered in a [`SymbolTable`], and a name declared twice is rejected.
//! 2. **validation** — the literals, character sets, ranges and repetitions of
//!    every body are evaluated (see [`check`]).
//! 3. **linking** — every bare name a body refers to is resolved against the
//!    definitions of the file, a terminal pattern may not refer to a `syntax`
//!    node, and a file declares its trivia with at most one `whitespace`
//!    statement.
//! 4. **recursion** — the leftmost dependencies of the definitions are computed
//!    and a node that can start with itself is rejected (see [`graph`]).
//!
//! A `<call>` is deliberately *not* resolved: it denotes the matcher the
//! generated code calls, which may be provided by the backend, so the names a
//! file expects from outside are collected in [`Module::matchers`] for the
//! linker instead of being diagnosed here. The analyzer never reads the file
//! system: one source file at a time goes in, the diagnostics of that file come
//! out.
//!
//! ```
//! use parserc::syntax::Syntax;
//! use unsyn::{input::TokenStream, semantics::analyze, syntax::Crate};
//!
//! let mut input = TokenStream::from("lexer FOO -> 'f' 'o' 'o';");
//! let file = Crate::parse(&mut input).unwrap();
//!
//! let module = analyze(&file).unwrap();
//!
//! assert_eq!(module.symbols().len(), 1);
//! assert_eq!(module.definition("FOO").unwrap().kind().keyword(), "lexer");
//! ```

mod check;
mod graph;
mod symbol;
mod value;

pub use check::validate_expr;
pub use graph::{WILDCARD, first_edges, left_recursive, nullable_nodes, walk_calls, walk_refs};
pub use symbol::{Definition, NodeKind, SymbolTable};
pub use value::{dec_usize, dec_value, str_chars, str_value, unicode_char, unicode_value};

use parserc::Span;

use crate::{
    errors::{SemanticsKind, UnsynError},
    input::UnsynInput,
    syntax::{Crate, Item, Stmt},
};

use self::graph::{syntax_refs_of_terminal, unresolved_refs};

/// Runs every semantic check over one parsed source file.
///
/// Returns all diagnostics found instead of stopping at the first one.
#[inline]
pub fn analyze<'a, I>(file: &'a Crate<I>) -> Result<Module<'a, I>, Vec<UnsynError>>
where
    I: UnsynInput,
{
    Analyzer::new(file).analyze()
}

/// The semantic model of one analyzed source file.
#[derive(Debug)]
pub struct Module<'a, I>
where
    I: UnsynInput,
{
    symbols: SymbolTable<'a, I>,
    matchers: Vec<&'a str>,
}

impl<'a, I> Module<'a, I>
where
    I: UnsynInput,
{
    /// The node definitions of this file.
    #[inline]
    pub fn symbols(&self) -> &SymbolTable<'a, I> {
        &self.symbols
    }

    /// Returns the definition of the node `name` declared by this file.
    #[inline]
    pub fn definition(&self, name: &str) -> Option<&Definition<'a, I>> {
        self.symbols.get(name)
    }

    /// The matchers this file calls, in declaration order and without
    /// duplicates.
    ///
    /// A matcher whose name is not a node of this file has to be provided by
    /// the backend, which is a question the linker answers rather than an error
    /// of one file.
    #[inline]
    pub fn matchers(&self) -> &[&'a str] {
        &self.matchers
    }

    /// Returns `true` when the file calls the matcher `name`.
    #[inline]
    pub fn uses_matcher(&self, name: &str) -> bool {
        self.matchers.contains(&name)
    }
}

/// The single-file semantic analyzer.
pub struct Analyzer<'a, I>
where
    I: UnsynInput,
{
    /// The parsed source file being analyzed.
    file: &'a Crate<I>,
    /// The collected node definitions.
    symbols: SymbolTable<'a, I>,
    /// The diagnostics collected so far.
    errors: Vec<UnsynError>,
}

impl<'a, I> Analyzer<'a, I>
where
    I: UnsynInput,
{
    /// Creates an analyzer for `file`.
    #[inline]
    pub fn new(file: &'a Crate<I>) -> Self {
        Self {
            file,
            symbols: SymbolTable::new(),
            errors: Vec::new(),
        }
    }

    /// Returns the diagnostics found so far.
    #[inline]
    pub fn errors(&self) -> &[UnsynError] {
        &self.errors
    }

    /// Runs every phase of the analysis over the file.
    pub fn analyze(mut self) -> Result<Module<'a, I>, Vec<UnsynError>> {
        self.collect();
        self.validate_bodies();
        self.link_bodies();
        self.check_whitespace();
        self.check_recursion();

        let matchers = self.collect_matchers();

        if self.errors.is_empty() {
            Ok(Module {
                symbols: self.symbols,
                matchers,
            })
        } else {
            Err(self.errors)
        }
    }

    /// Phase one: registers every definition of the file.
    fn collect(&mut self) {
        for item in self.file.items.iter() {
            match item {
                Item::Stmt(stmt) => self.collect_stmt(stmt),
                // Trivia carries no meaning.
                Item::S(_) | Item::OuterDoc(_) => {}
            }
        }
    }

    fn collect_stmt(&mut self, stmt: &'a Stmt<I>) {
        let (kind, ident) = match stmt {
            Stmt::Whitespace { ident, .. } => (NodeKind::Whitespace, ident),
            Stmt::Lexer { ident, .. } => (NodeKind::Lexer, ident),
            Stmt::Syntax { ident, .. } => (NodeKind::Syntax, ident),
        };

        if let Err(error) = self
            .symbols
            .insert(Definition::from_stmt(kind, ident, stmt))
        {
            self.errors.push(error);
        }
    }

    /// Phase two: evaluates every literal, set, range and repetition.
    fn validate_bodies(&mut self) {
        for def in self.symbols.iter() {
            validate_expr(def.expr(), &mut self.errors);
        }
    }

    /// Phase three: resolves every reference of every body.
    fn link_bodies(&mut self) {
        let mut errors = Vec::new();

        for def in self.symbols.iter() {
            let terminal = def.kind().is_terminal();

            for ident in unresolved_refs(def.expr(), &self.symbols) {
                let name = ident.0.as_str();

                errors.push(UnsynError::Semantics(
                    SemanticsKind::UndefinedSymbol(name.to_owned()),
                    ident.0.to_span(),
                ));
            }

            if !terminal {
                continue;
            }

            for ident in syntax_refs_of_terminal(def.expr(), &self.symbols) {
                let name = ident.0.as_str().to_owned();

                errors.push(UnsynError::Semantics(
                    SemanticsKind::UnexpectedSyntax(name),
                    ident.0.to_span(),
                ));
            }
        }

        self.errors.extend(errors);
    }

    /// Phase four: rejects a file that declares trivia more than once.
    fn check_whitespace(&mut self) {
        let mut whitespace: Vec<Span> = self
            .symbols
            .iter()
            .filter(|def| def.kind() == NodeKind::Whitespace)
            .map(|def| def.name_span().clone())
            .collect();

        // The first declaration is the trivia of the file; the rest are noise.
        if whitespace.len() < 2 {
            return;
        }

        whitespace.remove(0);

        for span in whitespace {
            self.errors.push(UnsynError::Semantics(
                SemanticsKind::DuplicateWhitespace,
                span,
            ));
        }
    }

    /// Phase five: rejects the nodes that can start with themselves.
    fn check_recursion(&mut self) {
        let mut errors = Vec::new();

        for name in left_recursive(&self.symbols) {
            let span = self
                .symbols
                .get(name)
                .map(|def| def.name_span().clone())
                .unwrap_or(Span::None);

            errors.push(UnsynError::Semantics(
                SemanticsKind::LeftRecursion(name.to_owned()),
                span,
            ));
        }

        self.errors.extend(errors);
    }

    /// Gathers the matchers the file calls.
    fn collect_matchers(&self) -> Vec<&'a str> {
        let mut matchers: Vec<&'a str> = Vec::new();

        for def in self.symbols.iter() {
            walk_calls(def.expr(), &mut |call| {
                let name = call.body.0.as_str();

                if !matchers.contains(&name) {
                    matchers.push(name);
                }
            });
        }

        matchers
    }
}

#[cfg(test)]
mod tests {
    use parserc::syntax::Syntax;

    use crate::input::TokenStream;

    use super::*;

    /// Analyzes `source` and renders the diagnostics it produced.
    ///
    /// The messages are compared instead of the kinds because a test should say
    /// what the user would read, and the span of a diagnostic is noise here.
    fn diagnostics(source: &str) -> Vec<String> {
        let mut input = TokenStream::from(source);
        let file = Crate::parse(&mut input).expect("the test input should parse");

        match analyze(&file) {
            Ok(_) => vec![],
            Err(errors) => errors.iter().map(|error| error.to_string()).collect(),
        }
    }

    /// Asserts that `source` has no semantic problem at all.
    fn assert_well_formed(source: &str) {
        assert_eq!(diagnostics(source), Vec::<String>::new(), "{source}");
    }

    #[test]
    fn collects_every_definition() {
        let mut input =
            TokenStream::from("whitespace W -> ' '; lexer LF -> U+000A; syntax Stmt -> LF;");
        let file = Crate::parse(&mut input).unwrap();
        let module = analyze(&file).unwrap();

        assert_eq!(module.symbols().len(), 3);
        assert_eq!(module.definition("W").unwrap().kind(), NodeKind::Whitespace);
        assert_eq!(module.definition("LF").unwrap().kind(), NodeKind::Lexer);
        assert_eq!(module.definition("Stmt").unwrap().kind(), NodeKind::Syntax);
    }

    #[test]
    fn rejects_a_name_declared_twice() {
        let errors = diagnostics("lexer A -> 'a'; syntax A -> 'b';");

        assert_eq!(errors.len(), 1);
        assert!(errors[0].starts_with("unexpect/invalid: name collision"));
    }

    #[test]
    fn rejects_an_unknown_reference() {
        let errors = diagnostics("lexer A -> MISSING;");

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0],
            "unexpect/invalid: undefined symbol `MISSING`, Range(11..18)"
        );
    }

    #[test]
    fn a_reference_is_resolved_forward_too() {
        assert_well_formed("lexer A -> B; lexer B -> 'b';");
    }

    #[test]
    fn the_wildcard_needs_no_definition() {
        assert_well_formed("lexer A -> _ _*;");
    }

    #[test]
    fn a_set_member_may_be_a_reference() {
        let errors = diagnostics("lexer A -> ['a'-'z', MISSING];");

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("undefined symbol `MISSING`"));
    }

    #[test]
    fn a_call_is_not_a_reference() {
        // `<MISSING>` is a matcher of the backend: only the linker can say
        // whether it exists.
        assert_well_formed("lexer A -> <MISSING>;");

        let mut input = TokenStream::from("lexer A -> <X> <Y> <X>;");
        let file = Crate::parse(&mut input).unwrap();
        let module = analyze(&file).unwrap();

        assert_eq!(module.matchers().to_vec(), vec!["X", "Y"]);
        assert!(module.uses_matcher("Y"));
        assert!(!module.uses_matcher("A"));
    }

    #[test]
    fn a_node_bound_to_a_matcher_of_its_name_is_not_recursive() {
        assert_well_formed("lexer XID_Start -> <XID_Start>;");
    }

    #[test]
    fn rejects_a_reference_to_a_tree_node_from_a_terminal() {
        let errors = diagnostics("lexer A -> B; syntax B -> 'b';");

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0],
            "unexpect/invalid: `syntax` node `B` can not be used by a terminal pattern, Range(11..12)"
        );

        // The other direction is the normal shape of a terminal.
        assert_well_formed("syntax B -> A; lexer A -> 'b';");
        assert_well_formed("whitespace W -> A; lexer A -> ' ';");
    }

    #[test]
    fn rejects_a_second_trivia_declaration() {
        let errors = diagnostics("whitespace A -> ' '; whitespace B -> '\\t';");

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("duplicate `whitespace` declaration"));
    }

    #[test]
    fn rejects_a_directly_left_recursive_node() {
        let errors = diagnostics("lexer A -> A 'a';");

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0],
            "unexpect/invalid: left-recursive definition of node `A`, Range(6..7)"
        );
    }

    #[test]
    fn rejects_a_mutually_left_recursive_cycle() {
        let errors = diagnostics("lexer A -> B; lexer B -> A;");

        assert_eq!(errors.len(), 2);
        assert!(errors.iter().all(|error| error.contains("left-recursive")));
    }

    #[test]
    fn recursion_behind_an_optional_prefix_is_found() {
        // `A` can start with nothing but `A` itself, because `B` is optional.
        let errors = diagnostics("lexer A -> B A 'a'; lexer B -> 'b'?;");

        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert!(errors[0].contains("left-recursive definition of node `A`"));

        // Without the optionality the recursive call is not leftmost.
        assert_well_formed("lexer A -> B A 'a'; lexer B -> 'b';");
    }

    #[test]
    fn a_node_that_consumes_before_recurring_is_fine() {
        assert_well_formed("lexer A -> 'a' A;");
        assert_well_formed("lexer A -> B; lexer B -> 'b';");
    }

    #[test]
    fn rejects_an_empty_set() {
        let errors = diagnostics("lexer A -> [];");

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("empty set expression"));
    }

    #[test]
    fn rejects_an_inverted_range() {
        let errors = diagnostics("lexer A -> ['z'-'a'];");

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("invalid range"));
    }

    #[test]
    fn a_set_member_is_a_string_not_only_a_char() {
        // `'…'` is a string or a char, so `['*/']` is a one-member set of the
        // two-character sequence the comment rules match.
        assert_well_formed("lexer A -> ~['*/', 'ab'];");
    }

    #[test]
    fn a_range_needs_single_characters() {
        // A range is drawn over code points, so both bounds must be a char.
        let errors = diagnostics("lexer A -> ['ab'-'cd'];");

        assert_eq!(errors.len(), 2);
        assert!(
            errors
                .iter()
                .all(|error| error.contains("single character"))
        );
    }

    #[test]
    fn rejects_a_surrogate_code_point() {
        let errors = diagnostics("lexer A -> U+D800;");

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("not a valid unicode code point"));
    }

    #[test]
    fn rejects_a_surrogate_escape() {
        let errors = diagnostics(r"lexer A -> 'x\u{D800}y';");

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("not a valid unicode code point"));
    }

    #[test]
    fn rejects_meaningless_repetitions() {
        assert!(diagnostics("lexer A -> 'a'{0};")[0].contains("invalid repetition range"));
        assert!(diagnostics("lexer A -> 'a'{3..1};")[0].contains("invalid repetition range"));
        assert!(diagnostics("lexer A -> 'a'{..0};")[0].contains("invalid repetition range"));

        assert_well_formed("lexer A -> 'a'{2..3};");
        assert_well_formed("lexer A -> 'a'{0..3};");
        assert_well_formed("lexer A -> 'a'{3};");
    }

    #[test]
    fn separators_in_a_repetition_are_ignored() {
        assert_well_formed("lexer A -> 'a'{1_0..2_0};");
    }

    #[test]
    fn the_analyzer_reports_every_problem_of_the_file() {
        let errors = diagnostics(
            r"
            lexer A -> MISSING;
            lexer A -> A;
            whitespace W -> ' ';
            whitespace V -> '\\t';
            lexer E -> [];
            ",
        );

        assert_eq!(errors.len(), 4, "{errors:#?}");
    }
}
