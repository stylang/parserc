//! The type a `syntax` rule leaves behind: the fields it holds.
//!
//! A node definition is read against other nodes rather than against characters,
//! so its body is not compiled to a pattern over characters but to the *type* of
//! the node it reads: one field per element of the sequence the rule is, with
//! the repetitions and the assertions the rule asks for written into the type of
//! the field rather than into the code that reads it ([`FieldType`]).
//!
//! Four things make part of a rule need a type of its own rather than a field:
//!
//! - a repetition counted out (`X{n}`, `X{n}..{m}`), whose bound the code that
//!   reads it has to check, which is [`TypeDefKind::Bounds`];
//! - an operand with a question asked about the text around it (`except`,
//!   `followed`), which is [`TypeDefKind::Asserted`];
//! - a group with more than one alternative, which is a
//!   [`TypeDefKind::Enum`] of its own, named after the rule it was written in;
//! - anything a rule reaches through more than sixteen operands, since the
//!   shortcut of [`FieldType::Concat`] is a tuple and a tuple stops at sixteen.
//!
//! # Whitespace
//!
//! The language a file describes is read with runs of trivia between its tokens,
//! and a generated token type keeps only the text it matched: none of them takes
//! the trivia around itself. Every element of a sequence after the first is
//! therefore reached over a [`Field::separator`], which is the whitespace type
//! the file declares, and a repetition takes one between each two of its runs.
//! A separator is a run rather than one piece of trivia, so that it takes as
//! much of it as the text offers and none at all where two tokens are written
//! together.
//!
//! `concat` is the exception the DSL makes, and the only place the rule does not
//! hold: it asks for both operands as one token, with nothing between, which is
//! why it is [`FieldType::Concat`] and not a field and a separator.
//!
//! A file that declares no `whitespace` rule has no trivia to take, so it has no
//! separators at all.
//!
//! A rule may still write its separator out, as `syntax Pair -> A WS B;` does
//! when `WS` is the trivia node. Such a bare mention of the trivia asks for
//! nothing the run between the operands around it does not already take, so it
//! is folded away and earns no field: the type is the one
//! `syntax Pair -> A B;` makes, and the [analysis](crate::semantics) notes the
//! mention. At either end of a sequence the mention is folded for the same
//! reason: the trivia before the first operand of a rule belongs to the
//! separator of the rule around it, which reaches this one just as greedily.
//! A mention with a suffix on it (`WS?`, `WS*`) is not folded, since it asks
//! for a repetition a separator only happens to be; a sequence of nothing but
//! mentions is a rule *about* the trivia, and keeps them all.

use std::collections::{HashMap, HashSet};

use parserc::syntax::Syntax;

use crate::input::UnsynInput;
use crate::semantics::WILDCARD;
use crate::syntax::{Expr, ExprNoTopAlt, ExprNoTopAlts, ExprWithSuffix, ExprWithoutSuffix};

use super::error::{CodegenError, CodegenKind};
use super::name::{Names, keyword_name, literal_name, punct_name};
use super::pattern::{CharClass, PatternBuilder, RuleNames, Step, TokenPattern};

/// The most operands a sequence may be folded into, which is how many elements
/// a tuple of the backend has.
const MAX_TUPLE: usize = 16;

/// One type codegen writes for a node rule, or for a construct inside one.
pub(crate) struct TypeDef {
    /// The name of the type.
    pub(crate) name: String,
    /// What its fields are.
    pub(crate) kind: TypeDefKind,
}

/// The body of a shape, which decides the declaration its type is given.
pub(crate) enum TypeDefKind {
    /// A tuple struct of fields, which is what a rule of one alternative is.
    Struct {
        /// The elements of the sequence, in the order they are taken.
        fields: Vec<Field>,
    },
    /// An enum of tuple variants, which is what a rule of several alternatives
    /// is: each alternative of the rule is one variant of the type.
    Enum {
        /// The alternatives, in the order they are written.
        variants: Vec<Variant>,
    },
    /// One operand taken as often as the rule counts out, which is a struct of
    /// the run it matched and a check that the run is long enough.
    Bounds {
        /// What each repetition takes.
        item: FieldType,
        /// The least number of times it is taken.
        min: usize,
        /// The most, or nothing when the text decides.
        max: Option<usize>,
        /// Whether the runs of trivia between the repetitions are part of it.
        separated: bool,
    },
    /// One operand with a question asked about the text around it.
    Asserted {
        /// What is taken.
        item: FieldType,
        /// The question, and where it is asked.
        assert: Assert,
    },
}

