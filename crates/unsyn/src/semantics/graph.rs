//! Dependency analysis over the node definitions of a source file.
//!
//! A bare name in a body is a dependency on the node of that name, so the
//! definitions of one file form a graph. A `<call>` is not part of that graph:
//! it names a matcher the backend provides, which the semantic phase of a
//! single file knows nothing about (see [`Module::matchers`](super::Module::matchers)).
//!
//! Two facts about a body drive the recursion check: whether it can match the
//! empty string, and which nodes it can start with. An unknown name is treated
//! as non-empty and as starting with nothing, which never reports a false cycle.

use std::collections::{HashMap, HashSet};

use crate::{
    input::UnsynInput,
    lexical::{delimiter::Angle, ident::Ident},
    semantics::{NodeKind, SymbolTable, check::items, check::repeat_lower, value::str_chars},
    syntax::{Expr, ExprNoTopAlt, ExprNoTopAlts, ExprWithSuffix, ExprWithoutSuffix, SetItem},
};

/// The name of the built-in matcher of any single character.
///
/// `_` is written like a node reference but is never declared, so it takes part
/// in neither resolution nor recursion.
pub const WILDCARD: &str = "_";

/// Computes which of the defined nodes can match the empty string.
///
/// The result is the least fixed point of the definition equations, so cyclic
/// definitions converge instead of overflowing the stack.
pub fn nullable_nodes<'a, I: UnsynInput>(table: &SymbolTable<'a, I>) -> HashMap<&'a str, bool> {
    let mut nullable: HashMap<&'a str, bool> =
        table.iter().map(|def| (def.name(), false)).collect();

    // A round can only turn `false` into `true`, so once no definition changed
    // the map is the fixed point; one round per definition is always enough.
    for _ in 0..=table.len() {
        let mut changed = false;

        for def in table.iter() {
            let value = expr_nullable(def.expr(), table, &nullable);

            if nullable[def.name()] != value {
                nullable.insert(def.name(), value);
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    nullable
}

/// Builds the leftmost dependency graph: every node mapped to the nodes it can
/// start with.
pub fn first_edges<'a, I: UnsynInput>(
    table: &SymbolTable<'a, I>,
    nullable: &HashMap<&'a str, bool>,
) -> HashMap<&'a str, Vec<&'a str>> {
    let mut edges: HashMap<&'a str, Vec<&'a str>> = HashMap::new();

    for def in table.iter() {
        let mut names = vec![];

        expr_first(def.expr(), table, nullable, &mut names);

        edges.insert(def.name(), names);
    }

    edges
}

/// Returns the names of the nodes that are directly or mutually left-recursive.
///
/// A node is left-recursive when it can start with itself: matching it would
/// expand the same rule again before consuming any input, so such a definition
/// can not be compiled.
pub fn left_recursive<'a, I: UnsynInput>(table: &SymbolTable<'a, I>) -> Vec<&'a str> {
    let nullable = nullable_nodes(table);
    let edges = first_edges(table, &nullable);

    let mut done: HashSet<&str> = HashSet::new();
    let mut on_stack: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = Vec::new();
    let mut found: HashSet<&'a str> = HashSet::new();

    for def in table.iter() {
        if !done.contains(def.name()) {
            visit_first(
                def.name(),
                &edges,
                &mut done,
                &mut on_stack,
                &mut stack,
                &mut found,
            );
        }
    }

    table
        .iter()
        .filter(|def| found.contains(def.name()))
        .map(|def| def.name())
        .collect()
}

fn visit_first<'a>(
    name: &'a str,
    edges: &HashMap<&'a str, Vec<&'a str>>,
    done: &mut HashSet<&'a str>,
    on_stack: &mut HashSet<&'a str>,
    stack: &mut Vec<&'a str>,
    found: &mut HashSet<&'a str>,
) {
    stack.push(name);
    on_stack.insert(name);

    if let Some(nexts) = edges.get(name) {
        for &next in nexts {
            if on_stack.contains(next) {
                // The cycle closes on `next`, so every node of the current chain
                // from it on expands itself before consuming any input.
                let start = stack.iter().position(|node| *node == next).unwrap_or(0);

                for member in &stack[start..] {
                    found.insert(member);
                }
            } else if !done.contains(next) {
                visit_first(next, edges, done, on_stack, stack, found);
            }
        }
    }

    stack.pop();
    on_stack.remove(name);
    done.insert(name);
}

