//! Semantic analyzer for the `unsyn` language.
//!
//! The grammar of `unsyn` is context-free, so the CST produced by
//! [`crate::syntax`] accepts programs that mean nothing. This module turns one
//! parsed source file into a [`Module`] and reports what is wrong with it:
//!
//! 1. **collection** — every `whitespace`, `lexer` and `syntax` statement is
//!    registered in a [`SymbolTable`], and a name declared twice is rejected.
//! 2. **validation** — the literals, character sets, ranges and repetitions of
//!    every body are evaluated, and the members of a set are compared with one
//!    another (see the `check` pass).
//! 3. **linking** — every bare name a body refers to is resolved against the
//!    definitions of the file, a terminal pattern may not refer to a `syntax`
//!    node, and a file declares its trivia with at most one `whitespace`
//!    statement. The node that statement declares is the separator every rule
//!    of nodes reaches its operands over, so a rule that writes it out as an
//!    operand of its own is noted for it.
//! 4. **recursion** — the leftmost dependencies of the definitions are computed
//!    and a node that can start with itself is rejected, together with the
//!    dependencies of the same position, which catch the nodes that can only be
//!    decided by deciding themselves (see the `graph` pass).
//!
//! The findings of these passes leave in two channels. An [`UnsynError`] says a
//! file denotes nothing, and [`analyze`] answers it with the errors and no model
//! at all. A [`Lint`] says a file denotes something twice, which is worth
//! hearing about but is still a program, so lints travel with the [`Module`] of
//! the file that was accepted (see [`Module::lints`]). A caller that wants a
//! stricter language is free to treat a lint as fatal; the analyzer does not
//! decide that for it.
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
//!
//! // A definition that says one thing twice is still a program: the note comes
//! // back with its model rather than in place of it.
//! let mut input = TokenStream::from("lexer VOWEL -> ['a', 'a'];");
//! let file = Crate::parse(&mut input).unwrap();
//!
//! let module = analyze(&file).unwrap();
//!
//! assert_eq!(module.lints().len(), 1);
//! assert!(
//!     module.lints()[0]
//!         .to_string()
//!         .contains("member `'a'` of this set is already declared")
//! );
//! ```

mod check;
mod graph;
mod symbol;
mod value;

pub use check::validate_expr;
pub use graph::{
    WILDCARD, first_edges, left_recursive, nullable_nodes, same_offset_edges,
    same_offset_recursive, walk_calls, walk_refs,
};
pub use symbol::{Definition, NodeKind, SymbolTable};
pub use value::{dec_usize, dec_value, str_chars, str_value, unicode_char, unicode_value};

use parserc::Span;

use crate::{
    errors::{Diagnostics, Lint, LintKind, SemanticsKind, UnsynError},
    input::UnsynInput,
    syntax::{Crate, Item, Stmt},
};

use self::graph::{syntax_refs_of_terminal, unresolved_refs, walk_sequences};