/// One alternative of a rule that has several.
pub(crate) struct Variant {
    /// The name of the variant, which comes from what the alternative is built
    /// on: the node or literal it starts with, where there is one to read.
    pub(crate) name: String,
    /// The elements of the alternative's sequence.
    pub(crate) fields: Vec<Field>,
}

/// The question an asserted operand asks.
pub(crate) enum Assert {
    /// The operand is taken unless what removes it fits where the operand does.
    Except(FieldType),
    /// The operand is taken only where what follows it fits next.
    Followed(FieldType),
}

/// One field of a def.
pub(crate) struct Field {
    /// What the field holds.
    pub(crate) ty: FieldType,
    /// Whether a run of trivia is taken before it, which is needed when neither
    /// the element before it nor this one carries the whitespace itself.
    pub(crate) separator: bool,
}

/// What a field holds, written so that taking it is one call of
/// [`parse`](parserc::syntax::Syntax::parse) whatever the field's type holds:
/// the tuple and the collection are both syntax of their own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FieldType {
    /// The type of one named rule, or of one the builder wrote itself.
    Named(String),
    /// `T?`, which is the operand taken where it fits.
    Opt(Box<FieldType>),
    /// `T*`, which is the run of them the text offers.
    Many {
        /// What each takes.
        item: Box<FieldType>,
        /// Whether the trivia between two of them is part of the run.
        separated: bool,
    },
    /// `T+`, which is one and then as many more as there are.
    OneOrMore {
        /// What each takes.
        item: Box<FieldType>,
        /// Whether the trivia between two of them is part of the run.
        separated: bool,
    },
    /// `A concat B`, which is both taken back to back with nothing between.
    Concat {
        /// What comes first.
        first: Box<FieldType>,
        /// What comes next.
        second: Box<FieldType>,
    },
}

/// The types of a sequence of fields, with whatever each of them holds.
fn fields_of(fields: &[Field], out: &mut Vec<String>) {
    for field in fields {
        field.ty.names(out);
    }
}

impl TypeDef {
    /// The types a rule's type names, which are the edges of the recursion it may
    /// take part in: a rule that names itself, or names its way back to itself,
    /// has to hold what it names behind a pointer.
    pub(crate) fn names(&self, out: &mut Vec<String>) {
        match &self.kind {
            TypeDefKind::Struct { fields } => fields_of(fields, out),
            TypeDefKind::Enum { variants } => {
                for variant in variants {
                    fields_of(&variant.fields, out);
                }
            }
            TypeDefKind::Bounds { item, .. } => item.names(out),
            TypeDefKind::Asserted { item, assert } => {
                item.names(out);

                match assert {
                    Assert::Except(removed) => removed.names(out),
                    Assert::Followed(next) => next.names(out),
                }
            }
        }
    }
}

impl FieldType {
    /// Returns `true` when this type can be taken without moving the text.
    pub(crate) fn takes_nothing(&self, empty: &HashSet<String>) -> bool {
        match self {
            FieldType::Named(name) => empty.contains(name),
            FieldType::Opt(..) | FieldType::Many { .. } => true,
            FieldType::OneOrMore { item, .. } => item.takes_nothing(empty),
            FieldType::Concat { first, second, .. } => {
                first.takes_nothing(empty) && second.takes_nothing(empty)
            }
        }
    }

    /// What the type is called in the report about it.
    pub(crate) fn describe(&self) -> String {
        match self {
            FieldType::Named(name) => name.clone(),
            FieldType::Opt(item) => format!("{}?", item.describe()),
            FieldType::Many { item, .. } => format!("{}*", item.describe()),
            FieldType::OneOrMore { item, .. } => format!("{}+", item.describe()),
            FieldType::Concat { first, second } => {
                format!("{} concat {}", first.describe(), second.describe())
            }
        }
    }

    /// The names of the types this one is built out of, which is what decides
    /// whether it has to be held behind a pointer.
    pub(crate) fn names(&self, out: &mut Vec<String>) {
        match self {
            FieldType::Named(name) => out.push(name.clone()),
            FieldType::Opt(item)
            | FieldType::Many { item, .. }
            | FieldType::OneOrMore { item, .. } => {
                item.names(out);
            }
            FieldType::Concat { first, second, .. } => {
                first.names(out);
                second.names(out);
            }
        }
    }
}

