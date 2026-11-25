//! What a file can get wrong once it reads well enough to be parsed.
//!
//! The reader answers whether the text is a file of the language. This pass
//! answers whether the file denotes a grammar, which is a question no single node
//! can answer on its own: whether a name means anything here depends on the rest
//! of the file, whether a member of a set is worth writing depends on the members
//! before it, and whether a definition can be matched at all depends on what
//! following it would ask of the same position again.
//!
//! # What is reported, and on which channel
//!
//! [`SemanticsKind`]s say that the file denotes no model, and are collected into
//! [`Diagnostics::errors`]; [`LintKind`]s say that a file which does denote one
//! says something twice, or says something that cannot happen, and are collected
//! into [`Diagnostics::lints`]. A caller that would rather refuse such a file is
//! free to treat a lint as fatal; nothing here decides that.
//!
//! A value that is well formed but denotes nothing — `\x8F`, `U+D800`, an escape
//! of no character — is reported by the reader, which is the pass that has the
//! text in front of it. Only `U+D800` and its like are left to this pass, since
//! four hex digits is a shape the reader can check and a surrogate is a meaning
//! only the code point can be asked for.
//!
//! # How the pass is built
//!
//! The pass is a [`Visit`], and the only source of truth it reads is the tree.
//! Every method it overrides walks its node first and then takes back what the
//! children left, so the analysis is post-order and nothing is judged before it
//! is known:
//!
//! - each part of a pattern leaves exactly one `Pattern` on a stack, which says
//!   what that part can begin a match with, whether it can match nothing at all,
//!   and which names it mentions;
//! - each member of a set leaves one `Member`, each range one `Ends` and each
//!   repetition one `Bound`, on stacks of their own, since what they carry is a
//!   value and not a shape;
//! - each definition finishes into a `Rule`, and the file as a whole is judged
//!   when the last of them has, which is what [`Visit::visit_crate`] is for.
//!
//! A method takes back as many entries as its node has operands that leave one.
//! That number comes from the node the method was handed, and the one construct
//! the grammar writes without naming is `X+`, the sequence: no production has it
//! on its left side, so no method is called for it, and the operands of an
//! alternative can only be counted from the alternative. Everything else the pass
//! reads of a node is a value the node holds in its parts — a literal decoded
//! from its digits and escapes — which no walk carries to a visitor either.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use parserc::{Span, ToSpan, as_str};

use super::errors::{Diagnostics, LintKind, SemanticsKind, UnsynError};
use super::input::UnsynInput;
use super::visit::{Visit, walk};
use super::{codegen, lexer, read, syntax};
use crate::compiler::decode;

/// The kind of definition a statement opens with, which is a reading of the tree
/// and not a part of it (see [`read`]).
use super::read::Decl;

/// Reports what `file` gets wrong beyond the shape of its text.
///
/// The file is read as a whole, and so is reported as a whole: a definition is
/// not dropped at the first name that means nothing, since what the rest of the
/// file says is what the author meant around it.
pub fn analyze<'a, I>(source: &'a I, file: &syntax::Crate<I>) -> Diagnostics
where
    I: UnsynInput + 'a,
{
    // The rules whose answers are characters, one apiece: the names a set
    // may reach without saying more than a character — and every name the
    // file defined, which the mention check alone speaks for.
    let char_rules = codegen::denotations(source, file).into_keys().collect();
    let skeletons: Vec<Skeleton> = file
        .item
        .iter()
        .filter_map(|item| match item {
            syntax::Item::Stmt(stmt) => Some(Skeleton {
                key: codegen::external_key(as_str(source, &stmt.ident)),
                nullable: writes_nothing(&stmt.expr),
                bare: bare_name(&stmt.expr, source).map(codegen::external_key),
            }),
            _ => None,
        })
        .collect();
    let empty = definitions_matched_by_nothing(&skeletons);
    let defined = skeletons
        .iter()
        .map(|skeleton| skeleton.key.clone())
        .collect();

    let mut checker = Checker::new(source, char_rules, defined, empty);
    checker.visit_crate(file);

    debug_assert!(
        checker.stack.is_empty()
            && checker.members.is_empty()
            && checker.ranges.is_empty()
            && checker.bounds.is_empty(),
        "a whole file should leave nothing on the stacks"
    );

    checker.diagnostics
}

/// The two facts about a definition that say whether it can be matched by
/// nothing, read off the file before any pattern has been assembled.
struct Skeleton {
    /// The name, sigil folded.
    key: String,
    /// Whether the definition writes a quantity of its own — a `?`, a `*`, a
    /// bound that may be zero — which is settled by the pattern's shape and by
    /// nothing the rest of the file has to say.
    nullable: bool,
    /// The one name the definition is written as, when it is nothing but one.
    bare: Option<String>,
}

/// The names of the definitions that can be matched by nothing.
///
/// A definition is one when it writes a nothing of its own, or when it is
/// written as nothing but a name that is one — which is why this is told by
/// repeating until nothing new answers, the order the file wrote its rules in
/// meaning nothing here.
///
/// This is asked of the file before it is walked, because a set is judged as it
/// is read and has to be told which of its members answer with nothing. What is
/// left out of the answer is a definition reached through a sequence that only
/// begins with a nothing — `B -> 'b'? C;` — since the shape of a whole pattern
/// is not known until the file has been walked. A rule left out that way is no
/// character either, and `WideSetMember` is the one that says of it.
fn definitions_matched_by_nothing(skeletons: &[Skeleton]) -> BTreeSet<String> {
    let mut empty: BTreeSet<String> = skeletons
        .iter()
        .filter(|skeleton| skeleton.nullable)
        .map(|skeleton| skeleton.key.clone())
        .collect();

    loop {
        let mut grew = false;

        for skeleton in skeletons {
            let reached = skeleton
                .bare
                .as_deref()
                .is_some_and(|bare| empty.contains(bare));

            if reached && !empty.contains(&skeleton.key) {
                empty.insert(skeleton.key.clone());
                grew = true;
            }
        }

        if !grew {
            return empty;
        }
    }
}

// ---------------------------------------------------------------------------
// What a pattern is written with
// ---------------------------------------------------------------------------

/// Whether the expression can be matched by nothing at all.
///
/// This is read off the shape of what the file wrote, without reaching past any
/// name: only a quantity the expression itself carries answers here, since every
/// other operand takes at least one character and a sequence of several is
/// matched by nothing never.
///
/// A plain function and not a method of the node, which is what a build writes
/// with no opinion of its own about what its types mean.
fn writes_nothing<I: UnsynInput>(expr: &syntax::Expr<I>) -> bool {
    read::is_single(expr)
        && read::branches(expr).any(|branch| read::elements(branch).any(alt_writes_nothing))
}

/// Whether one part of a sequence can be matched by nothing.
///
/// An operand standing alone always consumes — that is what every operator of
/// the language holds to — so only a suffix that bounds nothing says so here.
/// `~X` is no exception written the other way: a suffix beside a `~` stands on
/// the refusal and not under it, so `~X*` is the refusal repeated, which is the
/// whole of a sequence once more.
fn alt_writes_nothing<I: UnsynInput>(alt: &syntax::ExprNoTopAlt<I>) -> bool {
    match alt {
        syntax::ExprNoTopAlt::ExprWithoutSuffix(_) => false,
        syntax::ExprNoTopAlt::ExprWithSuffix(with_suffix) => match with_suffix {
            syntax::ExprWithSuffix::Suffixed { suffix, .. } => suffix_may_be_zero(suffix),
            syntax::ExprWithSuffix::Concat { .. } | syntax::ExprWithSuffix::Except { .. } => false,
        },
    }
}

/// Whether the suffix is satisfied by matching the operand no times at all.
fn suffix_may_be_zero<I: UnsynInput>(suffix: &syntax::Suffix<I>) -> bool {
    match suffix {
        syntax::Suffix::Star(_) | syntax::Suffix::Question(_) => true,
        syntax::Suffix::Plus(_) => false,
        syntax::Suffix::Repeat(repeat) => bound_may_be_zero(&repeat.repeat_bound),
    }
}

/// Whether the bound asks for the operand no fewer than no times.
///
/// `{,n}` asks it and `{0,n}` asks it; `{n}` asks it only when `n` is `0`, which
/// is a bound the analyzer refuses for another reason.
fn bound_may_be_zero<I: UnsynInput>(bound: &syntax::RepeatBound<I>) -> bool {
    match bound {
        syntax::RepeatBound::RangeTo(..) => true,
        syntax::RepeatBound::RangeFrom(from, ..) => {
            matches!(decode::decimal(from), Some(0))
        }
        syntax::RepeatBound::LitDec(count) => matches!(decode::decimal(count), Some(0)),
    }
}

/// The one name the expression is written as, when it is nothing but one.
///
/// A definition written as one name is matched by nothing exactly when the rule
/// it names is, which is what lets the nothing a file writes travel from a
/// definition to the names that stand for it.
fn bare_name<'s, I: UnsynInput>(expr: &'s syntax::Expr<I>, source: &'s I) -> Option<&'s str> {
    if !read::is_single(expr) || read::elements(&expr.first).nth(1).is_some() {
        return None;
    }

    let syntax::ExprNoTopAlt::ExprWithoutSuffix(syntax::ExprWithoutSuffix::Ident(ident)) =
        &expr.first.first
    else {
        return None;
    };

    Some(as_str(source, ident))
}