/// Calls `f` with every node reference of `expr`, in source order.
///
/// Both the reference of a body and the member of a character set are reported;
/// the `_` wildcard is reported like any other name, so the caller decides what
/// to make of it.
pub fn walk_refs<'a, I, F>(expr: &'a Expr<I>, f: &mut F)
where
    I: UnsynInput,
    F: FnMut(&'a Ident<I>),
{
    alts_refs(&expr.first, f);

    for (_, alts) in expr.rest.iter() {
        alts_refs(alts, f);
    }
}

fn alts_refs<'a, I: UnsynInput, F: FnMut(&'a Ident<I>)>(alts: &'a ExprNoTopAlts<I>, f: &mut F) {
    alt_refs(&alts.first, f);

    for (_, expr) in alts.rest.iter() {
        alt_refs(expr, f);
    }
}

fn alt_refs<'a, I: UnsynInput, F: FnMut(&'a Ident<I>)>(expr: &'a ExprNoTopAlt<I>, f: &mut F) {
    match expr {
        ExprNoTopAlt::WithSuffix(expr) => with_suffix_refs(expr, f),
        ExprNoTopAlt::WithoutSuffix(expr) => without_suffix_refs(expr, f),
    }
}

fn with_suffix_refs<'a, I: UnsynInput, F: FnMut(&'a Ident<I>)>(
    expr: &'a ExprWithSuffix<I>,
    f: &mut F,
) {
    match expr {
        ExprWithSuffix::Star(target, _)
        | ExprWithSuffix::Question(target, _)
        | ExprWithSuffix::Plus(target, _) => without_suffix_refs(target, f),
        // Whatever is excepted is still matched, so it is a dependency too.
        ExprWithSuffix::Except { target, tokens, .. } => {
            without_suffix_refs(target, f);
            without_suffix_refs(tokens, f);
        }
        // The bounds of a repetition are decimal literals, not names.
        ExprWithSuffix::Repeat { target, .. } => without_suffix_refs(target, f),
        ExprWithSuffix::Concat { target, suffix, .. }
        | ExprWithSuffix::Followed { target, suffix, .. } => {
            without_suffix_refs(target, f);
            alt_refs(suffix, f);
        }
    }
}

fn without_suffix_refs<'a, I: UnsynInput, F: FnMut(&'a Ident<I>)>(
    expr: &'a ExprWithoutSuffix<I>,
    f: &mut F,
) {
    match expr {
        ExprWithoutSuffix::Tilde(_, target) => without_suffix_refs(target, f),
        // A call names a backend matcher, so it is not a reference.
        ExprWithoutSuffix::Call(_) => {}
        ExprWithoutSuffix::Ident(ident) => f(ident),
        ExprWithoutSuffix::Paren(paren) => walk_refs(&paren.body, f),
        ExprWithoutSuffix::Set(bracket) => {
            for item in items(&bracket.body) {
                if let SetItem::Ident(ident) = item {
                    f(ident);
                }
            }
        }
        ExprWithoutSuffix::Str(_) | ExprWithoutSuffix::Unicode(_) => {}
    }
}

/// Calls `f` with every `<call>` of `expr`, in source order.
pub fn walk_calls<'a, I, F>(expr: &'a Expr<I>, f: &mut F)
where
    I: UnsynInput,
    F: FnMut(&'a Angle<I, Ident<I>>),
{
    alts_calls(&expr.first, f);

    for (_, alts) in expr.rest.iter() {
        alts_calls(alts, f);
    }
}

fn alts_calls<'a, I: UnsynInput, F: FnMut(&'a Angle<I, Ident<I>>)>(
    alts: &'a ExprNoTopAlts<I>,
    f: &mut F,
) {
    alt_calls(&alts.first, f);

    for (_, expr) in alts.rest.iter() {
        alt_calls(expr, f);
    }
}

fn alt_calls<'a, I: UnsynInput, F: FnMut(&'a Angle<I, Ident<I>>)>(
    expr: &'a ExprNoTopAlt<I>,
    f: &mut F,
) {
    match expr {
        ExprNoTopAlt::WithSuffix(expr) => with_suffix_calls(expr, f),
        ExprNoTopAlt::WithoutSuffix(expr) => without_suffix_calls(expr, f),
    }
}