/// Builds the types of a file's node rules.
///
/// The builder owns the names it hands out and the types it had to write for
/// itself, so a file's rules can be shaped one at a time while the anonymous
/// types they need are collected in one place.
pub(crate) struct TypeBuilder {
    /// The names already given, so that none is given twice.
    names: Names,
    /// How many anonymous types each rule has needed so far.
    counters: HashMap<String, usize>,
    /// The token rules, which a node may name.
    tokens: HashSet<String>,
    /// The node rules, which a node may name.
    syntaxes: HashSet<String>,
    /// The rules that can take nothing, which decides a repetition that sticks.
    empty: HashSet<String>,
    /// The name of the whitespace type, where the file declares one.
    whitespace: Option<String>,
    /// The type each construct that is shared between rules has already been
    /// given, keyed by the name it is shared under: an external call and the
    /// wildcard, each of which is the same pattern wherever it is written.
    shared: HashMap<String, String>,
    /// The type each literal has already been given, keyed by the text it
    /// matches, which is what makes `'if'` here and `'if'` there one token.
    literals: HashMap<String, String>,
    /// The same for the code points, keyed by the one character they are.
    code_points: HashMap<char, String>,
    /// The variant names already given for one owner, which are the names only
    /// the alternatives of one rule have to keep apart from each other.
    variants: HashMap<String, std::collections::HashSet<String>>,
    /// The anonymous types of a rule that are separated runs, which carry the
    /// trivia before their own elements.
    runs: HashSet<String>,
    /// The patterns of the token types the types needed, which are written out
    /// beside the rules of the file.
    anon_tokens: Vec<TokenPattern>,
    /// The types written for a construct inside a rule, which are emitted with
    /// the rule that needed them.
    pub(crate) pending: Vec<TypeDef>,
}

impl TypeBuilder {
    /// Prepares a builder over the names `refs` describes, with the definitions'
    /// own `reserved` names already claimed and `whitespace` the type of the
    /// trivia where the file declares any.
    pub(crate) fn new(
        refs: &RuleNames,
        reserved: impl Iterator<Item = String>,
        whitespace: Option<String>,
    ) -> Self {
        let reserved: Vec<String> = reserved.collect();
        let names = Names::with_reserved(reserved.iter().map(String::as_str));

        Self {
            names,
            counters: HashMap::new(),
            tokens: refs.tokens.clone(),
            syntaxes: refs.syntaxes.clone(),
            empty: refs.empty.clone(),
            whitespace,
            shared: HashMap::new(),
            literals: HashMap::new(),
            code_points: HashMap::new(),
            runs: HashSet::new(),
            variants: HashMap::new(),
            anon_tokens: Vec::new(),
            pending: Vec::new(),
        }
    }

    /// The type of one node rule, named `owner`, whose body is `expr`.
    ///
    /// Any type the rule needed written for itself comes out of the builder with
    /// it, in [`TypeBuilder::pending`].
    pub(crate) fn type_def<I: UnsynInput>(
        &mut self,
        owner: &str,
        expr: &Expr<I>,
    ) -> Result<TypeDef, CodegenError> {
        Ok(TypeDef {
            name: owner.to_owned(),
            kind: self.kind(owner, expr)?,
        })
    }

    /// The body of a rule's type: the sum the top-level `|` makes, or the
    /// product of the rule's one sequence.
    fn kind<I: UnsynInput>(
        &mut self,
        owner: &str,
        expr: &Expr<I>,
    ) -> Result<TypeDefKind, CodegenError> {
        if expr.rest.is_empty() {
            return Ok(TypeDefKind::Struct {
                fields: self.fields_of_sequence(owner, &expr.first)?,
            });
        }

        self.enum_of(owner, expr)
    }

    /// The fields of one sequence of operands, each after the first reached over
    /// a run of trivia.
    ///
    /// A bare mention of the trivia node of the file is that run rather than an
    /// operand of its own, so it is folded away before the fields are counted
    /// (see [`ExprNoTopAlts::fold_separators`]); the analysis notes the mention
    /// from the same reading.
    fn fields_of_sequence<I: UnsynInput>(
        &mut self,
        owner: &str,
        seq: &ExprNoTopAlts<I>,
    ) -> Result<Vec<Field>, CodegenError> {
        let (elements, _) = seq.fold_separators(self.whitespace.as_deref());

        let mut fields = Vec::with_capacity(elements.len());

        for expr in elements {
            fields.push(self.field(owner, expr)?);
        }

        // The first field of a type is reached at the position its caller left
        // it at, so it never takes a run of trivia of its own. A field that is
        // a separated run does not need one either: the run carries the trivia
        // before each of its elements, and where the run takes nothing the
        // trivia belongs to the field after it rather than being eaten here.
        for (index, field) in fields.iter_mut().enumerate() {
            field.separator =
                index > 0 && self.whitespace.is_some() && !self.opens_with_a_separator(&field.ty);
        }

        Ok(fields)
    }