// ---------------------------------------------------------------------------
// What a part of a pattern can do
// ---------------------------------------------------------------------------

/// What one part of a definition's pattern can do, which is all this pass needs
/// of it.
struct Pattern {
    /// The nodes a match of this part can begin with.
    ///
    /// Only a mention of a node by name begins with that node: a literal, a set,
    /// a call of an external and a range all begin with a character, which is no
    /// name of the file. What a part can begin with is what says whether reaching
    /// it asks the same question again.
    heads: BTreeSet<String>,
    /// Whether the part can match nothing at all.
    nullable: bool,
    /// Whether the part is exactly one name mentioned once.
    ///
    /// This is what lets a mention stand for the whole of the alternative that
    /// offers it. A suffix, a keyword, a group or a set around the name makes the
    /// part something more than the mention, and the mention then says what it
    /// says alongside something else.
    bare: bool,
    /// The names the part mentions, each where it was written.
    uses: Vec<Use>,
}

impl Pattern {
    /// A part written as nothing but one name, which is what a mention of a node
    /// in a pattern always is.
    fn bare(name: String, span: Span) -> Self {
        let mut heads = BTreeSet::new();
        heads.insert(name.clone());

        Self {
            heads,
            nullable: false,
            bare: true,
            uses: vec![Use {
                name,
                span,
                alone: false,
                mention: Mention::Plain,
            }],
        }
    }

    /// A part that takes at least one character and begins with no node of the
    /// file: a literal, a set, or a call of an external.
    fn terminal() -> Self {
        Self {
            heads: BTreeSet::new(),
            nullable: false,
            bare: false,
            uses: Vec::new(),
        }
    }

    /// Nothing, which is what the stack holds where the tree left nothing.
    ///
    /// Only a tree this pass did not read can do that, and reaching for the
    /// stack rather than panicking keeps a broken tree a report rather than a
    /// crash.
    fn nothing() -> Self {
        Self::terminal()
    }

    /// `'a 'b 'c'`: the operands of a sequence, each reached from where the one
    /// before it ended.
    ///
    /// A sequence begins with what its first operand that must match begins with,
    /// so the operands before that one — every one of which can match nothing —
    /// add what they begin with too, and the operands after it add nothing.
    fn sequence(mut parts: Vec<Self>) -> Self {
        if parts.len() == 1 {
            return parts.pop().expect("one part of a sequence");
        }

        let mut heads = BTreeSet::new();
        let mut uses = Vec::new();
        let mut nullable = true;

        for part in parts {
            if nullable {
                heads.extend(part.heads);
            }

            nullable &= part.nullable;
            uses.extend(part.uses);
        }

        Self {
            heads,
            nullable,
            bare: false,
            uses,
        }
    }

    /// `'a' | 'b' | 'c'`: the alternatives of one expression, of which a match
    /// follows exactly one.
    fn alternative(mut parts: Vec<Self>) -> Self {
        if parts.len() == 1 {
            return parts.pop().expect("one alternative of an expression");
        }

        let mut heads = BTreeSet::new();
        let mut uses = Vec::new();
        let mut nullable = false;

        for part in parts {
            heads.extend(part.heads);
            nullable |= part.nullable;
            uses.extend(part.uses);
        }

        Self {
            heads,
            nullable,
            bare: false,
            uses,
        }
    }

    /// `left concat right`: the two operands matched as one, the second from
    /// where the first ended.
    ///
    /// The right operand is only reached where the left one may match nothing,
    /// which is what says whether a definition guarded by the left one is still
    /// guarded.
    fn joined(left: Self, right: Self) -> Self {
        let nullable = left.nullable && right.nullable;

        // The left side is in the answer either way, so the right is merged into
        // the set it already holds rather than a third set built from both.
        let mut heads = left.heads;
        if left.nullable {
            heads.extend(right.heads);
        }

        Self {
            heads,
            nullable,
            bare: false,
            uses: left.uses.into_iter().chain(right.uses).collect(),
        }
    }

    /// `left except right`: what the right side asserts is tested against the
    /// text the left side read, so it is never what a match begins with, although
    /// it stays a name the file has to mean.
    fn excepted(left: Self, right: Self) -> Self {
        Self {
            heads: left.heads,
            nullable: left.nullable,
            bare: false,
            uses: left.uses.into_iter().chain(right.uses).collect(),
        }
    }
}

/// Where a mention stands, which is what decides what its definition has to
/// answer with.
#[derive(Debug)]
enum Mention {
    /// Anywhere else: the file is only asked to have defined the name.
    Plain,
    /// A member of a set, which is one question of one character and so can
    /// hold nothing and nothing wider.
    SetMember,
    /// The operand of a refusal, probed at the position it would be matched at
    /// — which a definition matched by nothing always answers to.
    Operand,
}

/// A rule named in a pattern, and where it was named.
#[derive(Debug)]
struct Use {
    /// The name as it was written, assembled from the characters the name read.
    name: String,
    /// Where it was written.
    span: Span,
    /// Whether the name is the whole of the alternative that offers it, which for
    /// the trivia of the language is the only thing a mention of it can mean.
    alone: bool,
    /// Where the name stands, which says what its definition owes it.
    mention: Mention,
}

// ---------------------------------------------------------------------------
// What a member of a set denotes
// ---------------------------------------------------------------------------

/// One member of a set, read to the values it denotes.
struct Member {
    /// Where it was written, which is where a report about it stands.
    span: Span,
    /// What it denotes.
    value: Value,
}

/// The trivia of the language, as the one `whitespace` declaration named it.
#[derive(Clone)]
struct Trivia {
    /// The name, sigil folded.
    name: String,
    /// Where the declaration wrote the name, which is what a rule of nodes is
    /// pointed at when it names the trivia without needing to.
    span: Span,
}

/// What a member of a set denotes, as far as this pass can tell without reading
/// the definitions of the other members.
enum Value {
    /// The characters it denotes, as intervals of code points.
    Chars(Vec<(u32, u32)>),
    /// The strings of more than one character it denotes, which no character is
    /// ever the same value as.
    Strings(Vec<String>),
    /// A rule whose members are whatever its definition denotes.
    ///
    /// Only the name can be compared here. What the rule denotes is settled by
    /// the pass that reads the definitions together, which comes after this one,
    /// and a member of a set is worth as much as what it adds to it.
    Name(String),
    /// Nothing is known, because the member was already refused.
    Unknown,
}

/// The two ends of a range, once both have been read.
struct Ends {
    /// The interval the two ends spell, when each of them denotes exactly one
    /// character and the lower is not above the higher.
    interval: Option<(u32, u32)>,
}

/// The number of times a repetition asks for its operand.
struct Bound {
    /// How many times it must.
    from: u32,
    /// How many times it may, when the form writes a bound above.
    to: Option<u32>,
}

impl Bound {
    /// Whether the repetition is satisfied by matching nothing, which is all
    /// `{0,n}` and `{,n}` ask for.
    fn may_match_nothing(&self) -> bool {
        self.from == 0
    }
}

// ---------------------------------------------------------------------------
// What a file says
// ---------------------------------------------------------------------------

/// One definition the file wrote, and what its pattern came to.
struct Rule {
    /// The name the definition gives its node, assembled from what the name read.
    name: String,
    /// Which kind of node it declares.
    kind: Decl,
    /// Where the name was written.
    span: Span,
    /// The pattern written after the `->`.
    body: Pattern,
}

// ---------------------------------------------------------------------------
// The pass
// ---------------------------------------------------------------------------

/// The visitor that gathers the definitions of a file and then judges them.
struct Checker<'ast, I> {
    /// What the file has been found to do wrong.
    diagnostics: Diagnostics,
    /// The definitions the file has finished, in the order it wrote them.
    rules: Vec<Rule>,
    /// The trivia the file's `whitespace` declaration named, when it named one.
    trivia: Option<Trivia>,
    /// The rules whose definitions settle down to characters, one a piece.
    ///
    /// A set may name only these — a member that settles wider denotes no
    /// character to test, and a set is one test of one character however
    /// many members list.
    char_rules: HashSet<String>,
    /// The names of every definition the file wrote, sigil folded.
    defined: HashSet<String>,
    /// The names of the definitions that can be matched by nothing.
    ///
    /// Computed before the file is walked, and told through the names a
    /// definition reaches as well as through what it writes: both a set member
    /// and the operand of a refusal are asked for a character, and a definition
    /// matched by nothing has none to offer whoever names it however far away.
    empty: BTreeSet<String>,
    /// The patterns of the operands read but not yet joined into a larger one.
    stack: Vec<Pattern>,
    /// The members of a set read but not yet taken as an operand.
    members: Vec<Member>,
    /// The ends of a range read but not yet taken as a member.
    ranges: Vec<Ends>,
    /// The bounds of a repetition read but not yet applied to an operand.
    bounds: Vec<Bound>,
    /// How deep into the pattern of the definition being visited the pass stands.
    ///
    /// A pattern begins at depth `0`, which is the one depth at which an `|`
    /// names the node the definition declares; every deeper `|` is written
    /// inside a group and names nothing.
    expr_depth: usize,
    /// Whether the definition being visited is a node of the tree.
    ///
    /// A `lexer` rule is read as the one token it spells: what its pattern
    /// reaches is thrown away once it has bounded the run, and the rule's own
    /// name is what the tree says about it. A `syntax` rule is read as a node,
    /// whose every part the tree keeps and has to name. The difference decides
    /// whether a part of a pattern that reads no word costs the tree anything.
    holds_node: bool,
    /// The input the tree was read from, whole — not a piece of it. Every node
    /// holds a stream narrowed to its own token, so none of those reaches the
    /// text of a node around it; this one does, and is what any part of the pass
    /// that wants a node's written text reads it out of.
    source: &'ast I,
}

