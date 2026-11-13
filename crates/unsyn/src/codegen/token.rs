//! What a token rule matches: the character program a definition body compiles
//! to.
//!
//! A `lexer` definition is read against the source text rather than against
//! other nodes, so its body is compiled here into a small program over
//! characters ([`Inst`]) before any code is emitted. That step exists so the
//! emission can be about running the pattern rather than about working out what
//! it means: the shortcuts of [`scan`](super::scan) are all justified by a
//! program that says the same thing more plainly, and the program is what the
//! tests of this module can compare against.
//!
//! The program keeps the difference the DSL draws between *taking* something and
//! *asking* about it. `~`, `except` and `followed` ask; only `~` then takes
//! something anyway, and takes exactly one character, which is why a line
//! comment's `~[LF]*` reads to the end of the line rather than stopping before
//! it.

use std::collections::{HashMap, HashSet};

use parserc::Span;
use parserc::syntax::Syntax;

use crate::input::UnsynInput;
use crate::lexical::lit::{LitStr, LitUnicode};
use crate::semantics::WILDCARD;
use crate::semantics::{dec_usize, str_value, unicode_value};
use crate::syntax::{
    Expr, ExprNoTopAlt, ExprNoTopAlts, ExprWithSuffix, ExprWithoutSuffix, Range, Repeat, SetItem,
};

use super::error::{CodegenError, CodegenKind};
use super::set_items;

/// The characters one step of a pattern may take.
///
/// A class is a value rather than a piece of code, which is what lets
/// `['a'-'z', 'A'-'Z']` and `U+0061-U+007A | U+0041-U+005A` be read as one
/// pattern, and lets a repetition of either be scanned in a single pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Pred {
    /// Every character, which is what the wildcard `_` denotes.
    Any,
    /// No character at all.
    Nothing,
    /// Exactly these characters and these intervals of them.
    Chars(Vec<char>, Vec<(char, char)>),
    /// Whatever both sides take.
    And(Box<Pred>, Box<Pred>),
    /// Whatever either side takes.
    Or(Box<Pred>, Box<Pred>),
    /// Every character the operand does not take.
    Not(Box<Pred>),
}

impl Pred {
    /// The class of exactly one character.
    pub(crate) fn of(c: char) -> Self {
        Self::Chars(vec![c], vec![])
    }

    /// The class of the interval both ends belong to, inclusive.
    pub(crate) fn of_range(start: char, end: char) -> Self {
        if start == end {
            Self::of(start)
        } else {
            Self::Chars(vec![], vec![(start, end)])
        }
    }

    /// The union of several classes.
    pub(crate) fn union(classes: impl IntoIterator<Item = Pred>) -> Pred {
        classes
            .into_iter()
            .fold(Pred::Nothing, |acc, next| acc.or(next))
    }

    /// This class's complement.
    pub(crate) fn negated(self) -> Pred {
        match self {
            // `_` complemented is nothing, and a double complement cancels: both
            // read better composed than as `!Any` or `!!class`.
            Pred::Any => Pred::Nothing,
            Pred::Nothing => Pred::Any,
            Pred::Not(inner) => *inner,
            other => Pred::Not(Box::new(other)),
        }
    }

    pub(crate) fn and(self, other: Pred) -> Pred {
        match (self, other) {
            (Pred::Nothing, _) | (_, Pred::Nothing) => Pred::Nothing,
            (Pred::Any, other) | (other, Pred::Any) => other,
            (first, second) => Pred::And(Box::new(first), Box::new(second)),
        }
    }

    pub(crate) fn or(self, other: Pred) -> Pred {
        match (self, other) {
            (Pred::Any, _) | (_, Pred::Any) => Pred::Any,
            (Pred::Nothing, other) | (other, Pred::Nothing) => other,
            (first, second) => Pred::Or(Box::new(first), Box::new(second)),
        }
    }