    /// Returns `true` when a field is a run that takes the trivia before each
    /// of its elements itself, which is what makes a separator in front of it
    /// both needless and wrong: it would be taken even when the run takes
    /// nothing at all.
    fn opens_with_a_separator(&self, ty: &FieldType) -> bool {
        match ty {
            FieldType::Many { separated, .. } | FieldType::OneOrMore { separated, .. } => {
                *separated
            }
            FieldType::Named(name) => self.runs.contains(name),
            _ => false,
        }
    }

    /// The field one operand of a sequence makes.
    fn field<I: UnsynInput>(
        &mut self,
        owner: &str,
        expr: &ExprNoTopAlt<I>,
    ) -> Result<Field, CodegenError> {
        Ok(Field {
            ty: self.ty(owner, expr)?,
            separator: false,
        })
    }

    /// The type one operand is held as.
    fn ty<I: UnsynInput>(
        &mut self,
        owner: &str,
        expr: &ExprNoTopAlt<I>,
    ) -> Result<FieldType, CodegenError> {
        match expr {
            ExprNoTopAlt::WithSuffix(expr) => self.with_suffix(owner, expr),
            ExprNoTopAlt::WithoutSuffix(expr) => self.leaf(owner, expr),
        }
    }

    /// The type of an operand with a suffix on it.
    fn with_suffix<I: UnsynInput>(
        &mut self,
        owner: &str,
        expr: &ExprWithSuffix<I>,
    ) -> Result<FieldType, CodegenError> {
        match expr {
            ExprWithSuffix::Star(target, _) => self.repeated(target, 0, None, owner, expr),
            ExprWithSuffix::Question(target, _) => {
                Ok(FieldType::Opt(Box::new(self.leaf(owner, target)?)))
            }
            ExprWithSuffix::Plus(target, _) => self.repeated(target, 1, None, owner, expr),
            ExprWithSuffix::Repeat { target, suffix } => {
                let (min, max) =
                    super::pattern::repeat_bounds(&suffix.body).ok_or_else(|| CodegenError {
                        kind: CodegenKind::InvalidRepeat,
                        span: expr.to_span(),
                    })?;

                self.counted(target, min, max, owner, &expr.to_span())
            }
            ExprWithSuffix::Concat { target, suffix, .. } => Ok(FieldType::Concat {
                first: Box::new(self.leaf(owner, target)?),
                second: Box::new(self.ty(owner, suffix)?),
            }),
            ExprWithSuffix::Followed { target, suffix, .. } => {
                let item = self.leaf(owner, target)?;
                let next = self.ty(owner, suffix)?;

                Ok(FieldType::Named(self.anonymous(
                    owner,
                    TypeDefKind::Asserted {
                        item,
                        assert: Assert::Followed(next),
                    },
                )))
            }
            ExprWithSuffix::Except { target, tokens, .. } => {
                let item = self.leaf(owner, target)?;
                let removed = self.leaf(owner, tokens)?;

                Ok(FieldType::Named(self.anonymous(
                    owner,
                    TypeDefKind::Asserted {
                        item,
                        assert: Assert::Except(removed),
                    },
                )))
            }
        }
    }

    /// The type of a repetition whose bounds the text decides, which is one of
    /// the types of [`FieldType`] where the count is only an upper bound or nothing.
    fn repeated<I: UnsynInput>(
        &mut self,
        target: &ExprWithoutSuffix<I>,
        min: usize,
        max: Option<usize>,
        owner: &str,
        reported: &ExprWithSuffix<I>,
    ) -> Result<FieldType, CodegenError> {
        let item = self.leaf(owner, target)?;

        if item.takes_nothing(&self.empty) && max.is_none() {
            return Err(CodegenError {
                kind: CodegenKind::EmptyRepetition(item.describe()),
                span: reported.to_span(),
            });
        }

        let separated = self.whitespace.is_some();

        match (min, max) {
            (0, None) => Ok(FieldType::Many {
                item: Box::new(item),
                separated,
            }),
            (1, None) => Ok(FieldType::OneOrMore {
                item: Box::new(item),
                separated,
            }),
            _ => {
                let name = self.counted_type(item, min, max, separated, owner);

                Ok(FieldType::Named(name))
            }
        }
    }

