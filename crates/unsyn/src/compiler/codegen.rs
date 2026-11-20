//! The generator: the code of a parser for a file of the DSL.
//!
//! The input is a tree, and only a tree. Nothing here knows what a language
//! looks like, which rules a file usually declares, or what an external
//! combinator does: a definition is translated by the kind of node it declares
//! ([`Decl`]) and by the shape of the pattern on the right side of its `->`, and
//! every value a rule reaches outside the file is answered from the table the
//! caller of [`Builder::external`] supplied.
//!
//! # What a production becomes
//!
//! A node is named for the left side of a production and nothing else is, so one
//! `lexer` rule is one type of that name, and its right side is what the type
//! holds: a sequence is a struct with one field per operand, a choice of names
//! is an enum with one variant per name, and anything the type holds besides is
//! the text it read — one field of `I` with the reader of it written beside the
//! field.
//!
//! | the DSL writes          | the code writes                                        |
//! |-------------------------|--------------------------------------------------------|
//! | `'a'`, `U+0009`         | `::parserc::next(c)`                                     |
//! | `'abc'`                 | `::parserc::keyword("abc")`                              |
//! | `Name` as a whole rule  | the node `Name<I>`                                       |
//! | `Name` beside others    | the text the rule read, of the input it was handed       |
//! | `A \| B` of names only    | `pub enum` with one variant per name                      |
//! | `<Ext>`                 | the tokens the caller bound `Ext` to                     |
//! | `['a'-'z', '_']`        | `::parserc::next_if(\|c: char\| matches!(c, …))`           |
//! | `~[…]`, `A \| B`        | the same predicate refused, or a fold of `.or()`         |
//! | `A B`                   | a field for `A`, a field for `B`, in that order          |
//! | `A?`, `A*`              | `Option<A<I>>`, `Vec<A<I>>` — the derive's own shapes    |
//! | `A+`, `{n}`, `{n, m}`   | a `read_` function that counts the nodes it took         |
//! | `except`, `followed`    | a probe the field's reader runs beside the text it took  |
//! | `whitespace W -> […]`   | `#[parserc(take_while = is_w)]` and the predicate        |
//!
//! A field that names no rule is the text its operand read, and is called by
//! its place: `f1`, `f2`, and onward. A rule whose sequence holds nothing but
//! such fields is written as a tuple of them, since where they stand already
//! says what they are, and where one field reaches in by name every other
//! takes its number beside it.
//!
//! A right side that reads as one expression is written into the `parser` of
//! that one field, so that the declaration of a type is the declaration of the
//! rule. Only when a reading needs statements — a group that folds, a bound
//! over anything wider than a character, a probe — does it get a function of
//! its own, named `read_` after the rule in lower case with underscores, and
//! after the rule and its place in it for a group inside a rule.
//!
//! Which rules come out as nodes and which as the text they read is settled
//! over the whole file before any rule is written, because it is a fact about
//! the other rules' code as much as about its own. A name whose answer flows
//! back as text — an alternative of a folded choice, a member of a set read
//! one character at a time, the operand an `except` is written over, the one
//! name a rule or a group answers with whole — asks its rule for the text, and
//! the rule asked for is written as text even where its own right side is a
//! choice of names or a sequence of fields; and a text rule folds arms that
//! reach names of its own, so the question is asked again of what the first
//! answers settle, until the two sets hold still. What comes out is that a
//! choice of names is an enum only where nothing reads the choice beside
//! another thing: `Ident` and `Keywords` are enums, while `BlockCommentOrDoc`
//! — read among the alternatives of `BlockComment` — is the text its three
//! rules read.
//!
//! Three of the combinators of [`parserc`] are worth knowing about, since the
//! generator reaches for them whenever a rule fits: a set of single characters
//! becomes one [`next_if`](parserc::next_if) rather than a choice of one member
//! apiece, and a repetition of such a set becomes
//! [`take_while`](parserc::take_while_with), which counts items and needs no loop
//! of its own. What they cannot do is bound a repetition of anything wider than a
//! character, so `{n, m}` over a name is a `read_` of its own.
//!
//! # The nodes of a `syntax` rule
//!
//! A `syntax` rule is read the same way: the root of a choice is an enum, a
//! sequence is a struct of fields, and every field is the shape its operand
//! holds. What differs from the text level is what stands between the fields:
//! the trivia of the language is a separator no rule has to name, so the run
//! between two operands is a field of its own — the file's `whitespace` node,
//! read once or not at all — and a repetition of a rule that reads the trivia
//! itself stands without one, since the trivia between two of its matches is
//! an item of the repetition rather than a boundary the code lays down.
//!
//! The shapes an operand can hold are the shapes [`parserc`] already has: a
//! node of a rule the file names, [`Box`]ed where the node is of the tree —
//! a cycle can only be built through a node, and the box is what makes the
//! type finite — a [`Char`](parserc::syntax::Char) of one character, a field
//! of `I` with the `keyword` of a word beside it, a [`Vec`] or an [`Option`]
//! of a repetition, a tuple of a group, and the nested
//! [`Or`](parserc::syntax::Or) of a choice the file left unnamed: the shape
//! says what the group holds, and no name of the generator is in it. An
//! operator of the grammar is never a field of its word: `concat` lays its
//! operands down with no run between them, and `followed` and `except` are
//! what the reader of the field they stand on runs beside the text it took.
//!
//! What no field can hold is refused rather than written wrong: a counted
//! repetition of nodes, an operator inside a group, and a name asked for its
//! text where a tuple answers with types.
//!
//! Beside the nodes the build writes a visitor: one hook per rule descending
//! through the walk of it, and one hook for the text every leaf ends with,
//! in the shape of [`syn`]'s. The descents are read back from the shapes of
//! the fields themselves, so the visitor is a second view of the same code
//! and can never claim a tree the nodes do not hold.
//!
//! [`syn`]: https://docs.rs/syn/latest/syn/visit/index.html
//!
//! # Where the generated code differs from this crate's own
//!
//! [`lexer`] is written by hand and is tuned where a generated file
//! is merely correct. The differences that remain are deliberate:
//!
//! - The hand file gives the parts of a rule names that say what they hold —
//!   `open`, `head`, `tail` — where the generator has only the file to go by and
//!   says `f1`, `f2`, and the rule's own name. A group of a right side that the
//!   spec never named used to be answered the same way, by the hand file naming
//!   it anyway; those are now rules of `unsyn.us` where a reader of a pattern
//!   matches on them, and text where nobody does, which is what the
//!   [`UnnamedAlternative`](super::errors::LintKind::UnnamedAlternative) lint
//!   asks of the author of a file either way.
//! - A refusal the generated code reports is a [`Kind`](parserc::Kind) of the
//!   combinator that met it, where the hand-written file names what it expected
//!   with its own [`UnsynError`] variants. Naming them
//!   is a table the caller of a build would have to supply, in the same way that
//!   it supplies the externals.
//!
//! # What the generated code asks of its crate
//!
//! Every reference it writes is rooted at `::parserc`, so the crate that holds a
//! generated file must have a dependency *named* `parserc`. Every node is generic
//! over one parameter, and it is always named `I`: the derive of
//! [`Syntax`] reads the input type off that name unless the item says otherwise.
//!
//! That parameter is bounded by
//! [`CharsInput`](parserc::chars::CharsInput), which is what a stream of
//! characters has to be to run the code above: it is consumed one character at a
//! time, split, cloned and compared. The cloning is the one a caller is likely
//! to meet, since a `followed` and the refusal of an `except` are both decided
//! by taking the text once on a copy of the stream and once on the stream
//! itself. What the error type of that stream is, the generated code neither
//! knows nor asks: a refusal it cannot place is the combinator's own.
//!
//! # When a build refuses
//!
//! A file that asks for something the generator does not write is refused, and
//! never written wrong: an external the caller left unbound, a `whitespace` rule
//! that denotes no set of characters, a name no Rust item can carry, a literal
//! that spells no value, and a shape of a `syntax` rule no field of a node can
//! hold — a counted repetition of nodes, an operator inside a group, or a name
//! asked for its text where a tuple answers with types. The precondition of
//! [`Builder::build`] is that
//! [`analyze`](super::semantic::analyze) has already found nothing in the file;
//! what these refusals guard is what a clean file can still ask for that no code
//! answers, and they are the last line rather than the first.

use std::collections::{HashMap, HashSet};

