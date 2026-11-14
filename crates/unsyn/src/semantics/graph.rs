//! Dependency analysis over the node definitions of a source file.
//!
//! A bare name in a body is a dependency on the node of that name, so the
//! definitions of one file form a graph. A `<call>` is not part of that graph:
//! it names an external the backend provides, which the semantic phase of a
//! single file knows nothing about (see [`Module::externals`](super::Module::externals)).
//!
//! Three facts about a body drive the recursion checks: whether it can match the
//! empty string, which nodes it can start with, and which nodes it must consult
//! at the position it is consulted at. The first two build the graph of
//! [`first_edges`], whose cycles are the left-recursive definitions; the third
//! builds [`same_offset_edges`], whose cycles also cover the operands that are
//! only ever tested (what an `except` takes away, and the lookahead of a
//! `followed`) and so can never be the beginning of a match. An unknown name is
//! treated as non-empty and as starting with nothing, which never reports a
//! false cycle.

use std::collections::{HashMap, HashSet};

use crate::{
    input::UnsynInput,
    lexical::{delimiter::Angle, ident::Ident},
    semantics::{NodeKind, SymbolTable, check::items, check::repeat_lower, value::str_chars},
    syntax::{Expr, ExprNoTopAlt, ExprNoTopAlts, ExprWithSuffix, ExprWithoutSuffix, SetItem},
};

/// The name of the pattern that matches any single character.
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

/// Builds the same-offset dependency graph: every node mapped to the nodes it
/// has to consult at the position it is itself consulted at.
///
/// Where [`first_edges`] asks which nodes can *begin* the text a node matches,
/// this asks which nodes must be *evaluated* before that match can be decided.
/// The two differ exactly for the operands that never become part of the match:
/// what an `except` takes away is tested at the position its target started at,
/// however much the target consumes, and the lookahead of `followed` is here too
/// whenever the operand before it can match nothing.
///
/// A complement is not one of them: `~ […]` asks its question of the character
/// here and then takes it, so it is an operand like any other that consumes.
pub fn same_offset_edges<'a, I: UnsynInput>(
    table: &SymbolTable<'a, I>,
    nullable: &HashMap<&'a str, bool>,
) -> HashMap<&'a str, Vec<&'a str>> {
    let mut edges: HashMap<&'a str, Vec<&'a str>> = HashMap::new();

    for def in table.iter() {
        let mut names = vec![];

        expr_same_offset(def.expr(), table, nullable, &mut names);

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
    cyclic_nodes(table, &first_edges(table, &nullable_nodes(table)))
}

/// Returns the names of the nodes that must be evaluated in order to be
/// evaluated.
///
/// This is the wider question: a node can be asked about a position without its
/// answer becoming the text matched there, as `A except B` asks whether `B`
/// matches here in order to match something else. Such a definition can not be
/// compiled either, since deciding it at one position is deciding it at that
/// position, which neither advances the input nor terminates. Every
/// left-recursive node is also one of these, so a report of both must not name
/// a node twice.
pub fn same_offset_recursive<'a, I: UnsynInput>(table: &SymbolTable<'a, I>) -> Vec<&'a str> {
    cyclic_nodes(table, &same_offset_edges(table, &nullable_nodes(table)))
}