    /// The type of a repetition whose count the rule states, which is a run of
    /// its operand and a check that the run came out long enough.
    fn counted<I: UnsynInput>(
        &mut self,
        target: &ExprWithoutSuffix<I>,
        min: usize,
        max: Option<usize>,
        owner: &str,
        span: &parserc::Span,
    ) -> Result<FieldType, CodegenError> {
        let item = self.leaf(owner, target)?;

        if item.takes_nothing(&self.empty) {
            return Err(CodegenError {
                kind: CodegenKind::EmptyRepetition(item.describe()),
                span: span.clone(),
            });
        }

        let separated = self.whitespace.is_some();

        Ok(FieldType::Named(
            self.counted_type(item, min, max, separated, owner),
        ))
    }

    /// Writes the type of a counted run and returns the name it was given.
    fn counted_type(
        &mut self,
        item: FieldType,
        min: usize,
        max: Option<usize>,
        separated: bool,
        owner: &str,
    ) -> String {
        let name = self.anonymous(
            owner,
            TypeDefKind::Bounds {
                item,
                min,
                max,
                separated,
            },
        );

        if separated {
            self.runs.insert(name.clone());
        }

        name
    }

    /// The type of an operand with no suffix on it.
    fn leaf<I: UnsynInput>(
        &mut self,
        owner: &str,
        expr: &ExprWithoutSuffix<I>,
    ) -> Result<FieldType, CodegenError> {
        match expr {
            // A literal is a token of its own, shared between every rule that
            // writes it the same way.
            ExprWithoutSuffix::Str(lit) => Ok(FieldType::Named(self.literal(lit)?)),
            ExprWithoutSuffix::Unicode(lit) => Ok(FieldType::Named(self.code_point(lit)?)),
            ExprWithoutSuffix::Ident(ident) => {
                self.reference(owner, ident.0.as_str(), &ident.to_span())
            }
            ExprWithoutSuffix::Call(angle) => self.call(angle.body.0.as_str(), &angle.to_span()),
            ExprWithoutSuffix::Set(bracket) => self.set(owner, bracket),
            ExprWithoutSuffix::Paren(paren) => self.group(owner, &paren.body),
            // `~` in a rule of nodes asks about text and takes one character of
            // it, so it is a token rather than a pair of fields.
            ExprWithoutSuffix::Tilde(_, bracket) => self.tilde(owner, bracket),
        }
    }

    /// The type a name in a pattern stands for.
    fn reference(
        &mut self,
        _owner: &str,
        name: &str,
        span: &parserc::Span,
    ) -> Result<FieldType, CodegenError> {
        if name == WILDCARD {
            return self.wildcard();
        }

        if self.tokens.contains(name) || self.syntaxes.contains(name) {
            return Ok(FieldType::Named(name.to_owned()));
        }

        Err(CodegenError::unresolved(name, span.clone()))
    }

    /// The type of `<NAME>`, which is a token that runs the external.
    ///
    /// The call is the same pattern wherever it is written, whatever the table of
    /// the [`Codegen`](super::Codegen) says the name means: what a name stands
    /// for is settled before the file is written, so two rules cannot mean two
    /// things by one name and the whole file shares one type per name. The type
    /// keeps the span of the first rule that asked, which is where a name no
    /// table gave is reported against.
    fn call(&mut self, name: &str, span: &parserc::Span) -> Result<FieldType, CodegenError> {
        Ok(FieldType::Named(self.shared(
            &format!("External{name}"),
            Step::External(name.to_owned(), span.clone()),
        )))
    }

    /// The type of `_`, which is any one character.
    fn wildcard(&mut self) -> Result<FieldType, CodegenError> {
        Ok(FieldType::Named(
            self.shared("Any", Step::Class(CharClass::Any)),
        ))
    }

    /// The name of a type every operand that matches it shares, writing the
    /// pattern out the first time it is asked for.
    fn shared(&mut self, base: &str, step: Step) -> String {
        if let Some(given) = self.shared.get(base) {
            return given.clone();
        }

        let name = self.name(base);

        self.shared.insert(base.to_owned(), name.clone());
        self.anon_tokens.push(TokenPattern::new(name.clone(), step));

        name
    }