impl<'ast, I: UnsynInput> Checker<'ast, I> {
    /// A pass over a file that has found nothing yet.
    fn new(
        source: &'ast I,
        char_rules: HashSet<String>,
        defined: HashSet<String>,
        empty: BTreeSet<String>,
    ) -> Self {
        Self {
            diagnostics: Diagnostics::default(),
            rules: Vec::new(),
            trivia: None,
            char_rules,
            defined,
            empty,
            stack: Vec::new(),
            members: Vec::new(),
            ranges: Vec::new(),
            bounds: Vec::new(),
            expr_depth: 0,
            holds_node: true,
            source,
        }
    }

    // -----------------------------------------------------------------------
    // Taking back what the children left
    // -----------------------------------------------------------------------

    /// The `count` patterns the operands of the node being visited left, in the
    /// order they were written.
    fn take(&mut self, count: usize) -> Vec<Pattern> {
        debug_assert!(
            self.stack.len() >= count,
            "a node took more patterns than it has operands"
        );

        let start = self.stack.len().saturating_sub(count);
        self.stack.drain(start..).collect()
    }

    /// The pattern of the one operand of the node being visited.
    fn take_one(&mut self) -> Pattern {
        self.take(1).pop().unwrap_or_else(Pattern::nothing)
    }

    /// The `count` members of the set being visited, in the order they were
    /// written.
    fn take_members(&mut self, count: usize) -> Vec<Member> {
        debug_assert!(
            self.members.len() >= count,
            "a set took more members than it was written with"
        );

        let start = self.members.len().saturating_sub(count);
        self.members.drain(start..).collect()
    }

    /// The bounds of the repetition being visited, which the braces hold whole.
    fn take_bound(&mut self) -> Option<Bound> {
        self.bounds.pop()
    }

    // -----------------------------------------------------------------------
    // Reading the text of a node
    // -----------------------------------------------------------------------

    /// The characters a string literal holds.
    ///
    /// The reader has refused a literal whose escapes are not among those the
    /// spec writes, so the only thing left to find is an escape of a code point
    /// that is no character, which `\u{D800}` and its like are. The decoder
    /// hands those out by span — the literal is one run of text, so nothing
    /// below it has a span of its own to point at.
    fn chars_of(&mut self, lit: &'ast lexer::LitStr<I>) -> Vec<char> {
        let decoded = decode::literal(lit);

        for span in decoded.invalid {
            self.diagnostics
                .push(UnsynError::Semantics(SemanticsKind::InvalidCodePoint, span));
        }

        decoded.chars
    }

    /// The code point a `U+` literal spells, when it spells one.
    ///
    /// Four hex digits is what the rule asks for and what the reader counted, so
    /// the only thing left for this pass to say is that `D800` and its like are
    /// no character, which no count of digits can tell.
    fn code_of(&mut self, lit: &'ast lexer::LitUnicode<I>) -> Option<u32> {
        match decode::code_point(lit) {
            Some(c) => Some(c as u32),
            None => {
                self.diagnostics.push(UnsynError::Semantics(
                    SemanticsKind::InvalidCodePoint,
                    lit.to_span(),
                ));

                None
            }
        }
    }

    /// The one character the end of a range denotes, which is what the rule asks
    /// of both of them: a range of code points is between two characters, and a
    /// string of two of them is no end of one.
    fn end_of_range(&mut self, lit: &'ast lexer::LitStr<I>) -> Option<u32> {
        let span = lit.to_span();

        match self.chars_of(lit).as_slice() {
            [c] => Some(*c as u32),
            _ => {
                self.diagnostics
                    .push(UnsynError::Semantics(SemanticsKind::NotAChar, span));

                None
            }
        }
    }

    /// The count a [`lexer::LitDec`] spells, with the underscores the rule allows
    /// between its digits dropped.
    fn count_of(&mut self, lit: &'ast lexer::LitDec<I>) -> u32 {
        match decode::decimal(lit) {
            Some(count) => count,
            // A bound past what a count of repetitions can be kept to is no
            // bound, whatever the digits of it say.
            None => {
                self.diagnostics.push(UnsynError::Semantics(
                    SemanticsKind::InvalidRepeat,
                    lit.to_span(),
                ));

                u32::MAX
            }
        }
    }

    // -----------------------------------------------------------------------
    // Judging a set
    // -----------------------------------------------------------------------

    /// `[ 'a', 'b'-'c' ]` and its negation: the members of one set, compared to
    /// the members before them.
    ///
    /// A set is a union, so a member can add nothing to it, can be swallowed by
    /// what is already in it, or can cut across what is; each of those says
    /// something the file would have written differently had it known. A set with
    /// no members at all denotes nothing, which is not a set but a mistake about
    /// where the brackets end.
    fn set(&mut self, set: &'ast syntax::Set<I>) -> Pattern {
        let members = self.take_members(read::members(set).count());
        let span = set.to_span();

        if members.is_empty() {
            self.diagnostics
                .push(UnsynError::Semantics(SemanticsKind::EmptySet, span));

            return Pattern::terminal();
        }

        self.compare(&members);

        // A set denotes the characters one position may hold. A member that
        // spells more — a word, a range reaching over a word, a name whose
        // definition answers with more than a character — says what no test
        // of the character here can ask: the refusal the file meant is `~`
        // over an expression, and the choice it meant is a choice.
        for member in &members {
            let wide = match &member.value {
                Value::Chars(_) => false,
                // A name the file never defined is the mention check's to
                // say; a defined one that settles wider is the set's own.
                Value::Name(name) => {
                    let key = codegen::external_key(name);

                    // A name answering wider than a character is the member's
                    // wrong to say — unless it answers with *nothing*, which is
                    // the narrower report and is told of once the definitions
                    // have been settled (`EmptySetMember`, from `resolve`).
                    self.defined.contains(&key)
                        && !self.char_rules.contains(&key)
                        && !self.empty.contains(&key)
                }
                // A range whose ends are not characters, or stand the wrong
                // way, is what the range's own reports already say.
                Value::Unknown => false,
                // A word is no character to test, and nothing beside says so.
                Value::Strings(_) => true,
            };

            if wide {
                self.diagnostics.push(UnsynError::Semantics(
                    SemanticsKind::WideSetMember,
                    member.span.clone(),
                ));
            }
        }

        Pattern {
            heads: BTreeSet::new(),
            nullable: false,
            bare: false,
            uses: members
                .into_iter()
                .filter_map(|member| match member.value {
                    // A rule named as a member mentions that rule, whatever its
                    // definition goes on to denote.
                    Value::Name(name) => Some(Use {
                        name,
                        span: member.span,
                        alone: false,
                        mention: Mention::SetMember,
                    }),
                    _ => None,
                })
                .collect(),
        }
    }

