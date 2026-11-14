//! What a token rule matches: the character pattern a definition body compiles
//! to.
//!
//! A `lexer` definition is read against the source text rather than against
//! other nodes, so its body is compiled here into a small pattern over
//! characters ([`Step`]) before any code is emitted. That step exists so the
//! emission can be about running the pattern rather than about working out what
//! it means: the shortcuts of [`scan`](super::scan) are all justified by a
//! pattern that says the same thing more plainly, and the pattern is what the
//! tests of this module can compare against.
//!
//! The pattern keeps the difference the DSL draws between *taking* something and
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

use super::Paths;
use super::error::{CodegenError, CodegenKind};
use super::set_items;

/// The characters one step of a pattern may take.
///
/// A class is a value rather than a piece of code, which is what lets
/// `['a'-'z', 'A'-'Z']` and `U+0061-U+007A | U+0041-U+005A` be read as one
/// pattern, and lets a repetition of either be scanned in a single pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CharClass {
    /// Every character, which is what the wildcard `_` denotes.
    Any,
    /// No character at all.
    Nothing,
    /// Every character that may continue an identifier of the Unicode
    /// standard.
    ///
    /// This is the one class the generator asks of itself rather than of a
    /// caller: it ends every word-like literal, so that `'let'` is refused
    /// inside `letter`, and a spec that never wrote a `<NAME>` has that
    /// ending all the same. It is therefore written through the `unsyn` path
    /// of a generation, and never through the path a table settles, which is
    /// what keeps an empty table meaning no unmapped names.
    XidContinue,
    /// Exactly these characters and these intervals of them.
    Chars(Vec<char>, Vec<(char, char)>),
    /// Whatever both sides take.
    And(Box<CharClass>, Box<CharClass>),
    /// Whatever either side takes.
    Or(Box<CharClass>, Box<CharClass>),
    /// Every character the operand does not take.
    Not(Box<CharClass>),
}

impl CharClass {
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
    pub(crate) fn union(classes: impl IntoIterator<Item = CharClass>) -> CharClass {
        classes
            .into_iter()
            .fold(CharClass::Nothing, |acc, next| acc.or(next))
    }

    /// This class's complement.
    pub(crate) fn negated(self) -> CharClass {
        match self {
            // `_` complemented is nothing, and a double complement cancels: both
            // read better composed than as `!Any` or `!!class`.
            CharClass::Any => CharClass::Nothing,
            CharClass::Nothing => CharClass::Any,
            CharClass::Not(inner) => *inner,
            other => CharClass::Not(Box::new(other)),
        }
    }

    pub(crate) fn and(self, other: CharClass) -> CharClass {
        match (self, other) {
            (CharClass::Nothing, _) | (_, CharClass::Nothing) => CharClass::Nothing,
            (CharClass::Any, other) | (other, CharClass::Any) => other,
            (first, second) => CharClass::And(Box::new(first), Box::new(second)),
        }
    }

    pub(crate) fn or(self, other: CharClass) -> CharClass {
        match (self, other) {
            (CharClass::Any, _) | (_, CharClass::Any) => CharClass::Any,
            (CharClass::Nothing, other) | (other, CharClass::Nothing) => other,
            (first, second) => CharClass::Or(Box::new(first), Box::new(second)),
        }
    }