    /// This class as a condition over one character named `c`.
    pub(crate) fn render(&self) -> proc_macro2::TokenStream {
        use proc_macro2::{Literal, TokenStream};
        use quote::quote;

        match self {
            Pred::Any => quote!(true),
            Pred::Nothing => quote!(false),
            Pred::Chars(chars, ranges) => {
                let tests = chars
                    .iter()
                    .map(|c| {
                        let c = Literal::character(*c);

                        quote!(#c)
                    })
                    .chain(ranges.iter().map(|(start, end)| {
                        let (start, end) = (Literal::character(*start), Literal::character(*end));

                        quote!(#start ..= #end)
                    }));

                let tests: Vec<TokenStream> = tests.collect();

                quote!(::core::matches!(c, #(#tests)|*))
            }
            Pred::And(first, second) => {
                let (first, second) = (first.render(), second.render());

                quote!((#first && #second))
            }
            Pred::Or(first, second) => {
                let (first, second) = (first.render(), second.render());

                quote!((#first || #second))
            }
            Pred::Not(inner) => {
                let inner = inner.render();

                quote!(!#inner)
            }
        }
    }

    /// This class as the condition a scan is handed.
    ///
    /// A class that takes every character is written over a nameless character,
    /// since the question it answers does not turn on the one it is asked about
    /// and a generated file should not warn about a binding it never reads.
    pub(crate) fn closure(&self) -> proc_macro2::TokenStream {
        use quote::quote;

        let body = self.render();

        match self {
            Pred::Any => quote!(|_: char| #body),
            _ => quote!(|c: char| #body),
        }
    }
}

/// One step of a token pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Inst {
    /// Take one character of this class.
    Class(Pred),
    /// Take exactly these characters, in this order.
    Lit(String),
    /// Take what a matcher of the backend takes, which is at least one character
    /// and as many as it likes.
    Matcher(String),
    /// Take what the token rule of that name takes.
    Ref(String),
    /// Take what each step takes, in order.
    Seq(Vec<Inst>),
    /// Take what the first alternative that fits takes.
    Alt(Vec<Inst>),
    /// Take the step as often as the text allows, at least `min` times and at
    /// most `max`.
    Rep {
        /// The least number of times the step is taken.
        min: usize,
        /// The most number of times it is taken, or nothing when the text decides.
        max: Option<usize>,
        /// What is repeated.
        item: Box<Inst>,
    },
    /// Fail when the set fits here, and take one character when it does not.
    Not(Box<Inst>),
    /// Take the operand unless what removes it fits where the operand starts.
    Except {
        /// What is taken.
        item: Box<Inst>,
        /// What is taken out of it, tested at the start of the operand.
        removed: Box<Inst>,
    },
    /// Take the operand, and ask that what follows it come next without being
    /// taken here.
    Followed {
        /// What is taken.
        item: Box<Inst>,
        /// What has to come after it.
        next: Box<Inst>,
    },
    /// Fail when the operand fits next, and take nothing when it does not.
    ///
    /// This is what keeps a keyword from matching the front of a longer word:
    /// `'let'` is `'let'` only where an `XID_Continue` character does not
    /// follow, and the question is asked about the text without taking any of
    /// it, which is what separates it from `~`.
    NotFollowed(Box<Inst>),
}

impl Inst {
    /// Returns `true` when this step can take nothing at all.
    fn takes_nothing(&self, empty: &HashSet<String>) -> bool {
        match self {
            Inst::Class(_) | Inst::Matcher(_) => false,
            Inst::Lit(text) => text.is_empty(),
            Inst::Ref(name) => empty.contains(name.as_str()),
            Inst::Seq(steps) => steps.iter().all(|step| step.takes_nothing(empty)),
            Inst::Alt(steps) => steps.iter().any(|step| step.takes_nothing(empty)),
            Inst::Rep { min, item, .. } => *min == 0 && item.takes_nothing(empty),
            // A complement always takes the one character it was asked about, the
            // other two always take their operand, and a lookahead takes nothing.
            Inst::Not(_) | Inst::Except { .. } | Inst::Followed { .. } => false,
            Inst::NotFollowed(_) => true,
        }
    }

    /// Returns `true` when repeating this step would match at one position
    /// forever.
    fn is_stuck(&self, empty: &HashSet<String>) -> bool {
        match self {
            Inst::Rep { max, item, .. } => {
                // Only an unbounded repetition can turn forever: a bound is
                // counted out of the loop, however the item behaves, and `X?` is
                // tried once. An unbounded one is entered again whenever its
                // item fits, which it always does when the item takes nothing.
                max.is_none() && item.takes_nothing(empty)
            }
            Inst::Seq(steps) | Inst::Alt(steps) => steps.iter().any(|step| step.is_stuck(empty)),
            // A lookahead never moves, but it is also never what a repetition
            // repeats: `~` and the suffixes only ever sit on a set or an
            // operand, and the wildcard below already answers for it.
            Inst::Not(item) => item.is_stuck(empty),
            Inst::Except { item, .. } | Inst::Followed { item, .. } => item.is_stuck(empty),
            _ => false,
        }
    }

    /// The name a stuck repetition should be reported under: the rule it names,
    /// where one is reached, or nothing when the operand is written out.
    fn named(&self) -> Option<&str> {
        match self {
            Inst::Ref(name) => Some(name),
            Inst::Seq(steps) | Inst::Alt(steps) => steps.iter().find_map(Inst::named),
            Inst::Rep { item, .. } => item.named(),
            _ => None,
        }
    }

    /// How far this step reaches into the text at the least, which is how the
    /// members of a set are ordered: the longer one first, so that a set never
    /// reads less of the text than it was offered.
    fn reaches(&self) -> usize {
        match self {
            Inst::Lit(text) => text.chars().count(),
            Inst::Seq(steps) => steps.iter().map(Inst::reaches).sum(),
            Inst::Alt(steps) => steps.iter().map(Inst::reaches).min().unwrap_or_default(),
            Inst::Rep { min, item, .. } => *min * item.reaches(),
            Inst::Except { item, .. } | Inst::Followed { item, .. } => item.reaches(),
            // A lookahead promises nothing about the text it does not take.
            Inst::NotFollowed(_) => 0,
            // A class, a matcher, a reference or a complement: one character is
            // all any of them promises, since none of them is measured here.
            _ => 1,
        }
    }

    /// The class this step takes, when it takes exactly one character and
    /// nothing else about the text decides the answer.
    pub(crate) fn single_char(&self, classes: &HashMap<String, Pred>) -> Option<Pred> {
        match self {
            Inst::Class(pred) => Some(pred.clone()),
            Inst::Lit(text) => {
                let mut chars = text.chars();

                match (chars.next(), chars.next()) {
                    (Some(one), None) => Some(Pred::of(one)),
                    _ => None,
                }
            }
            Inst::Ref(name) => classes.get(name).cloned(),
            // A matcher's length is its own business, so it is never a class.
            Inst::Matcher(_) => None,
            Inst::Seq(steps) => steps
                .first()
                .filter(|_| steps.len() == 1)
                .and_then(|step| step.single_char(classes)),
            Inst::Alt(steps) => steps
                .iter()
                .map(|step| step.single_char(classes))
                .collect::<Option<Vec<_>>>()
                .map(Pred::union),
            Inst::Not(item) => item.single_char(classes).map(Pred::negated),
            Inst::Rep { .. } => None,
            Inst::Except { item, removed } => {
                // Removing is a complement of one character only when what is
                // removed is one character too: `'a'-'c' except 'b'` is a class,
                // while `IDENTIFIER except KEYWORDS` turns on the text that
                // follows the character being decided on.
                let item = item.single_char(classes)?;
                let removed = removed.single_char(classes)?;

                Some(item.and(removed.negated()))
            }
            // An assertion about what follows is not a question about one
            // character.
            Inst::Followed { .. } | Inst::NotFollowed(_) => None,
        }
    }
}

/// What a token pattern needs to know about the names it refers to.
#[derive(Debug, Default)]
pub(crate) struct LexRefs {
    /// The names of rules that read text, which a pattern may step over.
    pub(crate) tokens: HashSet<String>,
    /// The names of node rules, which a token cannot be built out of.
    pub(crate) syntaxes: HashSet<String>,
    /// The names of the rules that can take nothing, whose unbounded repetition
    /// would never move forward.
    pub(crate) empty: HashSet<String>,
}

/// One token type to write: its name and the program its `parse` runs.
pub(crate) struct Scanner {
    /// The name of the type.
    pub(crate) name: String,
    /// The program of the rule it was built from.
    pub(crate) inst: Inst,
}

impl Scanner {
    /// A token type from the program it runs.
    pub(crate) fn plain(name: impl Into<String>, inst: Inst) -> Self {
        Self {
            name: name.into(),
            inst,
        }
    }
}

/// Builds the program of a token pattern out of the CST of a definition body.
pub(crate) struct Builder<'a> {
    refs: &'a LexRefs,
}

impl<'a> Builder<'a> {
    /// Prepares a builder over the names `refs` describes.
    pub(crate) fn new(refs: &'a LexRefs) -> Self {
        Self { refs }
    }

    /// The program of a whole definition body.
    pub(crate) fn expr<I: UnsynInput>(&self, expr: &Expr<I>) -> Result<Inst, CodegenError> {
        let mut branches = vec![self.alts(&expr.first)?];

        for (_, alts) in expr.rest.iter() {
            branches.push(self.alts(alts)?);
        }

        Ok(alt_of(branches))
    }

    fn alts<I: UnsynInput>(&self, seq: &ExprNoTopAlts<I>) -> Result<Inst, CodegenError> {
        let mut steps = vec![self.alt(&seq.first)?];

        for (_, expr) in seq.rest.iter() {
            steps.push(self.alt(expr)?);
        }

        Ok(seq_of(steps))
    }

    fn alt<I: UnsynInput>(&self, expr: &ExprNoTopAlt<I>) -> Result<Inst, CodegenError> {
        match expr {
            ExprNoTopAlt::WithSuffix(expr) => self.with_suffix(expr),
            ExprNoTopAlt::WithoutSuffix(expr) => self.operand(expr),
        }
    }

    fn with_suffix<I: UnsynInput>(&self, expr: &ExprWithSuffix<I>) -> Result<Inst, CodegenError> {
        match expr {
            ExprWithSuffix::Star(target, _) => self.rep(0, None, target, expr),
            ExprWithSuffix::Question(target, _) => self.rep(0, Some(1), target, expr),
            ExprWithSuffix::Plus(target, _) => self.rep(1, None, target, expr),
            ExprWithSuffix::Repeat { target, suffix } => {
                let (min, max) = repeat_bounds(&suffix.body).ok_or_else(|| CodegenError {
                    kind: CodegenKind::InvalidRepeat,
                    span: expr.to_span(),
                })?;

                self.rep(min, max, target, expr)
            }
            ExprWithSuffix::Concat { target, suffix, .. } => {
                // Both operands are taken, the second from where the first
                // ended, and the two together are one token.
                Ok(seq_of(vec![self.operand(target)?, self.alt(suffix)?]))
            }
            ExprWithSuffix::Followed { target, suffix, .. } => Ok(Inst::Followed {
                item: Box::new(self.operand(target)?),
                next: Box::new(self.alt(suffix)?),
            }),
            ExprWithSuffix::Except { target, tokens, .. } => Ok(Inst::Except {
                item: Box::new(self.operand(target)?),
                removed: Box::new(self.operand(tokens)?),
            }),
        }
    }

    fn rep<I: UnsynInput>(
        &self,
        min: usize,
        max: Option<usize>,
        target: &ExprWithoutSuffix<I>,
        reported: &ExprWithSuffix<I>,
    ) -> Result<Inst, CodegenError> {
        let item = Box::new(self.operand(target)?);

        let inst = Inst::Rep { min, max, item };

        if inst.is_stuck(&self.refs.empty) {
            let name = inst
                .named()
                .unwrap_or("an operand that can match nothing")
                .to_owned();

            return Err(CodegenError {
                kind: CodegenKind::EmptyRepetition(name),
                span: reported.to_span(),
            });
        }

        Ok(inst)
    }

    /// The program of an operand that carries no suffix of its own, which is
    /// what a `syntax` rule needs when it asks a character-level question
    /// (`~ […]`, `[…]`) inside a pattern of nodes.
    pub(crate) fn operand<I: UnsynInput>(
        &self,
        expr: &ExprWithoutSuffix<I>,
    ) -> Result<Inst, CodegenError> {
        match expr {
            ExprWithoutSuffix::Str(lit) => literal(lit),
            ExprWithoutSuffix::Unicode(lit) => one_char(lit).map(Inst::Class),
            ExprWithoutSuffix::Ident(ident) => self.reference(ident.0.as_str(), &ident.to_span()),
            ExprWithoutSuffix::Call(angle) => Ok(Inst::Matcher(angle.body.0.as_str().to_owned())),
            ExprWithoutSuffix::Paren(paren) => self.expr(&paren.body),
            ExprWithoutSuffix::Set(bracket) => self.set_of(bracket),
            ExprWithoutSuffix::Tilde(_, bracket) => Ok(Inst::Not(Box::new(self.set_of(bracket)?))),
        }
    }

    /// The program a name stands for: the rule it names, or any single character
    /// for the wildcard.
    fn reference(&self, name: &str, span: &Span) -> Result<Inst, CodegenError> {
        if name == WILDCARD {
            return Ok(Inst::Class(Pred::Any));
        }

        if self.refs.tokens.contains(name) {
            return Ok(Inst::Ref(name.to_owned()));
        }

        if self.refs.syntaxes.contains(name) {
            return Err(CodegenError::not_a_token(
                name,
                "a token rule is written against text, not against nodes",
                span.clone(),
            ));
        }

        Err(CodegenError::unresolved(name, span.clone()))
    }

    /// The program of a set: one class of characters, plus whatever members
    /// reach further than one character or ask a question about the text.
    pub(crate) fn set_of<I: UnsynInput>(
        &self,
        bracket: &super::SetOf<I>,
    ) -> Result<Inst, CodegenError> {
        let mut steps: Vec<Inst> = vec![];

        for item in set_items(&bracket.body) {
            steps.push(self.set_item(item)?);
        }

        // A member that reaches further than one character is tried first,
        // because a set offers all of its members and taking less of the text
        // than it offers would be reading the pattern narrowly. `alt_of` then
        // folds the members that are one character each into the one class the
        // set is, and leaves it last of the branches.
        steps.sort_by_key(|step| std::cmp::Reverse(step.reaches()));

        Ok(alt_of(steps))
    }

    /// The program of one member of a set, which is a class where the member
    /// reads as one character and a step of its own otherwise.
    fn set_item<I: UnsynInput>(&self, item: &SetItem<I>) -> Result<Inst, CodegenError> {
        match item {
            SetItem::Str(lit) => literal(lit),
            SetItem::Unicode(lit) => one_char(lit).map(Inst::Class),
            SetItem::Range(range) => {
                let (start, end) = range_ends(range)?;

                Ok(Inst::Class(Pred::of_range(start, end)))
            }
            // A member that names a rule is that rule's program, which joins the
            // set's class only when it turns out to be one character.
            SetItem::Ident(ident) => self.reference(ident.0.as_str(), &ident.to_span()),
        }
    }
}

/// The program of a string literal, which is one character where it can be.
///
/// A literal that reads as a word is a keyword of the language being described,
/// and a keyword ends where an identifier would carry on: `'let'` must not match
/// the front of `letter`. The question is asked after the literal and takes
/// nothing, so it costs one probe of one character.
pub(crate) fn literal<I: UnsynInput>(lit: &LitStr<I>) -> Result<Inst, CodegenError> {
    let (text, error) = str_value(lit);

    if error.is_some() {
        return Err(CodegenError {
            kind: CodegenKind::Unsupported("a literal with an escape that decodes to nothing"),
            span: lit.to_span(),
        });
    }

    let matched = if text.chars().count() == 1 {
        Inst::Class(Pred::of(text.chars().next().expect("one character")))
    } else {
        Inst::Lit(text.clone())
    };

    if text.chars().count() > 1 && super::name::is_word(&text) {
        return Ok(Inst::Seq(vec![
            matched,
            Inst::NotFollowed(Box::new(Inst::Matcher(KEYWORD_END.to_owned()))),
        ]));
    }

    Ok(matched)
}

/// The matcher a keyword asks about to find where it ends, which is one of the
/// two the [`matcher`](crate::runtime::matcher) module gives without a backend.
const KEYWORD_END: &str = "XID_Continue";

/// The character a code point literal denotes.
fn one_char<I: UnsynInput>(lit: &LitUnicode<I>) -> Result<Pred, CodegenError> {
    unicode_value(lit).map(Pred::of).map_err(|_| CodegenError {
        kind: CodegenKind::Unsupported("a code point literal that denotes nothing"),
        span: lit.to_span(),
    })
}

/// The two ends of a range, both of which the analysis already asked to be
/// characters.
fn range_ends<I: UnsynInput>(range: &Range<I>) -> Result<(char, char), CodegenError> {
    let ends = |span: Span| -> Result<(char, char), CodegenError> {
        Err(CodegenError {
            kind: CodegenKind::Unsupported("a range whose end is not one character"),
            span,
        })
    };

    let (first, last) = match range {
        Range::Unicode(start, _, end) => {
            let (Ok(start), Ok(end)) = (unicode_value(start), unicode_value(end)) else {
                return ends(range.to_span());
            };

            (start, end)
        }
        Range::Str(start, _, end) => {
            let (Ok(start), Ok(end)) = (literal(start), literal(end)) else {
                return ends(range.to_span());
            };

            let (Inst::Class(Pred::Chars(start, _)), Inst::Class(Pred::Chars(end, _))) =
                (start, end)
            else {
                return ends(range.to_span());
            };

            match (start.as_slice(), end.as_slice()) {
                ([first], [last]) => (*first, *last),
                _ => return ends(range.to_span()),
            }
        }
    };

    if first > last {
        return Err(CodegenError {
            kind: CodegenKind::Unsupported("a range drawn backwards"),
            span: range.to_span(),
        });
    }

    Ok((first, last))
}

/// The bounds a repetition asks for.
pub(crate) fn repeat_bounds<I: UnsynInput>(repeat: &Repeat<I>) -> Option<(usize, Option<usize>)> {
    match repeat {
        Repeat::RangeTo(_, max) => Some((0, dec_usize(max))),
        Repeat::RangeFrom(min, _, None) => Some((dec_usize(min)?, None)),
        Repeat::RangeFrom(min, _, Some(max)) => Some((dec_usize(min)?, dec_usize(max))),
        Repeat::Count(count) => dec_usize(count).map(|count| (count, Some(count))),
    }
}

/// A sequence of steps, read plainly: nothing in it that takes nothing at all,
/// and one step only where that leaves one step.
pub(crate) fn seq_of(steps: Vec<Inst>) -> Inst {
    let steps: Vec<Inst> = steps
        .into_iter()
        .filter(|step| !matches!(step, Inst::Lit(text) if text.is_empty()))
        .collect();

    match steps.len() {
        0 => Inst::Lit(String::new()),
        1 => steps.into_iter().next().expect("one step"),
        _ => Inst::Seq(steps),
    }
}

/// An alternation of branches, read plainly: the branches that are classes
/// become one class, which is what a set of characters is.
pub(crate) fn alt_of(branches: Vec<Inst>) -> Inst {
    let mut classes: Vec<Pred> = vec![];
    let mut rest: Vec<Inst> = vec![];

    for branch in branches {
        match branch {
            Inst::Class(pred) => classes.push(pred),
            other => rest.push(other),
        }
    }

    if !classes.is_empty() {
        rest.push(Inst::Class(Pred::union(classes)));
    }

    match rest.len() {
        0 => Inst::Lit(String::new()),
        1 => rest.into_iter().next().expect("one branch"),
        _ => Inst::Alt(rest),
    }
}