    /// Reports the members of one set whose values conflict with a member before
    /// them, naming the earlier member as the one the later says twice.
    fn compare(&mut self, members: &[Member]) {
        for (index, member) in members.iter().enumerate() {
            for earlier in &members[..index] {
                let kind = match conflict(&member.value, &earlier.value) {
                    None => continue,
                    Some(Conflict::Same) => LintKind::DuplicateMember,
                    Some(Conflict::Covered) => LintKind::CoveredMember(earlier.span.clone()),
                    Some(Conflict::Covers) => LintKind::CoversMember(earlier.span.clone()),
                    Some(Conflict::Overlap) => LintKind::OverlappingMember(earlier.span.clone()),
                };

                self.diagnostics.lint(kind, member.span.clone());
                break;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Judging the file
    // -----------------------------------------------------------------------

    /// Reports the definitions that declare the same name, and the file that
    /// declares its trivia more than once.
    ///
    /// Both are said where the second definition stands, since the first is the
    /// one the rest of the file means, and which definition of a name a name means
    /// has to be settled before anything is asked of it.
    fn collisions(&mut self, rules: &[Rule]) {
        let mut seen: BTreeMap<String, Span> = BTreeMap::new();

        for rule in rules {
            // `r#gen` and `gen` are one name worn two ways, so a file that
            // declares both declares one name twice — which is what this is
            // looking for, and what the bare text alone would have let pass.
            if let Some(previous) =
                seen.insert(codegen::external_key(&rule.name), rule.span.clone())
            {
                self.diagnostics.push(UnsynError::Semantics(
                    SemanticsKind::NameCollision(previous),
                    rule.span.clone(),
                ));
            }

            if rule.kind == Decl::Whitespace && self.trivia.is_none() {
                self.trivia = Some(Trivia {
                    name: codegen::external_key(&rule.name),
                    span: rule.span.clone(),
                });
            }
        }

        let mut whitespace = rules.iter().filter(|rule| rule.kind == Decl::Whitespace);

        for duplicate in whitespace.by_ref().skip(1) {
            self.diagnostics.push(UnsynError::Semantics(
                SemanticsKind::DuplicateWhitespace,
                duplicate.span.clone(),
            ));
        }
    }

    /// Reports the names that mean nothing here, the nodes a terminal reaches by
    /// name, and the separator a rule of nodes names without needing to.
    fn resolve(&mut self, rules: &[Rule]) {
        let trivia = self.trivia.clone();

        // The first definition of a name is the one the rest of the file means by
        // it, which is what a name defined twice was already told.
        let mut first: BTreeMap<String, &Rule> = BTreeMap::new();

        for rule in rules {
            first
                .entry(codegen::external_key(&rule.name))
                .or_insert(rule);
        }

        for rule in rules {
            for mention in &rule.body.uses {
                let Some(defined) = first.get(&codegen::external_key(&mention.name)) else {
                    self.diagnostics.push(UnsynError::Semantics(
                        SemanticsKind::UndefinedSymbol,
                        mention.span.clone(),
                    ));

                    continue;
                };

                // A terminal matches characters, so it can only be built out of
                // other terminals: the node of a tree is nothing to match. The
                // report is told at the mention and carries the definition, for
                // it is the definition's kind that makes the mention mean
                // nothing here.
                if rule.kind != Decl::Syntax && defined.kind == Decl::Syntax {
                    self.diagnostics.push(UnsynError::Semantics(
                        SemanticsKind::UnexpectedSyntax(defined.span.clone()),
                        mention.span.clone(),
                    ));
                }

                // Both of these ask a definition for the one thing a pattern
                // cannot ask a name for and still mean anything, and both are
                // told through the names as far as they reach: a set is one
                // question of one character, so a member matched by nothing is
                // no character to ask about; a refusal takes the character at a
                // position where its operand does not begin, and a definition
                // matched by nothing begins everywhere, so there is no
                // character left for it to take.
                if self.empty.contains(&codegen::external_key(&mention.name)) {
                    let kind = match mention.mention {
                        Mention::SetMember => {
                            Some(SemanticsKind::EmptySetMember(defined.span.clone()))
                        }
                        Mention::Operand => {
                            Some(SemanticsKind::UnsatisfiableRefusal(defined.span.clone()))
                        }
                        Mention::Plain => None,
                    };

                    if let Some(kind) = kind {
                        self.diagnostics
                            .push(UnsynError::Semantics(kind, mention.span.clone()));
                    }
                }

                // The trivia between the operands of a rule of nodes is taken
                // whether the rule names it or not, so naming it says it twice —
                // unless naming it is the whole of what the rule offers there,
                // which is then the tree's own field and not a separator.
                if let Some(trivia) = trivia.as_ref()
                    && rule.kind == Decl::Syntax
                    && !mention.alone
                    && trivia.name == codegen::external_key(mention.name.as_str())
                {
                    self.diagnostics.lint(
                        LintKind::RedundantSeparator(trivia.span.clone()),
                        mention.span.clone(),
                    );
                }
            }
        }
    }

    /// Reports every definition that can begin a match of itself.
    ///
    /// What a match of a definition begins with is settled by what its operands
    /// begin with, so a definition that reaches itself there can only be matched
    /// by first matching itself. `X -> X 'a'` is the shape that still advances, so
    /// a matcher for it at least terminates; `X -> X?` neither advances nor
    /// terminates, and is reported as the other thing it is.
    ///
    /// A name that is no definition of the file stops the walk, since nothing past
    /// it is known, and a definition reached only after anything that consumes is
    /// not reached at all, because what consumes stands between it and itself.
    fn recursion(&mut self, rules: &[Rule]) {
        let mut first: BTreeMap<String, &Rule> = BTreeMap::new();

        for rule in rules {
            first
                .entry(codegen::external_key(&rule.name))
                .or_insert(rule);
        }

        for (name, definition) in &first {
            let mut reached: BTreeSet<String> = BTreeSet::new();
            // Walked by the meaning of each name, which is the only spelling
            // the map above answers to.
            let mut to_visit: Vec<String> = definition
                .body
                .heads
                .iter()
                .map(|head| codegen::external_key(head))
                .collect();

            while let Some(current) = to_visit.pop() {
                if !reached.insert(current.clone()) {
                    continue;
                }

                if let Some(next) = first.get(&current) {
                    to_visit.extend(next.body.heads.iter().cloned());
                }
            }

            if !reached.contains(name.as_str()) {
                continue;
            }

            let kind = if definition.body.nullable {
                SemanticsKind::SameOffsetRecursion
            } else {
                SemanticsKind::LeftRecursion
            };

            self.diagnostics
                .push(UnsynError::Semantics(kind, definition.span.clone()));
        }
    }
}

impl<'ast, I: UnsynInput> Visit<'ast, I> for Checker<'ast, I> {
    fn visit_crate(&mut self, node: &'ast syntax::Crate<I>) {
        walk::walk_crate(self, node);

        // The file is judged once every definition in it has been read, since a
        // name means nothing until the last of them has had its chance to define
        // it. Which definition a name means is settled first, and what is asked of
        // the definitions after that never depends on the order of the reports.
        let rules = std::mem::take(&mut self.rules);

        self.collisions(&rules);

        // A file with no separator is told before its names are resolved: every
        // definition it holds still denotes something read on its own, so the
        // missing rule is the file's fault and not each definition's.
        if self.trivia.is_none() {
            self.diagnostics.push(UnsynError::Semantics(
                SemanticsKind::MissingWhitespace,
                Span::None,
            ));
        }

        self.resolve(&rules);
        self.recursion(&rules);
    }

    fn visit_stmt(&mut self, node: &'ast syntax::Stmt<I>) {
        // Whether the parts of this definition's pattern have to be named is
        // settled before the pattern is walked, because the pattern is where
        // the question is asked.
        let held = self.holds_node;
        self.holds_node = matches!(read::decl(node), Decl::Syntax);

        walk::walk_stmt(self, node);

        self.holds_node = held;

        let body = self.take_one();

        self.rules.push(Rule {
            name: as_str(self.source, &node.ident).to_owned(),
            kind: read::decl(node),
            span: node.ident.to_span(),
            body,
        });
    }

    fn visit_expr(&mut self, node: &'ast syntax::Expr<I>) {
        // A choice written below the whole of a right side belongs to no left
        // side, so it becomes no node of the tree — however well its
        // alternatives could have been named. That is a property of the choice
        // as a whole, so it is told once, at the choice.
        //
        // Only a rule that makes a node is asked. Within a `lexer` rule the
        // code reads what a group matched as one run of text and keeps no part
        // of it, which is why a string literal's body and a block comment's are
        // a field of `I` and not a list of nodes: there is nothing in the tree
        // for the group to have been named for. A `lexer` rule is still asked
        // about the alternatives of its whole right side, which do become the
        // variants of an enum — see below.
        if self.holds_node && self.expr_depth > 0 && !read::is_single(node) {
            self.diagnostics
                .lint(LintKind::UnnamedAlternative, node.to_span());
        }

        // A choice at the whole of a right side does become an enum, in a
        // `lexer` rule as much as in a `syntax` one, and cannot come out
        // nameless as a whole; what may come out nameless is one of its
        // variants. The naming is asked of the same function the generator
        // names variants with, so the warning can neither promise a name the
        // code declines to write nor stay silent about one the code declines —
        // including a name another alternative wore first, which is a way of
        // having none that only the whole choice can see.
        //
        // Only an alternative with parts is told for reading no word. A single
        // operand is the whole of its variant, so the place it holds in the
        // choice says everything a name would: there is nothing inside it for a
        // reader to tell apart, and the value it reads stands on the face of
        // the tree wherever the variant is matched. A word cannot be had from a
        // `*`, a `'\n'` or a set, and making the author name each of those
        // would be making them name a rule that has no parts.
        //
        // A name the alternative did offer and lost to another is a different
        // case, whatever its size: the author wrote a word here, two
        // alternatives wrote the same one, and only the whole choice can see
        // that one of them goes unnamed.
        if self.expr_depth == 0 && !read::is_single(node) {
            let names = codegen::variant_names(self.source, node);
            let offers = codegen::variant_offers(self.source, node);

            for ((branch, name), offer) in read::branches(node).zip(names).zip(offers) {
                let part = read::elements(branch).count() > 1;

                if name.is_none() && (part || offer.is_some()) {
                    self.diagnostics
                        .lint(LintKind::UnnamedAlternative, codegen::branch_head(branch));
                }
            }
        }

        self.expr_depth += 1;
        walk::walk_expr(self, node);
        self.expr_depth -= 1;

        // `X+` names no node of the grammar, so the number of operands an
        // alternative was written with, and so the number of patterns it left, is
        // only to be had from the expression. This is the one place the pass
        // reaches past the node it was handed for a number.
        let counts = read::branches(node)
            .map(|branch| read::elements(branch).count())
            .collect::<Vec<_>>();

        // The alternatives left their operands on one stack, in the order the
        // file wrote them, so the whole of the expression is taken at once and
        // then divided, rather than each alternative taking from the end of what
        // the alternatives after it left.
        let mut parts = self.take(counts.iter().sum());
        let mut branches = Vec::with_capacity(counts.len());

        for count in counts {
            let mut branch = parts.drain(..count).collect::<Vec<_>>();

            debug_assert_eq!(branch.len(), count, "an alternative took its own operands");

            // An alternative that is one name says nothing but that name, which
            // is what makes a mention of the trivia of the language mean
            // something: it is then the tree's own field, and not a separator the
            // generated rule takes anyway.
            if count == 1 && branch.first().is_some_and(|part| part.bare) {
                for mention in &mut branch[0].uses {
                    mention.alone = true;
                }
            }

            branches.push(Pattern::sequence(branch));
        }

        self.stack.push(Pattern::alternative(branches));
    }

    fn visit_expr_without_suffix(&mut self, node: &'ast syntax::ExprWithoutSuffix<I>) {
        walk::walk_expr_without_suffix(self, node);

        let pattern = match node {
            // `'~' X` takes the one character that X must not begin. What the
            // refusal answers with is a character whoever the operand reaches,
            // and the only thing kept of the operand is what it mentions —
            // those names still have to be defined somewhere, and a name whose
            // definition can be matched by nothing is a probe that always
            // answers, which is what `resolve` is told of.
            //
            // The refusal of a repetition has no spelling of its own: a suffix
            // written beside `~` stands on the refusal and not under it, so
            // `~X*` repeats the refusal and no operand of a `~` is ever one to
            // carry a quantity — which is what keeps this part plain.
            syntax::ExprWithoutSuffix::NegatedSet { .. } => {
                let refused = self.take_one();

                debug_assert!(
                    !refused.nullable,
                    "an operand of a refusal carries no quantity, so none can match nothing"
                );

                Pattern {
                    heads: BTreeSet::new(),
                    nullable: false,
                    bare: false,
                    uses: refused
                        .uses
                        .into_iter()
                        .map(|mut mention| {
                            mention.mention = Mention::Operand;

                            mention
                        })
                        .collect(),
                }
            }
            // `[…]` asks which character the position holds, or which member
            // of it the position begins — and mentions the names it lists.
            syntax::ExprWithoutSuffix::Set(set) => self.set(set),
            // A call of an external is a link the caller of the parser binds, and
            // so mentions no name of this file: nothing here can ask that the file
            // have defined it, and nothing here can reach past it.
            syntax::ExprWithoutSuffix::Call { .. } => Pattern::terminal(),
            // A group is one operand, so what it can do is what the expression
            // written inside it can do, the name it holds bare and all.
            syntax::ExprWithoutSuffix::Paren { .. } => self.take_one(),
            syntax::ExprWithoutSuffix::LitStr(lit) => {
                _ = self.chars_of(lit);

                Pattern::terminal()
            }
            syntax::ExprWithoutSuffix::LitUnicode(lit) => {
                _ = self.code_of(lit);

                Pattern::terminal()
            }
            syntax::ExprWithoutSuffix::Ident(ident) => {
                Pattern::bare(as_str(self.source, ident).to_owned(), ident.to_span())
            }
        };

        self.stack.push(pattern);
    }

    fn visit_expr_with_suffix(&mut self, node: &'ast syntax::ExprWithSuffix<I>) {
        walk::walk_expr_with_suffix(self, node);

        let pattern = match node {
            syntax::ExprWithSuffix::Suffixed { suffix, .. } => {
                let mut target = self.take_one();

                match suffix {
                    // `X*` and `X?` are both satisfied by matching nothing, and
                    // `X+` is only where `X` is.
                    syntax::Suffix::Star(_) | syntax::Suffix::Question(_) => target.nullable = true,
                    syntax::Suffix::Plus(_) => {}
                    syntax::Suffix::Repeat(_) => {
                        if let Some(bound) = self.take_bound() {
                            target.nullable |= bound.may_match_nothing();
                        }
                    }
                }

                // A suffix is something more than the name it stands on, however
                // many times it asks for it.
                target.bare = false;

                target
            }
            syntax::ExprWithSuffix::Concat { .. } => {
                let mut parts = self.take(2);
                let suffix = parts.pop().expect("the operand after the keyword");
                let target = parts.pop().expect("the operand before it");

                Pattern::joined(target, suffix)
            }
            syntax::ExprWithSuffix::Except { .. } => {
                let mut parts = self.take(2);
                let tokens = parts.pop().expect("the operand after the keyword");
                let target = parts.pop().expect("the operand before it");

                Pattern::excepted(target, tokens)
            }
        };

        self.stack.push(pattern);
    }

    fn visit_repeat(&mut self, node: &'ast syntax::Repeat<I>) {
        walk::walk_repeat(self, node);

        let Some(bound) = self.take_bound() else {
            return;
        };

        // `{0}` asks for nothing at all, which is the operand left out rather than
        // a number of times, and a bound above nothing says the same. Two bounds
        // the other way round ask for a number no operand can be matched to.
        if bound.to == Some(0) || bound.from > bound.to.unwrap_or(u32::MAX) {
            self.diagnostics.push(UnsynError::Semantics(
                SemanticsKind::InvalidRepeat,
                node.to_span(),
            ));
        }

        // The operand the repetition stands on is the one that reads it, so the
        // bounds are handed back to it rather than kept here.
        self.bounds.push(bound);
    }

    fn visit_repeat_bound(&mut self, node: &'ast syntax::RepeatBound<I>) {
        walk::walk_repeat_bound(self, node);

        let bound = match node {
            // `{,n}` is as much `{0,n}` as `{n}` is `{n,n}`, which is what both
            // the bound below and the bound above are asked for.
            syntax::RepeatBound::RangeTo(_, _, _, to) => Bound {
                from: 0,
                to: Some(self.count_of(to)),
            },
            syntax::RepeatBound::RangeFrom(from, _, _, to) => Bound {
                from: self.count_of(from),
                to: to.as_ref().map(|(_, to)| self.count_of(to)),
            },
            syntax::RepeatBound::LitDec(count) => {
                let count = self.count_of(count);

                Bound {
                    from: count,
                    to: Some(count),
                }
            }
        };

        self.bounds.push(bound);
    }

    fn visit_range(&mut self, node: &'ast syntax::Range<I>) {
        walk::walk_range(self, node);

        let ends = match node {
            syntax::Range::UnicodeRange(from, _, to) => [self.code_of(from), self.code_of(to)],
            syntax::Range::StrRange(from, _, to) => {
                [self.end_of_range(from), self.end_of_range(to)]
            }
        };

        let interval = match ends {
            [Some(from), Some(to)] if from <= to => Some((from, to)),
            [Some(_), Some(_)] => {
                self.diagnostics.push(UnsynError::Semantics(
                    SemanticsKind::InvalidRange,
                    node.to_span(),
                ));

                None
            }
            // An end that is no character was reported where it was read, and a
            // range of nothing is not worth comparing to the members about it.
            _ => None,
        };

        self.ranges.push(Ends { interval });
    }

    fn visit_set_item(&mut self, node: &'ast syntax::SetItem<I>) {
        walk::walk_set_item(self, node);

        let span = node.to_span();

        let member = match node {
            syntax::SetItem::Range(_) => {
                let ends = self.ranges.pop().unwrap_or(Ends { interval: None });

                Member {
                    span,
                    value: match ends.interval {
                        Some((from, to)) => Value::Chars(vec![(from, to)]),
                        None => Value::Unknown,
                    },
                }
            }
            // A literal of one character is a character of the set; a longer one
            // is a string, which no character is ever the same value as, and which
            // is only the same as another member when the two are the same string.
            syntax::SetItem::LitStr(lit) => {
                let chars = self.chars_of(lit);

                let value = match chars.len() {
                    0 => Value::Unknown,
                    1 => Value::Chars(vec![(chars[0] as u32, chars[0] as u32)]),
                    _ => Value::Strings(vec![chars.into_iter().collect()]),
                };

                Member { span, value }
            }
            syntax::SetItem::LitUnicode(lit) => Member {
                span,
                value: match self.code_of(lit) {
                    Some(code) => Value::Chars(vec![(code, code)]),
                    None => Value::Unknown,
                },
            },
            syntax::SetItem::Ident(ident) => {
                let name = as_str(self.source, ident).to_owned();

                Member {
                    span,
                    value: Value::Name(name),
                }
            }
        };

        self.members.push(member);
    }
}

// ---------------------------------------------------------------------------
// Comparing two members of a set
// ---------------------------------------------------------------------------

/// How the value of one member of a set stands against the value of another.
enum Conflict {
    /// The two denote the same values.
    Same,
    /// The later member denotes nothing that the earlier one does not.
    Covered,
    /// The later member denotes everything the earlier one does, and more.
    Covers,
    /// The two denote some values in common, and some that are each their own.
    Overlap,
}

/// What the later member of a set has to say about the earlier one, when it has
/// anything to say.
///
/// A character and a string of more than one character are never the same value,
/// and a rule named as a member is only compared by its name, since what it
/// denotes is what its definition denotes, which is not this pass's business.
fn conflict(later: &Value, earlier: &Value) -> Option<Conflict> {
    match (later, earlier) {
        (Value::Name(later), Value::Name(earlier)) if later == earlier => Some(Conflict::Same),
        (Value::Strings(later), Value::Strings(earlier)) => later
            .iter()
            .any(|string| earlier.contains(string))
            .then_some(Conflict::Same),
        (Value::Chars(later), Value::Chars(earlier)) => {
            if later == earlier {
                Some(Conflict::Same)
            } else if covers(earlier, later) {
                Some(Conflict::Covered)
            } else if covers(later, earlier) {
                Some(Conflict::Covers)
            } else if intersects(later, earlier) {
                Some(Conflict::Overlap)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Whether every code point of `inner` lies in one of the intervals of `outer`.
fn covers(outer: &[(u32, u32)], inner: &[(u32, u32)]) -> bool {
    inner
        .iter()
        .all(|(from, to)| outer.iter().any(|(low, high)| low <= from && to <= high))
}

/// Whether the two intervals share a code point at all.
fn intersects(one: &[(u32, u32)], other: &[(u32, u32)]) -> bool {
    one.iter()
        .any(|(from, to)| other.iter().any(|(low, high)| from <= high && low <= to))
}

#[cfg(test)]
mod tests {

    use parserc::{Input, syntax::SyntaxInput};

    use super::*;
    use crate::compiler::input::TokenStream;

    /// Reads `source` as a whole file and analyzes it, exactly as it stands.
    fn report_raw(source: &str) -> Diagnostics {
        let whole = TokenStream::from(source);
        let tree = {
            let mut input = TokenStream::from(source);
            let tree = input
                .parse::<syntax::Crate<_>>()
                .expect("the source should be a file of the language");

            assert!(input.is_empty(), "the file left text unread");

            tree
        };

        analyze(&whole, &tree)
    }

    /// `source` with a separator added, since a file without one means nothing
    /// whatever the rest of it says.
    ///
    /// The rule is appended and not prefixed, so that every span the source's
    /// own text earns keeps the offset it was written at.
    fn declared(source: &str) -> String {
        if source
            .lines()
            .any(|line| line.trim_start().starts_with("whitespace "))
        {
            source.to_owned()
        } else {
            format!("{source}\nwhitespace _WS -> U+0020;\n")
        }
    }

    /// Reads `source` as a whole file and analyzes it, with the separator it
    /// does not name itself.
    fn report(source: &str) -> Diagnostics {
        report_raw(&declared(source))
    }

    /// The kinds of the errors of `source`, in the order they were reported.
    fn errors(source: &str) -> Vec<SemanticsKind> {
        report(source)
            .errors
            .into_iter()
            .map(|error| match error {
                UnsynError::Semantics(kind, _) => kind,
                error => panic!("the analyzer should report semantics, but reported {error:?}"),
            })
            .collect()
    }

    /// The kinds of the lints of `source`, in the order they were reported.
    fn lints(source: &str) -> Vec<LintKind> {
        report(source)
            .lints
            .into_iter()
            .map(|lint| lint.kind)
            .collect()
    }

    #[test]
    fn the_spec_of_the_language_denotes_a_model_of_itself() {
        // The spec is the largest file this pass has to take, and the only one
        // whose every name, range, set and repetition is written on purpose: an
        // error about any of it is a fault of this pass and not of the file.
        assert_eq!(
            report(include_str!("../../unsyn.us")).errors,
            Vec::new(),
            "the spec denotes something"
        );
    }

    #[test]
    fn a_name_defined_twice_is_the_first_of_those_definitions() {
        assert_eq!(
            errors("lexer A -> 'a'; lexer A -> 'b';"),
            [SemanticsKind::NameCollision(Span::Range(6..7))]
        );
    }

    #[test]
    fn a_file_declares_its_trivia_once() {
        assert_eq!(
            errors("whitespace S -> U+0020; whitespace T -> U+0009;"),
            [SemanticsKind::DuplicateWhitespace]
        );
    }

    #[test]
    fn a_file_that_declares_no_separator_is_refused() {
        // Nothing else the file says matters: without a separator the reader of
        // a rule of nodes has no way to tell where one token ends and the next
        // begins, so no code is written for any of its rules.
        assert_eq!(
            report_raw("lexer A -> 'a'; syntax B -> A;"),
            Diagnostics {
                errors: vec![UnsynError::Semantics(
                    SemanticsKind::MissingWhitespace,
                    Span::None,
                )],
                lints: Vec::new(),
            }
        );

        // The one rule of a separator is enough to say it, wherever it stands.
        assert_eq!(
            report_raw("whitespace S -> U+0020;"),
            Diagnostics::default()
        );
        assert_eq!(
            report_raw("lexer A -> 'a'; whitespace S -> U+0020;"),
            Diagnostics::default()
        );
    }

    #[test]
    fn a_name_that_is_no_definition_means_nothing() {
        assert_eq!(errors("syntax A -> B;"), [SemanticsKind::UndefinedSymbol]);

        // A member of a set is a mention of a name like any other.
        assert_eq!(
            errors("lexer A -> [MISSING];"),
            [SemanticsKind::UndefinedSymbol]
        );
    }

    #[test]
    fn an_external_is_the_callers_to_define() {
        // `<NAME>` is a link whoever runs the parser binds, so the file that
        // writes one is not asked to have defined the name.
        assert_eq!(report("lexer A -> <NAME>;"), Diagnostics::default());

        // The rule that calls one is defined under the same name, which is no
        // collision with a link that defines nothing.
        assert_eq!(
            report("lexer XID_Start -> <XID_Start>;"),
            Diagnostics::default()
        );
    }

    #[test]
    fn a_terminal_can_not_reach_a_node_of_the_tree() {
        assert_eq!(
            errors("lexer A -> B; syntax B -> 'b';"),
            [SemanticsKind::UnexpectedSyntax(Span::Range(21..22))]
        );

        // A rule of nodes reaches one, and the trivia of the language is a
        // terminal, so neither of those says anything.
        assert_eq!(
            report("syntax A -> B; syntax B -> 'b';"),
            Diagnostics::default()
        );
        assert_eq!(
            report("whitespace S -> U+0020; lexer A -> S;"),
            Diagnostics::default()
        );
    }

    #[test]
    fn a_set_of_nothing_denotes_nothing() {
        assert_eq!(errors("lexer A -> [];"), [SemanticsKind::EmptySet]);
        assert_eq!(errors("lexer A -> ~[];"), [SemanticsKind::EmptySet]);
    }

    #[test]
    fn a_set_holds_only_characters() {
        // A word is no character to test, and a member that spells more says
        // what no test of the character here can ask: the or it meant stands
        // where it is written, and the refusal it meant is `~`.
        assert_eq!(
            errors("lexer A -> ['ab', 'c'];"),
            [SemanticsKind::WideSetMember]
        );

        // A name answers the set only where its definition settles down to
        // characters; settling wider is the set member's wrong, not the
        // definition's own — a rule is free to spell a word.
        assert_eq!(
            errors("lexer W -> 'ab'; lexer A -> [W];"),
            [SemanticsKind::WideSetMember]
        );
        assert_eq!(report("lexer W -> 'ab';"), Diagnostics::default());

        // What settles to characters answers plainly: a code point, a class,
        // a choice of one-character arms, and names reaching those.
        assert_eq!(
            report("lexer C -> U+000A; lexer A -> ['a', C];"),
            Diagnostics::default()
        );
        assert_eq!(
            report("lexer C -> ['a'-'z']; lexer A -> [C, 'A'];"),
            Diagnostics::default()
        );
        assert_eq!(
            report("lexer C -> 'a' | 'b'; lexer A -> [C];"),
            Diagnostics::default()
        );

        // What `~` refuses is no set's business: it probes any operand, words
        // and choices included, and takes the character after the refusal.
        assert_eq!(report("lexer A -> ~('ab' | 'c');"), Diagnostics::default());
        assert_eq!(report("lexer A -> ~'ab';"), Diagnostics::default());
    }

    #[test]
    fn a_set_member_denotes_at_least_one_character() {
        // The other half of §4: a member denotes one character, which is as
        // much to say that it denotes something. A definition matched by
        // nothing answers a set with no character to test, and the set that
        // names one has written a choice of characters it never named.
        assert_eq!(
            errors("lexer B -> 'b'*; lexer A -> [B];"),
            [SemanticsKind::EmptySetMember(Span::Range(6..7))]
        );
        assert_eq!(
            errors("lexer B -> 'b'?; lexer A -> ['a', B, 'c'];"),
            [SemanticsKind::EmptySetMember(Span::Range(6..7))]
        );
        // The nothing is told through the names as far as it reaches, and the
        // member is told of rather than the rule that wrote the nothing: a
        // definition matched by nothing is no fault of its own, only of the
        // set that asked it for a character.
        assert_eq!(
            errors("lexer B -> C; lexer C -> 'c'?; lexer A -> [B];"),
            [SemanticsKind::EmptySetMember(Span::Range(6..7))]
        );
        assert_eq!(
            errors("lexer B -> 'b'{,3}; lexer A -> [B];"),
            [SemanticsKind::EmptySetMember(Span::Range(6..7))]
        );

        // A rule matched by nothing is still a rule to write, and a choice of
        // characters is still a set to name: only the set that asks one for a
        // character is told of.
        assert_eq!(report("lexer B -> 'b'*;"), Diagnostics::default());
        assert_eq!(
            report("lexer B -> 'b'*; syntax A -> B | 'a';"),
            Diagnostics::default()
        );

        // What `~` refuses is no set's business: it probes any operand, words
        // and choices included, and answers with a character whoever it
        // reaches. A refusal of a definition matched by nothing is the other
        // report below, and not this one.
        assert_eq!(
            errors("lexer B -> 'b'*; lexer A -> ~B;"),
            [SemanticsKind::UnsatisfiableRefusal(Span::Range(6..7))]
        );
    }

    #[test]
    fn a_refusal_of_a_rule_that_can_match_nothing_is_satisfied_by_nothing() {
        // `~X` takes the character where `X` does not begin, and a definition
        // that can be matched by nothing begins everywhere: the probe always
        // answers, so the refusal never reaches the character it would have
        // taken. Only a name can be asked so of, because a suffix written
        // beside `~` stands on the refusal and not under it (§4, "what binds
        // where") — `~X*` repeats a refusal, and no operand of a `~` carries a
        // quantity of its own.
        assert_eq!(
            errors("lexer A -> ~B; lexer B -> 'b'*;"),
            [SemanticsKind::UnsatisfiableRefusal(Span::Range(21..22))]
        );
        assert_eq!(
            errors("lexer A -> ~B; lexer B -> 'b'?;"),
            [SemanticsKind::UnsatisfiableRefusal(Span::Range(21..22))]
        );
        assert_eq!(
            errors("lexer A -> ~B; lexer B -> 'b'{0,3};"),
            [SemanticsKind::UnsatisfiableRefusal(Span::Range(21..22))]
        );
        // `{,n}` is `{0,n}`, and `{0}` asks for nothing at all.
        assert_eq!(
            errors("lexer A -> ~B; lexer B -> 'b'{,3};"),
            [SemanticsKind::UnsatisfiableRefusal(Span::Range(21..22))]
        );

        // The nothing a refusal reaches is told through the names as well: B
        // writes no suffix itself, and is only an answer to the one it names.
        assert_eq!(
            errors("lexer A -> ~B; lexer B -> C; lexer C -> 'c'?;"),
            [SemanticsKind::UnsatisfiableRefusal(Span::Range(21..22))]
        );

        // A refusal of a rule that has to answer is the refusal of a
        // delimiter, which is what a comment body is made of.
        assert_eq!(
            report("lexer B -> 'b'; lexer A -> ~B;"),
            Diagnostics::default()
        );

        // A group is no quantity either, and what stands beside the `~` is the
        // refusal being repeated: `~X?` may match nothing and is no trap.
        assert_eq!(report("lexer A -> ~( 'x' );"), Diagnostics::default());
        assert_eq!(report("lexer A -> ~'x'?;"), Diagnostics::default());
        assert_eq!(report("lexer A -> ~'x'*;"), Diagnostics::default());
        assert_eq!(
            report("lexer A -> ~[ 'a'-'z' ]{0,3};"),
            Diagnostics::default()
        );

        // What a refusal can answer with is a character whoever the operand
        // reaches — words and choices included — and a set always takes its one
        // character, even the empty one, which `EmptySet` is the one to say of.
        assert_eq!(report("lexer A -> ~('ab' | 'c');"), Diagnostics::default());
        assert_eq!(
            errors("lexer A -> ~[];"),
            [SemanticsKind::EmptySet],
            "an empty set is no nullable operand"
        );
    }

    #[test]
    fn the_ends_of_a_range_are_characters() {
        assert_eq!(errors("lexer A -> ['ab'-'z'];"), [SemanticsKind::NotAChar]);

        // An empty literal is no character either, and a code point literal is
        // always one, so only the ends of the string form can be more than one.
        assert_eq!(errors("lexer A -> [''-'z'];"), [SemanticsKind::NotAChar]);
        assert_eq!(
            report("lexer A -> [U+0009-U+000D];"),
            Diagnostics::default()
        );
    }

    #[test]
    fn the_lower_end_of_a_range_is_not_above_the_higher() {
        assert_eq!(
            errors("lexer A -> ['z'-'a'];"),
            [SemanticsKind::InvalidRange]
        );

        // Two ends that are the same character are a range of one of them.
        assert_eq!(report("lexer A -> ['a'-'a'];"), Diagnostics::default());
    }

    #[test]
    fn a_surrogate_is_no_character() {
        // The four hex digits of a code point literal are a shape, and a shape the
        // reader checks; that `D800` is no character is only known of the value.
        assert_eq!(
            errors("lexer A -> U+D800;"),
            [SemanticsKind::InvalidCodePoint]
        );

        assert_eq!(
            errors("lexer A -> '\\u{d800}';"),
            [SemanticsKind::InvalidCodePoint]
        );

        // The two characters a surrogate is written between are both characters,
        // and so is the replacement character past them.
        assert_eq!(
            report("lexer A -> U+D7FF; lexer B -> U+E000;"),
            Diagnostics::default()
        );
    }

    #[test]
    fn the_bounds_of_a_repeat_mean_a_number_of_times() {
        assert_eq!(
            errors("syntax A -> 'x'{3,2};"),
            [SemanticsKind::InvalidRepeat]
        );

        // A repetition of nothing at all is the operand left out, whichever of the
        // three forms wrote it.
        assert_eq!(
            errors("syntax A -> 'x'{0};"),
            [SemanticsKind::InvalidRepeat]
        );
        assert_eq!(
            errors("syntax A -> 'x'{,0};"),
            [SemanticsKind::InvalidRepeat]
        );

        assert_eq!(report("syntax A -> 'x'{1, 6};"), Diagnostics::default());
        assert_eq!(report("syntax A -> 'x'{0,6};"), Diagnostics::default());
    }

    #[test]
    fn a_definition_can_not_begin_with_itself() {
        assert_eq!(
            errors("syntax A -> A 'x' | 'y';"),
            [SemanticsKind::LeftRecursion]
        );

        // Through another definition is the same fault, and is told for every
        // definition in the cycle.
        assert_eq!(
            errors("syntax A -> B; syntax B -> A 'x';"),
            [SemanticsKind::LeftRecursion, SemanticsKind::LeftRecursion,]
        );
    }

    #[test]
    fn a_definition_that_begins_with_itself_may_not_match_nothing() {
        assert_eq!(
            errors("syntax A -> A?;"),
            [SemanticsKind::SameOffsetRecursion]
        );

        // `{0}` asks for nothing at all and is reported for that; a bound of one
        // asks for the operand, and so advances.
        assert_eq!(
            errors("syntax A -> 'x'{0}; syntax B -> B{1};"),
            [SemanticsKind::InvalidRepeat, SemanticsKind::LeftRecursion,]
        );
    }

    #[test]
    fn a_definition_guarded_by_what_consumes_is_not_recursive() {
        // The shape the spec writes for a nest of comments: the definition is
        // reached only after `/*` has been taken, so a matcher for it is never
        // asked the same question twice at one position. The choice inside the
        // group is named nowhere, which is a lint of its own and no recursion.
        assert_eq!(
            report(
                "
lexer A -> '/*' ( B | ~['*','/'] )* '*/';
lexer B -> A | '/*' A;
"
            )
            .errors,
            Vec::new()
        );

        // The `except` of a terminal is tested against text its target read, and
        // is never what a match begins with.
        assert_eq!(
            report("lexer A -> B except C; lexer B -> 'a'+; lexer C -> 'a';"),
            Diagnostics::default()
        );
    }

    #[test]
    fn a_member_of_a_set_that_adds_nothing_is_reported() {
        // Every report is told at the later member, and the member it compares
        // is pointed at by span rather than written out: the file holds the text
        // of both, at the place each is said of.
        let kinds = lints("lexer A -> ['a', 'a'];");
        assert_eq!(
            kinds,
            [LintKind::DuplicateMember],
            "the second `'a'`, at 17..20"
        );

        let kinds = lints("lexer A -> ['a'-'d', 'b'];");
        assert_eq!(
            kinds,
            [LintKind::CoveredMember(Span::Range(12..19))],
            "`'b'` at 21..24 is covered by `'a'-'d'` at 12..19"
        );

        let kinds = lints("lexer A -> ['b', 'a'-'d'];");
        assert_eq!(
            kinds,
            [LintKind::CoversMember(Span::Range(12..15))],
            "`'a'-'d'` at 17..24 covers `'b'` at 12..15"
        );

        let kinds = lints("lexer A -> ['a'-'c', 'b'-'d'];");
        assert_eq!(
            kinds,
            [LintKind::OverlappingMember(Span::Range(12..19))],
            "`'b'-'d'` at 21..28 overlaps `'a'-'c'` at 12..19"
        );

        // A character and a string of two are never the same value, and a code
        // point and the string of the one character it is are.
        assert_eq!(
            lints("lexer A -> ['a', 'ab', U+0061];"),
            [LintKind::DuplicateMember],
            "`U+0061` at 23..29 is the `'a'` at 12..15"
        );

        // A rule named as a member is compared by its name alone, since what it
        // denotes is what its definition goes on to settle.
        assert_eq!(
            lints("lexer LF -> U+000A; lexer A -> [LF, LF];"),
            [LintKind::DuplicateMember],
            "the second `LF`, at 32..34"
        );
        assert_eq!(
            report("lexer LF -> U+000A; lexer CR -> U+000D; lexer A -> [LF, CR];"),
            Diagnostics::default()
        );

        // Only the members of one set are compared, and the same name twice in two
        // alternatives of a pattern says nothing about either. A set names no
        // word, though, so the two alternatives are each told for that — which
        // is a different report and does not speak to the members.
        // Only the members of one set are compared, and the same name twice in
        // two alternatives of a pattern says nothing about either. A set names
        // no word, so each of the two alternatives is also told for that — the
        // one report the file earns besides, and nothing about the members.
        let report = report("lexer LF -> U+000A; lexer A -> [LF] | [LF];");
        assert_eq!(report.errors, Vec::new());
        assert_eq!(
            report
                .lints
                .iter()
                .filter(|lint| !matches!(lint.kind, LintKind::UnnamedAlternative))
                .collect::<Vec<_>>(),
            Vec::<&crate::compiler::errors::Lint>::new(),
            "the members of the two sets are no business of each other"
        );
    }

    #[test]
    fn only_the_first_of_two_conflicting_members_is_named_once() {
        // `'a'` is already written, and then covered, and then overlapped: each
        // report is about the later member, and is said once for it.
        assert_eq!(
            lints("lexer A -> ['a', 'a', 'a'-'b', 'b'-'c'];"),
            [
                LintKind::DuplicateMember,
                LintKind::CoversMember(Span::Range(12..15)),
                LintKind::OverlappingMember(Span::Range(22..29)),
            ]
        );
    }

    #[test]
    fn a_rule_of_nodes_names_the_trivia_it_replaces_once() {
        // Anywhere but the whole of an alternative, a mention of the trivia asks
        // for a run the generated rule takes anyway, and is said once for each
        // time it is asked for.
        assert_eq!(
            lints("whitespace S -> U+0020; syntax A -> S 'x';"),
            [LintKind::RedundantSeparator(Span::Range(11..12))],
            "the mention is told of, and pointed at the declaration at 11..12"
        );
        assert_eq!(
            lints("whitespace S -> U+0020; syntax A -> 'x' S;"),
            [LintKind::RedundantSeparator(Span::Range(11..12))]
        );
        assert_eq!(
            lints("whitespace S -> U+0020; syntax A -> S S;"),
            [
                LintKind::RedundantSeparator(Span::Range(11..12)),
                LintKind::RedundantSeparator(Span::Range(11..12)),
            ]
        );
    }

    #[test]
    fn the_whole_of_an_alternative_is_the_one_thing_a_trivia_mention_can_be() {
        // The item the spec gives its tree for: a run of the trivia is then a
        // field of the node that holds it, and not a separator.
        assert_eq!(
            report("whitespace S -> U+0020; syntax A -> 'a' | S | 'b';"),
            Diagnostics::default()
        );

        // `(S)` is the mention itself, and nothing about the group around it
        // makes it ask for the separator a second time.
        assert_eq!(
            report("whitespace S -> U+0020; syntax A -> (S);"),
            Diagnostics::default()
        );
    }

    #[test]
    fn a_suffix_on_the_trivia_is_still_a_second_mention_of_it() {
        assert_eq!(
            lints("whitespace S -> U+0020; syntax A -> S*;"),
            [LintKind::RedundantSeparator(Span::Range(11..12))]
        );
        assert_eq!(
            lints("whitespace S -> U+0020; syntax A -> S concat 'x';"),
            [LintKind::RedundantSeparator(Span::Range(11..12))]
        );
    }

    #[test]
    fn only_the_trivia_of_the_language_is_a_separator() {
        // A name of a node that no `whitespace` declaration gave is a mention
        // like any other, and a mention of the trivia inside a terminal is no
        // rule of nodes at all.
        assert_eq!(
            report("syntax B -> 'b'; syntax A -> B B;"),
            Diagnostics::default()
        );
        assert_eq!(
            report("whitespace S -> U+0020; lexer A -> S;"),
            Diagnostics::default()
        );
    }

    #[test]
    fn an_alternative_that_names_nothing_is_told() {
        // Every alternative names a rule or reads a word a variant can be
        // called by, so the enum says what each of its variants is.
        assert_eq!(
            lints("lexer A -> 'a'; lexer B -> 'b'; lexer C -> A | B | 'c';"),
            Vec::new()
        );
        assert_eq!(
            lints("lexer A -> 'a' 'b'; lexer B -> 'c' 'd'; lexer C -> A | B;"),
            Vec::new(),
            "a sequence that ends in a name still names its variant"
        );

        // A name is enough however wide the rule it names is, and a word is
        // enough however many operands stand around it.
        assert_eq!(
            lints("lexer A -> _; lexer _ -> ['a'-'z']; lexer C -> A | 'c';"),
            Vec::new(),
            "a name and a word both say a variant"
        );

        // What is told is an alternative written of more than one operand and
        // offering no word: the variant holds parts, and nothing about it says
        // which part is which. The word is sought in every operand, so a single
        // alphabetic literal anywhere in the sequence is enough to name it.
        assert_eq!(
            lints("lexer C -> ['*'-'/'] 'c' | 'd';"),
            Vec::new(),
            "one word among the operands says which variant it is"
        );
        assert_eq!(
            lints("lexer C -> ['*'-'/'] '!' | 'd';"),
            [LintKind::UnnamedAlternative],
            "parts that all read past the alphabet name nothing"
        );

        // A single operand is exempt. A `*`, a `'\n'`, a set and a reference
        // all read no word, and there is nothing inside one operand for a
        // reader to tell apart, so its place in the choice is the whole of what
        // a name would have said. Naming each of these would be naming a rule
        // that has no parts.
        assert_eq!(
            lints("lexer C -> ['*'-'/'] | 'c';"),
            Vec::new(),
            "one operand keeps its place to itself"
        );
        assert_eq!(
            lints("lexer C -> '*' | '?' | '+';"),
            Vec::new(),
            "three operands that read no word are still three operands"
        );

        // A word the author did offer and lost to another alternative is a
        // different matter, whatever the size of the alternative: one of the
        // two is left named by nothing but where it was written, and only the
        // whole choice can see that it was meant to have a name.
        assert_eq!(
            lints("lexer A -> 'x'; lexer C -> A | A;"),
            [LintKind::UnnamedAlternative],
            "the second of two alternatives reaching one rule has no name left"
        );

        // One report per such alternative, not one per choice.
        assert_eq!(
            lints("lexer C -> ['*'-'/'] '!' | ['0'-'9'] '?';"),
            [LintKind::UnnamedAlternative, LintKind::UnnamedAlternative]
        );
    }

    #[test]
    fn the_spec_names_every_part_it_writes() {
        // The spec is the file this pass is judged on, and the one whose every
        // rule was written on purpose. A lint against it is therefore a fault
        // of the file and not of the pass: either a choice left inside a group
        // of a node, which becomes no node of the tree, or an alternative of
        // parts left with no word to be called its variant, which becomes an
        // `F7` in the code a build writes. A file that describes a language
        // should not leave either of those about itself, and this is the gate
        // that keeps it so: every rule that answers one is named.
        assert_eq!(
            lints(include_str!("../../unsyn.us")),
            Vec::new(),
            "the spec names the whole of its own tree"
        );
    }

    #[test]
    fn what_is_only_tested_is_still_mentioned() {
        // The target of an `except` never begins a match, but it is a name the
        // file has to mean all the same.
        assert_eq!(
            errors("lexer A -> B except MISSING; lexer B -> 'b';"),
            [SemanticsKind::UndefinedSymbol]
        );
    }

    #[test]
    fn a_choice_inside_a_group_names_no_node_and_says_so() {
        // The whole of a right side is where an `|` becomes the alternatives of
        // the node the definition declares, and a choice written deeper is the
        // one thing the tree will not have a name for.
        assert_eq!(lints("syntax A -> 'a' | 'b';"), Vec::<LintKind>::new());
        assert_eq!(
            lints("syntax A -> ( 'a' | 'b' ) 'c';"),
            [LintKind::UnnamedAlternative]
        );

        // A group with only one way of being read is a grouping and not a
        // choice, and each of the nested groups of a real choice is judged on
        // its own, the enclosing one first.
        assert_eq!(lints("syntax A -> ( 'a' ) 'b';"), Vec::<LintKind>::new());
        assert_eq!(
            lints("syntax A -> ( 'a' | ( 'b' | 'c' ) );"),
            [LintKind::UnnamedAlternative, LintKind::UnnamedAlternative]
        );

        // A `lexer` rule is not asked, because the code it is written as keeps
        // no node there: a group inside a terminal is read as the one run of
        // text its alternatives made, which is what a string literal's body and
        // a block comment's are in the tree. Naming the group would make a rule
        // of it, and a rule is a node — so the name the warning would ask for is
        // a name that turns one token into a list of them, which is not what a
        // terminal is for.
        assert_eq!(
            lints("lexer A -> 'x' ( 'a' | 'b' )* ;"),
            Vec::new(),
            "a group inside a terminal becomes no node, so it owes no name"
        );

        // A `lexer` rule is still asked about the alternatives of its whole
        // right side, which do become the variants of an enum.
        assert_eq!(
            lints("lexer A -> ['*'-'/'] '!' | ['0'-'9'] '?';"),
            [LintKind::UnnamedAlternative, LintKind::UnnamedAlternative],
            "two variants of parts that read no word are told twice"
        );

        // A word anywhere in the sequence is enough, however wide it is: the
        // variant then says which form it is, and no name was owed.
        assert_eq!(
            lints("lexer A -> ['*'-'/'] 'x' | ['0'-'9'] 'y';"),
            Vec::new()
        );
    }

    #[test]
    fn a_definition_reached_past_an_operand_that_may_match_nothing_is_recursive() {
        // `'x'? A` begins with `A` wherever the optional literal matches nothing,
        // which is what the run of leading nullable operands counts.
        assert_eq!(
            errors("syntax A -> 'x'? A;"),
            [SemanticsKind::LeftRecursion]
        );
    }

    #[test]
    fn an_empty_file_denotes_nothing_and_says_nothing() {
        assert_eq!(report(""), Diagnostics::default());
        assert_eq!(report("//! only a document"), Diagnostics::default());
    }
}
