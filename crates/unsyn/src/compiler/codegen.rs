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
//! # Where the generated code differs from this crate's own
//!
//! [`lexer`] is written by hand and is tuned where a generated file
//! is merely correct. The differences that remain are deliberate:
//!
//! - [`BlockCommentOrDoc`](super::lexer::BlockCommentOrDoc) is an enum of the three
//!   rules it names, where the generator writes it as the text it read: the hand-
//!   written rule reaches it from among the alternatives of `BlockComment`, where
//!   the two sides of a `.or()` have to answer with one type — and the generator
//!   lets that question decide, while the hand file answers it by naming the
//!   node anyway.
//! - The hand file gives the parts of a rule names that say what they hold —
//!   `open`, `head`, `tail` — and names the unnamed choices of a right side as
//!   rules of their own (`LitStrContent`, `BlockCommentTail`), which is what the
//!   [`UnnamedAlternative`](super::errors::LintKind::UnnamedAlternative) lint
//!   asks the author of a file to do in the spec. The generator has only the
//!   file to go by, and says `f1`, `f2`, and the rule's own name.
//! - A refusal the generated code reports is a [`Kind`](parserc::Kind) of the
//!   combinator that met it, where the hand-written file names what it expected
//!   with its own [`UnsynError`] variants. Naming them
//!   is a table the caller of a build would have to supply, in the same way that
//!   it supplies the externals.
//! - A `LitUnicode` is read as the four digits its rule asks for and no more, so
//!   a fifth is left to the rule that encloses it. The hand-written reader takes
//!   every digit it is offered and refuses the literal; a file that says anything
//!   else is already refused by the analyzer, and both readings accept the same
//!   files.
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
//! that spells no value, and any `syntax` rule, whose nodes are trees rather than
//! text and are not generated yet. The precondition of [`Builder::build`] is that
//! [`analyze`](super::semantic::analyze) has already found nothing in the file;
//! what these refusals guard is what a clean file can still ask for that no code
//! answers, and they are the last line rather than the first.

use std::collections::{HashMap, HashSet};

use parserc::{Span, syntax::Syntax};
use proc_macro2::{Ident, Literal, TokenStream};
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

        let mut generator = Generator {
            file: self.file,
            external: &self.external,
            denotations: &denotations,
            nodes: &nodes,
            taken: HashSet::new(),
            helpers: Vec::new(),
            seq: 0,
            field_no: 0,
            field_names: HashSet::new(),
            rule: String::new(),
        };

        generator.crate_of()
    }
}

/// How many times something is read, written as a number that is free to take the
/// type of the loop or the range it bounds.
fn count(n: usize) -> Literal {
    Literal::usize_unsuffixed(n)
}

/// The key an external is bound under: its name, of the sigil a raw name wears.
fn external_key(name: &str) -> String {
    name.strip_prefix("r#").unwrap_or(name).to_owned()
}