    /// This class as a condition over one character named `c`.
    pub(crate) fn render(&self, paths: &Paths) -> proc_macro2::TokenStream {
        use proc_macro2::{Literal, TokenStream};
        use quote::quote;

        match self {
            CharClass::Any => quote!(true),
            CharClass::Nothing => quote!(false),
            CharClass::XidContinue => {
                let unsyn = &paths.unsyn;

                quote!(#unsyn::runtime::keyword::is_xid_continue(c))
            }
            CharClass::Chars(chars, ranges) => {
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
            CharClass::And(first, second) => {
                let (first, second) = (first.render(paths), second.render(paths));

                quote!((#first && #second))
            }
            CharClass::Or(first, second) => {
                let (first, second) = (first.render(paths), second.render(paths));

                quote!((#first || #second))
            }
            CharClass::Not(inner) => {
                let inner = inner.render(paths);

                quote!(!#inner)
            }
        }
    }

    /// This class as the condition a scan is handed.
    ///
    /// A class that takes every character is written over a nameless character,
    /// since the question it answers does not turn on the one it is asked about
    /// and a generated file should not warn about a binding it never reads.
    pub(crate) fn closure(&self, paths: &Paths) -> proc_macro2::TokenStream {
        use quote::quote;

        let body = self.render(paths);

        match self {
            CharClass::Any => quote!(|_: char| #body),
            _ => quote!(|c: char| #body),
        }
    }
}

/// One step of a token pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Step {
    /// Take one character of this class.
    Class(CharClass),
    /// Take exactly these characters, in this order.
    Lit(String),
    /// Take what an external of the caller's table takes, which is at least one
    /// character and as many as it likes.
    ///
    /// The [`Span`] is where the `<NAME>` was written, kept so that a name no
    /// table was given can be reported against the source rather than as a bare
    /// refusal. Only a `<NAME>` of the spec ever makes one: the generator asks
    /// nothing of a caller's table on its own account.
    External(String, Span),
    /// Take what the token rule of that name takes.
    Ref(String),
    /// Take what each step takes, in order.
    Seq(Vec<Step>),
    /// Take what the first alternative that fits takes.
    Alt(Vec<Step>),
    /// Take the step as often as the text allows, at least `min` times and at
    /// most `max`.
    Rep {
        /// The least number of times the step is taken.
        min: usize,
        /// The most number of times it is taken, or nothing when the text decides.
        max: Option<usize>,
        /// What is repeated.
        item: Box<Step>,
    },
    /// Fail when the set fits here, and take one character when it does not.
    Not(Box<Step>),
    /// Take the operand unless what removes it fits where the operand starts.
    Except {
        /// What is taken.
        item: Box<Step>,
        /// What is taken out of it, tested at the start of the operand.
        removed: Box<Step>,
    },
    /// Take the operand, and ask that what follows it come next without being
    /// taken here.
    Followed {
        /// What is taken.
        item: Box<Step>,
        /// What has to come after it.
        next: Box<Step>,
    },
    /// Fail when the operand fits next, and take nothing when it does not.
    ///
    /// This is what keeps a keyword from matching the front of a longer word:
    /// `'let'` is `'let'` only where an `XID_Continue` character does not
    /// follow, and the question is asked about the text without taking any of
    /// it, which is what separates it from `~`.
    NotFollowed(Box<Step>),
}

impl Step {
    /// Returns `true` when this step can take nothing at all.
    fn takes_nothing(&self, empty: &HashSet<String>) -> bool {
        match self {
            Step::Class(_) | Step::External(..) => false,
            Step::Lit(text) => text.is_empty(),
            Step::Ref(name) => empty.contains(name.as_str()),
            Step::Seq(steps) => steps.iter().all(|step| step.takes_nothing(empty)),
            Step::Alt(steps) => steps.iter().any(|step| step.takes_nothing(empty)),
            Step::Rep { min, item, .. } => *min == 0 && item.takes_nothing(empty),
            // A complement always takes the one character it was asked about, the
            // other two always take their operand, and a lookahead takes nothing.
            Step::Not(_) | Step::Except { .. } | Step::Followed { .. } => false,
            Step::NotFollowed(_) => true,
        }
    }

    /// Returns `true` when repeating this step would match at one position
    /// forever.
    fn is_stuck(&self, empty: &HashSet<String>) -> bool {
        match self {
            Step::Rep { max, item, .. } => {
                // Only an unbounded repetition can turn forever: a bound is
                // counted out of the loop, however the item behaves, and `X?` is
                // tried once. An unbounded one is entered again whenever its
                // item fits, which it always does when the item takes nothing.
                max.is_none() && item.takes_nothing(empty)
            }
            Step::Seq(steps) | Step::Alt(steps) => steps.iter().any(|step| step.is_stuck(empty)),
            // A lookahead never moves, but it is also never what a repetition
            // repeats: `~` and the suffixes only ever sit on a set or an
            // operand, and the wildcard below already answers for it.
            Step::Not(item) => item.is_stuck(empty),
            Step::Except { item, .. } | Step::Followed { item, .. } => item.is_stuck(empty),
            _ => false,
        }
    }

    /// The name a stuck repetition should be reported under: the rule it names,
    /// where one is reached, or nothing when the operand is written out.
    fn named(&self) -> Option<&str> {
        match self {
            Step::Ref(name) => Some(name),
            Step::Seq(steps) | Step::Alt(steps) => steps.iter().find_map(Step::named),
            Step::Rep { item, .. } => item.named(),
            _ => None,
        }
    }

    /// How far this step reaches into the text at the least, which is how the
    /// members of a set are ordered: the longer one first, so that a set never
    /// reads less of the text than it was offered.
    fn reaches(&self) -> usize {
        match self {
            Step::Lit(text) => text.chars().count(),
            Step::Seq(steps) => steps.iter().map(Step::reaches).sum(),
            Step::Alt(steps) => steps.iter().map(Step::reaches).min().unwrap_or_default(),
            Step::Rep { min, item, .. } => *min * item.reaches(),
            Step::Except { item, .. } | Step::Followed { item, .. } => item.reaches(),
            // A lookahead promises nothing about the text it does not take.
            Step::NotFollowed(_) => 0,
            // A class, an external, a reference or a complement: one character is
            // all any of them promises, since none of them is measured here.
            _ => 1,
        }
    }

    /// The class this step takes, when it takes exactly one character and
    /// nothing else about the text decides the answer.
    pub(crate) fn char_class(&self, classes: &HashMap<String, CharClass>) -> Option<CharClass> {
        match self {
            Step::Class(class) => Some(class.clone()),
            Step::Lit(text) => {
                let mut chars = text.chars();

                match (chars.next(), chars.next()) {
                    (Some(one), None) => Some(CharClass::of(one)),
                    _ => None,
                }
            }
            Step::Ref(name) => classes.get(name).cloned(),
            // An external's length is its own business, so it is never a class.
            Step::External(..) => None,
            Step::Seq(steps) => steps
                .first()
                .filter(|_| steps.len() == 1)
                .and_then(|step| step.char_class(classes)),
            Step::Alt(steps) => steps
                .iter()
                .map(|step| step.char_class(classes))
                .collect::<Option<Vec<_>>>()
                .map(CharClass::union),
            Step::Not(item) => item.char_class(classes).map(CharClass::negated),
            Step::Rep { .. } => None,
            Step::Except { item, removed } => {
                // Removing is a complement of one character only when what is
                // removed is one character too: `'a'-'c' except 'b'` is a class,
                // while `IDENTIFIER except KEYWORDS` turns on the text that
                // follows the character being decided on.
                let item = item.char_class(classes)?;
                let removed = removed.char_class(classes)?;

                Some(item.and(removed.negated()))
            }
            // An assertion about what follows is not a question about one
            // character.
            Step::Followed { .. } | Step::NotFollowed(_) => None,
        }
    }
}

/// What a token pattern needs to know about the names it refers to.
#[derive(Debug, Default)]
pub(crate) struct RuleNames {
    /// The names of rules that read text, which a pattern may step over.
    pub(crate) tokens: HashSet<String>,
    /// The names of node rules, which a token cannot be built out of.
    pub(crate) syntaxes: HashSet<String>,
    /// The names of the rules that can take nothing, whose unbounded repetition
    /// would never move forward.
    pub(crate) empty: HashSet<String>,
}

/// One token type to write: its name and the pattern its `parse` runs.
pub(crate) struct TokenPattern {
    /// The name of the type.
    pub(crate) name: String,
    /// The pattern of the rule it was built from.
    pub(crate) step: Step,
}

impl TokenPattern {
    /// A token type from the pattern it runs.
    pub(crate) fn new(name: impl Into<String>, step: Step) -> Self {
        Self {
            name: name.into(),
            step,
        }
    }
}

/// Builds the pattern of a token pattern out of the CST of a definition body.
pub(crate) struct PatternBuilder<'a> {
    refs: &'a RuleNames,
}

impl<'a> PatternBuilder<'a> {
    /// Prepares a builder over the names `refs` describes.
    pub(crate) fn new(refs: &'a RuleNames) -> Self {
        Self { refs }
    }

    /// The pattern of a whole definition body.
    pub(crate) fn expr<I: UnsynInput>(&self, expr: &Expr<I>) -> Result<Step, CodegenError> {
        let mut branches = vec![self.alts(&expr.first)?];

        for (_, alts) in expr.rest.iter() {
            branches.push(self.alts(alts)?);
        }

        Ok(alternative_of(branches))
    }

    fn alts<I: UnsynInput>(&self, seq: &ExprNoTopAlts<I>) -> Result<Step, CodegenError> {
        let mut steps = vec![self.alt(&seq.first)?];

        for (_, expr) in seq.rest.iter() {
            steps.push(self.alt(expr)?);
        }

        Ok(sequence_of(steps))
    }

    fn alt<I: UnsynInput>(&self, expr: &ExprNoTopAlt<I>) -> Result<Step, CodegenError> {
        match expr {
            ExprNoTopAlt::WithSuffix(expr) => self.with_suffix(expr),
            ExprNoTopAlt::WithoutSuffix(expr) => self.operand(expr),
        }
    }

    fn with_suffix<I: UnsynInput>(&self, expr: &ExprWithSuffix<I>) -> Result<Step, CodegenError> {
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
                Ok(sequence_of(vec![self.operand(target)?, self.alt(suffix)?]))
            }
            ExprWithSuffix::Followed { target, suffix, .. } => Ok(Step::Followed {
                item: Box::new(self.operand(target)?),
                next: Box::new(self.alt(suffix)?),
            }),
            ExprWithSuffix::Except { target, tokens, .. } => Ok(Step::Except {
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
    ) -> Result<Step, CodegenError> {
        let item = Box::new(self.operand(target)?);

        let step = Step::Rep { min, max, item };

        if step.is_stuck(&self.refs.empty) {
            let name = step
                .named()
                .unwrap_or("an operand that can match nothing")
                .to_owned();

            return Err(CodegenError {
                kind: CodegenKind::EmptyRepetition(name),
                span: reported.to_span(),
            });
        }

        Ok(step)
    }

    /// The pattern of an operand that carries no suffix of its own, which is
    /// what a `syntax` rule needs when it asks a character-level question
    /// (`~ […]`, `[…]`) inside a pattern of nodes.
    pub(crate) fn operand<I: UnsynInput>(
        &self,
        expr: &ExprWithoutSuffix<I>,
    ) -> Result<Step, CodegenError> {
        match expr {
            ExprWithoutSuffix::Str(lit) => literal(lit),
            ExprWithoutSuffix::Unicode(lit) => one_char(lit).map(Step::Class),
            ExprWithoutSuffix::Ident(ident) => self.reference(ident.0.as_str(), &ident.to_span()),
            ExprWithoutSuffix::Call(angle) => Ok(Step::External(
                angle.body.0.as_str().to_owned(),
                angle.to_span(),
            )),
            ExprWithoutSuffix::Paren(paren) => self.expr(&paren.body),
            ExprWithoutSuffix::Set(bracket) => self.set_of(bracket),
            ExprWithoutSuffix::Tilde(_, bracket) => Ok(Step::Not(Box::new(self.set_of(bracket)?))),
        }
    }

    /// The pattern a name stands for: the rule it names, or any single character
    /// for the wildcard.
    fn reference(&self, name: &str, span: &Span) -> Result<Step, CodegenError> {
        if name == WILDCARD {
            return Ok(Step::Class(CharClass::Any));
        }

        if self.refs.tokens.contains(name) {
            return Ok(Step::Ref(name.to_owned()));
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

    /// The pattern of a set: one class of characters, plus whatever members
    /// reach further than one character or ask a question about the text.
    pub(crate) fn set_of<I: UnsynInput>(
        &self,
        bracket: &super::SetOf<I>,
    ) -> Result<Step, CodegenError> {
        let mut steps: Vec<Step> = vec![];

        for item in set_items(&bracket.body) {
            steps.push(self.set_item(item)?);
        }

        // A member that reaches further than one character is tried first,
        // because a set offers all of its members and taking less of the text
        // than it offers would be reading the pattern narrowly. `alternative_of` then
        // folds the members that are one character each into the one class the
        // set is, and leaves it last of the branches.
        steps.sort_by_key(|step| std::cmp::Reverse(step.reaches()));

        Ok(alternative_of(steps))
    }

    /// The pattern of one member of a set, which is a class where the member
    /// reads as one character and a step of its own otherwise.
    fn set_item<I: UnsynInput>(&self, item: &SetItem<I>) -> Result<Step, CodegenError> {
        match item {
            SetItem::Str(lit) => literal(lit),
            SetItem::Unicode(lit) => one_char(lit).map(Step::Class),
            SetItem::Range(range) => {
                let (start, end) = range_ends(range)?;

                Ok(Step::Class(CharClass::of_range(start, end)))
            }
            // A member that names a rule is that rule's pattern, which joins the
            // set's class only when it turns out to be one character.
            SetItem::Ident(ident) => self.reference(ident.0.as_str(), &ident.to_span()),
        }
    }
}

/// The pattern of a string literal, which is one character where it can be.
///
/// A literal that reads as a word is a keyword of the language being described,
/// and a keyword ends where an identifier would carry on: `'let'` must not match
/// the front of `letter`. The question is asked after the literal and takes
/// nothing, and it is a question about one character, so it costs one probe.
pub(crate) fn literal<I: UnsynInput>(lit: &LitStr<I>) -> Result<Step, CodegenError> {
    let (text, error) = str_value(lit);

    if error.is_some() {
        return Err(CodegenError {
            kind: CodegenKind::Unsupported("a literal with an escape that decodes to nothing"),
            span: lit.to_span(),
        });
    }

    let matched = if text.chars().count() == 1 {
        Step::Class(CharClass::of(text.chars().next().expect("one character")))
    } else {
        Step::Lit(text.clone())
    };

    if text.chars().count() > 1 && super::name::is_word(&text) {
        return Ok(Step::Seq(vec![
            matched,
            Step::NotFollowed(Box::new(Step::Class(CharClass::XidContinue))),
        ]));
    }

    Ok(matched)
}

/// The character a code point literal denotes.
fn one_char<I: UnsynInput>(lit: &LitUnicode<I>) -> Result<CharClass, CodegenError> {
    unicode_value(lit)
        .map(CharClass::of)
        .map_err(|_| CodegenError {
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

            let (Step::Class(CharClass::Chars(start, _)), Step::Class(CharClass::Chars(end, _))) =
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
pub(crate) fn sequence_of(steps: Vec<Step>) -> Step {
    let steps: Vec<Step> = steps
        .into_iter()
        .filter(|step| !matches!(step, Step::Lit(text) if text.is_empty()))
        .collect();

    match steps.len() {
        0 => Step::Lit(String::new()),
        1 => steps.into_iter().next().expect("one step"),
        _ => Step::Seq(steps),
    }
}

/// An alternation of branches, read plainly: the branches that are classes
/// become one class, which is what a set of characters is.
pub(crate) fn alternative_of(branches: Vec<Step>) -> Step {
    let mut classes: Vec<CharClass> = vec![];
    let mut rest: Vec<Step> = vec![];

    for branch in branches {
        match branch {
            Step::Class(class) => classes.push(class),
            other => rest.push(other),
        }
    }

    if !classes.is_empty() {
        rest.push(Step::Class(CharClass::union(classes)));
    }

    match rest.len() {
        0 => Step::Lit(String::new()),
        1 => rest.into_iter().next().expect("one branch"),
        _ => Step::Alt(rest),
    }
}
