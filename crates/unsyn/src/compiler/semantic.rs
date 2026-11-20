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

use std::{
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
};

use parserc::{Span, syntax::Syntax};

use super::errors::{Diagnostics, LintKind, SemanticsKind, UnsynError};
use super::input::UnsynInput;
use super::visit::{Visit, walk};
use super::{lexer, syntax, syntax::Decl};

/// Reports what `file` gets wrong beyond the shape of its text.
///
/// The file is read as a whole, and so is reported as a whole: a definition is
/// not dropped at the first name that means nothing, since what the rest of the
/// file says is what the author meant around it.
pub fn analyze<I>(file: &syntax::Crate<I>) -> Diagnostics
where
    I: UnsynInput,
{
    let mut checker = Checker::new();
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

    /// `left concat right` and `left followed right`, which differ in whether the
    /// right operand is part of what matched: a `followed` tests its operand and
    /// then leaves it where it was.
    ///
    /// Either way the right operand is only reached where the left one may match
    /// nothing, which is what says whether a definition guarded by the left one is
    /// still guarded.
    fn joined(left: Self, right: Self, consumed: bool) -> Self {
        let nullable = if consumed {
            left.nullable && right.nullable
        } else {
            left.nullable
        };

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

/// One mention of a name in a pattern.
struct Use {
    /// The name as it was written, assembled from the characters the name read.
    name: String,
    /// Where it was written.
    span: Span,
    /// Whether the name is the whole of the alternative that offers it, which for
    /// the trivia of the language is the only thing a mention of it can mean.
    alone: bool,
}

// ---------------------------------------------------------------------------
// What a member of a set denotes
// ---------------------------------------------------------------------------

/// One member of a set, read to the values it denotes.
struct Member {
    /// The member as it was written, for the report about it.
    text: String,
    /// Where it was written.
    span: Span,
    /// What it denotes.
    value: Value,
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
    /// The range as it was written, for the report about the member it makes.
    text: String,
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
    /// The name the `whitespace` declaration gave the trivia, when there is one.
    trivia: Option<String>,
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
    /// The input the tree was read from, a piece of which every node holds.
    _input: PhantomData<&'ast I>,
}

impl<'ast, I: UnsynInput> Checker<'ast, I> {
    /// A pass over a file that has found nothing yet.
    fn new() -> Self {
        Self {
            diagnostics: Diagnostics::default(),
            rules: Vec::new(),
            trivia: None,
            stack: Vec::new(),
            members: Vec::new(),
            ranges: Vec::new(),
            bounds: Vec::new(),
            expr_depth: 0,
            _input: PhantomData,
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
    /// The reader has refused a literal whose escapes are not among those the spec
    /// writes, so the only thing left to find is an escape of a code point that is
    /// no character, which `\u{D800}` and its like are. Its place is reported as a
    /// piece of the literal's own span, since the literal is the only node that
    /// reaches it.
    fn chars_of(&mut self, lit: &'ast lexer::LitStr<I>) -> Vec<char> {
        let mut chars = Vec::new();

        for piece in &lit.body {
            if let lexer::LitStrContent::Unicode(escape) = piece
                && escape.value().is_none()
            {
                // The escape was read whole and well; only the number it
                // spells is no character, and its own span is where to say so.
                self.diagnostics.push(UnsynError::Semantics(
                    SemanticsKind::InvalidCodePoint,
                    escape.to_span(),
                ));

                continue;
            }

            chars.extend(piece.chars());
        }

        chars
    }

    /// The code point a `U+` literal spells, when it spells one.
    ///
    /// Four hex digits is what the rule asks for and what the reader counted, so
    /// the only thing left for this pass to say is that `D800` and its like are
    /// no character, which no count of digits can tell.
    fn code_of(&mut self, lit: &'ast lexer::LitUnicode<I>) -> Option<u32> {
        match lit.value() {
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
        match lit.count() {
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
        let members = self.take_members(set.len());
        let span = set.to_span();

        if members.is_empty() {
            self.diagnostics
                .push(UnsynError::Semantics(SemanticsKind::EmptySet, span));

            return Pattern::terminal();
        }

        self.compare(&members);

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
                    Some(Conflict::Same) => LintKind::DuplicateMember(member.text.clone()),
                    Some(Conflict::Covered) => {
                        LintKind::CoveredMember(member.text.clone(), earlier.text.clone())
                    }
                    Some(Conflict::Covers) => {
                        LintKind::CoversMember(member.text.clone(), earlier.text.clone())
                    }
                    Some(Conflict::Overlap) => {
                        LintKind::OverlappingMember(member.text.clone(), earlier.text.clone())
                    }
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
            if let Some(previous) = seen.insert(rule.name.clone(), rule.span.clone()) {
                self.diagnostics.push(UnsynError::Semantics(
                    SemanticsKind::NameCollision(previous),
                    rule.span.clone(),
                ));
            }

            if rule.kind == Decl::Whitespace && self.trivia.is_none() {
                self.trivia = Some(rule.name.clone());
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
            first.entry(rule.name.clone()).or_insert(rule);
        }

        for rule in rules {
            for mention in &rule.body.uses {
                let Some(defined) = first.get(&mention.name) else {
                    self.diagnostics.push(UnsynError::Semantics(
                        SemanticsKind::UndefinedSymbol(mention.name.clone()),
                        mention.span.clone(),
                    ));

                    continue;
                };

                // A terminal matches characters, so it can only be built out of
                // other terminals: the node of a tree is nothing to match.
                if rule.kind != Decl::Syntax && defined.kind == Decl::Syntax {
                    self.diagnostics.push(UnsynError::Semantics(
                        SemanticsKind::UnexpectedSyntax(mention.name.clone()),
                        mention.span.clone(),
                    ));
                }

                // The trivia between the operands of a rule of nodes is taken
                // whether the rule names it or not, so naming it says it twice —
                // unless naming it is the whole of what the rule offers there,
                // which is then the tree's own field and not a separator.
                if rule.kind == Decl::Syntax
                    && !mention.alone
                    && Some(mention.name.as_str()) == trivia.as_deref()
                {
                    self.diagnostics.lint(
                        LintKind::RedundantSeparator(mention.name.clone()),
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
            first.entry(rule.name.clone()).or_insert(rule);
        }

        for (name, definition) in &first {
            let mut reached: BTreeSet<String> = BTreeSet::new();
            let mut to_visit: Vec<String> = definition.body.heads.iter().cloned().collect();

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
                SemanticsKind::SameOffsetRecursion(name.to_string())
            } else {
                SemanticsKind::LeftRecursion(name.to_string())
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
        self.resolve(&rules);
        self.recursion(&rules);
    }

    fn visit_stmt(&mut self, node: &'ast syntax::Stmt<I>) {
        walk::walk_stmt(self, node);

        let body = self.take_one();

        self.rules.push(Rule {
            name: node.ident.text(),
            kind: node.decl(),
            span: node.ident.to_span(),
            body,
        });
    }

    fn visit_expr(&mut self, node: &'ast syntax::Expr<I>) {
        // An `|` says which of the node's alternatives was read only where the
        // alternatives are the whole of a right side; a choice written inside a
        // group belongs to no left side and names nothing, which is worth
        // telling the author of a file before the tree is read by anyone.
        if self.expr_depth > 0 && !node.is_single() {
            self.diagnostics
                .lint(LintKind::UnnamedAlternative, node.to_span());
        }

        self.expr_depth += 1;
        walk::walk_expr(self, node);
        self.expr_depth -= 1;

        // `X+` names no node of the grammar, so the number of operands an
        // alternative was written with, and so the number of patterns it left, is
        // only to be had from the expression. This is the one place the pass
        // reaches past the node it was handed for a number.
        let counts = node
            .branches()
            .map(|branch| branch.elements().count())
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
            // `'~' […]` and `[…]` differ in what they ask of the character here,
            // and not in what they can begin with or in what they mention.
            syntax::ExprWithoutSuffix::Tilde { set, .. } | syntax::ExprWithoutSuffix::Set(set) => {
                self.set(set)
            }
            // A call of an external is a link the caller of the parser binds, and
            // so mentions no name of this file: nothing here can ask that the file
            // have defined it, and nothing here can reach past it.
            syntax::ExprWithoutSuffix::Call { .. } => Pattern::terminal(),
            // A group is one operand, so what it can do is what the expression
            // written inside it can do, the name it holds bare and all.
            syntax::ExprWithoutSuffix::Paren { .. } => self.take_one(),
            syntax::ExprWithoutSuffix::Str(lit) => {
                _ = self.chars_of(lit);

                Pattern::terminal()
            }
            syntax::ExprWithoutSuffix::Unicode(lit) => {
                _ = self.code_of(lit);

                Pattern::terminal()
            }
            syntax::ExprWithoutSuffix::Reference(ident) => {
                Pattern::bare(ident.text(), ident.to_span())
            }
        };

        self.stack.push(pattern);
    }

    fn visit_expr_with_suffix(&mut self, node: &'ast syntax::ExprWithSuffix<I>) {
        walk::walk_expr_with_suffix(self, node);

        let pattern = match node {
            syntax::ExprWithSuffix::Grouped { suffix, .. } => {
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
            syntax::ExprWithSuffix::Concat { .. } | syntax::ExprWithSuffix::Followed { .. } => {
                let consumed = matches!(node, syntax::ExprWithSuffix::Concat { .. });
                let mut parts = self.take(2);
                let suffix = parts.pop().expect("the operand after the keyword");
                let target = parts.pop().expect("the operand before it");

                Pattern::joined(target, suffix, consumed)
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
            syntax::RepeatBound::Count(count) => {
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

        let (text, ends) = match node {
            syntax::Range::Unicode(from, _, _, _, to) => (
                format!("{}-{}", from.text(), to.text()),
                [self.code_of(from), self.code_of(to)],
            ),
            syntax::Range::Str(from, dash, to) => (
                format!("{}{}{}", from.text(), dash.as_str(), to.text()),
                [self.end_of_range(from), self.end_of_range(to)],
            ),
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

        self.ranges.push(Ends { interval, text });
    }

    fn visit_set_item(&mut self, node: &'ast syntax::SetItem<I>) {
        walk::walk_set_item(self, node);

        let span = node.to_span();

        let member = match node {
            syntax::SetItem::Range(_) => {
                let ends = self.ranges.pop().unwrap_or_else(|| Ends {
                    text: String::new(),
                    interval: None,
                });

                Member {
                    text: ends.text,
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
            syntax::SetItem::Str(lit) => {
                let text = lit.text();
                let chars = self.chars_of(lit);

                let value = match chars.len() {
                    0 => Value::Unknown,
                    1 => Value::Chars(vec![(chars[0] as u32, chars[0] as u32)]),
                    _ => Value::Strings(vec![chars.into_iter().collect()]),
                };

                Member { text, span, value }
            }
            syntax::SetItem::Unicode(lit) => {
                let text = lit.text();

                Member {
                    span,
                    text,
                    value: match self.code_of(lit) {
                        Some(code) => Value::Chars(vec![(code, code)]),
                        None => Value::Unknown,
                    },
                }
            }
            syntax::SetItem::Reference(ident) => {
                let name = ident.text();

                Member {
                    text: name.clone(),
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

    /// Reads `source` as a whole file and analyzes it.
    fn report(source: &str) -> Diagnostics {
        let tree = {
            let mut input = TokenStream::from(source);
            let tree = input
                .parse::<syntax::Crate<_>>()
                .expect("the source should be a file of the language");

            assert!(input.is_empty(), "the file left text unread");

            tree
        };

        analyze(&tree)
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
        // What the file does earn is the lints of the choices it writes inside
        // its groups, which no rule of its own names.
        assert_eq!(
            report(include_str!("../../unsyn.us")).errors,
            Vec::new(),
            "the spec denotes something"
        );
    }

    #[test]
    fn the_choices_the_spec_writes_inside_its_groups_are_the_ones_it_names_nowhere() {
        // Every group of a right side that offers more than one way of being
        // read is a choice that names no node, and the spec is written with
        // exactly this many of them. The count is pinned so that a group added
        // to the spec shows up here rather than only in whoever reads the file.
        let unnamed = report(include_str!("../../unsyn.us"))
            .lints
            .into_iter()
            .filter(|lint| lint.kind == LintKind::UnnamedAlternative)
            .count();

        assert_eq!(unnamed, 7, "the choices of unsyn.us that name no node");
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
    fn a_name_that_is_no_definition_means_nothing() {
        assert_eq!(
            errors("syntax A -> B;"),
            [SemanticsKind::UndefinedSymbol("B".into())]
        );

        // A member of a set is a mention of a name like any other.
        assert_eq!(
            errors("lexer A -> [MISSING];"),
            [SemanticsKind::UndefinedSymbol("MISSING".into())]
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
            [SemanticsKind::UnexpectedSyntax("B".into())]
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
            [SemanticsKind::LeftRecursion("A".into())]
        );

        // Through another definition is the same fault, and is told for every
        // definition in the cycle.
        assert_eq!(
            errors("syntax A -> B; syntax B -> A 'x';"),
            [
                SemanticsKind::LeftRecursion("A".into()),
                SemanticsKind::LeftRecursion("B".into()),
            ]
        );
    }

    #[test]
    fn a_definition_that_begins_with_itself_may_not_match_nothing() {
        assert_eq!(
            errors("syntax A -> A?;"),
            [SemanticsKind::SameOffsetRecursion("A".into())]
        );

        // `{0}` asks for nothing at all and is reported for that; a bound of one
        // asks for the operand, and so advances.
        assert_eq!(
            errors("syntax A -> 'x'{0}; syntax B -> B{1};"),
            [
                SemanticsKind::InvalidRepeat,
                SemanticsKind::LeftRecursion("B".into()),
            ]
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
        let kinds = lints("lexer A -> ['a', 'a'];");
        assert_eq!(kinds, [LintKind::DuplicateMember("'a'".into())]);

        let kinds = lints("lexer A -> ['a'-'d', 'b'];");
        assert_eq!(
            kinds,
            [LintKind::CoveredMember("'b'".into(), "'a'-'d'".into())]
        );

        let kinds = lints("lexer A -> ['b', 'a'-'d'];");
        assert_eq!(
            kinds,
            [LintKind::CoversMember("'a'-'d'".into(), "'b'".into())]
        );

        let kinds = lints("lexer A -> ['a'-'c', 'b'-'d'];");
        assert_eq!(
            kinds,
            [LintKind::OverlappingMember(
                "'b'-'d'".into(),
                "'a'-'c'".into()
            )]
        );

        // A character and a string of two are never the same value, and a code
        // point and the string of the one character it is are.
        assert_eq!(
            lints("lexer A -> ['a', 'ab', U+0061];"),
            [LintKind::DuplicateMember("U+0061".into())]
        );

        // A rule named as a member is compared by its name alone, since what it
        // denotes is what its definition goes on to settle.
        assert_eq!(
            lints("lexer LF -> U+000A; lexer A -> [LF, LF];"),
            [LintKind::DuplicateMember("LF".into())]
        );
        assert_eq!(
            report("lexer LF -> U+000A; lexer CR -> U+000D; lexer A -> [LF, CR];"),
            Diagnostics::default()
        );

        // Only the members of one set are compared, and the same name twice in two
        // alternatives of a pattern says nothing about either.
        assert_eq!(
            report("lexer LF -> U+000A; lexer A -> [LF] | [LF];"),
            Diagnostics::default()
        );
    }

    #[test]
    fn only_the_first_of_two_conflicting_members_is_named_once() {
        // `'a'` is already written, and then covered, and then overlapped: each
        // report is about the later member, and is said once for it.
        assert_eq!(
            lints("lexer A -> ['a', 'a', 'a'-'b', 'b'-'c'];"),
            [
                LintKind::DuplicateMember("'a'".into()),
                LintKind::CoversMember("'a'-'b'".into(), "'a'".into()),
                LintKind::OverlappingMember("'b'-'c'".into(), "'a'-'b'".into()),
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
            [LintKind::RedundantSeparator("S".into())]
        );
        assert_eq!(
            lints("whitespace S -> U+0020; syntax A -> 'x' S;"),
            [LintKind::RedundantSeparator("S".into())]
        );
        assert_eq!(
            lints("whitespace S -> U+0020; syntax A -> S S;"),
            [
                LintKind::RedundantSeparator("S".into()),
                LintKind::RedundantSeparator("S".into()),
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
            [LintKind::RedundantSeparator("S".into())]
        );
        assert_eq!(
            lints("whitespace S -> U+0020; syntax A -> S concat 'x';"),
            [LintKind::RedundantSeparator("S".into())]
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
    fn what_is_only_tested_is_still_mentioned() {
        // The target of an `except` never begins a match, and neither does the
        // operand of a `followed`, but both are names the file has to mean.
        assert_eq!(
            errors("lexer A -> B except MISSING; lexer B -> 'b';"),
            [SemanticsKind::UndefinedSymbol("MISSING".into())]
        );
        assert_eq!(
            errors("syntax A -> 'x' followed MISSING;"),
            [SemanticsKind::UndefinedSymbol("MISSING".into())]
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

        // The level of the rule says nothing about the judgment: what a lexer
        // writes inside its groups is as unnamed as what a node writes.
        assert_eq!(
            lints("lexer A -> 'x' ( 'a' | 'b' )* ;"),
            [LintKind::UnnamedAlternative]
        );
    }

    #[test]
    fn a_definition_reached_past_an_operand_that_may_match_nothing_is_recursive() {
        // `'x'? A` begins with `A` wherever the optional literal matches nothing,
        // which is what the run of leading nullable operands counts.
        assert_eq!(
            errors("syntax A -> 'x'? A;"),
            [SemanticsKind::LeftRecursion("A".into())]
        );

        // A `followed` that comes first in its sequence is tested at the offset
        // the definition begins at, and consumes nothing to get past it.
        assert_eq!(
            errors("syntax A -> A followed 'x';"),
            [SemanticsKind::LeftRecursion("A".into())]
        );
    }

    #[test]
    fn an_empty_file_denotes_nothing_and_says_nothing() {
        assert_eq!(report(""), Diagnostics::default());
        assert_eq!(report("//! only a document"), Diagnostics::default());
    }
}