/// The alternatives of a rule that is nothing but a choice of other rules by name.
///
/// What such a rule says is *which* of the rules it names was read, and the text
/// it took does not say that: `r#foo` read raw and `foo` read plain are both names,
/// and only the alternative they came through tells them apart. The rule therefore
/// comes out as a node with one part per name rather than as the text it read,
/// which is what [`named_choice_rules`] collects the names of.
///
/// Anything else is not that shape, and `None` says so: an alternative that is a
/// sequence, a single operand that is a literal or a set, and a choice of one are
/// all rules whose text is all they are.
fn named_choice<I>(body: &syntax::Expr<I>) -> Option<Vec<&lexer::Ident<I>>>
where
    I: UnsynInput,
{
    let mut names = Vec::new();

    for branch in body.branches() {
        let mut elements = branch.elements();

        let (Some(one), None) = (elements.next(), elements.next()) else {
            return None;
        };

        let syntax::ExprNoTopAlt::WithoutSuffix(syntax::ExprWithoutSuffix::Reference(ident)) = one
        else {
            return None;
        };

        names.push(ident);
    }

    (names.len() >= 2).then_some(names)
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
/// is read through until the two sets settle, since one rule written as text
/// reads a choice whose other parts then have to be text too, and what is
/// settled for one rule is a fact about the file rather than about the order
/// the file declares its rules in.
fn node_rules<I>(file: &syntax::Crate<I>) -> HashSet<String>
where
    I: UnsynInput,
{
    let mut nodes = HashSet::new();
    let mut sequences = HashSet::new();

    for item in &file.items {
        let syntax::Item::Stmt(stmt) = item else {
            continue;
        };

        if !matches!(stmt.decl(), Decl::Lexer) {
            continue;
        }

        let key = external_key(stmt.ident.text().as_str());

        if named_choice(&stmt.definition.body).is_some() {
            nodes.insert(key);
        } else if is_sequence(&stmt.definition.body) {
            sequences.insert(key.clone());
            nodes.insert(key);
        }
    }

    loop {
        let mut forced = HashSet::new();

        for item in &file.items {
            let syntax::Item::Stmt(stmt) = item else {
                continue;
            };

            if !matches!(stmt.decl(), Decl::Lexer) {
                continue;
            }

            // A rule written as text reads its whole answer back as text, and
            // a rule written as a sequence reads the parts it names back as
            // parts: in both, every choice the body folds, every set the body
            // lists as a choice of characters, and every removal written over
            // a name asks that name for the text it read. A rule written as an
            // enum of names is the one body that asks for nothing: its parts
            // are the names, and no text of them is read back.
            let key = external_key(stmt.ident.text().as_str());
            let body = &stmt.definition.body;

            if nodes.contains(&key) && !sequences.contains(&key) {
                continue;
            }

            // A rule whose whole body is one name answers with that name, and
            // a rule that is not written as a node answers with text: the
            // name it reads has to answer with text too.
            if body.is_single()
                && let Some(element) = body
                    .branches()
                    .next()
                    .expect("one branch")
                    .elements()
                    .next()
                && matches!(
                    element,
                    syntax::ExprNoTopAlt::WithoutSuffix(syntax::ExprWithoutSuffix::Reference(_))
                )
            {
                operand_forced(element, true, &mut forced);
            }

            expr_forced(body, false, &mut forced);
        }

        let before = nodes.len();
        nodes.retain(|rule| !forced.contains(rule));

        if nodes.len() == before {
            return nodes;
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
            .is_some_and(|branch| branch.elements().count() > 1)
}

/// The names whose text is asked for by `expr`: by the alternatives it folds,
/// by the members of the sets it lists, and by the operand a removal is written
/// over. A name read as the whole answer of a rule or of an operand is not
/// among them; that answer keeps its shape.
/// The names whose answer flows as text wherever `expr` is read as one
/// answer: the alternatives it folds, the member of a set read as a choice of
/// characters, the group whose whole answer is one name, and the operand a
/// removal is written over.
fn expr_forced<I>(expr: &syntax::Expr<I>, folds: bool, forced: &mut HashSet<String>)
where
    I: UnsynInput,
{
    // The alternatives of a pattern that has more than one of them answer a
    // fold, and a fold asks one shape of every part that reaches it.
    let folds = folds || !expr.is_single();

    for branch in expr.branches() {
        for element in branch.elements() {
            operand_forced(element, folds, forced);
        }
    }
}

fn operand_forced<I>(operand: &syntax::ExprNoTopAlt<I>, folds: bool, forced: &mut HashSet<String>)
where
    I: UnsynInput,
{
    match operand {
        syntax::ExprNoTopAlt::WithoutSuffix(atom) => atom_forced(atom, folds, forced),
        syntax::ExprNoTopAlt::WithSuffix(suffix) => match suffix {
            syntax::ExprWithSuffix::Grouped { target, .. } => atom_forced(target, folds, forced),
            syntax::ExprWithSuffix::Concat {
                target,
                suffix: next,
                ..
            } => {
                atom_forced(target, folds, forced);
                operand_forced(next, folds, forced);
            }
            // What follows is only probed, and its answer thrown away: the
            // probe asks the shape of the answer of nothing. What a removal is
            // written over is the one operand whose text is read back.
            syntax::ExprWithSuffix::Followed { target, .. } => atom_forced(target, folds, forced),
            syntax::ExprWithSuffix::Except { target, tokens, .. } => {
                if let syntax::ExprWithoutSuffix::Reference(ident) = target {
                    forced.insert(external_key(ident.text().as_str()));
                }

                atom_forced(tokens, folds, forced);
            }
        },
    }
}

fn atom_forced<I>(atom: &syntax::ExprWithoutSuffix<I>, folds: bool, forced: &mut HashSet<String>)
where
    I: UnsynInput,
{
    match atom {
        // A name read alone hands back its whole answer. Read among the parts
        // of a fold, the fold asks that answer of every part as one thing, and
        // the thing a fold of text answers with is text; read on its own, as
        // the field of a struct or the target of a probe that throws the
        // answer away, the name keeps whichever shape it was written in.
        syntax::ExprWithoutSuffix::Reference(ident) if folds => {
            forced.insert(external_key(ident.text().as_str()));
        }
        // A group whose whole answer is one name hands that name's answer
        // back, and a group is read back as text wherever it stands: a field
        // of `I`, an operand the code of a rule folds. What a group reads
        // besides — a sequence, a choice of its own — it answers with its
        // reading, and the names inside answer only to that.
        syntax::ExprWithoutSuffix::Paren(group) => {
            if group
                .body
                .branches()
                .next()
                .is_some_and(|branch| branch.elements().count() == 1)
                && group.body.is_single()
                && let Some(syntax::ExprNoTopAlt::WithoutSuffix(
                    syntax::ExprWithoutSuffix::Reference(ident),
                )) = group
                    .body
                    .branches()
                    .next()
                    .expect("one element")
                    .elements()
                    .next()
            {
                forced.insert(external_key(ident.text().as_str()));
            } else {
                expr_forced(&group.body, false, forced);
            }
        }
        // A set of more than one member is a choice of its members, and a
        // choice answers with one thing however wide the members are.
        syntax::ExprWithoutSuffix::Set(set) | syntax::ExprWithoutSuffix::Tilde(_, set)
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
                bodies.push((stmt.ident.text(), &stmt.definition.body));
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
        syntax::ExprWithoutSuffix::Paren(group) => expr_arms(&group.body, known),
        syntax::ExprWithoutSuffix::Reference(ident) => known.get(ident.text().as_str()).cloned(),
        // A refusal of a set is every character the universe holds besides it,
        // which is no set to list, whatever the set itself denotes.
        syntax::ExprWithoutSuffix::Tilde(..) => None,
        // An external is whatever the caller binds, and nothing here may say more
        // of it than that.
        syntax::ExprWithoutSuffix::Call(..) => None,
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

/// The test of a character that a class of these arms is.
fn pred_body(arms: &[Arm]) -> TokenStream {
    if arms.is_empty() {
        // A class of nothing, which no character passes, and which a `matches!` of
        // no arms cannot even be written as.
        return quote! { false };
    }

    let arms = arms.iter().copied().map(Arm::to_pattern);
    quote! { matches!(c, #(#arms)|*) }
}

/// The closure a repetition of a class of these arms asks its items to fit.
fn class_pred(arms: &[Arm]) -> TokenStream {
    let body = pred_body(arms);

    match arms.is_empty() {
        true => quote! { |_c: char| false },
        false => quote! { |c: char| #body },
    }
}

/// The closure a repetition of what a class of these arms refuses asks its items
/// to fit.
fn refused_pred(arms: &[Arm]) -> TokenStream {
    let body = pred_body(arms);

    match arms.is_empty() {
        true => quote! { |_c: char| true },
        false => quote! { |c: char| !(#body) },
    }
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
    /// The names of the functions the module has been given already.
    taken: HashSet<String>,
    /// The functions the rule being written has asked for, in the order asked.
    helpers: Vec<TokenStream>,
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
        let span = stmt.ident.to_span();
        let name = item_name(&text, span.clone())?;
        let mut allow = wants_allow(&text);

        self.rule = text.clone();
        self.seq = 0;
        self.helpers.clear();
        self.field_no = 0;
        self.field_names.clear();

        // A rule that is a choice of names is a node, and the names it chooses
        // between were read on their own: nothing is asked of the body here, and
        // the part of each alternative is the node that alternative names. Which
        // rules are of that shape was settled over the whole file first, since a
        // rule asked for among the parts of another is not written as a node.
        if self.nodes.contains(&external_key(&text))
            && let Some(names) = named_choice(&stmt.definition.body)
        {
            let mut variants = Vec::new();

            for ident in names {
                let name = ident.text();
                let variant = item_name(&name, ident.to_span())?;
                allow |= wants_allow(&name);
                variants.push(variant);
            }

            let allow = allow.then(|| quote! { #[allow(non_camel_case_types)] });
            let enum_item = quote! {
                #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, ::parserc::syntax::Syntax)]
                #allow
                pub enum #name<I>
                where
                    I: ::parserc::chars::CharsInput,
                {
                    #( #variants ( #variants<I> ), )*
                }
            };

            let mut items = std::mem::take(&mut self.helpers);
            items.push(enum_item);

            return Ok(items);
        }

        let allow = allow.then(|| quote! { #[allow(non_camel_case_types)] });

        let code = match stmt.decl() {
            Decl::Syntax => return Err(UnsynError::Build(BuildKind::Syntax(text), span)),
            Decl::Whitespace => self.whitespace(&text, stmt)?,
            Decl::Lexer => self.lexer_rule(&text, stmt)?,
        };

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

                    // A struct with braces settles its bounds before them.
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
        });

        Ok(items)
    }

    /// The predicate of a `whitespace` rule, which is the one rule the derive of
    /// the syntax asks a function of a character rather than a parser.
    fn whitespace(&mut self, text: &str, stmt: &syntax::Stmt<I>) -> Result<Rule, UnsynError> {
        let arms = expr_arms(&stmt.definition.body, self.denotations).ok_or_else(|| {
            UnsynError::Build(
                BuildKind::Whitespace(text.to_owned()),
                stmt.definition.body.to_span(),
            )
        })?;

        let pred = self.unique(format!("is_{}", to_snake_case(text)));
        let body = pred_body(&arms);
        let arg = if arms.is_empty() {
            quote! { _c }
        } else {
            quote! { c }
        };

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
        let body = &stmt.definition.body;

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
        let mut fields = Vec::new();

        for element in branch.elements() {
            fields.extend(self.field_of(element)?);
        }

        // A struct of nothing but nameless operands is a tuple of them: where
        // they stand says what they are, and a name would only say it twice.
        // Where a name reaches in, every other field takes its number.
        let tuple = fields.iter().all(|field| field.name.is_none());

        if !tuple {
            for field in &mut fields {
                if field.name.is_none() {
                    field.name = self.anonymous_name();
                }
            }
        }

        Ok(Rule::Fields { tuple, fields })
    }

    /// The fields one operand of a sequence is written as.
    ///
    /// Most operands are one field. A `concat` is the two operands it joins,
    /// which at this level is what a sequence of them already is.
    fn field_of(&mut self, operand: &syntax::ExprNoTopAlt<I>) -> Result<Vec<Field>, UnsynError> {
        match operand {
            syntax::ExprNoTopAlt::WithoutSuffix(atom) => Ok(vec![self.atom_field(atom, None)?]),
            syntax::ExprNoTopAlt::WithSuffix(suffix) => match suffix {
                syntax::ExprWithSuffix::Grouped { target, suffix } => {
                    let bounds = match suffix {
                        syntax::Suffix::Star(_) => (0, None),
                        syntax::Suffix::Question(_) => (0, Some(1)),
                        syntax::Suffix::Plus(_) => (1, None),
                        syntax::Suffix::Repeat(repeat) => self.bounds(&repeat.0.body)?,
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

            let (field_ty, attr) = match bounds {
                None => (quote! { #ty<I> }, None),
                Some((0, None)) => (quote! { ::std::vec::Vec<#ty<I>> }, None),
                Some((0, Some(1))) => (quote! { ::core::option::Option<#ty<I>> }, None),
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
            syntax::ExprWithoutSuffix::Tilde(_, set) => {
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
                    let Some(one) = #each else { break };
                    taken.push(one);
                }
            },
        };

        let rule = self.rule.clone();

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
                if taken.len() < #at_least {
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

    // -- expressions and statements -----------------------------------------

    /// The code of a pattern: the alternatives of it tried in the order written.
    fn code_of_expr(&mut self, expr: &syntax::Expr<I>) -> Result<Code, UnsynError> {
        let mut branches = Vec::new();

        for branch in expr.branches() {
            branches.push(self.code_of_branch(branch)?);
        }

        if branches.len() == 1 {
            return Ok(branches.pop().expect("one branch counted twice"));
        }

        let mut choice = branches.into_iter();
        let mut parser = self.expr_of(choice.next().expect("a pattern has a branch"))?;

        for branch in choice {
            let other = self.expr_of(branch)?;
            parser = quote! { #parser.or(#other) };
        }

        Ok(Code::Expr(parser))
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
            syntax::ExprWithoutSuffix::Call(call) => {
                let name = call.body.text();

                let parser = self.external.get(&external_key(&name)).ok_or_else(|| {
                    UnsynError::Build(
                        BuildKind::External(self.rule.clone(), name.clone()),
                        call.body.to_span(),
                    )
                })?;

                Ok(Code::Expr(parser.clone()))
            }
            syntax::ExprWithoutSuffix::Set(set) => Ok(Code::Expr(self.code_of_set(set)?)),
            syntax::ExprWithoutSuffix::Paren(group) => self.code_of_expr(&group.body),
            syntax::ExprWithoutSuffix::Tilde(_, set) => {
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
            syntax::ExprWithSuffix::Grouped { target, suffix } => {
                let (at_least, at_most) = match suffix {
                    syntax::Suffix::Star(_) => (0, None),
                    syntax::Suffix::Question(_) => (0, Some(1)),
                    syntax::Suffix::Plus(_) => (1, None),
                    syntax::Suffix::Repeat(repeat) => self.bounds(&repeat.0.body)?,
                };

                self.code_of_repetition(target, at_least, at_most)
            }
            // `concat` joins two operands with nothing between them, and a node of
            // a `lexer` rule owns no trivia, so it is what a sequence of two
            // operands already is.
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
            syntax::ExprWithoutSuffix::Tilde(_, set) => {
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
            syntax::RepeatBound::RangeTo(_, to) => (0, Some(self.count_of(to)?)),
            syntax::RepeatBound::RangeFrom(from, _, to) => (
                self.count_of(from)?,
                to.as_ref().map(|to| self.count_of(to)).transpose()?,
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
fn item_name(text: &str, span: Span) -> Result<Ident, UnsynError> {
    let base = text.strip_prefix("r#").unwrap_or(text);
    let mut chars = base.chars();

    let named = match chars.next() {
        // A lone underscore is a name the reader accepts and no item can wear.
        None => false,
        Some(first) => {
            !(base == "_") && (first == '_' || is_xid_start(first)) && chars.all(is_xid_continue)
        }
    };

    if !named {
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
    let body = &set.body;

    body.pairs
        .iter()
        .map(|(item, _)| item)
        .chain(body.tail.iter().map(|tail| &**tail))
        .collect()
}

#[cfg(test)]
mod tests {
    use parserc::{Input, syntax::SyntaxInput};

    use super::*;
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
    fn a_choice_of_names_is_written_as_a_node() {
        let soup = code("lexer A -> 'a'; lexer B -> 'b'; lexer C -> A | B;");

        assert!(soup.contains("pub enum C < I > where I : :: parserc :: chars :: CharsInput , { A (A < I >) , B (B < I >) , }"));
    }

    #[test]
    fn a_name_read_among_others_is_written_as_the_text_it_read() {
        // `C` is a choice of names, but `D` reads `C` beside a word of its own,
        // where the two alternatives have to answer with the same thing.
        let soup =
            code("lexer A -> 'a'; lexer B -> 'b'; lexer C -> A | B; lexer D -> ( C | 'z' ) 'x';");

        assert!(!soup.contains("pub enum C"));
        assert!(has(
            &soup,
            quote! { <C<I> as ::parserc::syntax::Syntax<I>>::into_parser().map(|node| node.0) }
        ));
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
    fn a_joining_is_what_a_sequence_already_is() {
        let soup = code("lexer A -> 'ab' concat 'cd';");

        // Nothing stands between the two operands, which is what reading one after
        // the other already is.
        assert!(soup.contains(":: parserc :: keyword (\"ab\") . parse (input) ? ; :: parserc :: keyword (\"cd\") . parse (input) ?"));
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

        // A rule that is one expression keeps the text of the expression it is.
        let soup = code("lexer A -> 'a' | 'b';");
        assert!(has(
            &soup,
            quote! { ::parserc::next('a').or(::parserc::next('b')) }
        ));
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
    fn a_syntax_rule_is_refused_until_its_nodes_are_written() {
        let file = parse("lexer A -> 'a'; syntax B -> A;");
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