use parserc::{Span, syntax::Syntax};
use proc_macro2::{Ident, Literal, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use unicode_ident::{is_xid_continue, is_xid_start};

use super::input::UnsynInput;

use super::{
    errors::{BuildKind, UnsynError},
    lexer, syntax,
    syntax::Decl,
};

// ---------------------------------------------------------------------------
// The builder
// ---------------------------------------------------------------------------

/// Writes the code of a parser for one file of the DSL.
///
/// The file is held as the tree [`analyze`](super::semantic::analyze) was run
/// over, so that a refusal can point at the text it was written in.
pub struct Builder<'a, I>
where
    I: UnsynInput,
{
    file: &'a syntax::Crate<I>,
    external: HashMap<String, TokenStream>,
}

impl<'a, I> Builder<'a, I>
where
    I: UnsynInput,
{
    /// Prepares to generate the code of `file`.
    #[inline]
    pub fn new(file: &'a syntax::Crate<I>) -> Self {
        Self {
            file,
            external: HashMap::new(),
        }
    }

    /// Binds the external `<#name#>` to the parser `parser`.
    ///
    /// A call of an external is the one part of a rule whose code the file does
    /// not say, so the code that reaches it is whatever the caller hands here: a
    /// path, a closure, a call of a function of some crate the file knows nothing
    /// about. It is written into the generated code wherever the file writes
    /// `<#name#>`, and must be a [`Parser`](parserc::Parser) over the input of the
    /// rule that reaches it.
    ///
    /// The name is the one the file writes, and a name written raw and the same
    /// name written plain are one external: `r#gen` and `gen` bind together.
    #[inline]
    pub fn external<P>(&mut self, name: &str, parser: P) -> &mut Self
    where
        P: ToTokens,
    {
        self.external
            .insert(external_key(name), parser.to_token_stream());
        self
    }

    /// Writes the code of the file.
    ///
    /// The tokens are the items of a module in the order the file declares them,
    /// with the function a rule needed beside the rule that needed it; they say
    /// nothing about which file they came from, so a caller may write them into a
    /// file of its own choosing.
    ///
    /// # Errors
    ///
    /// An [`UnsynError::Build`] says which rule asked for code there is none of. The
    /// precondition is a file the analyzer found nothing in; a rule that reaches a
    /// name the file never declares, for instance, is refused there and not here,
    /// and the code written for it then names a type that is not.
    pub fn build(&self) -> Result<TokenStream, UnsynError> {
        let denotations = denotations(self.file);
        let nodes = node_rules(self.file);
        let trivia = trivia_rule(self.file);
        let syntaxes = syntax_rules(self.file);

        let mut generator = Generator {
            file: self.file,
            external: &self.external,
            denotations: &denotations,
            nodes: &nodes,
            trivia,
            syntaxes: &syntaxes,
            taken: HashSet::new(),
            helpers: Vec::new(),
            prelude: Vec::new(),
            seq: 0,
            field_no: 0,
            field_names: HashSet::new(),
            rule: String::new(),
            descriptors: Vec::new(),
        };

        let items = generator.crate_of()?;
        let visit = generator.visit_items();
        let prelude = std::mem::take(&mut generator.prelude);

        Ok(quote! { #(#prelude)* #items #visit })
    }
}

/// The name of the file's `whitespace` rule, which is the node every run of
/// the trivia between the operands of a `syntax` sequence is read as.
///
/// A file without the declaration has no trivia, and the sequences of its
/// nodes are written without a run between any two of their operands.
fn trivia_rule<I>(file: &syntax::Crate<I>) -> Option<String>
where
    I: UnsynInput,
{
    file.items.iter().find_map(|item| {
        let syntax::Item::Stmt(stmt) = item else {
            return None;
        };

        matches!(stmt.decl(), Decl::Whitespace).then(|| stmt.ident.text())
    })
}

/// The keys of the file's `syntax` rules, which a `Box` reaches.
fn syntax_rules<I>(file: &syntax::Crate<I>) -> HashSet<String>
where
    I: UnsynInput,
{
    file.items
        .iter()
        .filter_map(|item| {
            let syntax::Item::Stmt(stmt) = item else {
                return None;
            };

            matches!(stmt.decl(), Decl::Syntax).then(|| external_key(stmt.ident.text().as_str()))
        })
        .collect()
}

/// How many times something is read, written as a number that is free to take the
/// type of the loop or the range it bounds.
fn count(n: usize) -> Literal {
    Literal::usize_unsuffixed(n)
}

/// The key an external is bound under: its name, of the sigil a raw name wears.
///
/// A name written `r#gen` and a name written `gen` are one name to the file, to
/// the analyzer and to the builder that binds an external, so every place a
/// rule is looked up by its name goes through here. Only the *spelling* a node
/// and a diagnostic wear keeps the sigil.
pub(crate) fn external_key(name: &str) -> String {
    name.strip_prefix("r#").unwrap_or(name).to_owned()
}

/// The alternatives of a rule that is nothing but a choice of other rules by name.
///
/// The word one alternative of a choice offers as its variant's name, when it
/// offers one.
///
/// A joining says the word it joins with, a literal says the character it
/// spells — provided that character begins with a letter, since a variant is
/// named as an identifier and `*` is not one. A quantity operator, braced or
/// not, says no word at all. Anything else offers nothing, and `None` says so:
/// the generator then calls the variant by its place, which is what
/// [`variant_names`] asks about for the whole choice.
pub(crate) fn variant_word<I>(
    branch: &syntax::OneOrMore<I, syntax::ExprNoTopAlt<I>>,
) -> Option<String>
where
    I: UnsynInput,
{
    branch.elements().find_map(|element| match element {
        syntax::ExprNoTopAlt::WithSuffix(suffix) => match suffix {
            syntax::ExprWithSuffix::Concat { keyword, .. } => Some(keyword.0.as_str().to_owned()),
            syntax::ExprWithSuffix::Followed { keyword, .. } => Some(keyword.0.as_str().to_owned()),
            syntax::ExprWithSuffix::Except { keyword, .. } => Some(keyword.0.as_str().to_owned()),
            syntax::ExprWithSuffix::Grouped { suffix, .. } => match suffix {
                // A brace opens a bound and is punctuation, not a word: like
                // the three operators beside it, it offers no name, and
                // offering one would be offering a name no item can wear.
                syntax::Suffix::Star(_)
                | syntax::Suffix::Question(_)
                | syntax::Suffix::Plus(_)
                | syntax::Suffix::Repeat(_) => None,
            },
        },
        syntax::ExprNoTopAlt::WithoutSuffix(syntax::ExprWithoutSuffix::Str(lit)) => {
            // The value the literal spells names the variant, not the quotes
            // it was written between.
            let text: String = lit.chars().into_iter().collect();

            text.chars()
                .next()
                .is_some_and(|c| c.is_alphabetic())
                .then_some(text)
        }
        _ => None,
    })
}

/// The place in the file where one alternative begins.
///
/// An unnamed alternative is told at its own beginning rather than at the
/// whole choice it sits in, because it is this alternative and no other that
/// gives the variant nothing to be called.
pub(crate) fn branch_head<I>(branch: &syntax::OneOrMore<I, syntax::ExprNoTopAlt<I>>) -> Span
where
    I: UnsynInput,
{
    branch
        .elements()
        .next()
        .map_or(Span::None, |element| element.to_span())
}

/// The identifier a name the generator wrote itself names.
///
/// The shapes of the written types are read back out of the tokens the
/// generator made, and a name written raw comes back with its sigil still on
/// it — `r#gen`, not `gen`. An identifier is worn bare and spelled raw, so the
/// two are told apart here rather than assumed.
fn written_name(text: &str) -> Ident {
    match text.strip_prefix("r#") {
        Some(bare) => Ident::new_raw(bare, proc_macro2::Span::call_site()),
        None => Ident::new(text, proc_macro2::Span::call_site()),
    }
}

/// The word each alternative of a choice offers its variant, before any of it
/// is settled: `None` where the alternative reads nothing an item could be
/// called by.
///
/// This is the half of [`variant_names`] that says what the file wrote rather
/// than what the generator finally wore. A word offered and then lost to
/// another alternative is still a word the author meant, and a caller that
/// cannot tell the two apart cannot tell an operand that reads no word from a
/// name that was taken.
pub(crate) fn variant_offers<I>(expr: &syntax::Expr<I>) -> Vec<Option<String>>
where
    I: UnsynInput,
{
    expr.branches()
        .map(|branch| {
            // A single rule names its variant by the rule; anything else offers
            // the word it reads, if it reads one at all.
            match one_reference(branch) {
                Some(ident) => Some(ident.text()),
                None => variant_word(branch).map(|word| capitalize(&word)),
            }
        })
        .collect()
}

/// The name each alternative of a choice gives its variant, in the order the
/// choice wrote them; `None` is an alternative the generator can only call by
/// its place.
///
/// An alternative names its variant by the rule it reaches, or failing that by
/// the word it reads; a word no item can wear, and a word another alternative
/// wore first, are both as good as no word. This is the naming the generator
/// does, in one place, and the analyzer asks the same function — so a warning
/// can neither promise a name the code declines to write nor stay silent about
/// one the code declines.
pub(crate) fn variant_names<I>(expr: &syntax::Expr<I>) -> Vec<Option<String>>
where
    I: UnsynInput,
{
    let mut taken: HashSet<String> = HashSet::new();
    let mut names = Vec::new();

    for candidate in variant_offers(expr) {
        // A word is the variant's name only where an item can wear it and no
        // alternative wore it first; failing either, the variant is called by
        // its place, and this says so rather than inventing a name.
        let name = match candidate {
            Some(word) if is_item_name(&word) && taken.insert(word.clone()) => Some(word),
            _ => None,
        };

        names.push(name);
    }

    names
}

/// The identifier the variant of one alternative wears: the name
/// [`variant_names`] settled on for it, or its place where that settled none.
///
/// [`variant_names`] has already said whether the word is one an item can wear
/// and whether another alternative wore it first, so the only answer left here
/// is the identifier itself.
fn variant_ident(names: &[Option<String>], index: usize) -> Result<Ident, UnsynError> {
    Ok(match names.get(index) {
        Some(Some(word)) => item_name(word, Span::None)?,
        _ => Ident::new(&format!("F{}", index + 1), proc_macro2::Span::call_site()),
    })
}

/// The rules of `file` that are written as a node rather than as the text they
/// read.
///
/// Two shapes earn a node. A rule whose whole right side is a choice of other
/// rules by name says *which* of them was read, and the text alone does not: it
/// comes out as an enum with one part per name. A rule whose right side is one
/// alternative of more than one operand is a sequence, and a sequence is written
/// as a struct with one field per operand, each field holding what its operand
/// holds — a node, a list, an option, or the text of what was read.
///
/// Either shape is undone by the shape of another rule's code: wherever the
/// code of a name is asked for among the parts of a choice, of a set read as a
/// choice of characters, or as the text a removal is written over, the answer
/// has to be the text and not a part of the tree, and the rule asked for in
/// such a place is written as one field of that text after all. The whole file
/// is read through until the shapes settle, since one rule written as text
/// reads a choice whose other parts then have to be text too, and what is
/// settled for one rule is a fact about the file rather than about the order
/// the file declares its rules in.
fn node_rules<I>(file: &syntax::Crate<I>) -> HashSet<String>
where
    I: UnsynInput,
{
    // The shape each `lexer` rule is written as, settled once over the whole
    // file before any code is: a choice of names is an enum of nodes, and a
    // sequence of more than one operand is a struct of fields; everything
    // else is the text the rule read. The shape is then undone rule by rule
    // wherever the code of a rule reads a name's answer back as text, and an
    // undoing of one rule's shape is a change to what its own code reads —
    // so the file is walked until the shapes hold still.
    #[derive(Clone, Copy, PartialEq)]
    enum Shape {
        Text,
        Fields,
        Enum,
    }

    let mut shapes: HashMap<String, Shape> = HashMap::new();

    for item in &file.items {
        let syntax::Item::Stmt(stmt) = item else {
            continue;
        };

        if !matches!(stmt.decl(), Decl::Lexer) {
            continue;
        }

        let body = &stmt.expr;

        let shape = if !body.is_single() {
            // A choice is an enum of one variant per alternative, whether the
            // alternatives are names, words, or whole sequences of either.
            Shape::Enum
        } else if is_sequence(body) {
            Shape::Fields
        } else {
            // One plain operand — a word, a set, a call of an external — is the
            // text of the token it read, and is the one shape that is not made
            // of parts the first level could have written as fields.
            Shape::Text
        };

        shapes.insert(external_key(stmt.ident.text().as_str()), shape);
    }

    loop {
        let mut forced: HashSet<String> = HashSet::new();

        for (key, shape) in &shapes {
            if *shape == Shape::Text {
                continue;
            }

            let Some(stmt) = file.items.iter().find_map(|item| match item {
                syntax::Item::Stmt(stmt) => {
                    (&external_key(stmt.ident.text().as_str()) == key).then_some(stmt)
                }
                _ => None,
            }) else {
                continue;
            };

            // A choice of names answers with the node of the name it chose;
            // the names it chooses between are not asked for their text. A
            // sequence answers with the node of its fields; the names its
            // fields hold are not asked for their text either. What asks for
            // text is everything else the code reads back: the arms of a
            // folded choice, the members of a set read one character at a
            // time, the operand a removal is written over, and the group
            // whose whole answer is one name.
            // A choice of names answers with the node of the name it chose,
            // and a sequence answers with the node of its fields: neither asks
            // a name it reaches for its text. What does is a set read one
            // character at a time, the operand a removal is written over, and
            // the group whose whole answer is one name.
            expr_forced(&stmt.expr, &mut forced);
        }

        let mut changed = false;

        for name in forced {
            if let Some(shape) = shapes.get_mut(&name)
                && *shape != Shape::Text
            {
                *shape = Shape::Text;
                changed = true;
            }
        }

        if !changed {
            return shapes
                .into_iter()
                .filter_map(|(name, shape)| (shape != Shape::Text).then_some(name))
                .collect();
        }
    }
}

/// Whether the rule is one alternative of more than one operand, which is the
/// sequence the generator writes as a struct of fields.
fn is_sequence<I>(body: &syntax::Expr<I>) -> bool
where
    I: UnsynInput,
{
    body.is_single()
        && body
            .branches()
            .next()
            .is_some_and(|branch| operand_count(branch) > 1)
}

/// How many operands one alternative writes.
///
/// `concat` is an operator between two operands and not an operand of its own,
/// so `'U+' concat HexDigit{4}` is a sequence of two however the tree holds it
/// as one element — and a sequence is what earns a field apiece.
fn operand_count<I>(branch: &syntax::OneOrMore<I, syntax::ExprNoTopAlt<I>>) -> usize
where
    I: UnsynInput,
{
    branch.elements().map(operands_of).sum()
}

/// How many operands one element of an alternative writes.
fn operands_of<I>(operand: &syntax::ExprNoTopAlt<I>) -> usize
where
    I: UnsynInput,
{
    match operand {
        syntax::ExprNoTopAlt::WithSuffix(syntax::ExprWithSuffix::Concat { suffix, .. }) => {
            1 + operands_of(suffix)
        }
        _ => 1,
    }
}

/// The names whose text is asked for by `expr`: by the alternatives it folds,
/// by the members of the sets it lists, and by the operand a removal is written
/// over. A name read as the whole answer of a rule or of an operand is not
/// among them; that answer keeps its shape.
/// The names whose answer flows as text wherever `expr` is read as one
/// answer: the alternatives it folds, the member of a set read as a choice of
/// characters, the group whose whole answer is one name, and the operand a
/// removal is written over.
fn expr_forced<I>(expr: &syntax::Expr<I>, forced: &mut HashSet<String>)
where
    I: UnsynInput,
{
    for branch in expr.branches() {
        for element in branch.elements() {
            operand_forced(element, forced);
        }
    }
}

fn operand_forced<I>(operand: &syntax::ExprNoTopAlt<I>, forced: &mut HashSet<String>)
where
    I: UnsynInput,
{
    match operand {
        syntax::ExprNoTopAlt::WithoutSuffix(atom) => atom_forced(atom, forced),
        syntax::ExprNoTopAlt::WithSuffix(suffix) => match suffix {
            syntax::ExprWithSuffix::Grouped { target, .. } => atom_forced(target, forced),
            syntax::ExprWithSuffix::Concat {
                target,
                suffix: next,
                ..
            } => {
                atom_forced(target, forced);
                operand_forced(next, forced);
            }
            // What follows is only probed, and its answer thrown away: the
            // probe asks the shape of the answer of nothing. What a removal is
            // written over is the one operand whose text is read back.
            syntax::ExprWithSuffix::Followed { target, .. } => atom_forced(target, forced),
            syntax::ExprWithSuffix::Except { target, tokens, .. } => {
                if let syntax::ExprWithoutSuffix::Reference(ident) = target {
                    forced.insert(external_key(ident.text().as_str()));
                }

                atom_forced(tokens, forced);
            }
        },
    }
}

fn atom_forced<I>(atom: &syntax::ExprWithoutSuffix<I>, forced: &mut HashSet<String>)
where
    I: UnsynInput,
{
    match atom {
        // A name read alone hands back its whole answer, and a name read among
        // the parts of a fold hands back the text of it: the fold is wrapped in
        // the reading of text, so the name keeps whichever shape its own
        // production writes — one field per sub-expression of it — rather than
        // being flattened into one run of text to suit the fold it is read
        // among.
        syntax::ExprWithoutSuffix::Reference(_) => {}
        // A group whose whole answer is one name hands that name's answer back,
        // and a group is read back as text wherever it stands: a field of `I`,
        // an operand the code of a rule folds. What a group reads besides — a
        // sequence, a choice of its own — it answers with its reading, and the
        // names inside answer only to that.
        // A group whose whole answer is one name hands that name's answer
        // back, and a group is read back as text wherever it stands: a field
        // of `I`, an operand the code of a rule folds. What a group reads
        // besides — a sequence, a choice of its own — it answers with its
        // reading, and the names inside answer only to that.
        syntax::ExprWithoutSuffix::Paren { expr: group, .. } => {
            if group
                .branches()
                .next()
                .is_some_and(|branch| branch.elements().count() == 1)
                && group.is_single()
                && let Some(syntax::ExprNoTopAlt::WithoutSuffix(
                    syntax::ExprWithoutSuffix::Reference(ident),
                )) = group
                    .branches()
                    .next()
                    .expect("one element")
                    .elements()
                    .next()
            {
                forced.insert(external_key(ident.text().as_str()));
            } else {
                expr_forced(group, forced);
            }
        }
        // A set of more than one member is a choice of its members, and a
        // choice answers with one thing however wide the members are.
        syntax::ExprWithoutSuffix::Set(set) | syntax::ExprWithoutSuffix::Tilde { set, .. }
            if set_items(set).len() > 1 =>
        {
            for item in set_items(set) {
                if let syntax::SetItem::Reference(ident) = item {
                    forced.insert(external_key(ident.text().as_str()));
                }
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// What a name denotes
// ---------------------------------------------------------------------------

/// One character or one run of characters between two of them, which is all a set
/// of a rule can be written out of and still be one test of one character.
#[derive(Debug, Clone, Copy)]
enum Arm {
    /// One character, as `'a'` is.
    Char(char),
    /// The characters between two of them, as `'0'-'7'` is.
    Range(char, char),
}

impl Arm {
    /// The pattern of the arm, which is what a `matches!` of a class is built of.
    fn to_pattern(self) -> TokenStream {
        match self {
            Arm::Char(c) => quote! { #c },
            Arm::Range(lo, hi) => quote! { #lo..=#hi },
        }
    }
}

/// The characters the names of `file` denote, for the names that denote a set of
/// single characters and nothing besides.
///
/// A set may hold a name — `[LF, CR]`, `~['"', '\\', CR]` — and the code that
/// asks one question of a character has to know what the names in it answer. That
/// is settled here, from the trees of the rules alone: a name denotes characters
/// when every alternative of its rule is one operand, unwrapped, of a shape that
/// denotes characters.
///
/// A name that denotes more than a character is absent, and so is a name that
/// reaches one that is: `~` and `<…>` are absent by their nature, and a rule that
/// nests itself is absent until it is settled, which it never is. The round that
/// adds nothing ends the count, so the answer does not depend on the order the
/// file declares its rules in.
fn denotations<I>(file: &syntax::Crate<I>) -> HashMap<String, Vec<Arm>>
where
    I: UnsynInput,
{
    let mut bodies: Vec<(String, &syntax::Expr<I>)> = Vec::new();

    for item in &file.items {
        if let syntax::Item::Stmt(stmt) = item {
            // A `syntax` rule is no character to be a member of a set of one, and
            // is only ever asked of a file that is refused before it is read.
            if matches!(stmt.decl(), Decl::Whitespace | Decl::Lexer) {
                bodies.push((stmt.ident.text(), &stmt.expr));
            }
        }
    }

    let mut known: HashMap<String, Vec<Arm>> = HashMap::new();

    loop {
        let mut settled = false;

        for (name, body) in &bodies {
            if known.contains_key(name) {
                continue;
            }

            if let Some(arms) = expr_arms(body, &known) {
                known.insert(name.clone(), arms);
                settled = true;
            }
        }

        if !settled {
            return known;
        }
    }
}

/// The characters the pattern denotes, when a set of them is all it denotes.
fn expr_arms<I>(expr: &syntax::Expr<I>, known: &HashMap<String, Vec<Arm>>) -> Option<Vec<Arm>>
where
    I: UnsynInput,
{
    let mut arms = Vec::new();

    for branch in expr.branches() {
        let mut elements = branch.elements();
        let operand = elements.next()?;
        if elements.next().is_some() {
            return None;
        }

        let syntax::ExprNoTopAlt::WithoutSuffix(operand) = operand else {
            return None;
        };

        arms.extend(atom_arms(operand, known)?);
    }

    Some(arms)
}

/// The characters the operand denotes, when a set of them is all it denotes.
fn atom_arms<I>(
    atom: &syntax::ExprWithoutSuffix<I>,
    known: &HashMap<String, Vec<Arm>>,
) -> Option<Vec<Arm>>
where
    I: UnsynInput,
{
    match atom {
        // What a `'…'` spells is one character here and a token of several there,
        // and only one character is a value a class can hold.
        syntax::ExprWithoutSuffix::Str(lit) => match lit.chars()[..] {
            [c] => Some(vec![Arm::Char(c)]),
            _ => None,
        },
        syntax::ExprWithoutSuffix::Unicode(lit) => Some(vec![Arm::Char(lit.value()?)]),
        syntax::ExprWithoutSuffix::Set(set) => set_arms(set, known),
        syntax::ExprWithoutSuffix::Paren { expr: group, .. } => expr_arms(group, known),
        syntax::ExprWithoutSuffix::Reference(ident) => known.get(ident.text().as_str()).cloned(),
        // A refusal of a set is every character the universe holds besides it,
        // which is no set to list, whatever the set itself denotes.
        syntax::ExprWithoutSuffix::Tilde { .. } => None,
        // An external is whatever the caller binds, and nothing here may say more
        // of it than that.
        syntax::ExprWithoutSuffix::Call { .. } => None,
    }
}

/// The characters the members of the set denote, when every one of them denotes
/// characters and no more.
fn set_arms<I>(set: &syntax::Set<I>, known: &HashMap<String, Vec<Arm>>) -> Option<Vec<Arm>>
where
    I: UnsynInput,
{
    let mut arms = Vec::new();

    for item in set_items(set) {
        match item {
            syntax::SetItem::Range(range) => arms.push(range_arm(range)?),
            syntax::SetItem::Str(lit) => match lit.chars()[..] {
                [c] => arms.push(Arm::Char(c)),
                _ => return None,
            },
            syntax::SetItem::Unicode(lit) => arms.push(Arm::Char(lit.value()?)),
            syntax::SetItem::Reference(ident) => {
                arms.extend(known.get(ident.text().as_str())?.iter().copied());
            }
        }
    }

    Some(arms)
}

/// The two ends of a range, which the analyzer has already said are characters.
fn range_arm<I>(range: &syntax::Range<I>) -> Option<Arm>
where
    I: UnsynInput,
{
    let (from, to) = match range {
        syntax::Range::Unicode(from, _, to) => (from.value()?, to.value()?),
        syntax::Range::Str(from, _, to) => (one_char(from.chars())?, one_char(to.chars())?),
    };

    Some(Arm::Range(from, to))
}

/// The character of a literal that spells one, which is what a range asks its two
/// ends to be.
fn one_char(chars: Vec<char>) -> Option<char> {
    match chars[..] {
        [c] => Some(c),
        _ => None,
    }
}

/// The test a class of characters is written as: the name of its parameter,
/// and the body that decides it.
///
/// `accepts` says which of the two tests is being written — whether the class
/// itself is what a character must fit, or the characters it refuses. A class
/// of nothing passes nothing, and a refusal of nothing passes everything: the
/// two answers are opposite, and that is the whole reason they are settled
/// here once rather than at each place a class is read.
///
/// Whether a class of nothing can reach a build at all is the analyzer's
/// decision, not this one: `[]` and `~[]` both earn
/// [`EmptySet`](super::errors::SemanticsKind::EmptySet), which is fatal. The
/// answer is written for the case nonetheless, so that a file which ever got
/// past that gate without the refusal would still be given a test that means
/// what the class means, and not a `matches!` of no arms, which is not Rust.
fn char_test(arms: &[Arm], accepts: bool) -> (TokenStream, TokenStream) {
    // The two are asked at once because they are one decision: a body that
    // never reads the character wants a name it will not use, and an ignored
    // name is a warning in the file a build writes.
    let (param, body) = if arms.is_empty() {
        let nothing = if accepts {
            quote! { false }
        } else {
            quote! { true }
        };

        (quote! { _c }, nothing)
    } else {
        let patterns = arms.iter().copied().map(Arm::to_pattern);
        let class = quote! { matches!(c, #(#patterns)|*) };

        let body = if accepts {
            class
        } else {
            quote! { !(#class) }
        };

        (quote! { c }, body)
    };

    (param, body)
}

/// The closure a repetition of a class of these arms asks its items to fit.
fn class_pred(arms: &[Arm]) -> TokenStream {
    let (param, body) = char_test(arms, true);

    quote! { |#param: char| #body }
}

/// The closure a repetition of what a class of these arms refuses asks its
/// items to fit.
fn refused_pred(arms: &[Arm]) -> TokenStream {
    let (param, body) = char_test(arms, false);

    quote! { |#param: char| #body }
}

// ---------------------------------------------------------------------------
// The code of one rule
// ---------------------------------------------------------------------------

/// The code of one operand, in whichever of the two shapes it reads best.
///
/// Most operands are one expression: one combinator, or a choice of two. Some are
/// a run of statements, which no expression can be until it is given a function to
/// stand in. The two shapes are kept apart until the position an operand is
/// written into says which it needs, so that a rule that could be one expression
/// is.
#[derive(Clone)]
enum Code {
    /// An expression of some [`Parser`](parserc::Parser) over the input, which
    /// takes one match of the operand and hands back the text of it.
    Expr(TokenStream),
    /// Statements that take one match of the operand from `input`.
    Stmt(Vec<TokenStream>),
}

impl Code {
    /// The statements the code is, which is what a sequence of operands needs.
    fn into_stmts(self) -> Vec<TokenStream> {
        match self {
            Code::Expr(parser) => vec![quote! { #parser.parse(input)?; }],
            Code::Stmt(stmts) => stmts,
        }
    }
}

/// The generator, which writes one rule at a time and remembers what the whole
/// module has taken.
struct Generator<'a, I>
where
    I: UnsynInput,
{
    /// The file whose code is being written.
    file: &'a syntax::Crate<I>,
    /// What each external the file calls is written as.
    external: &'a HashMap<String, TokenStream>,
    /// What each name of the file denotes, of the names that denote characters.
    denotations: &'a HashMap<String, Vec<Arm>>,
    /// The rules that are written as a node — an enum of the names a choice
    /// reads, or a struct of the fields a sequence writes — rather than as the
    /// text they read.
    nodes: &'a HashSet<String>,
    /// The name the file's `whitespace` rule gave the trivia, when there is one.
    ///
    /// The runs of the trivia between the operands of a `syntax` sequence are
    /// fields of `Option<S<I>>` of the code, and this says what `S` is; a file
    /// without the declaration has no trivia for a field to hold.
    trivia: Option<String>,
    /// The names of the file's `syntax` rules, whose nodes a `syntax` rule
    /// reaches as a `Box` of itself: a cycle of the tree can only be built
    /// through a node of the tree, and the box is what makes the type finite.
    syntaxes: &'a HashSet<String>,
    /// The names of the functions the module has been given already.
    taken: HashSet<String>,
    /// The functions the rule being written has asked for, in the order asked.
    helpers: Vec<TokenStream>,
    /// The functions the whole file has asked for, written before its rules.
    ///
    /// A helper a single rule needs stands beside that rule; one every fold may
    /// reach is not any rule's, so it goes above all of them.
    prelude: Vec<TokenStream>,
    /// How many functions the rule being written has asked for.
    seq: usize,
    /// How many fields of the rule being written have been named by their
    /// place, counting up from one.
    field_no: usize,
    /// The field names the rule being written has taken, which no second field
    /// may wear as well.
    field_names: HashSet<String>,
    /// The rule being written, which is what a refusal is reported against.
    rule: String,
    /// The shape of every node the module has been written for, in the order
    /// the file declared them, which is what the visitor walks them with.
    descriptors: Vec<(Ident, NodeDesc)>,
}

impl<'a, I> Generator<'a, I>
where
    I: UnsynInput,
{
    /// The items of a whole file.
    fn crate_of(&mut self) -> Result<TokenStream, UnsynError> {
        let file = self.file;
        let mut items = Vec::new();

        for item in &file.items {
            let syntax::Item::Stmt(stmt) = item else {
                continue;
            };

            items.extend(self.stmt(stmt)?);
        }

        Ok(quote! { #(#items)* })
    }

    /// The code of one definition: the functions its pattern asked for, and the
    /// node its name declares.
    fn stmt(&mut self, stmt: &syntax::Stmt<I>) -> Result<Vec<TokenStream>, UnsynError> {
        let text = stmt.ident.text();
        let name = item_name(&text, stmt.ident.to_span())?;
        let mut allow = wants_allow(&text);

        self.rule = text.clone();
        self.seq = 0;
        self.helpers.clear();
        self.field_no = 0;
        self.field_names.clear();

        // A `syntax` rule is the node its pattern writes: a choice of
        // alternatives is an enum, a sequence is a struct, and every field is
        // the shape its operand holds — a node, a box of a node, a list or an
        // option of one, the trivia between two operands, or the text of what
        // a punct or a call read.
        if matches!(stmt.decl(), Decl::Syntax) {
            return self.syntax_item(&text, stmt);
        }

        // A `lexer` rule that is a choice of alternatives is an enum with one
        // variant apiece, exactly as a `syntax` choice is: the first level of a
        // right side says what the node is made of, and a choice says variants
        // however plain or compound the alternatives are. Which rules are
        // nodes at all was settled over the whole file first, since a rule
        // asked for among the parts of another is not written as one.
        if self.nodes.contains(&external_key(&text)) && !stmt.expr.is_single() {
            let mut variants = Vec::new();
            let mut variant_descs = Vec::new();
            // The names are settled for the whole choice before any of them is
            // worn, since which of two alternatives that offer one word gets it
            // is a question about the choice and not about either alternative.
            let names = variant_names(&stmt.expr);

            for (i, branch) in stmt.expr.branches().enumerate() {
                self.field_no = 0;
                self.field_names.clear();

                // An alternative that is one name is that name's variant, and
                // the node it reads is all the alternative says. Anything
                // else is a variant of the fields its operands write.
                if let Some(ident) = one_reference(branch) {
                    let ty = self.reference_type(ident)?;
                    let variant = variant_ident(&names, i)?;

                    allow |= wants_allow(&variant.to_string());
                    variants.push(quote! { #variant ( #ty ) });

                    variant_descs.push(VariantDesc {
                        ident: variant,
                        body: Box::new(NodeDesc::Fields {
                            tuple: true,
                            fields: vec![FieldDesc { name: None, ty }],
                        }),
                    });

                    continue;
                }

                let variant = variant_ident(&names, i)?;

                allow |= wants_allow(&variant.to_string());

                let fields = self.fields_from(branch)?;
                let fields = self.finish_fields(fields);
                let tuple = tuple_of(&fields);

                let body = if tuple {
                    let tys = fields.iter().map(|field| {
                        let attr = &field.attr;
                        let ty = &field.ty;

                        quote! { #attr #ty, }
                    });

                    quote! { ( #(#tys)* ) }
                } else {
                    let decls = fields.iter().map(|field| {
                        let attr = &field.attr;
                        let name = field.name.as_ref().expect("a named field");
                        let ty = &field.ty;

                        quote! { #attr #name: #ty, }
                    });

                    quote! { { #(#decls)* } }
                };

                variants.push(quote! { #variant #body });

                variant_descs.push(VariantDesc {
                    ident: variant.clone(),
                    body: Box::new(NodeDesc::Fields {
                        tuple,
                        fields: desc_fields(&fields),
                    }),
                });
            }

            let allow = allow.then(|| quote! { #[allow(non_camel_case_types)] });
            let enum_item = quote! {
                #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, ::parserc::syntax::Syntax)]
                #allow
                pub enum #name<I>
                where
                    I: ::parserc::chars::CharsInput,
                {
                    #( #variants ),*
                }
            };

            self.descriptors.push((
                name.clone(),
                NodeDesc::Enum {
                    variants: variant_descs,
                },
            ));

            let mut items = std::mem::take(&mut self.helpers);
            items.push(enum_item);

            return Ok(items);
        }

        let allow = allow.then(|| quote! { #[allow(non_camel_case_types)] });

        let code = match stmt.decl() {
            Decl::Syntax => unreachable!("a `syntax` rule is written by `syntax_item`"),
            Decl::Whitespace => self.whitespace(&text, stmt)?,
            Decl::Lexer => self.lexer_rule(&text, stmt)?,
        };

        self.descriptors.push((
            name.clone(),
            match &code {
                Rule::Field(_) | Rule::Read(_) | Rule::TakeWhile(_) => NodeDesc::Text,
                Rule::Fields { tuple, fields } => NodeDesc::Fields {
                    tuple: *tuple,
                    fields: fields
                        .iter()
                        .map(|field| FieldDesc {
                            name: field.name.clone(),
                            ty: field.ty.clone(),
                        })
                        .collect(),
                },
            },
        ));

        let mut items = std::mem::take(&mut self.helpers);
        items.push(match code {
            Rule::Field(parser) => quote! {
                #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, ::parserc::syntax::Syntax)]
                #allow
                pub struct #name<I>(#[parserc(parser = #parser)] pub I)
                where
                    I: ::parserc::chars::CharsInput;
            },
            Rule::Read(read) => quote! {
                #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, ::parserc::syntax::Syntax)]
                #allow
                pub struct #name<I>(#[parserc(parser = #read)] pub I)
                where
                    I: ::parserc::chars::CharsInput;
            },
            Rule::TakeWhile(pred) => quote! {
                #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, ::parserc::syntax::Syntax)]
                #allow
                #[parserc(take_while = #pred)]
                pub struct #name<I>(pub I)
                where
                    I: ::parserc::chars::CharsInput;
            },
            Rule::Fields { tuple, fields } => {
                use quote::ToTokens as _;

                struct_item(&name, &allow.to_token_stream(), tuple, &fields)
            }
        });

        Ok(items)
    }

    /// The predicate of a `whitespace` rule, which is the one rule the derive of
    /// the syntax asks a function of a character rather than a parser.
    fn whitespace(&mut self, text: &str, stmt: &syntax::Stmt<I>) -> Result<Rule, UnsynError> {
        let arms = expr_arms(&stmt.expr, self.denotations).ok_or_else(|| {
            UnsynError::Build(BuildKind::Whitespace(text.to_owned()), stmt.expr.to_span())
        })?;

        let pred = self.unique(format!("is_{}", to_snake_case(text)));
        let (arg, body) = char_test(&arms, true);

        self.helpers.push(quote! {
            #[inline]
            fn #pred(#arg: char) -> bool {
                #body
            }
        });

        Ok(Rule::TakeWhile(pred))
    }

    /// The parser of a `lexer` rule.
    fn lexer_rule(&mut self, text: &str, stmt: &syntax::Stmt<I>) -> Result<Rule, UnsynError> {
        // A rule whose whole right side is one alternative of more than one
        // operand is a sequence, and a sequence is modelled as the struct its
        // operands write: one field apiece, in the order they were written.
        let body = &stmt.expr;

        if is_sequence(body) && self.nodes.contains(&external_key(text)) {
            let branch = body.branches().next().expect("a pattern has a branch");

            return self.fields_of(branch);
        }

        match self.code_of_expr(body)? {
            // The whole rule reads as one expression, and the expression says so
            // beside the field it reads into. Any function the expression reaches
            // was asked for while it was being read, and stands beside the rule.
            Code::Expr(parser) => Ok(Rule::Field(parser)),
            // A rule that is a run of statements is declared by a function that
            // keeps the text those statements took.
            Code::Stmt(stmts) => {
                let read = self.unique(format!("read_{}", to_snake_case(text)));
                self.helpers.push(reader(&read, &stmts));
                Ok(Rule::Read(read))
            }
        }
    }

    // -- fields --------------------------------------------------------------

    /// The struct a sequence of operands came out as.
    ///
    /// A sequence of nothing but nameless operands is a tuple of them: the
    /// fields are told apart by where they stand, and a name would only say
    /// again what the order already says. Where a name reaches in — a reference
    /// to another rule, read as the node it names — every field is given one:
    /// the reference after the rule it reads, the rest as the count of anonymous
    /// fields.
    fn fields_of(
        &mut self,
        branch: &syntax::OneOrMore<I, syntax::ExprNoTopAlt<I>>,
    ) -> Result<Rule, UnsynError> {
        let fields = self.fields_from(branch)?;
        let fields = self.finish_fields(fields);

        Ok(Rule::Fields {
            tuple: tuple_of(&fields),
            fields,
        })
    }

    /// The fields of a sequence of operands, before any of them is named.
    fn fields_from(
        &mut self,
        branch: &syntax::OneOrMore<I, syntax::ExprNoTopAlt<I>>,
    ) -> Result<Vec<Field>, UnsynError> {
        let mut fields = Vec::new();

        for element in branch.elements() {
            fields.extend(self.field_of(element)?);
        }

        Ok(fields)
    }

    /// The naming pass of a struct's fields: where none of them has a name
    /// they stand in a tuple, and where one does every other takes its
    /// number — `f1`, `f2`, onward — which is all a nameless field can be
    /// called and all a named struct may call it.
    fn finish_fields(&mut self, mut fields: Vec<Field>) -> Vec<Field> {
        if !tuple_of(&fields) {
            for field in &mut fields {
                if field.name.is_none() {
                    field.name = self.anonymous_name();
                }
            }
        }

        fields
    }

    /// The fields one operand of a sequence is written as.
    ///
    /// Most operands are one field. A `concat` is the two operands it joins,
    /// which at this level is what a sequence of them already is.
    fn field_of(&mut self, operand: &syntax::ExprNoTopAlt<I>) -> Result<Vec<Field>, UnsynError> {
        match operand {
            syntax::ExprNoTopAlt::WithoutSuffix(atom) => Ok(vec![self.atom_field(atom, None)?]),
            syntax::ExprNoTopAlt::WithSuffix(suffix) => match suffix {
                syntax::ExprWithSuffix::Grouped { target, suffix, .. } => {
                    let bounds = match suffix {
                        syntax::Suffix::Star(_) => (0, None),
                        syntax::Suffix::Question(_) => (0, Some(1)),
                        syntax::Suffix::Plus(_) => (1, None),
                        syntax::Suffix::Repeat(repeat) => self.bounds(&repeat.bound)?,
                    };

                    Ok(vec![self.atom_field(target, Some(bounds))?])
                }
                syntax::ExprWithSuffix::Concat { target, suffix, .. } => {
                    let mut fields = vec![self.atom_field(target, None)?];

                    fields.extend(self.field_of(suffix)?);

                    Ok(fields)
                }
                // A `followed` asks of the text what came before it, and an
                // `except` refuses what all of it amounts to: neither is a
                // field of its own, and both are what one field's reader does.
                syntax::ExprWithSuffix::Followed { .. } | syntax::ExprWithSuffix::Except { .. } => {
                    let code = self.code_of_suffix(suffix)?;
                    let parser = self.expr_of(code)?;

                    Ok(vec![Field {
                        name: None,
                        ty: quote! { I },
                        attr: Some(field_parser(&parser)),
                    }])
                }
            },
        }
    }

    /// The field one operand that stands alone is written as, repeated as
    /// many times as its suffix asks.
    ///
    /// A reference keeps the shape it names: the node of the rule read, once or
    /// as an option or as a list of them. Anything else is read into the text
    /// it took, which is one field of `I` with the reader of it written beside
    /// the field. What a bound asks of a class of characters is one call of a
    /// combinator that counts items; what it asks of anything wider is a
    /// function of its own.
    fn atom_field(
        &mut self,
        atom: &syntax::ExprWithoutSuffix<I>,
        bounds: Option<(usize, Option<usize>)>,
    ) -> Result<Field, UnsynError> {
        if let syntax::ExprWithoutSuffix::Reference(ident) = atom {
            let name = ident.text();
            let span = ident.to_span();
            let ty = item_name(&name, span)?;

            // The field's own name is settled before anything else the field
            // asks, which is what lets the rest of the building borrow the
            // generator once.
            let field_name = self.named_field(&name);

            // What the reference reads is the node it names, boxed where that
            // node is one the tree can only be built through a pointer: a
            // `syntax` rule reached from another is the shape of the recursion,
            // and `Paren` and `Suffixed` both write it. A `Vec` is finite
            // whatever it holds, so a repetition wants no box inside it; an
            // `Option` is as wide as what it holds, so the box a rule wanted
            // alone is wanted there too.
            let held = self.reference_type(ident)?;

            let (field_ty, attr) = match bounds {
                None => (held.clone(), None),
                Some((0, None)) => (quote! { ::std::vec::Vec<#ty<I>> }, None),
                Some((0, Some(1))) => (quote! { ::core::option::Option<#held> }, None),
                Some((at_least, at_most)) => {
                    let read = self.bounded_reader(&ty, at_least, at_most);

                    (
                        quote! { ::std::vec::Vec<#ty<I>> },
                        Some(field_parser(&read.to_token_stream())),
                    )
                }
            };

            return Ok(Field {
                name: field_name,
                ty: field_ty,
                attr,
            });
        }

        let parser = match bounds {
            None => {
                let code = self.code_of_atom(atom)?;
                self.expr_of(code)?
            }
            Some(bounds) => match self.class_of(atom) {
                Some(pred) => bound_combinator(&pred, bounds.0, bounds.1),
                None => {
                    let code = self.code_of_repetition(atom, bounds.0, bounds.1)?;
                    self.expr_of(code)?
                }
            },
        };

        Ok(Field {
            name: None,
            ty: quote! { I },
            attr: Some(field_parser(&parser)),
        })
    }

    /// The characters a repeated operand can be one test of, when it is a class
    /// of them at all: what the code of a repetition asks of the arms it can
    /// count, counted here for the field it will be written into.
    fn class_of(&self, atom: &syntax::ExprWithoutSuffix<I>) -> Option<TokenStream> {
        match atom {
            syntax::ExprWithoutSuffix::Tilde { set, .. } => {
                set_arms(set, self.denotations).as_deref().map(refused_pred)
            }
            atom => atom_arms(atom, self.denotations).as_deref().map(class_pred),
        }
    }

    /// The function that reads one reference repeated a bounded number of
    /// times, and hands back the nodes it took.
    fn bounded_reader(&mut self, ty: &Ident, at_least: usize, at_most: Option<usize>) -> Ident {
        let seq = self.seq;
        self.seq += 1;

        let read = self.unique(format!(
            "read_{}_{}",
            to_snake_case(&self.rule.clone()),
            seq + 1
        ));

        let each = quote! {
            <#ty<I> as ::parserc::syntax::Syntax<I>>::into_parser()
                .ok()
                .parse(input)?
        };

        let loop_ = match at_most {
            None => quote! {
                while let Some(one) = #each {
                    taken.push(one);
                }
            },
            Some(at_most) => quote! {
                while taken.len() < #at_most {
                    // The `break` carries its semicolon because that is how
                    // `rustfmt` writes it, and the file this code is kept in is
                    // a file `rustfmt` has been over: tokens formatting changes
                    // would make the kept file say something a build does not.
                    let Some(one) = #each else { break; };
                    taken.push(one);
                }
            },
        };

        let rule = self.rule.clone();

        // A lower bound of one is asked as the emptiness it is, which is what
        // reads clearly and what a lint asks for; a bound above one is the
        // comparison it has to stay.
        let too_few = if at_least == 1 {
            quote! { taken.is_empty() }
        } else {
            quote! { taken.len() < #at_least }
        };

        self.helpers.push(quote! {
            fn #read<I>(
                input: &mut I,
            ) -> ::core::result::Result<::std::vec::Vec<#ty<I>>, <I as ::parserc::Input>::Error>
            where
                I: ::parserc::chars::CharsInput,
            {
                use ::parserc::Parser as _;

                let mut taken = ::std::vec::Vec::new();

                #loop_

                // Fewer than the bound asked for is the repetition itself
                // missing, told at the place the reading of it stands.
                if #too_few {
                    return Err(::parserc::Kind::Syntax(
                        #rule,
                        ::parserc::ControlFlow::Recovable,
                        input.to_span(),
                    )
                    .into());
                }

                Ok(taken)
            }
        });

        read
    }

    /// The name of the next field that has none of its own.
    fn anonymous_name(&mut self) -> Option<Ident> {
        self.field_no += 1;

        Some(Ident::new(
            &format!("f{}", self.field_no),
            proc_macro2::Span::call_site(),
        ))
    }

    /// The name a field that reads a rule by name wears: the rule's own, in
    /// the case the fields of a struct are written in. Where two operands
    /// reach the same name, the second of them stays nameless and takes its
    /// number with the others.
    ///
    /// The count of nameless fields says nothing about this one: the name is
    /// the rule's, and only the fields that have no other go by their number.
    fn named_field(&mut self, text: &str) -> Option<Ident> {
        let base = to_snake_case(text);

        self.field_names
            .insert(base.clone())
            .then(|| Ident::new(&base, proc_macro2::Span::call_site()))
    }

    // -- syntax rules --------------------------------------------------------

    /// The items of one `syntax` definition: the node its name declares.
    ///
    /// The root of the pattern decides the shape: a choice of alternatives is
    /// an enum with one variant apiece, and a single alternative is a struct —
    /// a tuple of its fields where none of them has a name. Each operand then
    /// becomes one field of the shape it holds, and the run of the trivia
    /// between two operands is a field of its own: the file's `whitespace`
    /// node, read once or not at all.
    fn syntax_item(
        &mut self,
        text: &str,
        stmt: &syntax::Stmt<I>,
    ) -> Result<Vec<TokenStream>, UnsynError> {
        let name = item_name(text, stmt.ident.to_span())?;
        let body = &stmt.expr;
        let mut allow = wants_allow(text);

        let item = if body.is_single() {
            let branch = body.branches().next().expect("a pattern has a branch");
            // A rule that is one alternative of one operand that is a name is
            // not a struct of one field but that operand's own shape: the
            // node reads as the node it reads, and the tree says nothing the
            // name it holds does not.
            let named = one_reference(branch);

            let fields = if named.is_some() {
                Vec::new()
            } else {
                self.branch_fields(branch)?
            };

            if let Some(ident) = named {
                let ty = self.reference_type(ident)?;
                let allow = allow.then(|| quote! { #[allow(non_camel_case_types)] });

                self.descriptors.push((
                    name.clone(),
                    NodeDesc::Fields {
                        tuple: true,
                        fields: vec![FieldDesc {
                            name: None,
                            ty: ty.clone(),
                        }],
                    },
                ));

                quote! {
                    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, ::parserc::syntax::Syntax)]
                    #allow
                    pub struct #name<I>(pub #ty)
                    where
                        I: ::parserc::chars::CharsInput;
                }
            } else {
                let fields = self.finish_fields(fields);
                let tuple = tuple_of(&fields);
                let allow = allow.then(|| quote! { #[allow(non_camel_case_types)] });

                self.descriptors.push((
                    name.clone(),
                    NodeDesc::Fields {
                        tuple,
                        fields: desc_fields(&fields),
                    },
                ));

                struct_item(&name, &allow, tuple, &fields)
            }
        } else {
            let mut variant_descs = Vec::new();
            let mut variants = Vec::new();
            // The names are settled for the whole choice before any of them is
            // worn, since which of two alternatives that offer one word gets it
            // is a question about the choice and not about either alternative.
            let names = variant_names(body);

            for (i, branch) in body.branches().enumerate() {
                self.field_no = 0;
                self.field_names.clear();

                // An alternative that is one name is that name's variant, and
                // the node it reads is all the alternative says. Anything
                // else is a variant of the fields its operands write.
                let named = one_reference(branch);

                let fields = if named.is_some() {
                    Vec::new()
                } else {
                    self.branch_fields(branch)?
                };

                if let Some(ident) = named {
                    let ty = self.reference_type(ident)?;
                    let variant = variant_ident(&names, i)?;

                    allow |= wants_allow(&variant.to_string());
                    variants.push(quote! { #variant ( #ty ) });

                    variant_descs.push(VariantDesc {
                        ident: variant,
                        body: Box::new(NodeDesc::Fields {
                            tuple: true,
                            fields: vec![FieldDesc { name: None, ty }],
                        }),
                    });

                    continue;
                }

                let variant = variant_ident(&names, i)?;

                allow |= wants_allow(&variant.to_string());

                let fields = self.finish_fields(fields);

                let body = if tuple_of(&fields) {
                    let tys = fields.iter().map(|field| {
                        let attr = &field.attr;
                        let ty = &field.ty;

                        quote! { #attr #ty, }
                    });

                    quote! { ( #(#tys)* ) }
                } else {
                    let decls = fields.iter().map(|field| {
                        let attr = &field.attr;
                        let name = field.name.as_ref().expect("a named field");
                        let ty = &field.ty;

                        // Variant fields share the visibility of the enum.
                        quote! { #attr #name: #ty, }
                    });

                    quote! { { #(#decls)* } }
                };

                variants.push(quote! { #variant #body });

                variant_descs.push(VariantDesc {
                    ident: variant.clone(),
                    body: Box::new(NodeDesc::Fields {
                        tuple: tuple_of(&fields),
                        fields: desc_fields(&fields),
                    }),
                });
            }

            let allow = allow.then(|| quote! { #[allow(non_camel_case_types)] });

            self.descriptors.push((
                name.clone(),
                NodeDesc::Enum {
                    variants: variant_descs,
                },
            ));

            quote! {
                #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, ::parserc::syntax::Syntax)]
                #allow
                pub enum #name<I>
                where
                    I: ::parserc::chars::CharsInput,
                {
                    #(#variants,)*
                }
            }
        };

        let mut items = std::mem::take(&mut self.helpers);
        items.push(item);

        Ok(items)
    }

    /// The fields one alternative writes: one per operand, with the run of
    /// the trivia between two of them — unless the operand that follows is a
    /// repetition that opens every match, the first included, with the run,
    /// which then says the boundary on its own.
    fn branch_fields(
        &mut self,
        branch: &syntax::OneOrMore<I, syntax::ExprNoTopAlt<I>>,
    ) -> Result<Vec<Field>, UnsynError> {
        let mut fields = Vec::new();
        let mut first = true;

        for element in branch.elements() {
            if !first && self.trivia.is_some() && !swallows_junction(element) {
                fields.push(self.trivia_field());
            }

            fields.extend(self.operand_fields(element)?);
            first = false;
        }

        Ok(fields)
    }

    /// Whether a rule can match the trivia itself, which is what lets a
    /// repetition of it stand without a run of its own between the matches:
    /// the trivia between two of them is an item of the repetition rather
    /// than a boundary the code lays down.
    fn eats_trivia(&self, key: &str, seen: &mut HashSet<String>) -> bool {
        let Some(trivia) = self.trivia.as_deref() else {
            return false;
        };

        if external_key(trivia) == key {
            return true;
        }

        if !seen.insert(key.to_owned()) {
            return false;
        }

        let Some(body) = self.file.items.iter().find_map(|item| match item {
            syntax::Item::Stmt(stmt) => {
                (external_key(stmt.ident.text().as_str()) == *key).then_some(&stmt.expr)
            }
            _ => None,
        }) else {
            return false;
        };

        // A choice reads the trivia where one of its alternatives is nothing
        // but the trivia, or a rule that reads it.
        body.branches().any(|branch| {
            let mut elements = branch.elements();

            match (elements.next(), elements.next()) {
                (
                    Some(syntax::ExprNoTopAlt::WithoutSuffix(
                        syntax::ExprWithoutSuffix::Reference(ident),
                    )),
                    None,
                ) => self.eats_trivia(&external_key(ident.text().as_str()), seen),
                _ => false,
            }
        })
    }

    /// Whether a repetition of the operand opens every match with the run of
    /// the trivia: every one but a repetition of a rule that reads the trivia
    /// itself.
    fn repetition_leads(&self, target: &syntax::ExprWithoutSuffix<I>) -> bool {
        let syntax::ExprWithoutSuffix::Reference(ident) = target else {
            return true;
        };

        !self.eats_trivia(&external_key(ident.text().as_str()), &mut HashSet::new())
    }

    /// One field of an operand repeated: the shape of the operand wrapped in
    /// the shape that reads it the bound over. Every match of a repetition
    /// that may match nothing opens with the run of the trivia before it;
    /// the first match of a repetition that must match one stands at the
    /// boundary the before-run was said at, and the matches after it carry
    /// their own. A repetition of several fields wraps the tuple they read
    /// as, whose runs between them are already inside.
    fn repetition_field(
        &mut self,
        bounds: (usize, Option<usize>),
        inner: Vec<Field>,
        leads: bool,
    ) -> Field {
        let types: Vec<TokenStream> = inner.into_iter().map(|field| field.ty).collect();

        let ty = self.repetition_ty(bounds, types, leads);

        Field {
            name: None,
            ty,
            attr: None,
        }
    }

    /// The types of an operand repeated, wrapped as `repetition_field` wraps
    /// the fields.
    fn repetition_type(
        &mut self,
        bounds: (usize, Option<usize>),
        types: Vec<TokenStream>,
        leads: bool,
    ) -> TokenStream {
        self.repetition_ty(bounds, types, leads)
    }

    fn repetition_ty(
        &self,
        bounds: (usize, Option<usize>),
        types: Vec<TokenStream>,
        leads: bool,
    ) -> TokenStream {
        let lead = (self.trivia.is_some() && leads).then(|| self.trivia_type());

        let elem = |with_lead: bool| {
            let mut parts: Vec<TokenStream> = Vec::new();

            if with_lead {
                parts.extend(lead.clone());
            }

            parts.extend(types.iter().cloned());

            if parts.len() == 1 {
                parts.pop().expect("one part of an element")
            } else {
                quote! { ( #(#parts,)* ) }
            }
        };

        match bounds {
            (0, None) => {
                let e = elem(true);

                quote! { ::std::vec::Vec<#e> }
            }
            (0, Some(1)) => {
                let e = elem(true);

                quote! { ::core::option::Option<#e> }
            }
            (1, None) => {
                let first = elem(false);
                let next = elem(true);

                quote! { ( #first, ::std::vec::Vec<#next> ) }
            }
            _ => unreachable!("checked by the caller"),
        }
    }

    /// The field of the run of the trivia between two operands: the file's
    /// `whitespace` node, read once or not at all.
    fn trivia_field(&self) -> Field {
        Field {
            name: None,
            ty: self.trivia_type(),
            attr: None,
        }
    }

    /// The type of the run of the trivia: the file's `whitespace` node,
    /// optional as the run between two operands is.
    fn trivia_type(&self) -> TokenStream {
        let s = self
            .trivia
            .as_deref()
            .expect("a field of the trivia is only asked for where the file has it");

        let name = item_name(s, Span::None).expect("the trivia rule was named once already");

        quote! { ::core::option::Option<#name<I>> }
    }

    /// The fields one operand writes. An operator of the pattern level —
    /// `concat`, `followed`, `except` — is data like any other operand at
    /// this level: the word it reads stands between the operands on each
    /// side of it, with a run of the trivia between them, which is exactly
    /// what the sequence of three writes.
    fn operand_fields(
        &mut self,
        operand: &syntax::ExprNoTopAlt<I>,
    ) -> Result<Vec<Field>, UnsynError> {
        let suffix = match operand {
            syntax::ExprNoTopAlt::WithoutSuffix(atom) => return self.atom_fields(atom),
            syntax::ExprNoTopAlt::WithSuffix(suffix) => suffix,
        };

        match suffix {
            syntax::ExprWithSuffix::Grouped { target, suffix, .. } => {
                // A repetition repeats what its operand reads: the field of
                // the operand wrapped in the shape that reads it the bound
                // over. The mark itself is not data — unlike the three
                // operator words, which the tree holds as fields of I.
                let bounds = match suffix {
                    syntax::Suffix::Star(_) => (0, None),
                    syntax::Suffix::Question(_) => (0, Some(1)),
                    syntax::Suffix::Plus(_) => (1, None),
                    syntax::Suffix::Repeat(_) => {
                        // A counted repetition of the tree level asks for a
                        // bound no shape of `parserc` counts nodes against.
                        return Err(self.unsupported(target.to_span()));
                    }
                };

                let inner = self.atom_fields(target)?;
                let leads = self.repetition_leads(target);

                Ok(vec![self.repetition_field(bounds, inner, leads)])
            }
            // `concat` is an operator of the grammar and no word of the text:
            // what it says is that the two operands it joins are read with
            // nothing between them, so the fields of the two are written with
            // no run of the trivia between them — and no field of a word,
            // which the text of the language never holds.
            syntax::ExprWithSuffix::Concat { target, suffix, .. } => {
                let mut fields = self.atom_fields(target)?;

                fields.extend(self.operand_fields(suffix)?);

                Ok(fields)
            }
            // A `followed` reads the operand it follows and probes what comes
            // after it without taking any of it: the field is the operand's,
            // and the probe is what the field's reader runs beside.
            syntax::ExprWithSuffix::Followed { target, suffix, .. } => {
                let mut fields = self.atom_fields(target)?;

                let Some(field) = (fields.len() == 1).then(|| fields.pop().expect("one field"))
                else {
                    return Err(self.unsupported(target.to_span()));
                };

                if field.attr.is_some() {
                    return Err(self.unsupported(target.to_span()));
                }

                let ty = field.ty.clone();
                let code = self.code_of_operand(suffix)?;
                let probe = self.expr_of(code)?;
                let read = self.probe_reader(&ty, probe, false);

                Ok(vec![Field {
                    name: field.name,
                    ty,
                    attr: Some(field_parser(&read.to_token_stream())),
                }])
            }
            // An `except` reads the operand and refuses it where the tokens
            // take the whole of what it read: the field is the operand's, and
            // the refusal is what the field's reader runs over the text it
            // took.
            syntax::ExprWithSuffix::Except { target, tokens, .. } => {
                let mut fields = self.atom_fields(target)?;

                let Some(field) = (fields.len() == 1).then(|| fields.pop().expect("one field"))
                else {
                    return Err(self.unsupported(target.to_span()));
                };

                if field.attr.is_some() {
                    return Err(self.unsupported(target.to_span()));
                }

                let mut token_fields = self.atom_fields(tokens)?;

                let Some(token_field) =
                    (token_fields.len() == 1).then(|| token_fields.pop().expect("one field"))
                else {
                    return Err(self.unsupported(tokens.to_span()));
                };

                if token_field.attr.is_some() {
                    return Err(self.unsupported(tokens.to_span()));
                }

                let ty = field.ty.clone();
                let read = self.probe_reader(&ty, token_field.ty, true);

                Ok(vec![Field {
                    name: field.name,
                    ty,
                    attr: Some(field_parser(&read.to_token_stream())),
                }])
            }
        }
    }

    /// The function that reads one field and asks of the text beside it what
    /// a `followed` or an `except` asks: the probe is run over a copy, and
    /// what it reads is taken from nothing. An `except` refuses the field
    /// where the tokens take the whole of what it read.
    fn probe_reader(&mut self, ty: &TokenStream, probe: TokenStream, removal: bool) -> Ident {
        let seq = self.seq;
        self.seq += 1;

        let read = self.unique(format!(
            "read_{}_{}",
            to_snake_case(&self.rule.clone()),
            seq + 1
        ));

        let rule = self.rule.clone();

        let body = if removal {
            quote! {
                let mut whole = input.clone();

                let value = <#ty as ::parserc::syntax::Syntax<I>>::into_parser().parse(input)?;

                let mut rest = whole.split_to(whole.len() - input.len());

                if <#probe as ::parserc::syntax::Syntax<I>>::into_parser()
                    .ok()
                    .parse(&mut rest)?
                    .is_some()
                    && rest.is_empty()
                {
                    return Err(::parserc::Kind::Syntax(
                        #rule,
                        ::parserc::ControlFlow::Fatal,
                        value.to_span(),
                    )
                    .into());
                }

                Ok(value)
            }
        } else {
            quote! {
                let value = <#ty as ::parserc::syntax::Syntax<I>>::into_parser().parse(input)?;

                let mut rest = input.clone();

                #probe.parse(&mut rest)?;

                Ok(value)
            }
        };

        self.helpers.push(quote! {
            fn #read<I>(
                input: &mut I,
            ) -> ::core::result::Result<#ty, <I as ::parserc::Input>::Error>
            where
                I: ::parserc::chars::CharsInput,
            {
                use ::parserc::Parser as _;
                use ::parserc::ToSpan as _;

                #body
            }
        });

        read
    }

    /// The fields one operand that stands alone writes: one, unless it is a
    /// group of one alternative, whose fields the enclosing shape takes
    /// inline; a group of several is one field of the choice of them.
    fn atom_fields(
        &mut self,
        atom: &syntax::ExprWithoutSuffix<I>,
    ) -> Result<Vec<Field>, UnsynError> {
        if let syntax::ExprWithoutSuffix::Paren { expr: group, .. } = atom {
            if group.is_single() {
                let branch = group.branches().next().expect("a pattern has a branch");

                return self.branch_fields(branch);
            }

            let ty = self.choice_type(group)?;

            return Ok(vec![Field {
                name: None,
                ty,
                attr: None,
            }]);
        }

        Ok(vec![self.syntax_atom_field(atom)?])
    }

    /// The field one operand that stands alone writes: the node of a name,
    /// the character of a single one, the text of a word read by its keyword,
    /// the text of a call and of one character of a set or its refusal.
    fn syntax_atom_field(
        &mut self,
        atom: &syntax::ExprWithoutSuffix<I>,
    ) -> Result<Field, UnsynError> {
        match atom {
            syntax::ExprWithoutSuffix::Reference(ident) => {
                let ty = self.reference_type(ident)?;

                Ok(Field {
                    name: self.named_field(ident.text().as_str()),
                    ty,
                    attr: None,
                })
            }
            syntax::ExprWithoutSuffix::Str(lit) => {
                let chars = lit.chars();

                match chars[..] {
                    [c] => Ok(Field {
                        name: None,
                        ty: quote! { ::parserc::syntax::Char<I, #c> },
                        attr: None,
                    }),
                    [] => Err(self.unsupported(lit.to_span())),
                    _ => {
                        let text: String = chars.into_iter().collect();

                        Ok(Field {
                            name: None,
                            ty: quote! { I },
                            attr: Some(quote! { #[parserc(keyword = #text)] }),
                        })
                    }
                }
            }
            syntax::ExprWithoutSuffix::Unicode(lit) => {
                let c = self.code_of(lit)?;

                Ok(Field {
                    name: None,
                    ty: quote! { ::parserc::syntax::Char<I, #c> },
                    attr: None,
                })
            }
            syntax::ExprWithoutSuffix::Call { ident: call, .. } => {
                let name = call.text();

                let parser = self.external.get(&external_key(&name)).ok_or_else(|| {
                    UnsynError::Build(
                        BuildKind::External(self.rule.clone(), name.clone()),
                        call.to_span(),
                    )
                })?;

                Ok(Field {
                    name: None,
                    ty: quote! { I },
                    attr: Some(field_parser(parser)),
                })
            }
            syntax::ExprWithoutSuffix::Set(set) | syntax::ExprWithoutSuffix::Tilde { set, .. } => {
                // A set of the tree level reads one character of what its
                // members denote, which is one test only while every member
                // denotes characters; anything wider has no field to read
                // into at this level.
                let arms = set_arms(set, self.denotations)
                    .ok_or_else(|| self.unsupported(atom.to_span()))?;

                let pred = match atom {
                    syntax::ExprWithoutSuffix::Tilde { .. } => refused_pred(&arms),
                    _ => class_pred(&arms),
                };

                Ok(Field {
                    name: None,
                    ty: quote! { I },
                    attr: Some(field_parser(&quote! { ::parserc::next_if(#pred) })),
                })
            }
            syntax::ExprWithoutSuffix::Paren { .. } => {
                unreachable!("a group is written by `atom_fields`")
            }
        }
    }

    /// The type a reference is read as: the node it names, boxed where the
    /// node is of the tree — a cycle of the tree can only be built through a
    /// node of the tree, and the box is what makes the type finite.
    fn reference_type(&self, ident: &lexer::Ident<I>) -> Result<TokenStream, UnsynError> {
        let text = ident.text();
        let ty = item_name(&text, ident.to_span())?;

        if self.syntaxes.contains(&external_key(&text)) {
            Ok(quote! { Box<#ty<I>> })
        } else {
            Ok(quote! { #ty<I> })
        }
    }

    /// The choice of the alternatives of a group: the nested [`Or`] of the
    /// types of them, tried in the order the file wrote them. This is what
    /// an unnamed middle choice is made of — the shape is the combinator's,
    /// and no name of the generator is in it.
    ///
    /// [`Or`]: parserc::syntax::Or
    fn choice_type(&mut self, body: &syntax::Expr<I>) -> Result<TokenStream, UnsynError> {
        let mut types = Vec::new();

        for branch in body.branches() {
            types.push(self.branch_type(branch)?);
        }

        let mut types = types.into_iter();
        let first = types.next().expect("a pattern has a branch");

        Ok(types.fold(first, |choice, next| {
            quote! { ::parserc::syntax::Or<#choice, #next> }
        }))
    }

    /// The type one alternative of a group reads: the tuple its operands read
    /// in order, with the runs of the trivia between them — or, where the
    /// alternative is one operand of one type, that type itself.
    fn branch_type(
        &mut self,
        branch: &syntax::OneOrMore<I, syntax::ExprNoTopAlt<I>>,
    ) -> Result<TokenStream, UnsynError> {
        let mut types = Vec::new();
        let mut first = true;

        for element in branch.elements() {
            if !first && self.trivia.is_some() && !swallows_junction(element) {
                types.push(self.trivia_type());
            }

            types.extend(self.operand_types(element)?);
            first = false;
        }

        if types.len() == 1 {
            return Ok(types.pop().expect("one type counted"));
        }

        Ok(quote! { ( #(#types,)* ) })
    }

    /// The types one operand of a group reads to, which is its fields' types
    /// when they are all of a shape a tuple can hold: no field of a tuple
    /// carries an attribute, so a punct of more than one character, a call,
    /// and a set of more than one character's denoting have no place inside
    /// a group of the tree, and a file that writes one is told so.
    fn operand_types(
        &mut self,
        operand: &syntax::ExprNoTopAlt<I>,
    ) -> Result<Vec<TokenStream>, UnsynError> {
        let atom = match operand {
            syntax::ExprNoTopAlt::WithoutSuffix(atom) => return self.atom_types(atom),
            syntax::ExprNoTopAlt::WithSuffix(suffix) => suffix,
        };

        match atom {
            syntax::ExprWithSuffix::Grouped { target, suffix, .. } => {
                let bounds = match suffix {
                    syntax::Suffix::Star(_) => (0, None),
                    syntax::Suffix::Question(_) => (0, Some(1)),
                    syntax::Suffix::Plus(_) => (1, None),
                    syntax::Suffix::Repeat(_) => {
                        return Err(self.unsupported(target.to_span()));
                    }
                };

                let inner = self.atom_types(target)?;
                let leads = self.repetition_leads(target);

                Ok(vec![self.repetition_type(bounds, inner, leads)])
            }
            syntax::ExprWithSuffix::Concat { target, suffix, .. } => {
                let mut types = self.atom_types(target)?;

                types.extend(self.operand_types(suffix)?);

                Ok(types)
            }
            syntax::ExprWithSuffix::Followed { .. } | syntax::ExprWithSuffix::Except { .. } => {
                Err(self.unsupported(atom.to_span()))
            }
        }
    }

    fn atom_types(
        &mut self,
        atom: &syntax::ExprWithoutSuffix<I>,
    ) -> Result<Vec<TokenStream>, UnsynError> {
        match atom {
            syntax::ExprWithoutSuffix::Reference(ident) => Ok(vec![self.reference_type(ident)?]),
            syntax::ExprWithoutSuffix::Str(lit) => match lit.chars()[..] {
                [c] => Ok(vec![quote! { ::parserc::syntax::Char<I, #c> }]),
                _ => Err(self.unsupported(lit.to_span())),
            },
            syntax::ExprWithoutSuffix::Unicode(lit) => {
                let c = self.code_of(lit)?;

                Ok(vec![quote! { ::parserc::syntax::Char<I, #c> }])
            }
            syntax::ExprWithoutSuffix::Paren { expr: group, .. } => {
                if group.is_single() {
                    let branch = group.branches().next().expect("a pattern has a branch");

                    return self.branch_type(branch).map(|ty| vec![ty]);
                }

                self.choice_type(group).map(|ty| vec![ty])
            }
            atom => Err(self.unsupported(atom.to_span())),
        }
    }

    /// The report of a shape the syntax level does not write code for.
    fn unsupported(&self, span: Span) -> UnsynError {
        UnsynError::Build(BuildKind::Syntax(self.rule.clone()), span)
    }

    // -- expressions and statements -----------------------------------------

    /// The code of a pattern: the alternatives of it tried in the order written.
    fn code_of_expr(&mut self, expr: &syntax::Expr<I>) -> Result<Code, UnsynError> {
        let mut branches = Vec::new();
        // Which of them answer with the node of a rule rather than with text,
        // settled before any code is written since writing it borrows the
        // generator.
        let answer_kinds: Vec<bool> = expr
            .branches()
            .map(|branch| self.branch_yields_node(branch))
            .collect();

        for branch in expr.branches() {
            branches.push(self.code_of_branch(branch)?);
        }

        if branches.len() == 1 {
            return Ok(branches.pop().expect("one branch counted twice"));
        }

        // A fold asks one answer of every alternative it joins, and the answer
        // a fold of a rule gives is the text the alternative took: an
        // alternative may be a node of its own, and what a `.or()` of a node
        // and of a run of text would answer with is neither. So the branches
        // that answer with a node are read for their text here, which is what
        // lets an alternative keep the fields its own production writes instead
        // of being flattened into one run to suit the fold it is read among.
        //
        // Only those branches are wrapped. A sequence and a repetition are both
        // written as a function that keeps the text it took, and so answers
        // with text already; reading the text of a text again would cost a
        // clone of the input per character of the hottest loop in a lexer for
        // nothing at all.
        let mut choice = branches.into_iter();
        let mut answers = answer_kinds.iter();
        let first = self.expr_of(choice.next().expect("a pattern has a branch"))?;
        let mut left_is_closure = answers.next().copied().unwrap_or(false);
        let mut parser = self.as_text(first, left_is_closure);

        for branch in choice {
            let node = answers.next().copied().unwrap_or(false);
            let other = self.expr_of(branch)?;
            let other = self.as_text(other, node);

            parser = if left_is_closure {
                quote! { (#parser).or(#other) }
            } else {
                quote! { #parser.or(#other) }
            };

            left_is_closure = false;
        }

        Ok(Code::Expr(parser))
    }

    /// The parser `parser` read for the text it took, whatever else it answers
    /// with.
    ///
    /// The helper this reaches for is written once, at the top of the file, the
    /// first time a fold needs it.
    /// The parser of one alternative of a fold, answering with the text it took.
    ///
    /// `yields_node` says whether the alternative answers with the node of a
    /// rule; only then is the reading of its text wrapped around it, since
    /// every other alternative is text already and wrapping it again would cost
    /// a clone of the input on every character a lexer's hottest loop reads.
    fn as_text(&mut self, parser: TokenStream, yields_node: bool) -> TokenStream {
        if !yields_node {
            return parser;
        }

        if self.prelude.is_empty() {
            // No document on the helper: `rustfmt` rewrites a document of more
            // than one line into one attribute per line, and the file this code
            // is kept in is a file `rustfmt` has been over, so the tokens kept
            // would not be the tokens a build writes. What the helper is for is
            // said here, beside the code that asks for it.
            self.prelude.push(quote! {
                fn text_of<P, I>(
                    parser: P,
                    input: &mut I,
                ) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
                where
                    I: ::parserc::chars::CharsInput,
                    P: ::parserc::Parser<I>,
                {
                    let mut whole = input.clone();
                    let before = input.len();

                    parser.parse(input)?;

                    Ok(whole.split_to(before - input.len()))
                }
            });
        }

        // The closure is written bare here and parenthesised only where it is
        // the left operand of a fold: `.or()` binds tighter than the closure
        // body, so a bare closure on the left would take the `.or()` into its
        // own body. As an argument it needs no parens, and writing them would
        // be a lint.
        //
        // The body is a block already, because that is what `rustfmt` rewrites a
        // body of any length into, and the file this code is kept in is a file
        // `rustfmt` has been over: tokens that formatting changes would make
        // the kept file say something other than what a build writes.
        quote! { |input: &mut I| { text_of(#parser, input) } }
    }

    /// Whether one alternative of a fold answers with the node of a rule rather
    /// than with the text it took.
    ///
    /// An alternative of more than one operand is written as a function that
    /// keeps the text those operands took, and so is a suffixed operand and a
    /// joined one; the only answer that is a node is a name read on its own,
    /// bare or behind a group that holds nothing else.
    fn branch_yields_node(&self, branch: &syntax::OneOrMore<I, syntax::ExprNoTopAlt<I>>) -> bool {
        let mut elements = branch.elements();
        let Some(first) = elements.next() else {
            return false;
        };

        if elements.next().is_some() {
            return false;
        }

        match first {
            syntax::ExprNoTopAlt::WithoutSuffix(atom) => self.atom_yields_node(atom),
            syntax::ExprNoTopAlt::WithSuffix(_) => false,
        }
    }

    /// Whether one operand answers with the node of a rule.
    fn atom_yields_node(&self, atom: &syntax::ExprWithoutSuffix<I>) -> bool {
        match atom {
            syntax::ExprWithoutSuffix::Reference(ident) => {
                self.nodes.contains(&external_key(ident.text().as_str()))
            }
            // A group around one operand adds nothing to what that operand
            // answers with.
            syntax::ExprWithoutSuffix::Paren { expr: group, .. } => {
                group.is_single()
                    && group
                        .branches()
                        .next()
                        .is_some_and(|branch| self.branch_yields_node(branch))
            }
            _ => false,
        }
    }

    /// The code of one alternative: the operands of it, one after the other.
    fn code_of_branch(
        &mut self,
        branch: &syntax::OneOrMore<I, syntax::ExprNoTopAlt<I>>,
    ) -> Result<Code, UnsynError> {
        let mut elements = branch.elements();
        let Some(first) = elements.next() else {
            return Ok(Code::Stmt(Vec::new()));
        };

        // An alternative of one operand is that operand, and keeps the shape of
        // it: a group around a single name adds nothing to the code of a rule.
        if elements.next().is_none() {
            return self.code_of_operand(first);
        }

        let mut stmts = Vec::new();

        for element in branch.elements() {
            stmts.extend(self.code_of_operand(element)?.into_stmts());
        }

        Ok(Code::Stmt(stmts))
    }

    /// The code of one operand, with whatever the operand carries after it.
    fn code_of_operand(&mut self, operand: &syntax::ExprNoTopAlt<I>) -> Result<Code, UnsynError> {
        match operand {
            syntax::ExprNoTopAlt::WithoutSuffix(atom) => self.code_of_atom(atom),
            syntax::ExprNoTopAlt::WithSuffix(suffix) => self.code_of_suffix(suffix),
        }
    }

    /// The code of an operand that stands alone.
    fn code_of_atom(&mut self, atom: &syntax::ExprWithoutSuffix<I>) -> Result<Code, UnsynError> {
        match atom {
            syntax::ExprWithoutSuffix::Str(lit) => self.code_of_str(lit),
            syntax::ExprWithoutSuffix::Unicode(lit) => {
                let c = self.code_of(lit)?;
                Ok(Code::Expr(quote! { ::parserc::next(#c) }))
            }
            syntax::ExprWithoutSuffix::Reference(ident) => Ok(Code::Expr(self.reference(ident)?)),
            syntax::ExprWithoutSuffix::Call { ident: call, .. } => {
                let name = call.text();

                let parser = self.external.get(&external_key(&name)).ok_or_else(|| {
                    UnsynError::Build(
                        BuildKind::External(self.rule.clone(), name.clone()),
                        call.to_span(),
                    )
                })?;

                Ok(Code::Expr(parser.clone()))
            }
            syntax::ExprWithoutSuffix::Set(set) => Ok(Code::Expr(self.code_of_set(set)?)),
            syntax::ExprWithoutSuffix::Paren { expr: group, .. } => self.code_of_expr(group),
            syntax::ExprWithoutSuffix::Tilde { set, .. } => {
                // What `~` asks is answered by the character here, and takes it, so
                // that a refusal of the set is a match of the operand.
                if let Some(arms) = set_arms(set, self.denotations) {
                    let pred = refused_pred(&arms);
                    return Ok(Code::Expr(quote! { ::parserc::next_if(#pred) }));
                }

                let parser = self.code_of_set(set)?;

                Ok(Code::Stmt(vec![quote! {
                    {
                        let mut rest = input.clone();
                        if #parser.ok().parse(&mut rest)?.is_some() {
                            return Err(::parserc::Kind::NextIf(
                                ::parserc::ControlFlow::Recovable,
                                input.to_span_at(1),
                            )
                            .into());
                        }
                    }

                    ::parserc::next_if(|_c: char| true).parse(input)?;
                }]))
            }
        }
    }

    /// The code of a set: one test of a character where every member is one, and
    /// a choice of its members where any of them is more.
    fn code_of_set(&mut self, set: &syntax::Set<I>) -> Result<TokenStream, UnsynError> {
        if let Some(arms) = set_arms(set, self.denotations) {
            let pred = class_pred(&arms);
            return Ok(quote! { ::parserc::next_if(#pred) });
        }

        let mut members = Vec::new();

        for item in set_items(set) {
            members.push(self.code_of_set_item(item)?);
        }

        let mut members = members.into_iter();

        let Some(first) = members.next() else {
            return Ok(quote! { ::parserc::next_if(|_c: char| false) });
        };

        let mut parser = self.expr_of(first)?;

        for member in members {
            let other = self.expr_of(member)?;
            parser = quote! { #parser.or(#other) };
        }

        Ok(parser)
    }

    /// The code of one member of a set.
    fn code_of_set_item(&mut self, item: &syntax::SetItem<I>) -> Result<Code, UnsynError> {
        match item {
            syntax::SetItem::Range(range) => {
                let arm = range_arm(range).ok_or_else(|| {
                    UnsynError::Build(BuildKind::Literal(self.rule.clone()), range.to_span())
                })?;

                match arm {
                    Arm::Char(c) => Ok(Code::Expr(quote! { ::parserc::next(#c) })),
                    Arm::Range(lo, hi) => Ok(Code::Expr(quote! {
                        ::parserc::next_if(|c: char| matches!(c, #lo..=#hi))
                    })),
                }
            }
            syntax::SetItem::Str(lit) => self.code_of_str(lit),
            syntax::SetItem::Unicode(lit) => {
                let c = self.code_of(lit)?;
                Ok(Code::Expr(quote! { ::parserc::next(#c) }))
            }
            syntax::SetItem::Reference(ident) => Ok(Code::Expr(self.reference(ident)?)),
        }
    }

    /// The code that reads one operand that is a literal string.
    ///
    /// One character is one item of the input, and more than one is a word of it,
    /// which the two combinators of `parserc` are asked apart.
    fn code_of_str(&self, lit: &lexer::LitStr<I>) -> Result<Code, UnsynError> {
        let chars = lit.chars();

        let parser = match chars[..] {
            [c] => quote! { ::parserc::next(#c) },
            [] => {
                return Err(UnsynError::Build(
                    BuildKind::Literal(self.rule.clone()),
                    lit.to_span(),
                ));
            }
            _ => {
                let text: String = chars.into_iter().collect();
                quote! { ::parserc::keyword(#text) }
            }
        };

        Ok(Code::Expr(parser))
    }

    /// The code that reads one rule by name, as the text the rule took.
    ///
    /// A field, a list and a probe all ask their operand to answer with one
    /// thing: what a choice or a repetition of two rules hands back has to be
    /// one type, and the text both took is that. A node that has been modelled
    /// as parts of its own has no text of a field to hand — the parts are the
    /// node — and reads as itself, which is what [`node_of`] is for.
    fn reference(&self, ident: &lexer::Ident<I>) -> Result<TokenStream, UnsynError> {
        let text = ident.text();
        let ty = item_name(&text, ident.to_span())?;

        if self.nodes.contains(&external_key(&text)) {
            return Ok(node_of(&ty));
        }

        Ok(quote! {
            <#ty<I> as ::parserc::syntax::Syntax<I>>::into_parser().map(|node| node.0)
        })
    }

    /// The code of an operand that carries one of the suffixes.
    fn code_of_suffix(&mut self, suffix: &syntax::ExprWithSuffix<I>) -> Result<Code, UnsynError> {
        match suffix {
            syntax::ExprWithSuffix::Grouped { target, suffix, .. } => {
                let (at_least, at_most) = match suffix {
                    syntax::Suffix::Star(_) => (0, None),
                    syntax::Suffix::Question(_) => (0, Some(1)),
                    syntax::Suffix::Plus(_) => (1, None),
                    syntax::Suffix::Repeat(repeat) => self.bounds(&repeat.bound)?,
                };

                self.code_of_repetition(target, at_least, at_most)
            }
            // `concat` joins two operands with nothing between them, and no node
            // of a `lexer` rule keeps a run of the trivia beside it, so the pair
            // is what a sequence of two operands already is.
            syntax::ExprWithSuffix::Concat { target, suffix, .. } => {
                let mut stmts = self.code_of_atom(target)?.into_stmts();
                stmts.extend(self.code_of_operand(suffix)?.into_stmts());
                Ok(Code::Stmt(stmts))
            }
            syntax::ExprWithSuffix::Followed { target, suffix, .. } => {
                let mut stmts = self.code_of_atom(target)?.into_stmts();
                let operand = self.code_of_operand(suffix)?;
                let parser = self.expr_of(operand)?;

                // The probe runs from where the operand it follows left the text,
                // and gives nothing back: the extent of the rule is the extent of
                // what came before the keyword.
                stmts.push(quote! {
                    {
                        let mut rest = input.clone();
                        #parser.parse(&mut rest)?;
                    }
                });

                Ok(Code::Stmt(stmts))
            }
            syntax::ExprWithSuffix::Except { target, tokens, .. } => {
                let rule = self.rule.clone();
                let left = self.code_of_atom(target)?;
                let left = self.expr_of(left)?;
                let right = self.code_of_atom(tokens)?;
                let right = self.expr_of(right)?;

                // What is removed is asked of the text the operand read and of
                // nothing else, and only a removal that takes all of it refuses.
                Ok(Code::Stmt(vec![quote! {
                    let taken = #left.parse(input)?;

                    {
                        let mut rest = taken.clone();
                        if #right.ok().parse(&mut rest)?.is_some() && rest.is_empty() {
                            return Err(::parserc::Kind::Syntax(
                                #rule,
                                ::parserc::ControlFlow::Fatal,
                                taken.to_span(),
                            )
                            .into());
                        }
                    }
                }]))
            }
        }
    }

    /// The code of an operand repeated.
    ///
    /// A repetition of a class of characters is one call of a combinator that
    /// counts items, bounds them, and never fails short of its bound on its own;
    /// a repetition of anything wider is a loop over the code of the operand.
    fn code_of_repetition(
        &mut self,
        target: &syntax::ExprWithoutSuffix<I>,
        at_least: usize,
        at_most: Option<usize>,
    ) -> Result<Code, UnsynError> {
        let class = match target {
            // A refusal is a class when what it refuses is one, and the class of
            // the refusal is the other half of the characters.
            syntax::ExprWithoutSuffix::Tilde { set, .. } => {
                set_arms(set, self.denotations).as_deref().map(refused_pred)
            }
            atom => atom_arms(atom, self.denotations).as_deref().map(class_pred),
        };

        if let Some(pred) = class {
            let parser = match (at_least, at_most) {
                (0, None) => quote! { ::parserc::take_while(#pred) },
                (at_least, None) => {
                    let at_least = count(at_least);
                    quote! { ::parserc::take_while_with(#at_least.., #pred) }
                }
                (at_least, Some(at_most)) => {
                    let (at_least, at_most) = (count(at_least), count(at_most));
                    quote! { ::parserc::take_while_with(#at_least..=#at_most, #pred) }
                }
            };

            return Ok(Code::Stmt(vec![quote! { #parser.parse(input)?; }]));
        }

        let code = self.code_of_atom(target)?;
        let mut stmts = Vec::new();

        // The times the bound asks for and no alternative about them.
        match at_least {
            0 => {}
            1 => stmts.extend(code.clone().into_stmts()),
            at_least => {
                let each = code.clone().into_stmts();
                let at_least = count(at_least);
                stmts.push(quote! { for _ in 0..#at_least { #(#each)* } });
            }
        }

        let parser = self.expr_of(code)?;

        match at_most {
            None => stmts.push(quote! {
                while #parser.ok().parse(input)?.is_some() {}
            }),
            // At most once is a question rather than a loop.
            Some(1) if at_least == 0 => stmts.push(quote! {
                let _ = #parser.ok().parse(input)?;
            }),
            Some(at_most) if at_most > at_least => {
                let (at_least, at_most) = (count(at_least), count(at_most));
                stmts.push(quote! {
                    for _ in #at_least..#at_most {
                        if #parser.ok().parse(input)?.is_none() {
                            break;
                        }
                    }
                })
            }
            Some(_) => {}
        }

        Ok(Code::Stmt(stmts))
    }

    /// How many times an operand is repeated, of the three forms a bound has.
    fn bounds(&self, bound: &syntax::RepeatBound<I>) -> Result<(usize, Option<usize>), UnsynError> {
        Ok(match bound {
            syntax::RepeatBound::RangeTo(_, _, _, to) => (0, Some(self.count_of(to)?)),
            syntax::RepeatBound::RangeFrom(from, _, _, to) => (
                self.count_of(from)?,
                to.as_ref().map(|(_, to)| self.count_of(to)).transpose()?,
            ),
            syntax::RepeatBound::Count(count) => {
                let count = self.count_of(count)?;
                (count, Some(count))
            }
        })
    }

    /// The character a `U+…` of the file spells.
    fn code_of(&self, lit: &lexer::LitUnicode<I>) -> Result<char, UnsynError> {
        lit.value()
            .ok_or_else(|| UnsynError::Build(BuildKind::Literal(self.rule.clone()), lit.to_span()))
    }

    /// The number of times a bound of the file repeats.
    fn count_of(&self, lit: &lexer::LitDec<I>) -> Result<usize, UnsynError> {
        lit.count()
            .map(|count| count as usize)
            .ok_or_else(|| UnsynError::Build(BuildKind::Literal(self.rule.clone()), lit.to_span()))
    }

    // -- names --------------------------------------------------------------

    /// The parser an expression is, given a function to be if it is statements.
    fn expr_of(&mut self, code: Code) -> Result<TokenStream, UnsynError> {
        Ok(match code {
            Code::Expr(parser) => parser,
            Code::Stmt(stmts) => {
                let seq = self.seq;
                self.seq += 1;

                let read = self.unique(format!("read_{}_{}", to_snake_case(&self.rule), seq + 1));
                self.helpers.push(reader(&read, &stmts));
                quote! { #read }
            }
        })
    }

    // -- the visitor ---------------------------------------------------------

    /// The visitor of the module: one hook per rule, each descending through
    /// the walk of it unless a visitor says otherwise, and one hook for the
    /// text every leaf ends with — the same shape the hand tree's visitor
    /// has, built from the shapes the file declared and nothing else.
    fn visit_items(&mut self) -> TokenStream {
        use quote::format_ident;

        let mut hooks = Vec::new();
        let mut walks = Vec::new();
        let mut names = Vec::new();

        for (rule, desc) in std::mem::take(&mut self.descriptors).iter() {
            let snake = to_snake_case(&rule.to_string());
            let visit = format_ident!("visit_{}", snake);
            let walk = format_ident!("walk_{}", snake);
            let body = walk_body(rule, desc);

            names.push(rule.clone());
            hooks.push(quote! {
                #[doc = concat!("The node of the rule `", stringify!(#rule), "`.")]
                fn #visit(&mut self, node: &'ast #rule<I>) {
                    walk::#walk(self, node)
                }
            });
            walks.push(quote! {
                #[doc = concat!("Descend through the node of `", stringify!(#rule), "`.")]
                pub fn #walk<'ast, I, V>(v: &mut V, node: &'ast #rule<I>)
                where
                    I: ::parserc::chars::CharsInput,
                    V: ?Sized + Visit<'ast, I>,
                {
                    #body
                }
            });
        }

        quote! {
            /// The visitor of the file's nodes, in the shape of `syn`'s: one
            /// method per rule descending through the walk of it unless a
            /// visitor says otherwise, and one method for the text that every
            /// walk ends with.
            ///
            /// [`syn`]: https://docs.rs/syn/latest/syn/visit/index.html
            #[rustfmt::skip]
            pub trait Visit<'ast, I>
            where
                I: ::parserc::chars::CharsInput,
            {
                /// The text of one token, which is the leaf every descent
                /// hands over.
                fn visit_text(&mut self, _text: &'ast I) {}

                #(#hooks)*
            }

            /// The descents of the visitor, one per rule, public so that a
            /// method which does its own work can still hand the children to
            /// the visitor below it.
            ///
            /// The module is machine-written; the skip keeps the formatter
            /// from rewriting the file into tokens the generator would not
            /// have written.
            #[rustfmt::skip]
            #[allow(clippy::needless_borrow, clippy::possible_missing_else)]
            pub mod walk {
                // The list keeps the order the file declared its rules in,
                // which is the order the walks below read them.
                #[rustfmt::skip]
                use super::{Visit, #(#names),*};

                #(#walks)*
            }
        }
    }

    /// A name for a function of the module, which no other function of it has.
    fn unique(&mut self, base: String) -> Ident {
        let mut name = base.clone();
        let mut over = 2;

        while !self.taken.insert(name.clone()) {
            name = format!("{base}_{over}");
            over += 1;
        }

        Ident::new(&name, proc_macro2::Span::call_site())
    }
}

/// Which of the four shapes a rule came to.
enum Rule {
    /// The rule is one expression, written beside the field it reads into.
    Field(TokenStream),
    /// The rule is a function, named by the rule.
    Read(Ident),
    /// The rule is the trivia, and is one test of a character.
    TakeWhile(Ident),
    /// The rule is a sequence, and the struct holds one field per operand.
    Fields {
        /// Whether the fields are nameless and stand in a tuple, which says
        /// what they are by where they are.
        tuple: bool,
        /// The fields, in the order the operands were written.
        fields: Vec<Field>,
    },
}

/// The shape of a written node, which is what the visitor walks it with.
enum NodeDesc {
    /// The node is one field of the text it read.
    Text,
    /// The node is a struct of fields, named or tuple.
    Fields { tuple: bool, fields: Vec<FieldDesc> },
    /// The node is an enum of alternatives.
    Enum { variants: Vec<VariantDesc> },
}

/// One field of a described node.
struct FieldDesc {
    name: Option<Ident>,
    ty: TokenStream,
}

/// One alternative of a described enum, named what the code named it and
/// holding the fields the alternative writes.
struct VariantDesc {
    ident: Ident,
    body: Box<NodeDesc>,
}

/// One field of the struct a sequence came out as.
struct Field {
    /// What the field is called, which is nothing where the struct is a tuple.
    name: Option<Ident>,
    /// The type the field holds: the node of a rule, a list or an option of
    /// one, or the text of whatever else was read.
    ty: TokenStream,
    /// The attribute that says how the field is read, where its type alone
    /// does not say it.
    attr: Option<TokenStream>,
}

/// The parser of a rule that is a node rather than the text it read: the node
/// it names, read whole.
fn node_of(ty: &Ident) -> TokenStream {
    quote! {
        <#ty<I> as ::parserc::syntax::Syntax<I>>::into_parser()
    }
}

/// Whether an operand arrives with the run before it of its own: a
/// repetition whose every match, the first included, opens with the optional
/// trivia, which then says the boundary on its own.
fn swallows_junction<I>(operand: &syntax::ExprNoTopAlt<I>) -> bool
where
    I: UnsynInput,
{
    match operand {
        syntax::ExprNoTopAlt::WithSuffix(syntax::ExprWithSuffix::Grouped { suffix, .. }) => {
            matches!(
                suffix,
                syntax::Suffix::Star(_) | syntax::Suffix::Question(_)
            )
        }
        _ => false,
    }
}

/// The statements that walk one node, handing each part to the visitor.
fn walk_body(rule: &Ident, desc: &NodeDesc) -> TokenStream {
    match desc {
        NodeDesc::Text => quote! { v.visit_text(&node.0); },
        NodeDesc::Fields { tuple, fields } => {
            let mut steps = Vec::new();

            for (i, field) in fields.iter().enumerate() {
                let acc = match &field.name {
                    Some(name) if !*tuple => quote! { &node.#name },
                    _ => {
                        let index = Literal::usize_unsuffixed(i);

                        quote! { &node.#index }
                    }
                };

                steps.push(walk_code(&ty_shape(&field.ty), &acc));
            }

            quote! { #(#steps)* }
        }
        NodeDesc::Enum { variants } => {
            let arms = variants.iter().map(|variant| {
                let ident = &variant.ident;

                match variant.body.as_ref() {
                    NodeDesc::Fields {
                        tuple: true,
                        fields,
                    } if fields.len() == 1 => {
                        let step = walk_code(&ty_shape(&fields[0].ty), &quote! { part });

                        quote! {
                            #rule::#ident(part) => { #step }
                        }
                    }
                    NodeDesc::Fields { tuple, fields } => {
                        let (binds, steps) = if *tuple {
                            let binds: Vec<TokenStream> = (0..fields.len())
                                .map(|i| {
                                    let name = quote::format_ident!("part{}", i);

                                    quote! { #name }
                                })
                                .collect();

                            let steps: Vec<TokenStream> = binds
                                .iter()
                                .zip(fields)
                                .map(|(bind, field)| {
                                    walk_code(&ty_shape(&field.ty), &quote! { #bind })
                                })
                                .collect();

                            (binds, steps)
                        } else {
                            let binds: Vec<TokenStream> = fields
                                .iter()
                                .filter_map(|field| field.name.as_ref())
                                .map(|name| quote! { #name })
                                .collect();

                            let steps: Vec<TokenStream> = fields
                                .iter()
                                .map(|field| {
                                    let name = field.name.as_ref().expect("a named variant");

                                    walk_code(&ty_shape(&field.ty), &quote! { #name })
                                })
                                .collect();

                            (binds, steps)
                        };

                        let pattern = if *tuple {
                            quote! { #rule::#ident( #(#binds,)* ) }
                        } else {
                            quote! { #rule::#ident { #(#binds,)* .. } }
                        };

                        quote! {
                            #pattern => { #(#steps)* }
                        }
                    }
                    _ => unreachable!("an alternative is always a struct"),
                }
            });

            quote! {
                match node {
                    #(#arms)*
                }
            }
        }
    }
}

/// The descent through one value of one shape. The expression reached the
/// value by reference, and every step hands the next one the same way.
/// The descent through one value of one shape. `acc` is an expression that
/// reaches the value by reference — a borrow of a field, a binding from a
/// pattern or a loop — and every step keeps the next one there: a place
/// borrowed through a reference, a binding passed under the rule that
/// borrow-through-reference already is, and a box of a node read as the
/// node, which is what reaching a node by reference through its box
/// already says.
fn walk_code(shape: &TyShape, acc: &TokenStream) -> TokenStream {
    use quote::format_ident;

    match shape {
        TyShape::Text => quote! { v.visit_text(#acc); },
        TyShape::Char => quote! { v.visit_text(&(#acc).0); },
        TyShape::Rule(name) => {
            let visit = format_ident!("visit_{}", to_snake_case(&name.to_string()));

            quote! { v.#visit(#acc); }
        }
        TyShape::Boxed(inner) => walk_code(inner, acc),
        TyShape::Opt(inner) => {
            let step = walk_code(inner, &quote! { inner });

            quote! {
                if let Some(inner) = #acc {
                    #step
                }
            }
        }
        TyShape::VecOf(inner) => {
            let step = walk_code(inner, &quote! { inner });

            quote! {
                for inner in #acc {
                    #step
                }
            }
        }
        TyShape::Tuple(parts) => {
            let steps: Vec<TokenStream> = parts
                .iter()
                .enumerate()
                .map(|(i, part)| {
                    let index = Literal::usize_unsuffixed(i);

                    walk_code(part, &quote! { &(#acc).#index })
                })
                .collect();

            quote! { #(#steps)* }
        }
        TyShape::Choice(first, second) => {
            let a = walk_code(first, &quote! { first });
            let b = walk_code(second, &quote! { second });

            quote! {
                match #acc {
                    ::parserc::syntax::Or::First(first) => { #a }
                    ::parserc::syntax::Or::Second(second) => { #b }
                }
            }
        }
    }
}

/// The described shape of a written list of fields.
fn desc_fields(fields: &[Field]) -> Vec<FieldDesc> {
    fields
        .iter()
        .map(|field| FieldDesc {
            name: field.name.clone(),
            ty: field.ty.clone(),
        })
        .collect()
}

/// The shape of a type the generator wrote, read back from the tokens of it
/// for the visitor to walk. Every type the syntax level emits is one of
/// these shapes, and nothing else is ever asked.
enum TyShape {
    /// One field of the input: the text of a word, a call, a class.
    Text,
    /// One character, read by `Char` and holding its text in the one field.
    Char,
    /// The node of a rule the file declares.
    Rule(Ident),
    /// A box of one shape, a list or an option of one, a tuple of several,
    /// and the nested choice of two.
    Boxed(Box<TyShape>),
    Opt(Box<TyShape>),
    VecOf(Box<TyShape>),
    Tuple(Vec<TyShape>),
    Choice(Box<TyShape>, Box<TyShape>),
}

/// The tokens of a written type, read as a `TyShape`.
fn ty_shape(ty: &TokenStream) -> TyShape {
    let tokens = flatten(ty.clone());

    // A tuple is one parenthesised group and nothing else.
    if tokens.len() == 1
        && let TokenTree::Group(group) = &tokens[0]
        && group.delimiter() == proc_macro2::Delimiter::Parenthesis
    {
        return TyShape::Tuple(
            split_trees(flatten(group.stream()))
                .iter()
                .map(|part| ty_shape_of_slice(part.as_slice()))
                .collect(),
        );
    }

    // A path, its last segment opening a generic list at most once. The
    // angle brackets are plain puncts to a token stream, so the list is cut
    // by counting them.
    let mut open = None;
    let mut depth = 0usize;
    let mut close = tokens.len();

    for (i, token) in tokens.iter().enumerate() {
        if let TokenTree::Punct(punct) = token {
            match punct.as_char() {
                '<' => {
                    if depth == 0 {
                        open = Some(i);
                    }

                    depth += 1;
                }
                '>' => {
                    depth -= 1;

                    if depth == 0 {
                        close = i;
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    let path = &tokens[..open.unwrap_or(tokens.len())];
    let parts: Vec<Vec<TokenTree>> = match open {
        Some(k) => split_trees(tokens[k + 1..close].to_vec()),
        None => Vec::new(),
    };

    let mut name = String::new();

    for token in path {
        if let TokenTree::Ident(ident) = token {
            if !name.is_empty() {
                name.push_str("::");
            }

            name.push_str(&ident.to_string());
        }
    }

    let mut parts = parts.into_iter();

    match (name.as_str(), parts.len()) {
        ("I", 0) => TyShape::Text,
        ("Box", 1) => TyShape::Boxed(Box::new(ty_shape_of_slice(&parts.next().unwrap()))),
        ("std::vec::Vec", 1) => TyShape::VecOf(Box::new(ty_shape_of_slice(&parts.next().unwrap()))),
        ("core::option::Option", 1) => {
            TyShape::Opt(Box::new(ty_shape_of_slice(&parts.next().unwrap())))
        }
        ("parserc::syntax::Char", _) => TyShape::Char,
        ("parserc::syntax::Or", 2) => {
            let first = ty_shape_of_slice(&parts.next().unwrap());
            let second = ty_shape_of_slice(&parts.next().unwrap());

            TyShape::Choice(Box::new(first), Box::new(second))
        }
        (last, 1) => TyShape::Rule(written_name(
            last.rsplit("::")
                .next()
                .expect("one segment of a written type"),
        )),
        (name, n) => {
            unreachable!("no type the generator writes looks like {name} with {n}")
        }
    }
}

fn ty_shape_of_slice(tokens: &[TokenTree]) -> TyShape {
    ty_shape(&TokenStream::from_iter(tokens.iter().cloned()))
}

/// The trees of a stream, with the invisible groups the macros leave opened.
fn flatten(ty: TokenStream) -> Vec<TokenTree> {
    let mut out = Vec::new();

    for token in ty {
        match token {
            TokenTree::Group(group) if group.delimiter() == proc_macro2::Delimiter::None => {
                out.extend(flatten(group.stream()));
            }
            other => out.push(other),
        }
    }

    out
}

/// The comma-separated parts of a generic list, cut at the commas that stand
/// at the bracket depth of the list itself.
fn split_trees(mut tokens: Vec<TokenTree>) -> Vec<Vec<TokenTree>> {
    let mut parts: Vec<Vec<TokenTree>> = Vec::new();
    let mut current: Vec<TokenTree> = Vec::new();
    let mut depth = 0usize;

    for token in tokens.drain(..) {
        if let TokenTree::Punct(punct) = &token {
            match punct.as_char() {
                '<' => depth += 1,
                '>' => depth -= 1,
                ',' if depth == 0 => {
                    parts.push(std::mem::take(&mut current));
                    continue;
                }
                _ => {}
            }
        }

        current.push(token);
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

/// Whether a sequence of fields is nameless throughout, which is what lets
/// them stand in a tuple where their places say what they are.
fn tuple_of(fields: &[Field]) -> bool {
    fields.iter().all(|field| field.name.is_none())
}

/// The one name an alternative that is nothing but a name holds, which is the
/// variant the enum of a choice wears after it.
fn one_reference<I>(
    branch: &syntax::OneOrMore<I, syntax::ExprNoTopAlt<I>>,
) -> Option<&lexer::Ident<I>>
where
    I: UnsynInput,
{
    let mut elements = branch.elements();

    let (
        Some(syntax::ExprNoTopAlt::WithoutSuffix(syntax::ExprWithoutSuffix::Reference(ident))),
        None,
    ) = (elements.next(), elements.next())
    else {
        return None;
    };

    Some(ident)
}

/// The word a pattern reads, as the name of the shape it makes: begun by an
/// upper case letter, which is how the parts of a node are told from the
/// node.
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();

    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::from("Empty"),
    }
}

/// The struct one sequence of fields writes, shared by the two levels: a
/// tuple where none of the fields has a name, and braces — with their bounds
/// settled before them — where any one of them does.
fn struct_item(
    name: &Ident,
    allow: &dyn quote::ToTokens,
    tuple: bool,
    fields: &[Field],
) -> TokenStream {
    let decl = if tuple {
        let tys = fields.iter().map(|field| {
            let attr = &field.attr;
            let ty = &field.ty;

            quote! { #attr pub #ty, }
        });

        quote! {
            pub struct #name<I>(
                #(#tys)*
            )
        }
    } else {
        let decls = fields.iter().map(|field| {
            let attr = &field.attr;
            let name = field.name.as_ref().expect("a named struct");
            let ty = &field.ty;

            quote! {
                #attr
                pub #name: #ty,
            }
        });

        quote! {
            pub struct #name<I>
            where
                I: ::parserc::chars::CharsInput,
            {
                #(#decls)*
            }
        }
    };

    let where_ = if tuple {
        quote! {
            where
                I: ::parserc::chars::CharsInput;
        }
    } else {
        quote! {}
    };

    quote! {
        #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, ::parserc::syntax::Syntax)]
        #allow
        #decl
        #where_
    }
}

/// The attribute that writes a field's reader beside the field.
fn field_parser(parser: &TokenStream) -> TokenStream {
    quote! { #[parserc(parser = #parser)] }
}

/// The combinator that reads a class of characters a bounded number of times,
/// which is one call that counts items and needs no function of its own.
fn bound_combinator(pred: &TokenStream, at_least: usize, at_most: Option<usize>) -> TokenStream {
    match (at_least, at_most) {
        (0, None) => quote! { ::parserc::take_while(#pred) },
        (at_least, None) => quote! { ::parserc::take_while_with(#at_least.., #pred) },
        (at_least, Some(at_most)) => {
            quote! { ::parserc::take_while_with(#at_least..=#at_most, #pred) }
        }
    }
}

/// The function that reads `stmts` and keeps the text they took.
fn reader(read: &Ident, stmts: &[TokenStream]) -> TokenStream {
    quote! {
        fn #read<I>(input: &mut I) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
        where
            I: ::parserc::chars::CharsInput,
        {
            use ::parserc::Parser as _;

            let mut whole = input.clone();
            let before = input.len();

            #(#stmts)*

            Ok(whole.split_to(before - input.len()))
        }
    }
}

/// The name of a rule, as the name of an item of Rust.
///
/// The name of a node is the name of its rule, sigil and all: a rule written `r#`
/// before a word of Rust is an item written the same way, and a rule written plain
/// is plain. What is no name at all is refused rather than mended, since a name the
/// file could not be read as holding is not a name the code should invent.
/// Whether a word is one an item of Rust can wear.
///
/// This is the test [`item_name`] makes, on its own, so that a caller holding
/// only a candidate word — a variant's, before the code that would wear it is
/// written — can ask whether there is one to wear.
fn is_item_name(text: &str) -> bool {
    let base = text.strip_prefix("r#").unwrap_or(text);
    let mut chars = base.chars();

    match chars.next() {
        // A lone underscore is a name the reader accepts and no item can wear.
        None => false,
        Some(first) => {
            !(base == "_") && (first == '_' || is_xid_start(first)) && chars.all(is_xid_continue)
        }
    }
}

fn item_name(text: &str, span: Span) -> Result<Ident, UnsynError> {
    let base = text.strip_prefix("r#").unwrap_or(text);

    if !is_item_name(text) {
        return Err(UnsynError::Build(BuildKind::Name(text.to_owned()), span));
    }

    // A name the file wrote raw is written raw here too, which is what lets a rule
    // borrow a word of Rust for its own: `r#gen` names the item `gen`, and no word
    // of Rust is a name an item may wear plain.
    Ok(if text.starts_with("r#") {
        Ident::new_raw(base, proc_macro2::Span::call_site())
    } else {
        Ident::new(base, proc_macro2::Span::call_site())
    })
}

/// Whether a name is one the compiler would warn an item of Rust for wearing.
fn wants_allow(text: &str) -> bool {
    let base = text.strip_prefix("r#").unwrap_or(text);

    base.contains('_') || base.chars().next().is_some_and(|c| c.is_lowercase())
}

/// The name of a rule in lower case, with the underscores a word of several is
/// written between.
fn to_snake_case(text: &str) -> String {
    let base = text.strip_prefix("r#").unwrap_or(text);
    let chars: Vec<char> = base.chars().collect();

    let mut name = String::with_capacity(base.len() + chars.len() / 2);

    for (i, c) in chars.iter().copied().enumerate() {
        if c == '_' {
            name.push('_');
            continue;
        }

        // A word begins where a lower case letter or a digit gives way to an
        // upper case one, and where a run of upper case letters gives way to a
        // lower case one with the last letter of the run.
        let begins = i > 0
            && c.is_uppercase()
            && match chars[i - 1] {
                '_' => false,
                prev if prev.is_lowercase() || prev.is_ascii_digit() => true,
                prev => {
                    prev.is_uppercase() && chars[i..].get(1).is_some_and(|next| next.is_lowercase())
                }
            };

        if begins {
            name.push('_');
        }

        name.extend(c.to_lowercase());
    }

    name
}

/// The members of a set, in the order they are written, comma or no last comma.
fn set_items<I>(set: &syntax::Set<I>) -> Vec<&syntax::SetItem<I>>
where
    I: UnsynInput,
{
    set.members().collect()
}

#[cfg(test)]
mod tests {
    use parserc::{Input, syntax::SyntaxInput};

    use super::*;
    use crate::compiler::errors::{LintKind, SemanticsKind};
    use crate::compiler::input::TokenStream as Spec;
    use crate::compiler::semantic;

    /// Reads `source` as a file of the DSL, and asks the analyzer first whether
    /// it is a file a build may be trusted to read.
    fn parse(source: &str) -> syntax::Crate<Spec<'_>> {
        let mut input = Spec::from(source);

        let tree = input
            .parse::<syntax::Crate<_>>()
            .expect("the source should be a file of the language");

        assert!(input.is_empty(), "the file left text unread");

        let report = semantic::analyze(&tree);
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        tree
    }

    /// The code the file is written as, with what the builder was given.
    ///
    /// `<IdentOrKeyword>` is bound to a scan of the module's own, which is what
    /// an external is: the code written for it reaches nothing of the file.
    fn build(source: &str) -> Result<TokenStream, UnsynError> {
        let file = parse(source);
        let mut builder = Builder::new(&file);
        builder.external(
            "XID_Start",
            quote! { ::parserc::next_if(::unicode_ident::is_xid_start) },
        );
        builder.external(
            "XID_Continue",
            quote! { ::parserc::next_if(::unicode_ident::is_xid_continue) },
        );
        builder.external(
            "IdentOrKeyword",
            quote! {
                ::parserc::take_while_with(1.., |c: char| {
                    ::unicode_ident::is_xid_continue(c) || ::unicode_ident::is_xid_start(c)
                        || c == '_'
                })
            },
        );
        builder.build()
    }

    /// The code of a file of terminals, which is what a build of one is asked for.
    fn code(source: &str) -> String {
        build(source)
            .expect("the file should be generated")
            .to_string()
    }

    /// The code of a file whose `syntax` rules are left out, since a `lexer` rule
    /// reaches no `syntax` one and what is left still reads.
    fn terminals(source: &str) -> String {
        let file = parse(source);

        let terminals = syntax::Crate {
            inner_docs: Vec::new(),
            items: file
                .items
                .iter()
                .filter(|item| matches!(item, syntax::Item::Stmt(stmt) if !matches!(stmt.decl(), Decl::Syntax)))
                .cloned()
                .collect(),
        };

        let mut builder = Builder::new(&terminals);
        builder.external(
            "IdentOrKeyword",
            quote! {
                ::parserc::take_while_with(1.., |c: char| {
                    ::unicode_ident::is_xid_continue(c) || ::unicode_ident::is_xid_start(c)
                        || c == '_'
                })
            },
        );

        builder
            .build()
            .expect("the file should be generated")
            .to_string()
    }

    /// Whether the code `soup` is written with the fragment `parser` in it.
    #[track_caller]
    fn has(soup: &str, parser: TokenStream) -> bool {
        soup.contains(&parser.to_string())
    }

    #[test]
    fn one_character_is_one_item() {
        let soup = code("lexer A -> 'a';");

        assert!(has(&soup, quote! { pub struct A < I > }));
        assert!(has(
            &soup,
            quote! { #[parserc(parser = ::parserc::next('a'))] pub I }
        ));
    }

    #[test]
    fn a_word_of_more_than_one_character_is_a_keyword() {
        let soup = code("lexer A -> 'abc';");

        assert!(has(&soup, quote! { ::parserc::keyword("abc") }));
    }

    #[test]
    fn a_class_is_one_test_of_a_character() {
        let soup = code("lexer D -> ['0'-'9', '_'];");

        assert!(has(
            &soup,
            quote! { ::parserc::next_if(|c: char| matches!(c, '0'..='9' | '_')) }
        ));
    }

    #[test]
    fn a_refusal_of_a_class_is_the_other_half() {
        let soup = code("lexer C -> ~['a'-'z'];");

        assert!(soup.contains("next_if (| c : char | ! (matches ! (c , 'a' ..= 'z')))"));
    }

    #[test]
    fn a_repetition_of_a_class_is_one_combinator_that_counts() {
        let soup = code("lexer A -> 'a'*;");
        assert!(soup.contains("take_while (| c : char | matches ! (c , 'a')) . parse (input)"));

        // The bounds are written without a suffix, so that they take the type of
        // the range rather than one of their own.
        let soup = code("lexer A -> 'a'{1, 6};");
        assert!(soup.contains("take_while_with (1 ..= 6 ,"));

        let soup = code("lexer A -> 'a'{3};");
        assert!(soup.contains("take_while_with (3 ..= 3 ,"));
    }

    #[test]
    fn a_repetition_of_anything_wider_is_a_loop() {
        // A rule that reads two characters in order denotes no set of characters
        // that one test could answer for, so its repetition is read a time at a
        // time until it fails.
        let soup = code("lexer A -> 'b' 'c'; lexer B -> A*;");

        assert!(soup.contains("while"));
        assert!(soup.contains(". ok () . parse (input) ? . is_some ()"));
    }

    #[test]
    fn an_external_is_written_as_the_builder_was_given_it() {
        let soup = code("lexer A -> <XID_Start>;");

        assert!(has(
            &soup,
            quote! { #[parserc(parser = ::parserc::next_if(::unicode_ident::is_xid_start))] pub I }
        ));
    }

    #[test]
    fn an_unbound_external_is_refused() {
        let file = parse("lexer A -> <NoSuchCombinator>;");
        let error = Builder::new(&file).build().unwrap_err();

        assert!(matches!(
            error,
            UnsynError::Build(BuildKind::External(rule, name), _)
                if rule == "A" && name == "NoSuchCombinator"
        ));
    }

    #[test]
    fn a_reference_in_a_sequence_is_written_as_the_node_it_names() {
        // A reference of a sequence is a field of the node it names, read by
        // nothing but the type; the text of it is what the node keeps in its
        // own parts.
        let soup = code("lexer A -> 'a'; lexer B -> A 'b';");

        assert!(has(
            &soup,
            quote! { #[parserc(parser = ::parserc::next('b'))] pub f1: I, }
        ));
        assert!(has(&soup, quote! { pub a: A<I>, }));
    }

    #[test]
    fn a_syntax_choice_is_written_as_an_enum_and_a_sequence_as_a_struct() {
        let soup = code("lexer A -> 'a'; lexer B -> 'b'; syntax C -> A | B;");

        assert!(soup.contains("pub enum C < I > where I : :: parserc :: chars :: CharsInput , { A (A < I >) , B (B < I >) , }"));

        // A word of the alternative names the variant that reads it.
        let soup = code("lexer A -> 'a'; syntax B -> A followed A | A except A;");

        assert!(soup.contains("Followed"), "{soup}");
        assert!(soup.contains("Except"), "{soup}");
    }

    #[test]
    fn a_name_read_among_others_keeps_its_shape_and_is_read_for_text() {
        // `C` is a choice of names, so it is an enum of the two nodes it names;
        // `D` reads `C` beside a word of its own, where the two alternatives
        // have to answer with one thing. The fold is what changes its answer to
        // text, and only for the alternative that had a node to give: the word
        // is text already, and reading the text of a text again would cost a
        // clone of the input per character for nothing.
        let soup =
            code("lexer A -> 'a'; lexer B -> 'b'; lexer C -> A | B; lexer D -> ( C | 'z' ) 'x';");

        assert!(soup.contains("pub enum C"), "{soup}");
        assert!(has(
            &soup,
            quote! { (|input: &mut I| { text_of(<C<I> as ::parserc::syntax::Syntax<I>>::into_parser(), input) }).or(::parserc::next('z')) }
        ));
        assert!(
            !soup.contains("text_of (:: parserc :: next ('z')"),
            "a word is text already: {soup}"
        );
    }

    #[test]
    fn a_sequence_is_read_in_order() {
        let soup = code("lexer A -> 'a' 'b' 'c';");

        let mut at = 0;
        for word in ["next ('a')", "next ('b')", "next ('c')"] {
            let found = soup[at..].find(word).expect("the order of the parts");
            at += found + word.len();
        }
    }

    #[test]
    fn a_removal_is_asked_of_the_text_it_was_written_over() {
        let soup = code("lexer A -> 'a'; lexer B -> 'b'; lexer C -> A except B;");

        assert!(soup.contains("< A < I > as :: parserc :: syntax :: Syntax < I >> :: into_parser () . map (| node | node . 0) . parse (input) ?"));
        assert!(soup.contains("let mut rest = taken . clone ()"));
        assert!(soup.contains("rest . is_empty ()"));
        assert!(soup.contains(
            "Err (:: parserc :: Kind :: Syntax (\"C\" , :: parserc :: ControlFlow :: Fatal , taken . to_span () ,) . into ())"
        ));
    }

    #[test]
    fn a_following_is_probed_without_being_taken() {
        let soup = code("lexer A -> 'a'; lexer B -> 'b'; lexer C -> A followed B;");

        assert!(soup.contains("let mut rest = input . clone ()"));
        assert!(soup.contains("< B < I > as :: parserc :: syntax :: Syntax < I >> :: into_parser () . map (| node | node . 0) . parse (& mut rest) ?"));
    }

    #[test]
    fn a_joining_writes_a_field_apiece_and_no_run_between_them() {
        let soup = code("lexer A -> 'ab' concat 'cd';");

        // `concat` joins two operands, so the rule is the sequence of two it
        // writes — a field apiece — and nothing stands between them, which is
        // what tells a joining from a plain sequence of a `syntax` rule.
        assert!(has(
            &soup,
            quote! {
                pub struct A<I>(
                    #[parserc(parser = ::parserc::keyword("ab"))] pub I,
                    #[parserc(parser = ::parserc::keyword("cd"))] pub I,
                )
                where
                    I: ::parserc::chars::CharsInput;
            }
        ));
        assert!(!soup.contains("Option < S < I >>"), "{soup}");
    }

    #[test]
    fn the_trivia_of_a_lexer_rule_is_not_taken() {
        // The sequence is a field apiece, and no field of a lexer node is a
        // run of the trivia: a literal's field is the keyword's own text.
        let soup = code("lexer A -> 'a'; lexer C -> A 'x';");
        assert!(has(
            &soup,
            quote! { #[parserc(parser = ::parserc::next('x'))] pub f1: I, }
        ));
        assert!(!soup.contains("whole . split_to"));

        // A choice of alternatives is an enum of one variant apiece even when
        // every alternative is one plain character: the first level of a right
        // side says what the node is made of, and a choice says variants. The
        // word each alternative reads names its variant.
        let soup = code("lexer A -> 'a' | 'b';");
        assert!(
            soup.contains(
                "pub enum A < I > where I : :: parserc :: chars :: CharsInput , { \
                 A (# [parserc (parser = :: parserc :: next ('a'))] I ,) , \
                 B (# [parserc (parser = :: parserc :: next ('b'))] I ,) }"
            ),
            "{soup}"
        );

        // Only a single operand is the text of the token it read.
        let soup = code("lexer A -> 'ab';");
        assert!(has(
            &soup,
            quote! {
                pub struct A<I>(#[parserc(parser = ::parserc::keyword("ab"))] pub I)
                where
                    I: ::parserc::chars::CharsInput;
            }
        ));
    }

    #[test]
    fn a_class_of_nothing_is_answered_once_and_opposite_ways() {
        // The analyzer refuses a class of nothing before any code is written, so
        // these are the one place the answer lives — and the two answers must
        // not be the same, which is the ease in which they would be mixed up
        // if each caller settled it for itself.
        assert_eq!(
            class_pred(&[]).to_string().replace(' ', ""),
            "|_c:char|false"
        );
        assert_eq!(
            refused_pred(&[]).to_string().replace(' ', ""),
            "|_c:char|true"
        );

        // A class of one arm reads the character, and its refusal is the same
        // test with the answer turned over — not a different one.
        let one = [Arm::Char('a')];

        assert_eq!(
            class_pred(&one).to_string().replace(' ', ""),
            "|c:char|matches!(c,'a')"
        );
        assert_eq!(
            refused_pred(&one).to_string().replace(' ', ""),
            "|c:char|!(matches!(c,'a'))"
        );

        // A range of arms is joined by `|`, and the parameter is read, not
        // ignored, as soon as there is anything to compare.
        let two = [Arm::Char('a'), Arm::Range('0', '9')];
        assert_eq!(
            class_pred(&two).to_string().replace(' ', ""),
            "|c:char|matches!(c,'a'|'0'..='9')"
        );
    }

    #[test]
    fn whitespace_is_one_test_of_a_character() {
        let soup = code("whitespace W -> [' ', '\\t'];");

        assert!(soup.contains("fn is_w (c : char) -> bool"));
        assert!(soup.contains("# [parserc (take_while = is_w)] pub struct W < I > (pub I)"));
    }

    #[test]
    fn whitespace_that_denotes_no_class_is_refused() {
        let file = parse("whitespace W -> <XID_Start>;");
        let error = Builder::new(&file).build().unwrap_err();

        assert!(matches!(
            error,
            UnsynError::Build(BuildKind::Whitespace(rule), _) if rule == "W"
        ));
    }

    #[test]
    fn a_shape_of_the_tree_no_field_can_hold_is_refused() {
        // A call of an external, a set of more than one character's denoting,
        // and a counted bound have no type of `parserc` to stand in a group
        // of the tree, and the file that writes one is told so.
        let file = parse("lexer A -> 'a'; syntax B -> A{2};");
        let error = Builder::new(&file).build().unwrap_err();

        assert!(matches!(
            error,
            UnsynError::Build(BuildKind::Syntax(rule), _) if rule == "B"
        ));
    }

    #[test]
    fn a_name_no_item_can_wear_is_refused() {
        let file = parse("lexer _ -> 'a';");
        let error = Builder::new(&file).build();
        assert!(matches!(
            error,
            Err(UnsynError::Build(BuildKind::Name(_), _))
        ));
    }

    #[test]
    fn a_raw_name_is_written_as_the_item_it_names() {
        let soup = code("lexer r#type -> 'a';");

        assert!(soup.contains("pub struct r#type < I >"));
    }

    #[test]
    fn a_group_inside_a_rule_is_named_after_the_rule() {
        let soup = code("lexer Abc -> ('a' 'b') | ('c' 'd');");

        assert!(soup.contains("fn read_abc_1"));
        assert!(soup.contains("fn read_abc_2"));
    }

    #[test]
    fn a_bound_offers_no_word_for_its_variant() {
        // `{2}` is punctuation and not a word, so the alternative that carries
        // it names its variant nothing — which is what used to hand the
        // generator a name no item could wear and refuse the whole build with
        // `BuildKind::Name`.
        //
        // It is not told, though: the alternative is one operand, so its place
        // in the choice says everything a name would.
        let soup = code("lexer A -> 'x'{2} | 'y';");

        assert!(soup.contains("pub enum A"), "{soup}");
        assert!(
            soup.contains("F1"),
            "the bounded alternative is called by its place"
        );
        assert!(soup.contains("Y"), "the word still names its variant");

        assert_eq!(
            semantic::analyze(&parse("lexer A -> 'x'{2} | 'y';"))
                .lints
                .iter()
                .filter(|lint| lint.kind == LintKind::UnnamedAlternative)
                .count(),
            0,
            "an alternative of one operand keeps its place to itself"
        );
    }

    #[test]
    fn two_alternatives_that_want_one_name_do_not_both_wear_it() {
        // Two alternatives that reach the same rule are a choice the second of
        // which has no name left. Written as the same name twice the enum would
        // not be Rust at all, so the second is called by its place — and the
        // file is told, since a variant called `F2` is a part of the tree that
        // says nothing.
        let soup = code("lexer X -> 'x'; lexer A -> X | X;");
        assert!(
            soup.contains("X (X < I >) , F2 (X < I >)"),
            "the second alternative wears its place: {soup}"
        );

        assert_eq!(
            semantic::analyze(&parse("lexer X -> 'x'; lexer A -> X | X;"))
                .lints
                .iter()
                .filter(|lint| lint.kind == LintKind::UnnamedAlternative)
                .count(),
            1,
            "the alternative left without a name is told"
        );
    }

    #[test]
    fn a_name_written_raw_and_one_written_plain_are_one_name() {
        // `r#gen` and `gen` are one name to the file: the sigil says how the
        // name is spelled, not which name it is. A second definition of one
        // name is a collision, and the collision is reported whatever sigil
        // the second of the two wears.
        let tree = {
            let mut input = Spec::from("lexer gen -> 'a'; lexer r#gen -> 'b';");

            input
                .parse::<syntax::Crate<_>>()
                .expect("two names is a file of the language")
        };
        let report = semantic::analyze(&tree);

        assert!(
            matches!(
                report.errors.as_slice(),
                [UnsynError::Semantics(SemanticsKind::NameCollision(_), _)]
            ),
            "one name written two ways is still defined twice: {:?}",
            report.errors
        );

        // And the item is written with the sigil it was written with, which is
        // the only way Rust has of naming an item `gen`. What the visitor reads
        // back out of that spelling is the name bare — and reading it back
        // through `Ident::new`, which refuses a name wearing a sigil, is the
        // panic this pins.
        let soup = code("lexer r#gen -> 'a';");
        assert!(soup.contains("pub struct r#gen"), "{soup}");
        assert!(
            soup.contains("fn visit_gen"),
            "the visitor of a raw name is the name bare: {soup}"
        );
    }

    #[test]
    fn the_code_of_a_file_of_the_language_is_well_formed_rust() {
        let soup = terminals(include_str!("../../unsyn.us"));

        let file = syn::parse_str::<syn::File>(&soup)
            .unwrap_or_else(|error| panic!("the generated code does not parse: {error}"));

        let items = file.items.len();

        assert!(
            items > 40,
            "the file should be all of its rules, not {items} items"
        );
    }
}