    /// The type of a set written inside a rule of nodes.
    fn set<I: UnsynInput>(
        &mut self,
        owner: &str,
        bracket: &super::SetOf<I>,
    ) -> Result<FieldType, CodegenError> {
        let refs = RuleNames {
            tokens: self.tokens.clone(),
            syntaxes: self.syntaxes.clone(),
            empty: self.empty.clone(),
        };

        let step = PatternBuilder::new(&refs).set_of(bracket)?;

        Ok(FieldType::Named(self.token(owner, step)))
    }

    /// The type of `~ […]` inside a rule of nodes.
    fn tilde<I: UnsynInput>(
        &mut self,
        owner: &str,
        set: &super::SetOf<I>,
    ) -> Result<FieldType, CodegenError> {
        let refs = RuleNames {
            tokens: self.tokens.clone(),
            syntaxes: self.syntaxes.clone(),
            empty: self.empty.clone(),
        };

        let pattern = PatternBuilder::new(&refs);

        let step = Step::Not(Box::new(pattern.set_of(set)?));

        Ok(FieldType::Named(self.token(owner, step)))
    }

    /// Writes the pattern of a char-level operand as a token type of its own and
    /// returns the name it was given.
    fn token(&mut self, owner: &str, step: Step) -> String {
        // An external call does not come this way, since it is named for the
        // external it runs and nothing about the rule it was written in
        // distinguishes one call of it from another.
        debug_assert!(
            !matches!(step, Step::External(..)),
            "an external call shares the one type written for its name"
        );

        let base = self.next(owner);
        let name = self.name(&base);

        self.anon_tokens.push(TokenPattern::new(name.clone(), step));

        name
    }

    /// The type of a parenthesised group.
    fn group<I: UnsynInput>(
        &mut self,
        owner: &str,
        expr: &Expr<I>,
    ) -> Result<FieldType, CodegenError> {
        // A group of one alternative, of no more operands than a tuple holds and
        // with nothing to keep between them, is the tuple they make: the trivia
        // before the group is the enclosing field's separator, and the group
        // needs no type and no separator of its own.
        if expr.rest.is_empty() && self.whitespace.is_none() {
            let fields = self.fields_of_sequence(owner, &expr.first)?;

            if fields.len() <= MAX_TUPLE {
                let mut types = fields.into_iter().map(|field| field.ty);

                let first = types.next().ok_or_else(|| {
                    CodegenError::unsupported("a group with nothing in it", expr.to_span())
                })?;

                return Ok(types.fold(first, |first, second| FieldType::Concat {
                    first: Box::new(first),
                    second: Box::new(second),
                }));
            }
        }

        // The group needs a type of its own. It is named first, for the rule it
        // was written in, and then shaped under that name: the alternatives of
        // the group are named for the group, not for the rule around it, which
        // is what keeps two groups of one rule from naming the same variant.
        let name = self.next(owner);
        let kind = self.kind(&name, expr)?;

        Ok(FieldType::Named(self.anonymous_as(name, kind)))
    }

    /// Writes a type of its own for a construct of a rule under a name that has
    /// already been worked out for it.
    fn anonymous_as(&mut self, base: String, kind: TypeDefKind) -> String {
        let name = self.name(&base);

        self.pending.push(TypeDef {
            name: name.clone(),
            kind,
        });

        name
    }

    /// Writes a type of its own for a construct of a rule and names it.
    fn anonymous(&mut self, owner: &str, kind: TypeDefKind) -> String {
        let base = self.next(owner);
        let name = self.name(&base);

        self.pending.push(TypeDef {
            name: name.clone(),
            kind,
        });

        name
    }

    /// The next name of the anonymous types of one rule, which counts from one
    /// and is only a candidate until [`TypeBuilder::name`] gives it out: a name
    /// handed out twice would be two types in one file.
    fn next(&mut self, owner: &str) -> String {
        let count = self.counters.entry(owner.to_owned()).or_insert(0);

        *count += 1;

        format!("{owner}{count}")
    }

    /// Gives out a name for a type codegen wrote itself, keeping it unique.
    fn name(&mut self, base: &str) -> String {
        self.names.register_anonymous(base)
    }

    /// The sum type of a rule or group of several alternatives.
    fn enum_of<I: UnsynInput>(
        &mut self,
        owner: &str,
        expr: &Expr<I>,
    ) -> Result<TypeDefKind, CodegenError> {
        let mut variants = Vec::new();

        variants.push(Variant {
            name: self.named_variant(owner, &expr.first, 0)?,
            fields: self.fields_of_sequence(owner, &expr.first)?,
        });

        for (index, (_, alts)) in expr.rest.iter().enumerate() {
            variants.push(Variant {
                name: self.named_variant(owner, alts, index + 1)?,
                fields: self.fields_of_sequence(owner, alts)?,
            });
        }

        Ok(TypeDefKind::Enum { variants })
    }