/// Names the nodes that sit on a cycle of `edges`, in declaration order.
///
/// A node that merely reaches a cycle is not named: the definitions on the cycle
/// are the ones that can not be compiled.
fn cyclic_nodes<'a, I: UnsynInput>(
    table: &SymbolTable<'a, I>,
    edges: &HashMap<&'a str, Vec<&'a str>>,
) -> Vec<&'a str> {
    let mut done: HashSet<&'a str> = HashSet::new();
    let mut on_stack: HashSet<&'a str> = HashSet::new();
    let mut stack: Vec<&'a str> = Vec::new();
    let mut found: HashSet<&'a str> = HashSet::new();

    for def in table.iter() {
        if !done.contains(def.name()) {
            visit_edges(
                def.name(),
                edges,
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

fn visit_edges<'a>(
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
                visit_edges(next, edges, done, on_stack, stack, found);
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
        // A complement names the set it refuses, exactly as a set does.
        ExprWithoutSuffix::Tilde(_, bracket) | ExprWithoutSuffix::Set(bracket) => {
            for item in items(&bracket.body) {
                if let SetItem::Ident(ident) = item {
                    f(ident);
                }
            }
        }
        // A call names a backend external, so it is not a reference.
        ExprWithoutSuffix::Call(_) => {}
        ExprWithoutSuffix::Ident(ident) => f(ident),
        ExprWithoutSuffix::Paren(paren) => walk_refs(&paren.body, f),
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
        // A complement refuses a set, and a set holds no external call.
        ExprWithoutSuffix::Tilde(..)
        | ExprWithoutSuffix::Set(_)
        | ExprWithoutSuffix::Str(_)
        | ExprWithoutSuffix::Unicode(_) => {}
        ExprWithoutSuffix::Call(call) => f(call),
        ExprWithoutSuffix::Ident(_) => {}
        ExprWithoutSuffix::Paren(paren) => walk_calls(&paren.body, f),
    }
}

/// Calls `f` with every sequence of operands of `expr`, in source order.
///
/// The sequence of each alternative is called for, as are the sequences inside
/// the groups of the rule, since a group is given a shape of its own and is
/// separated just as the rule around it is. What is not a sequence of operands
/// is not walked into as one: the members of a set and of a complement are
/// values rather than operands, and the operand a suffix asserts about the text
/// is taken out of the sequence it is written in.
///
/// This is the walk that reads the separators of a rule of nodes, which is
/// exactly what a shape turns into fields: [`ExprNoTopAlts::fold_separators`]
/// is applied to the sequences this calls `f` with.
pub(crate) fn walk_sequences<'a, I, F>(expr: &'a Expr<I>, f: &mut F)
where
    I: UnsynInput,
    F: FnMut(&'a ExprNoTopAlts<I>),
{
    alts_sequences(&expr.first, f);

    for (_, alts) in expr.rest.iter() {
        alts_sequences(alts, f);
    }
}

fn alts_sequences<'a, I: UnsynInput, F: FnMut(&'a ExprNoTopAlts<I>)>(
    alts: &'a ExprNoTopAlts<I>,
    f: &mut F,
) {
    f(alts);

    for expr in alts.elements() {
        alt_sequences(expr, f);
    }
}

fn alt_sequences<'a, I: UnsynInput, F: FnMut(&'a ExprNoTopAlts<I>)>(
    expr: &'a ExprNoTopAlt<I>,
    f: &mut F,
) {
    match expr {
        ExprNoTopAlt::WithSuffix(expr) => with_suffix_sequences(expr, f),
        ExprNoTopAlt::WithoutSuffix(expr) => without_suffix_sequences(expr, f),
    }
}

fn with_suffix_sequences<'a, I: UnsynInput, F: FnMut(&'a ExprNoTopAlts<I>)>(
    expr: &'a ExprWithSuffix<I>,
    f: &mut F,
) {
    match expr {
        ExprWithSuffix::Star(target, _)
        | ExprWithSuffix::Question(target, _)
        | ExprWithSuffix::Plus(target, _)
        | ExprWithSuffix::Repeat { target, .. } => without_suffix_sequences(target, f),
        // The operand a keyword introduces is an assertion about the text and
        // no element of the sequence, so only a group written inside it is.
        ExprWithSuffix::Concat { target, suffix, .. }
        | ExprWithSuffix::Followed { target, suffix, .. } => {
            without_suffix_sequences(target, f);
            alt_sequences(suffix, f);
        }
        // Whatever an `except` takes away is still shaped, group and all.
        ExprWithSuffix::Except { target, tokens, .. } => {
            without_suffix_sequences(target, f);
            without_suffix_sequences(tokens, f);
        }
    }
}