fn with_suffix_calls<'a, I: UnsynInput, F: FnMut(&'a Angle<I, Ident<I>>)>(
    expr: &'a ExprWithSuffix<I>,
    f: &mut F,
) {
    match expr {
        ExprWithSuffix::Star(target, _)
        | ExprWithSuffix::Question(target, _)
        | ExprWithSuffix::Plus(target, _) => without_suffix_calls(target, f),
        ExprWithSuffix::Except { target, tokens, .. } => {
            without_suffix_calls(target, f);
            without_suffix_calls(tokens, f);
        }
        ExprWithSuffix::Repeat { target, .. } => without_suffix_calls(target, f),
        ExprWithSuffix::Concat { target, suffix, .. }
        | ExprWithSuffix::Followed { target, suffix, .. } => {
            without_suffix_calls(target, f);
            alt_calls(suffix, f);
        }
    }
}

fn without_suffix_calls<'a, I: UnsynInput, F: FnMut(&'a Angle<I, Ident<I>>)>(
    expr: &'a ExprWithoutSuffix<I>,
    f: &mut F,
) {
    match expr {
        ExprWithoutSuffix::Tilde(_, target) => without_suffix_calls(target, f),
        ExprWithoutSuffix::Call(call) => f(call),
        ExprWithoutSuffix::Ident(_) => {}
        ExprWithoutSuffix::Paren(paren) => walk_calls(&paren.body, f),
        ExprWithoutSuffix::Set(_) | ExprWithoutSuffix::Str(_) | ExprWithoutSuffix::Unicode(_) => {}
    }
}

/// Returns the node `ident` refers to, `None` for the wildcard and for a name
/// the file does not define.
fn referenced_node<'a, I: UnsynInput>(
    ident: &Ident<I>,
    table: &SymbolTable<'a, I>,
) -> Option<&'a str> {
    let name = ident.0.as_str();

    if name == WILDCARD {
        return None;
    }

    table.get(name).map(|def| def.name())
}

/// Returns `true` when `ident` refers to a node that can match the empty string.
fn ref_nullable<I: UnsynInput>(
    ident: &Ident<I>,
    table: &SymbolTable<'_, I>,
    nullable: &HashMap<&str, bool>,
) -> bool {
    referenced_node(ident, table).is_some_and(|name| nullable.get(name).is_some_and(|empty| *empty))
}

/// Returns the calls of `expr` that denote a `syntax` node of the same file.
///
/// A terminal pattern is matched character by character, so it can not depend
/// on a node that carries trivia of its own.
pub(crate) fn syntax_refs_of_terminal<'a, I: UnsynInput>(
    expr: &'a Expr<I>,
    table: &SymbolTable<'a, I>,
) -> Vec<&'a Ident<I>> {
    let mut found = vec![];

    walk_refs(expr, &mut |ident| {
        if table
            .get(ident.0.as_str())
            .is_some_and(|def| def.kind() == NodeKind::Syntax)
        {
            found.push(ident);
        }
    });

    found
}

/// Returns the references of `expr` that denote no node of the same file.
///
/// The wildcard is not reported: it is built into the language.
pub(crate) fn unresolved_refs<'a, I: UnsynInput>(
    expr: &'a Expr<I>,
    table: &SymbolTable<'a, I>,
) -> Vec<&'a Ident<I>> {
    let mut found = vec![];

    walk_refs(expr, &mut |ident| {
        let name = ident.0.as_str();

        if name != WILDCARD && table.get(name).is_none() {
            found.push(ident);
        }
    });

    found
}

fn expr_nullable<I: UnsynInput>(
    expr: &Expr<I>,
    table: &SymbolTable<'_, I>,
    nullable: &HashMap<&str, bool>,
) -> bool {
    alts_nullable(&expr.first, table, nullable)
        || expr
            .rest
            .iter()
            .any(|(_, alts)| alts_nullable(alts, table, nullable))
}

fn alts_nullable<I: UnsynInput>(
    alts: &ExprNoTopAlts<I>,
    table: &SymbolTable<'_, I>,
    nullable: &HashMap<&str, bool>,
) -> bool {
    alt_nullable(&alts.first, table, nullable)
        && alts
            .rest
            .iter()
            .all(|(_, expr)| alt_nullable(expr, table, nullable))
}