    /// The name one alternative is given among its siblings, which is read from
    /// what it is built on and falls back to the place it holds.
    fn named_variant<I: UnsynInput>(
        &mut self,
        owner: &str,
        seq: &ExprNoTopAlts<I>,
        index: usize,
    ) -> Result<String, CodegenError> {
        let base = match &seq.first {
            // The word or the token an alternative opens with is what it is, so
            // long as there is only one of each among the alternatives.
            ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Ident(ident))
                if ident.0.as_str() != WILDCARD =>
            {
                keyword_name(ident.0.as_str())
            }
            // The wildcard is a token of any one character, which is what it is
            // named for; the underscore it is written with names no type.
            ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Ident(_)) => "Any".to_owned(),
            ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Str(lit)) => {
                let (text, _) = crate::semantics::str_value(lit);

                literal_name(&text)
            }
            ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Unicode(lit)) => {
                match crate::semantics::unicode_value(lit) {
                    Ok(c) => punct_name(&c.to_string()),
                    Err(_) => format!("Variant{}", index + 1),
                }
            }
            ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Tilde(..)) => "Tilde".to_owned(),
            ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Set(..)) => "Set".to_owned(),
            ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Paren(..)) => "Group".to_owned(),
            // An external call is named for what it calls, which the rule around
            // it does not say.
            ExprNoTopAlt::WithoutSuffix(ExprWithoutSuffix::Call(angle)) => {
                keyword_name(angle.body.0.as_str())
            }
            ExprNoTopAlt::WithSuffix(ExprWithSuffix::Concat { .. }) => "Concat".to_owned(),
            ExprNoTopAlt::WithSuffix(ExprWithSuffix::Followed { .. }) => "Followed".to_owned(),
            ExprNoTopAlt::WithSuffix(ExprWithSuffix::Except { .. }) => "Except".to_owned(),
            ExprNoTopAlt::WithSuffix(ExprWithSuffix::Star(..)) => "Star".to_owned(),
            ExprNoTopAlt::WithSuffix(ExprWithSuffix::Plus(..)) => "Plus".to_owned(),
            ExprNoTopAlt::WithSuffix(ExprWithSuffix::Question(..)) => "Question".to_owned(),
            ExprNoTopAlt::WithSuffix(ExprWithSuffix::Repeat { .. }) => "Repeat".to_owned(),
        };

        // A variant is named within the type it belongs to, so only the
        // alternatives of one rule have to be kept apart from each other: a
        // second alternative of the same rule steps aside rather than colliding.
        let given = self.variants.entry(owner.to_owned()).or_default();
        let named = if given.contains(&base) {
            format!("{base}Variant{}", given.len() + 1)
        } else {
            base
        };

        given.insert(named.clone());

        Ok(format!("{owner}{named}"))
    }

    /// The name of the type for the literal `lit`, shared by every rule that
    /// writes it.
    fn literal<I: UnsynInput>(&mut self, lit: &super::StrOf<I>) -> Result<String, CodegenError> {
        let (text, _) = crate::semantics::str_value(lit);

        if text.is_empty() {
            return Err(CodegenError::unsupported(
                "a literal of nothing",
                lit.to_span(),
            ));
        }

        if let Some(name) = self.literals.get(&text) {
            return Ok(name.clone());
        }

        let name = self
            .names
            .register(&literal_name(&text))
            .ok_or_else(|| CodegenError::unsupported("a literal", lit.to_span()))?;

        // The pattern is the one the token rules use, so that a keyword shared
        // between two rules ends where an identifier of the language would go
        // on in both of them.
        let step = super::pattern::literal(lit)?;

        self.literals.insert(text, name.clone());
        self.anon_tokens.push(TokenPattern::new(name.clone(), step));

        Ok(name)
    }

    /// The name of the type for the code point literal `lit`, shared by every
    /// rule that writes it.
    fn code_point<I: UnsynInput>(
        &mut self,
        lit: &super::UnicodeOf<I>,
    ) -> Result<String, CodegenError> {
        let c = crate::semantics::unicode_value(lit).map_err(|_| {
            CodegenError::unsupported("a code point literal that denotes nothing", lit.to_span())
        })?;

        if let Some(name) = self.code_points.get(&c) {
            return Ok(name.clone());
        }

        let name = self
            .names
            .register(&punct_name(&c.to_string()))
            .ok_or_else(|| CodegenError::unsupported("a code point", lit.to_span()))?;

        self.code_points.insert(c, name.clone());
        self.anon_tokens.push(TokenPattern::new(
            name.clone(),
            Step::Class(CharClass::of(c)),
        ));

        Ok(name)
    }

    /// The types the types needed, taken out of the builder: the token patterns
    /// that belong beside the file's own rules, and the node types cut out of
    /// them.
    pub(crate) fn finish(self) -> (Vec<TokenPattern>, Vec<TypeDef>) {
        (self.anon_tokens, self.pending)
    }
}