fn without_suffix_sequences<'a, I: UnsynInput, F: FnMut(&'a ExprNoTopAlts<I>)>(
    expr: &'a ExprWithoutSuffix<I>,
    f: &mut F,
) {
    match expr {
        ExprWithoutSuffix::Paren(paren) => walk_sequences(&paren.body, f),
        // A set and a complement are compared with the text as one value, so
        // the names they hold are not operands of any sequence.
        ExprWithoutSuffix::Set(_) | ExprWithoutSuffix::Tilde(..) => {}
        ExprWithoutSuffix::Ident(_)
        | ExprWithoutSuffix::Call(_)
        | ExprWithoutSuffix::Str(_)
        | ExprWithoutSuffix::Unicode(_) => {}
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
            // `{0, N}` may match nothing at all, and any other count matches the
            // empty string when its target does.
            repeat_lower(&suffix.body) == 0 || without_suffix_nullable(target, table, nullable)
        }
        ExprWithSuffix::Concat { target, suffix, .. } => {
            // Both operands of `concat` are matched and joined into one token,
            // so the sequence can match nothing only when both of them can.
            // The lookahead of `followed` and the taken-away side of `except`
            // are never matched, which is why they do not have to be empty for
            // their own expression to be.
            without_suffix_nullable(target, table, nullable)
                && alt_nullable(suffix, table, nullable)
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
        // Every operand of a complement is a set, which needs input.
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
        // Neither the lookahead of `followed` nor what an `except` takes away is
        // ever the beginning of the match here, so neither contributes a node to
        // the leftmost position. Both are consulted at this very position all the
        // same, which is the question the same-offset graph asks.
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

fn expr_same_offset<'a, I: UnsynInput>(
    expr: &'a Expr<I>,
    table: &SymbolTable<'a, I>,
    nullable: &HashMap<&'a str, bool>,
    names: &mut Vec<&'a str>,
) {
    alts_same_offset(&expr.first, table, nullable, names);

    // Deciding an alternation means trying every alternative here.
    for (_, alts) in expr.rest.iter() {
        alts_same_offset(alts, table, nullable, names);
    }
}

fn alts_same_offset<'a, I: UnsynInput>(
    alts: &'a ExprNoTopAlts<I>,
    table: &SymbolTable<'a, I>,
    nullable: &HashMap<&'a str, bool>,
    names: &mut Vec<&'a str>,
) {
    let mut elements = vec![&alts.first];
    elements.extend(alts.rest.iter().map(|(_, expr)| expr));

    for expr in elements {
        alt_same_offset(expr, table, nullable, names);

        // A later element is consulted where the sequence has already arrived,
        // which is this position only while what precedes it can be empty.
        if !alt_nullable(expr, table, nullable) {
            break;
        }
    }
}

fn alt_same_offset<'a, I: UnsynInput>(
    expr: &'a ExprNoTopAlt<I>,
    table: &SymbolTable<'a, I>,
    nullable: &HashMap<&'a str, bool>,
    names: &mut Vec<&'a str>,
) {
    match expr {
        ExprNoTopAlt::WithSuffix(expr) => with_suffix_same_offset(expr, table, nullable, names),
        ExprNoTopAlt::WithoutSuffix(expr) => {
            without_suffix_same_offset(expr, table, nullable, names)
        }
    }
}