fn alt_nullable<I: UnsynInput>(
    expr: &ExprNoTopAlt<I>,
    table: &SymbolTable<'_, I>,
    nullable: &HashMap<&str, bool>,
) -> bool {
    match expr {
        ExprNoTopAlt::WithSuffix(expr) => with_suffix_nullable(expr, table, nullable),
        ExprNoTopAlt::WithoutSuffix(expr) => without_suffix_nullable(expr, table, nullable),
    }
}

fn with_suffix_nullable<I: UnsynInput>(
    expr: &ExprWithSuffix<I>,
    table: &SymbolTable<'_, I>,
    nullable: &HashMap<&str, bool>,
) -> bool {
    match expr {
        ExprWithSuffix::Star(..) | ExprWithSuffix::Question(..) => true,
        ExprWithSuffix::Plus(target, _) => without_suffix_nullable(target, table, nullable),
        ExprWithSuffix::Repeat { target, suffix } => {
            // `{0..N}` may match nothing at all, and any other count matches the
            // empty string when its target does.
            repeat_lower(&suffix.body) == 0 || without_suffix_nullable(target, table, nullable)
        }
        ExprWithSuffix::Concat { target, suffix, .. } => {
            without_suffix_nullable(target, table, nullable)
                || alt_nullable(suffix, table, nullable)
        }
        // The right side of `followed` is a lookahead that is never consumed.
        ExprWithSuffix::Followed { target, .. } | ExprWithSuffix::Except { target, .. } => {
            without_suffix_nullable(target, table, nullable)
        }
    }
}

fn without_suffix_nullable<I: UnsynInput>(
    expr: &ExprWithoutSuffix<I>,
    table: &SymbolTable<'_, I>,
    nullable: &HashMap<&str, bool>,
) -> bool {
    match expr {
        // A complement, a set, a call and a code point all need input.
        ExprWithoutSuffix::Tilde(..)
        | ExprWithoutSuffix::Call(_)
        | ExprWithoutSuffix::Set(_)
        | ExprWithoutSuffix::Unicode(_) => false,
        ExprWithoutSuffix::Ident(ident) => ref_nullable(ident, table, nullable),
        ExprWithoutSuffix::Paren(paren) => expr_nullable(&paren.body, table, nullable),
        ExprWithoutSuffix::Str(lit) => str_chars(lit).is_empty(),
    }
}

fn expr_first<'a, I: UnsynInput>(
    expr: &'a Expr<I>,
    table: &SymbolTable<'a, I>,
    nullable: &HashMap<&'a str, bool>,
    names: &mut Vec<&'a str>,
) {
    alts_first(&expr.first, table, nullable, names);

    for (_, alts) in expr.rest.iter() {
        alts_first(alts, table, nullable, names);
    }
}

fn alts_first<'a, I: UnsynInput>(
    alts: &'a ExprNoTopAlts<I>,
    table: &SymbolTable<'a, I>,
    nullable: &HashMap<&'a str, bool>,
    names: &mut Vec<&'a str>,
) {
    let mut elements = vec![&alts.first];
    elements.extend(alts.rest.iter().map(|(_, expr)| expr));

    for expr in elements {
        alt_first(expr, table, nullable, names);

        // A sequence continues into its next element only when the current one
        // can match nothing.
        if !alt_nullable(expr, table, nullable) {
            break;
        }
    }
}

fn alt_first<'a, I: UnsynInput>(
    expr: &'a ExprNoTopAlt<I>,
    table: &SymbolTable<'a, I>,
    nullable: &HashMap<&'a str, bool>,
    names: &mut Vec<&'a str>,
) {
    match expr {
        ExprNoTopAlt::WithSuffix(expr) => with_suffix_first(expr, table, nullable, names),
        ExprNoTopAlt::WithoutSuffix(expr) => without_suffix_first(expr, table, nullable, names),
    }
}

fn with_suffix_first<'a, I: UnsynInput>(
    expr: &'a ExprWithSuffix<I>,
    table: &SymbolTable<'a, I>,
    nullable: &HashMap<&'a str, bool>,
    names: &mut Vec<&'a str>,
) {
    match expr {
        ExprWithSuffix::Star(target, _)
        | ExprWithSuffix::Question(target, _)
        | ExprWithSuffix::Plus(target, _)
        | ExprWithSuffix::Repeat { target, .. } => {
            without_suffix_first(target, table, nullable, names);
        }
        ExprWithSuffix::Concat { target, suffix, .. } => {
            without_suffix_first(target, table, nullable, names);

            if without_suffix_nullable(target, table, nullable) {
                alt_first(suffix, table, nullable, names);
            }
        }
        ExprWithSuffix::Followed { target, .. } | ExprWithSuffix::Except { target, .. } => {
            without_suffix_first(target, table, nullable, names);
        }
    }
}