/// Runs every semantic check over one parsed source file.
///
/// Returns every error found instead of stopping at the first one. A file with
/// no error is a [`Module`] carrying the lints of its definitions.
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
    lints: Vec<Lint>,
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

    /// The reports this file earned without ceasing to be a program.
    ///
    /// A lint never stops a [`Module`] from being built; these are the
    /// definitions that say what another of their members already says. They are
    /// reported in the order the file declares its nodes.
    #[inline]
    pub fn lints(&self) -> &[Lint] {
        &self.lints
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
    /// The diagnostics collected so far, in the two channels.
    report: Diagnostics,
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
            report: Diagnostics::default(),
        }
    }

    /// Returns the errors found so far.
    #[inline]
    pub fn errors(&self) -> &[UnsynError] {
        &self.report.errors
    }

    /// Returns the lints found so far.
    #[inline]
    pub fn lints(&self) -> &[Lint] {
        &self.report.lints
    }

    /// Runs every phase of the analysis over the file.
    pub fn analyze(mut self) -> Result<Module<'a, I>, Vec<UnsynError>> {
        self.collect();
        self.validate_bodies();
        self.link_bodies();
        self.check_whitespace();
        self.check_separators();
        self.check_recursion();

        let matchers = self.collect_matchers();

        // Only an error costs the file its model: a lint travels with it.
        let Diagnostics { errors, lints } = self.report;

        if errors.is_empty() {
            Ok(Module {
                symbols: self.symbols,
                matchers,
                lints,
            })
        } else {
            Err(errors)
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
            self.report.push(error);
        }
    }

    /// Phase two: evaluates every literal, set, range and repetition.
    ///
    /// This is the only phase that writes to both channels of the report: a
    /// value that means nothing is an error, and a value a set already means is
    /// a lint.
    fn validate_bodies(&mut self) {
        for def in self.symbols.iter() {
            validate_expr(def.expr(), &mut self.report);
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

        self.report.errors.extend(errors);
    }

    /// Phase four: rejects a file that declares trivia more than once, and
    /// notes the rules of nodes that write its separator out as an operand.
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
            self.report.push(UnsynError::Semantics(
                SemanticsKind::DuplicateWhitespace,
                span,
            ));
        }
    }

    /// Phase five: notes every operand of a rule of nodes that is nothing but a
    /// mention of the trivia node the file declares.
    ///
    /// A rule of nodes is read with the trivia of its language between every two
    /// of its operands, whether it says so or not, so a mention of the trivia
    /// there denotes the separator rather than an operand: the
    /// [codegen](crate::codegen) gives it no field, and this is where a file is
    /// told that. A rule with no trivia node of its own mentions an ordinary
    /// token, and is left alone.
    fn check_separators(&mut self) {
        let Some(trivia) = self.symbols.whitespace() else {
            return;
        };

        let name = trivia.name();

        // The spans are collected first: the walk borrows the definitions the
        // report is not allowed to disturb.
        let mut mentions = Vec::new();

        for def in self
            .symbols
            .iter()
            .filter(|def| def.kind() == NodeKind::Syntax)
        {
            walk_sequences(def.expr(), &mut |seq| {
                for ident in seq.fold_separators(Some(name)).1 {
                    mentions.push(ident.0.to_span());
                }
            });
        }

        for span in mentions {
            self.report
                .lint(LintKind::RedundantSeparator(name.to_owned()), span);
        }
    }

    /// Phase six: rejects the nodes that can start with themselves, and the
    /// nodes that can only be decided by deciding themselves.
    fn check_recursion(&mut self) {
        let span_of = |name: &str| {
            self.symbols
                .get(name)
                .map(|def| def.name_span().clone())
                .unwrap_or(Span::None)
        };

        // The leftmost question is the narrower one, and a node that answers it
        // answers the wider one too, so the wider pass must not name it again.
        let leftmost = left_recursive(&self.symbols);

        let mut errors: Vec<UnsynError> = leftmost
            .iter()
            .map(|name| {
                UnsynError::Semantics(
                    SemanticsKind::LeftRecursion((*name).to_owned()),
                    span_of(name),
                )
            })
            .collect();

        errors.extend(
            same_offset_recursive(&self.symbols)
                .into_iter()
                .filter(|name| !leftmost.contains(name))
                .map(|name| {
                    UnsynError::Semantics(
                        SemanticsKind::SameOffsetRecursion(name.to_owned()),
                        span_of(name),
                    )
                }),
        );

        self.report.errors.extend(errors);
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

    /// Analyzes `source` and renders the lints its model carries.
    ///
    /// A file with an error has no model to carry them, so this asks the
    /// question the lint channel exists to answer: what does the file say twice?
    fn lints_of(source: &str) -> Vec<String> {
        let mut input = TokenStream::from(source);
        let file = Crate::parse(&mut input).expect("the test input should parse");

        let module = analyze(&file).unwrap_or_else(|errors| {
            panic!("{source} should be a program, not an error: {errors:#?}")
        });

        module.lints().iter().map(ToString::to_string).collect()
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

    /// The trivia of the language lies between the operands of a rule of nodes
    /// whether the rule says so or not, so a rule that writes it out says the
    /// same thing twice: the note is for the lint channel, since the rule still
    /// denotes the language its separator reads.
    #[test]
    fn notes_a_trivia_node_written_out_as_an_operand() {
        let lints = lints_of("whitespace W -> ' '; lexer A -> 'a'; syntax P -> A W A;");

        assert_eq!(lints.len(), 1, "{lints:#?}");
        assert!(
            lints[0].contains("`W` is the natural separator of the language"),
            "{lints:#?}"
        );

        // Every mention of it is noted, in whatever sequence it stands.
        let lints = lints_of("whitespace W -> ' '; lexer A -> 'a'; syntax P -> A W A (A W A);");

        assert_eq!(lints.len(), 2, "{lints:#?}");

        // An alternative is a sequence of its own, and so is separated of its
        // own.
        let lints = lints_of("whitespace W -> ' '; lexer A -> 'a'; syntax P -> A W A | 'b';");

        assert_eq!(lints.len(), 1, "{lints:#?}");
    }

    /// A mention of the trivia node is a separator only where a separator is
    /// what a sequence takes: everywhere else the node is an ordinary token.
    #[test]
    fn leaves_a_trivia_node_mentioned_elsewhere_alone() {
        // A terminal pattern is matched against the text itself, which has no
        // separators in it.
        assert_eq!(
            lints_of("whitespace W -> ' '; lexer A -> 'a'; lexer B -> 'b' W;"),
            Vec::<String>::new()
        );

        // A repetition of the trivia asks for a run, which a separator only
        // happens to be.
        assert_eq!(
            lints_of("whitespace W -> ' '; lexer A -> 'a'; syntax P -> A W* A;"),
            Vec::<String>::new()
        );

        // The member of a set is a value rather than an operand of a sequence.
        assert_eq!(
            lints_of("whitespace W -> ' '; lexer A -> 'a'; syntax P -> A [W] A;"),
            Vec::<String>::new()
        );

        // A rule of nothing but the trivia is a rule about it, and keeps it.
        assert_eq!(
            lints_of("whitespace W -> ' '; syntax P -> W W;"),
            Vec::<String>::new()
        );

        // With no trivia declared, the name is any other node's to use.
        assert_eq!(
            lints_of("lexer W -> 'w'; lexer A -> 'a'; syntax P -> A W A;"),
            Vec::<String>::new()
        );
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
    fn a_concat_that_must_consume_before_recurring_is_fine() {
        // Either side of `concat` is matched, so one of them that needs input is
        // enough to move the recursion off the position the node starts at.
        assert_well_formed("lexer A -> B concat C A; lexer B -> 'b'; lexer C -> 'c'?;");
        assert_well_formed("lexer A -> B concat C A; lexer B -> 'b'?; lexer C -> 'c';");

        // With nothing to consume on either side the recursion is leftmost after
        // all.
        let errors = diagnostics("lexer A -> B concat C A; lexer B -> 'b'?; lexer C -> 'c'?;");

        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert!(errors[0].contains("left-recursive definition of node `A`"));
    }

    #[test]
    fn a_complement_that_names_the_node_is_accepted() {
        // A complement is a question about one character: the set it refuses is
        // consulted, and `A` there is one of its members rather than a pattern
        // matched here, so no question about `A` is asked of `A`.
        assert_well_formed("lexer A -> ~[A] 'a';");
    }

    #[test]
    fn rejects_a_node_that_is_tested_against_itself() {
        // What an `except` takes away is tested over the match of the target, so
        // excluding itself is asked at its own position whatever it consumes.
        assert_eq!(
            diagnostics("lexer A -> 'a' except A;").len(),
            1,
            "a node can not exclude itself"
        );
        assert_eq!(
            diagnostics("lexer A -> B except A; lexer B -> 'b';").len(),
            1,
            "nor be excluded by what excludes it"
        );

        // Excluding another node that is about its own business is the normal
        // shape of a keyword check.
        assert_well_formed("lexer A -> B except C; lexer B -> 'b'; lexer C -> 'c';");
    }

    #[test]
    fn a_lookahead_behind_mandatory_input_is_fine() {
        // Both the lookahead and the complement are asked one character on, so
        // the recursion is bounded by the input rather than circular.
        assert_well_formed("lexer A -> 'a' followed A;");
        assert_well_formed("lexer A -> 'a' ~[A];");
    }

    #[test]
    fn rejects_a_lookahead_that_can_be_asked_at_its_own_position() {
        // An optional target leaves the lookahead where the node was asked.
        let errors = diagnostics("lexer A -> B followed A; lexer B -> 'b'?;");

        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert!(errors[0].contains("must be evaluated to evaluate itself"));
    }

    #[test]
    fn a_left_recursive_node_is_not_named_twice() {
        // The same node answers both questions; the narrower report is kept.
        let errors = diagnostics("lexer A -> A 'a' | ~[A];");

        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert!(errors[0].contains("left-recursive definition of node `A`"));
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
    fn a_repeated_member_is_reported_without_refusing_the_file() {
        // The set `'a'` written twice is one character said twice: the pattern
        // still matches one language, so the file is a program and the report
        // travels with it instead of replacing it.
        let lints = lints_of("lexer A -> ['a', 'a'];");

        assert_eq!(lints.len(), 1, "{lints:#?}");
        assert!(lints[0].contains("member `'a'` of this set is already declared"));

        // Nothing in this file is wrong by any other rule.
        assert_eq!(diagnostics("lexer A -> ['a', 'a'];"), Vec::<String>::new());
    }

    #[test]
    fn a_member_of_a_complement_set_is_compared_too() {
        assert_eq!(
            lints_of(r"lexer A -> ~['\u{61}', U+0061];").len(),
            1,
            "two spellings of `'a'` are one member"
        );

        // A range covers a character the set already names.
        assert_eq!(lints_of("lexer A -> ['a'-'z', 'c'];").len(), 1);

        // Two ranges that reach past one another share part of what they say.
        assert_eq!(lints_of("lexer A -> ['a'-'c', 'b'-'d'];").len(), 1);
    }

    #[test]
    fn a_repeated_alternative_is_not_a_matter_of_the_set_rules() {
        // A set enumerates the members of one union of characters, which is what
        // makes a repeated member worth saying; a body that repeats an
        // alternative repeats a whole pattern, and the shapes it may match are
        // not what a single file can compare.
        assert!(lints_of("lexer A -> 'a' | 'a';").is_empty());
        assert!(lints_of("lexer A -> ( 'a' | 'a' )+;").is_empty());
        assert!(lints_of("lexer A -> 'ab' | 'a';").is_empty());

        // Two different names are still not comparable here, since the shape of
        // the node each of them denotes is not what one file knows (see
        // `validate_set_item`).
        assert!(lints_of("lexer A -> [LF, CR]; lexer LF -> '\\n'; lexer CR -> '\\r';").is_empty());

        // The same name twice, however, is one member written twice, whatever
        // the node it names turns out to denote.
        assert_eq!(lints_of("lexer A -> [LF, LF]; lexer LF -> '\\n';").len(), 1);
    }

    #[test]
    fn a_lint_of_one_definition_says_nothing_about_another() {
        let lints = lints_of(
            r"
            lexer A -> ['a', 'a'];
            lexer B -> ['b', 'b', 'b'];
            ",
        );

        // Each set reports its own repeats, in the order the file declares them.
        assert_eq!(lints.len(), 3, "{lints:#?}");
        assert!(lints[0].contains("member `'a'`"), "{lints:#?}");
        assert!(lints[1].contains("member `'b'`"), "{lints:#?}");
        assert!(lints[2].contains("member `'b'`"), "{lints:#?}");
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
        assert!(diagnostics("lexer A -> 'a'{3, 1};")[0].contains("invalid repetition range"));
        assert!(diagnostics("lexer A -> 'a'{, 0};")[0].contains("invalid repetition range"));

        assert_well_formed("lexer A -> 'a'{2, 3};");
        assert_well_formed("lexer A -> 'a'{0, 3};");
        assert_well_formed("lexer A -> 'a'{3};");
    }

    #[test]
    fn separators_in_a_repetition_are_ignored() {
        assert_well_formed("lexer A -> 'a'{1_0, 2_0};");
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