#[cfg(test)]
mod tests {
    use parserc::Span;

    use super::*;

    /// A builder over `names` as the node rules and `empty` as those that can
    /// match nothing, with a whitespace rule of the file.
    fn builder(tokens: &[&str], syntaxes: &[&str], empty: &[&str]) -> TypeBuilder {
        let refs = RuleNames {
            tokens: tokens.iter().copied().map(String::from).collect(),
            syntaxes: syntaxes.iter().copied().map(String::from).collect(),
            empty: empty.iter().copied().map(String::from).collect(),
        };

        TypeBuilder::new(&refs, std::iter::empty(), Some("WHITESPACE".to_owned()))
    }

    #[test]
    fn a_counted_run_is_a_type_of_its_own() {
        let mut builder = builder(&[], &[], &[]);
        let name =
            builder.counted_type(FieldType::Named("X".to_owned()), 2, Some(4), true, "Owner");

        assert_eq!(name, "Owner1");
        assert_eq!(builder.pending.len(), 1);

        match &builder.pending[0].kind {
            TypeDefKind::Bounds {
                item,
                min,
                max,
                separated,
            } => {
                assert_eq!(item, &FieldType::Named("X".to_owned()));
                assert_eq!((*min, *max, *separated), (2, Some(4), true));
            }
            other => panic!("expected bounds, got {:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn anonymous_names_of_two_rules_do_not_collide() {
        let mut builder = builder(&[], &[], &[]);

        let first = builder.anonymous("Owner", empty_kind());
        let second = builder.anonymous("Owner", empty_kind());
        let other = builder.anonymous("Other", empty_kind());

        assert_eq!(
            (first.as_str(), second.as_str(), other.as_str()),
            ("Owner1", "Owner2", "Other1")
        );
    }

    /// The body of a type a test names without caring what it holds.
    fn empty_kind() -> TypeDefKind {
        TypeDefKind::Bounds {
            item: FieldType::Named("X".to_owned()),
            min: 1,
            max: None,
            separated: false,
        }
    }

    #[test]
    fn a_type_of_nothing_to_take_says_so_and_what_it_was() {
        let empty: HashSet<String> = ["Space".to_owned()].into_iter().collect();
        let stuck = FieldType::Many {
            item: Box::new(FieldType::Named("Space".to_owned())),
            separated: false,
        };

        assert!(stuck.takes_nothing(&empty));
        assert_eq!(stuck.describe(), "Space*");

        // A run of one or more of something that cannot be taken at all is the
        // other way a repetition sticks, and `concat` sticks only when both of
        // its operands do.
        assert!(
            FieldType::OneOrMore {
                item: Box::new(FieldType::Named("Space".to_owned())),
                separated: false,
            }
            .takes_nothing(&empty)
        );
        assert!(
            !FieldType::Concat {
                first: Box::new(FieldType::Named("Space".to_owned())),
                second: Box::new(FieldType::Named("X".to_owned())),
            }
            .takes_nothing(&empty)
        );
    }

    #[test]
    fn the_names_a_type_holds_are_what_a_pointer_is_asked_for() {
        let mut names = Vec::new();

        FieldType::Concat {
            first: Box::new(FieldType::Named("Expr".to_owned())),
            second: Box::new(FieldType::Opt(Box::new(FieldType::Named(
                "Stmt".to_owned(),
            )))),
        }
        .names(&mut names);

        assert_eq!(names, ["Expr", "Stmt"]);
    }

    #[test]
    fn a_report_points_where_the_name_was_written() {
        let error = CodegenError::unresolved("Nope", Span::Range(3..7));

        assert_eq!(error.to_string(), "unresolved name `Nope`, at Range(3..7)");
    }
}