fn without_suffix_first<'a, I: UnsynInput>(
    expr: &'a ExprWithoutSuffix<I>,
    table: &SymbolTable<'a, I>,
    nullable: &HashMap<&'a str, bool>,
    names: &mut Vec<&'a str>,
) {
    match expr {
        // A complement, a call, a set, a string and a code point are terminals:
        // they contribute no node to the leftmost position.
        ExprWithoutSuffix::Tilde(..)
        | ExprWithoutSuffix::Call(_)
        | ExprWithoutSuffix::Set(_)
        | ExprWithoutSuffix::Str(_)
        | ExprWithoutSuffix::Unicode(_) => {}
        ExprWithoutSuffix::Ident(ident) => {
            if let Some(name) = referenced_node(ident, table) {
                names.push(name);
            }
        }
        ExprWithoutSuffix::Paren(paren) => expr_first(&paren.body, table, nullable, names),
    }
}

#[cfg(test)]
mod tests {
    use parserc::{AsStr, syntax::SyntaxInput};

    use crate::input::TokenStream;
    use crate::semantics::Definition;
    use crate::syntax::{Crate, Item, Stmt};

    use super::*;

    /// Collects the definitions of `source` and hands the table to `f`.
    ///
    /// A definition borrows the statement it was collected from, so the parsed
    /// file has to stay alive around the whole check.
    fn with_table<R>(source: &str, f: impl FnOnce(&SymbolTable<'_, TokenStream<'_>>) -> R) -> R {
        let mut input = TokenStream::from(source);
        let file = input
            .parse::<Crate<_>>()
            .expect("the test input should parse");

        let mut table = SymbolTable::new();

        for item in file.items.iter() {
            let Item::Stmt(stmt) = item else { continue };

            let kind = match stmt {
                Stmt::Whitespace { .. } => NodeKind::Whitespace,
                Stmt::Lexer { .. } => NodeKind::Lexer,
                Stmt::Syntax { .. } => NodeKind::Syntax,
            };

            let ident = match stmt {
                Stmt::Whitespace { ident, .. }
                | Stmt::Lexer { ident, .. }
                | Stmt::Syntax { ident, .. } => ident,
            };

            let _ = table.insert(Definition::from_stmt(kind, ident, stmt));
        }

        f(&table)
    }

    #[test]
    fn nullability_is_a_least_fixed_point() {
        // `Empty` is nullable because it calls `Skipped`, which is nullable
        // because it calls `Empty` again: the equations have no acyclic order.
        with_table(
            r"
            lexer Empty -> Skipped;
            lexer Skipped -> Empty?;
            lexer Full -> 'a';
            lexer Mixed -> Full Empty;
            ",
            |table| {
                let nullable = nullable_nodes(table);

                assert!(nullable["Empty"]);
                assert!(nullable["Skipped"]);
                assert!(!nullable["Full"]);
                // `Full` blocks the sequence, so `Mixed` needs input.
                assert!(!nullable["Mixed"]);
            },
        );
    }

    #[test]
    fn first_edges_stop_at_the_first_mandatory_node() {
        with_table(
            r"
            lexer Opt -> 'o'?;
            lexer Head -> Opt Tail 'x';
            lexer Tail -> 't';
            ",
            |table| {
                let nullable = nullable_nodes(table);
                let edges = first_edges(table, &nullable);

                assert_eq!(edges["Head"], vec!["Opt", "Tail"]);
                assert_eq!(edges["Tail"], Vec::<&str>::new());
            },
        );
    }

    #[test]
    fn references_and_calls_are_walked_in_order() {
        with_table("lexer A -> B ( C | [ D ] ) <E> except F;", |table| {
            let expr = table.get("A").unwrap().expr();

            let mut names = vec![];

            walk_refs(expr, &mut |ident| names.push(ident.0.as_str().to_owned()));

            assert_eq!(names, ["B", "C", "D", "F"]);

            let mut calls = vec![];

            walk_calls(expr, &mut |call| {
                calls.push(call.body.0.as_str().to_owned())
            });

            assert_eq!(calls, ["E"]);
        });
    }
}