fn with_suffix_same_offset<'a, I: UnsynInput>(
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
            without_suffix_same_offset(target, table, nullable, names);
        }
        // The operand after `concat` and after `followed` starts where the
        // target ended, so it is here too only when the target can be empty.
        ExprWithSuffix::Concat { target, suffix, .. }
        | ExprWithSuffix::Followed { target, suffix, .. } => {
            without_suffix_same_offset(target, table, nullable, names);

            if without_suffix_nullable(target, table, nullable) {
                alt_same_offset(suffix, table, nullable, names);
            }
        }
        // Whatever an `except` takes away is tested against the text of the
        // match itself, so the target has to consume for it to move off here.
        // A set, a string and a code point are compared with the input here.
        ExprWithSuffix::Except { target, tokens, .. } => {
            without_suffix_same_offset(target, table, nullable, names);
            without_suffix_same_offset(tokens, table, nullable, names);
        }
    }
}

fn without_suffix_same_offset<'a, I: UnsynInput>(
    expr: &'a ExprWithoutSuffix<I>,
    table: &SymbolTable<'a, I>,
    nullable: &HashMap<&'a str, bool>,
    names: &mut Vec<&'a str>,
) {
    match expr {
        // A complement always takes the one character it asks about, so nothing
        // is reached at the offset it stands at, and the shape of the set it
        // refuses is not asked about here.
        ExprWithoutSuffix::Tilde(..) => {}
        ExprWithoutSuffix::Ident(ident) => {
            if let Some(name) = referenced_node(ident, table) {
                names.push(name);
            }
        }
        ExprWithoutSuffix::Paren(paren) => expr_same_offset(&paren.body, table, nullable, names),
        // A call and its external are the backend's own affair, a string and a
        // code point are compared to the input directly, and the shape of a
        // referenced set member is deliberately not asked about here (see
        // `validate_set_item` in the `check` pass).
        ExprWithoutSuffix::Call(_)
        | ExprWithoutSuffix::Set(_)
        | ExprWithoutSuffix::Str(_)
        | ExprWithoutSuffix::Unicode(_) => {}
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
    fn a_concat_is_empty_only_when_both_operands_are() {
        // `concat` matches both operands and joins them into one token, so what
        // it matches is empty only when there is nothing on either side of the
        // keyword.
        with_table(
            r"
            lexer Left -> 'l'?;
            lexer Right -> 'r';
            lexer Join -> Left concat Right Tail;
            lexer Tail -> 't';
            ",
            |table| {
                let nullable = nullable_nodes(table);

                assert!(!nullable["Join"]);

                // Both operands of the `concat` begin at this position, but
                // `Tail` does not: the `concat` has to consume its right
                // operand, which closes the leftmost position behind it.
                assert_eq!(first_edges(table, &nullable)["Join"], vec!["Left", "Right"]);
            },
        );

        with_table(
            "lexer Left -> 'l'?; lexer Right -> 'r'?; lexer Join -> Left concat Right;",
            |table| {
                assert!(nullable_nodes(table)["Join"]);
            },
        );
    }

    #[test]
    fn same_offset_edges_reach_the_operands_that_are_only_tested() {
        with_table(
            r"
            lexer A -> 'a' except B;
            lexer B -> 'b' except C;
            lexer C -> 'c' followed D;
            lexer D -> 'd';
            ",
            |table| {
                let edges = same_offset_edges(table, &nullable_nodes(table));

                // What an `except` takes away is tested however much the target
                // consumes, so it is reached at this position too.
                assert_eq!(edges["A"], vec!["B"]);
                assert_eq!(edges["B"], vec!["C"]);
                // The lookahead of `followed` comes after the `'c'` this starts
                // with, so `D` is consulted one character on.
                assert_eq!(edges["C"], Vec::<&str>::new());
                assert_eq!(edges["D"], Vec::<&str>::new());
            },
        );
    }

    #[test]
    fn a_followed_lookahead_is_here_only_when_the_target_can_be_empty() {
        with_table("lexer A -> 'a' followed A;", |table| {
            assert!(same_offset_recursive(table).is_empty());
        });

        with_table("lexer A -> B followed A; lexer B -> 'b'?;", |table| {
            assert_eq!(same_offset_recursive(table), vec!["A"]);
        });
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
