//! The type a `syntax` rule leaves behind: the fields it holds.
//!
//! A node definition is read against other nodes rather than against characters,
//! so its body is not compiled to a pattern over characters but to the *type* of
//! the node it reads: one field per element of the sequence the rule is, with
//! the repetitions and the assertions the rule asks for written into the type of
//! the field rather than into the code that reads it ([`FieldType`]).
//!
//! Three things make part of a rule need a type of its own rather than a field:
//!
//! - a repetition counted out (`X{n}`, `X{n}..{m}`), whose bound the code that
//!   reads it has to check, which is [`TypeDefKind::Bounds`];
//! - an operand with a question asked about the text it read or the text after
//!   it (`except`, `followed`), which is [`TypeDefKind::Asserted`];
//! - an operand matched over characters rather than over nodes inside a rule of
//!   nodes, which needs a parser of its own and so earns the token type
//!   [`TypeBuilder::token`] writes.
//!
//! A name is what the *left* side of a production earns, and a
//! [`TypeDefKind::Enum`] is what one of several alternatives is worth. The right
//! side of a `->` names nothing: a group of several alternatives is one field
//! held as [`FieldType::Alts`], which the backend reads as its ordered choice,
//! and a sequence longer than the tuple the backend derives ([`MAX_TUPLE`]) is
//! nested rather than named. [`FieldType::Concat`] is no shortcut over long
//! sequences either: it is the one field of the two operands the DSL joins with
//! `concat`.
//!
//! # Trivia
//!
//! The language a file describes is read with runs of trivia between its tokens,
//! and one structural rule decides where each of those runs is kept: it belongs
//! to the operand next to it, or to the sequence around that operand.
//!
//! A terminal **owns** the trivia around itself when its pattern denotes finitely
//! many fixed strings and none of them could be a piece of trivia, when it is not
//! the file's own trivia node, and when no rule that reads text reaches it by
//! name. Such a type keeps `Option<WS>` on each side of the text it matched, as
//! the punctuation of a hand-written parser does: `'<'`, `';'`, `'->'` and
//! `'lexer'` all separate what stands around them from themselves. The test is
//! made over the *shape* of a pattern ([`Step::fixed_strings`]) and over who
//! reaches it ([`Step::refers`]), never over what its characters happen to be: a
//! rule reached by name is inlined into the text of the rule that reaches it, so
//! trivia taken there would end up inside another token's own text, and a
//! terminal that could match a whitespace character would have its own match
//! eaten by the run before it.
//!
//! Every other terminal — a class of characters, an external call, a repetition,
//! a complement — keeps only the text it matched. The run next to it is then
//! nobody's to keep, so it becomes a [`FieldType::Gap`] of the sequence it lies
//! in: a field holding `Option<WS>` between two operands neither of which took
//! it. The trivia node itself keeps a whole maximal run as one token, which is
//! what makes holding `Option` of it say either "a run" or "nothing".
//!
//! The rule changes where a run of trivia is kept, never what the grammar
//! accepts: a run is taken by the first operand that reaches it, and taken whole
//! when it is taken at all, so the operands are read at the same positions
//! however the runs are filed away.
//!
//! `concat` is the exception the DSL makes, and the only place the rule does not
//! hold: it asks for both operands as one token, with nothing between, which is
//! why it is [`FieldType::Concat`] and never a field, a gap and a field. A group
//! written in brackets is the other case: `(A B)` is a sequence like any other,
//! so a gap stands between its operands where they need one, which is what lets
//! it be one [`FieldType::Group`] rather than a type of its own.
//!
//! A file that declares no `whitespace` rule has no trivia to take, so it has no
//! gaps at all and no terminal owns any.
//!
//! A rule may still write its trivia out, as `syntax Pair -> A WS B;` does when
//! `WS` is the trivia node. Such a bare mention of the trivia asks for nothing
//! the gap around it does not already take, so it is folded away and earns no
//! field: the type is the one `syntax Pair -> A B;` makes, and the
//! [analysis](crate::semantics) notes the mention. A mention with a suffix on it
//! (`WS?`, `WS*`) is not folded, since it asks for a repetition a gap only
//! happens to be; a sequence of nothing but mentions is a rule *about* the
//! trivia, and keeps them all.
//!
//! Where the runs end up is settled by [`Trivia`], once the whole file has been
//! read: an anonymous type is written as the rule that needed it asks, and
//! whether a terminal owns its trivia turns on rules written after it, so no one
//! rule can decide the question while it is being read.

use std::collections::{HashMap, HashSet};

use parserc::syntax::Syntax;

use crate::input::UnsynInput;
use crate::semantics::WILDCARD;
use crate::syntax::{
    Expr, ExprNoTopAlt, ExprNoTopAlts, ExprWithSuffix, ExprWithoutSuffix, Operand, SuffixOp,
    Suffixed,
};

use super::error::{CodegenError, CodegenKind};
use super::name::{Names, keyword_name, literal_name, punct_name};
use super::pattern::{CharClass, PatternBuilder, RuleNames, Step, TokenPattern};

/// The most operands a sequence may be folded into, which is how many elements
/// a tuple of the backend has: `parserc` derives a syntax for a tuple of each
/// length up to this, and a longer one is [nested](super::render::Renderer::tuple)
/// rather than named.
pub(crate) const MAX_TUPLE: usize = 16;

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
    /// The name of the variant, which is the first operand of the alternative
    /// that no sibling of it has at the same place: what divides it from the rest
    /// of the alternatives is what it is, rather than where it was written.
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
    },
    /// `A concat B`, which is both taken back to back with nothing between.
    Concat {
        /// What comes first.
        first: Box<FieldType>,
        /// What comes next.
        second: Box<FieldType>,
    },
    /// A group of operands that needed no type of its own, folded into one
    /// field because nothing divides them.
    Group(Vec<FieldType>),
    /// The alternatives of a group, held as the ordered choice of the backend
    /// rather than as a type of their own: only the left side of a production
    /// earns a name, and what a rule writes on its right side is one of the
    /// shapes below however many ways it may be read.
    Alts(Vec<FieldType>),
    /// A run of trivia that neither of the operands around it owns, which is a
    /// field of the sequence it lies in rather than part of either of them.
    Gap,
    /// `(T P)* T?`, which is what the backend calls a punctuated list: the runs
    /// of trivia belong to the separators, so no gaps stand between the items.
    Punctuated {
        /// What each element of the list is.
        item: Box<FieldType>,
        /// What stands between two of them.
        sep: Box<FieldType>,
    },
    /// Two terminals that own their trivia holding whatever lies between them,
    /// which is what the backend calls a delimiter.
    Delimited {
        /// The terminal the run is opened by.
        open: Box<FieldType>,
        /// What lies between the two terminals, gaps included.
        body: Vec<FieldType>,
        /// The terminal the run is closed by.
        close: Box<FieldType>,
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
            FieldType::Opt(_) | FieldType::Many { .. } | FieldType::Gap => true,
            FieldType::Punctuated { .. } => true,
            FieldType::Concat { first, second, .. } => {
                first.takes_nothing(empty) && second.takes_nothing(empty)
            }
            FieldType::Group(tys) => tys.iter().all(|ty| ty.takes_nothing(empty)),
            // The choice is nothing as soon as one of its ways is.
            FieldType::Alts(tys) => tys.iter().any(|ty| ty.takes_nothing(empty)),
            // The terminals that hold a delimited run take what they match: a
            // fixed string is never nothing.
            FieldType::Delimited { .. } => false,
        }
    }

    /// What the type is called in the report about it.
    pub(crate) fn describe(&self) -> String {
        match self {
            FieldType::Named(name) => name.clone(),
            FieldType::Opt(item) => format!("{}?", item.describe()),
            FieldType::Many { item } => format!("{}*", item.describe()),
            FieldType::Concat { first, second } => {
                format!("{} concat {}", first.describe(), second.describe())
            }
            FieldType::Group(tys) => format!("({})", Self::described(tys)),
            FieldType::Alts(tys) => tys
                .iter()
                .map(FieldType::describe)
                .collect::<Vec<_>>()
                .join(" or "),
            FieldType::Gap => "a run of trivia".to_owned(),
            FieldType::Punctuated { item, sep } => {
                format!("{} separated by {}", item.describe(), sep.describe())
            }
            FieldType::Delimited { open, body, close } => format!(
                "{} held between {} and {}",
                Self::described(body),
                open.describe(),
                close.describe()
            ),
        }
    }

    /// What a sequence of operands is called in a report, read as the sequence
    /// it is.
    fn described(tys: &[FieldType]) -> String {
        tys.iter()
            .map(FieldType::describe)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The names of the types this one is built out of, which is what decides
    /// whether it has to be held behind a pointer.
    pub(crate) fn names(&self, out: &mut Vec<String>) {
        match self {
            FieldType::Named(name) => out.push(name.clone()),
            FieldType::Opt(item) | FieldType::Many { item } => item.names(out),
            FieldType::Concat { first, second, .. } => {
                first.names(out);
                second.names(out);
            }
            FieldType::Group(tys) => {
                for ty in tys {
                    ty.names(out);
                }
            }
            FieldType::Alts(tys) => {
                for ty in tys {
                    ty.names(out);
                }
            }
            FieldType::Gap => {}
            FieldType::Punctuated { item, sep } => {
                item.names(out);
                sep.names(out);
            }
            FieldType::Delimited { open, body, close } => {
                open.names(out);

                for ty in body {
                    ty.names(out);
                }

                close.names(out);
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
        if expr.is_single() {
            return Ok(TypeDefKind::Struct {
                fields: self.fields_of_sequence(owner, &expr.first)?,
            });
        }

        self.enum_of(owner, expr)
    }

    /// The fields of one sequence of operands.
    ///
    /// Where the runs of trivia between them are kept is not decided here: a
    /// terminal takes the trivia around itself only when the whole file says it
    /// can, which is a question the [settling](Trivia) answers once every rule
    /// has been read.
    ///
    /// A bare mention of the trivia node of the file is a separator rather than an
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

        Ok(fields)
    }

    /// The field one operand of a sequence makes.
    fn field<I: UnsynInput>(
        &mut self,
        owner: &str,
        expr: &ExprNoTopAlt<I>,
    ) -> Result<Field, CodegenError> {
        Ok(Field {
            ty: self.ty(owner, expr)?,
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
        match Suffixed::of(expr) {
            Suffixed::Repeated { target, op } => match op {
                SuffixOp::Star(_) => self.repeated(target, 0, None, owner, expr),
                SuffixOp::Question(_) => Ok(FieldType::Opt(Box::new(self.leaf(owner, target)?))),
                SuffixOp::Plus(_) => self.repeated(target, 1, None, owner, expr),
                SuffixOp::Repeat(repeat) => {
                    let (min, max) =
                        super::pattern::repeat_bounds(repeat).ok_or_else(|| CodegenError {
                            kind: CodegenKind::InvalidRepeat,
                            span: expr.to_span(),
                        })?;

                    self.counted(target, min, max, owner, &expr.to_span())
                }
            },
            Suffixed::Concat { target, suffix } => Ok(FieldType::Concat {
                first: Box::new(self.leaf(owner, target)?),
                second: Box::new(self.ty(owner, suffix)?),
            }),
            Suffixed::Followed { target, suffix } => {
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
            Suffixed::Except { target, tokens } => {
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

        match (min, max) {
            (0, None) => Ok(FieldType::Many {
                item: Box::new(item),
            }),
            // `X+` is one and then as many more as the text offers, which is a
            // sequence of two rather than a run with a count checked after it.
            (1, None) => Ok(FieldType::Concat {
                first: Box::new(item.clone()),
                second: Box::new(FieldType::Many {
                    item: Box::new(item),
                }),
            }),
            _ => {
                let name = self.counted_type(item, min, max, owner);

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

        Ok(FieldType::Named(self.counted_type(item, min, max, owner)))
    }

    /// Writes the type of a counted run and returns the name it was given.
    fn counted_type(
        &mut self,
        item: FieldType,
        min: usize,
        max: Option<usize>,
        owner: &str,
    ) -> String {
        self.anonymous(owner, TypeDefKind::Bounds { item, min, max })
    }

    /// The type of an operand with no suffix on it.
    fn leaf<I: UnsynInput>(
        &mut self,
        owner: &str,
        expr: &ExprWithoutSuffix<I>,
    ) -> Result<FieldType, CodegenError> {
        match Operand::of(expr) {
            // A literal is a token of its own, shared between every rule that
            // writes it the same way.
            Operand::Str(lit) => Ok(FieldType::Named(self.literal(lit)?)),
            Operand::Unicode(lit) => Ok(FieldType::Named(self.code_point(lit)?)),
            Operand::Reference(ident) => self.reference(owner, ident.0.as_str(), &ident.to_span()),
            Operand::External(angle) => self.call(angle.body.0.as_str(), &angle.to_span()),
            Operand::Set(bracket) => self.set(owner, bracket),
            Operand::Group(body) => self.group(owner, body),
            // `~` in a rule of nodes asks about text and takes one character of
            // it, so it is a token rather than a pair of fields.
            Operand::Complement(bracket) => self.tilde(owner, bracket),
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
    ///
    /// A group is written on the right side of a `->`, where nothing is being
    /// named: one way of being read is the sequence it is, and several ways are
    /// the ordered choice the backend already holds. Neither is a reason to
    /// invent a type, since a name is what the left side of a production earns.
    fn group<I: UnsynInput>(
        &mut self,
        owner: &str,
        expr: &Expr<I>,
    ) -> Result<FieldType, CodegenError> {
        if expr.is_single() {
            return self.alternative(owner, &expr.first);
        }

        // Several ways of being read, each settled as the sequence it offers. The
        // trivia inside one way belongs to that way alone, which is what the
        // settling of [`FieldType::Alts`] goes back over them for.
        let mut alts = vec![];

        for seq in expr.branches() {
            alts.push(self.alternative(owner, seq)?);
        }

        Ok(FieldType::Alts(alts))
    }

    /// The type one alternative of a group is held as: its operands folded into
    /// a tuple when there is more than one to hold, and the operand it already
    /// is where it stands alone.
    ///
    /// A tuple longer than the backend derives a syntax for is [nested by the
    /// renderer](super::render::Renderer::tuple) rather than named, so the length
    /// of a sequence is no reason to invent a type nobody wrote either.
    fn alternative<I: UnsynInput>(
        &mut self,
        owner: &str,
        seq: &ExprNoTopAlts<I>,
    ) -> Result<FieldType, CodegenError> {
        let mut tys: Vec<FieldType> = self
            .fields_of_sequence(owner, seq)?
            .into_iter()
            .map(|field| field.ty)
            .collect();

        if tys.is_empty() {
            return Err(CodegenError::unsupported(
                "a group with nothing in it",
                seq.to_span(),
            ));
        }

        if tys.len() == 1 {
            return Ok(tys.pop().expect("one operand, counted just now"));
        }

        Ok(FieldType::Group(tys))
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

    /// The sum type of a rule of several alternatives.
    ///
    /// A group written on the right side of a `->` is not read this way: only the
    /// left side of a production earns a type, and the alternatives of a group are
    /// the [`FieldType::Alts`] of the same sequences.
    fn enum_of<I: UnsynInput>(
        &mut self,
        owner: &str,
        expr: &Expr<I>,
    ) -> Result<TypeDefKind, CodegenError> {
        // The alternatives as they were written, because one of them cannot be
        // named before all of them have been read: what a variant is called is
        // whichever operand none of its siblings has at the same place.
        let alts: Vec<&ExprNoTopAlts<I>> = expr.branches().collect();

        let shapes: Vec<Vec<String>> = alts
            .iter()
            .map(|seq| operand_names(seq))
            .collect::<Vec<_>>();

        let mut variants = Vec::new();
        let mut names: Vec<String> = Vec::new();

        for (index, seq) in alts.into_iter().enumerate() {
            let mut name = variant_name(&shapes, index);

            // Two alternatives written the same say nothing about which is the
            // other, so the place one of them holds is all that is left to tell.
            if names.contains(&name) {
                name = format!("{name}Variant{}", index + 1);
            }

            names.push(name.clone());

            variants.push(Variant {
                name: format!("{owner}{name}"),
                fields: self.fields_of_sequence(owner, seq)?,
            });
        }

        Ok(TypeDefKind::Enum { variants })
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

/// Where the runs of trivia of a file's language are kept.
///
/// The table is built once the whole file has been read, since two of the four
/// tests that decide ownership turn on what the rest of the file says: which
/// types are anonymous is known only when every rule has been shaped, and
/// whether a type is reached by name is known only when every pattern has been
/// compiled. [`Trivia::settle`] then reads the types that table produced and
/// moves the trivia where it belongs, which is also where the shapes of the
/// backend — a punctuated list and a delimited run — are recognized out of the
/// operands a sequence was built from.
#[derive(Debug)]
pub(crate) struct Trivia {
    /// The type of the file's trivia, where it declares any.
    whitespace: Option<String>,
    /// Each terminal that owns the trivia around itself, keyed by its type name
    /// and mapped to the fixed strings its pattern matches, which is what tells
    /// one terminal's text from another's.
    owning: HashMap<String, Vec<String>>,
}

impl Trivia {
    /// Prepares the table over the file's `whitespace` type and the `owning`
    /// terminals with the strings each of them matches.
    pub(crate) fn new(whitespace: Option<String>, owning: HashMap<String, Vec<String>>) -> Self {
        Self { whitespace, owning }
    }

    /// The type of the file's trivia, where it declares any.
    pub(crate) fn whitespace(&self) -> Option<&str> {
        self.whitespace.as_deref()
    }

    /// Returns `true` when `name` is the terminal that holds a run of trivia, and
    /// when nothing else was settled from it: such a run is nobody's to keep
    /// beside another, and the terminal never owns its own neighbours.
    pub(crate) fn is_trivia(&self, name: &str) -> bool {
        self.whitespace.as_deref() == Some(name)
    }

    /// Returns `true` when the terminal keeps the trivia on each side of the text
    /// it matched.
    pub(crate) fn owns(&self, name: &str) -> bool {
        self.whitespace.is_some() && self.owning.contains_key(name)
    }

    /// The strings an operand matches, where it is a terminal that owns its
    /// trivia.
    ///
    /// Only such an operand can open or close a delimited run, since telling two
    /// brackets apart is a question about the text they match, and a terminal
    /// that owns nothing has no text of its own to tell.
    fn texts(&self, ty: &FieldType) -> Option<&[String]> {
        match ty {
            FieldType::Named(name) if self.owns(name) => self.owning.get(name).map(Vec::as_slice),
            _ => None,
        }
    }

    /// Returns `true` when taking this operand reaches the trivia that stands
    /// before it.
    ///
    /// A choice reaches its front only where every way it offers does, since one
    /// answer is kept for the whole field: where a way does not reach, the
    /// sequence around it keeps the run and that way's own terminal simply finds
    /// nothing there to take.
    fn eats_leading(&self, ty: &FieldType) -> bool {
        match ty {
            FieldType::Named(name) => self.owns(name),
            // A run may take nothing, which is when its own first operand never
            // sees the trivia before it, so the sequence around has to keep it.
            FieldType::Opt(_) | FieldType::Many { .. } | FieldType::Punctuated { .. } => false,
            FieldType::Gap => false,
            FieldType::Concat { first, .. } => self.eats_leading(first),
            FieldType::Group(tys) => tys.first().is_some_and(|ty| self.eats_leading(ty)),
            FieldType::Alts(tys) => !tys.is_empty() && tys.iter().all(|ty| self.eats_leading(ty)),
            FieldType::Delimited { open, .. } => self.eats_leading(open),
        }
    }

    /// Returns `true` when taking this operand reaches the trivia that stands
    /// after it.
    fn eats_trailing(&self, ty: &FieldType) -> bool {
        match ty {
            FieldType::Named(name) => self.owns(name),
            FieldType::Opt(_) | FieldType::Many { .. } | FieldType::Punctuated { .. } => false,
            FieldType::Gap => false,
            FieldType::Concat { second, .. } => self.eats_trailing(second),
            FieldType::Group(tys) => tys.last().is_some_and(|ty| self.eats_trailing(ty)),
            FieldType::Alts(tys) => !tys.is_empty() && tys.iter().all(|ty| self.eats_trailing(ty)),
            FieldType::Delimited { close, .. } => self.eats_trailing(close),
        }
    }

    /// Returns `true` when a gap has to stand between two neighbours because
    /// neither of them reaches the run between them.
    fn needs_gap(&self, before: &FieldType, next: &FieldType) -> bool {
        self.whitespace.is_some() && !self.eats_trailing(before) && !self.eats_leading(next)
    }

    /// Settles where the trivia of each field of every type goes.
    ///
    /// A type settles from what it holds, not from what is held inside a type of
    /// its own: those are separate definitions, each settled for itself, so one
    /// pass over the list covers the file.
    pub(crate) fn settle(&self, defs: &mut [TypeDef]) {
        for def in defs {
            match &mut def.kind {
                TypeDefKind::Struct { fields } => *fields = self.fields(fields),
                TypeDefKind::Enum { variants } => {
                    for variant in variants {
                        variant.fields = self.fields(&variant.fields);
                    }
                }
                TypeDefKind::Bounds { item, .. } => *item = self.run(item),
                TypeDefKind::Asserted { item, assert } => {
                    let settled = self.ty(item);
                    *item = settled;

                    match assert {
                        Assert::Except(removed) => {
                            let settled = self.ty(removed);
                            *removed = settled;
                        }
                        Assert::Followed(next) => {
                            let settled = self.ty(next);
                            *next = settled;
                        }
                    }
                }
            }
        }
    }

    /// The settled fields of one alternative, which is a sequence of operands.
    fn fields(&self, fields: &[Field]) -> Vec<Field> {
        let tys: Vec<FieldType> = fields.iter().map(|field| field.ty.clone()).collect();

        self.between(&tys)
            .into_iter()
            .map(|ty| Field { ty })
            .collect()
    }

    /// The settled form of a sequence of operands: each of them as a type, then
    /// the shapes folded out of the sequence they make.
    fn between(&self, tys: &[FieldType]) -> Vec<FieldType> {
        let mut settled: Vec<FieldType> = tys.iter().map(|ty| self.ty(ty)).collect();

        self.folded(&mut settled);

        settled
    }

    /// Folds the shapes of the backend out of a sequence, then puts the gaps
    /// where they belong.
    ///
    /// The gaps come last because a fold is a question about the operands as they
    /// were written: a gap standing between an item and its separator is the very
    /// reason the two cannot be one punctuated list, and folding away the pair
    /// first means the gap is not there to answer with.
    fn folded(&self, tys: &mut Vec<FieldType>) {
        self.punctuates(tys);
        self.closes(tys);
        self.gaps(tys);
    }

    /// Settles one operand of a sequence, which is where a group becomes one of
    /// its own and where the operands inside a type start again.
    fn ty(&self, ty: &FieldType) -> FieldType {
        match ty {
            FieldType::Named(name) => FieldType::Named(name.clone()),
            FieldType::Gap => FieldType::Gap,
            FieldType::Opt(item) => FieldType::Opt(Box::new(self.ty(item))),
            FieldType::Many { item } => FieldType::Many {
                item: Box::new(self.run(item)),
            },
            // `concat` asks for both operands as one token, so no gap stands
            // between them and neither is settled as a sequence of its own.
            FieldType::Concat { first, second } => FieldType::Concat {
                first: Box::new(self.ty(first)),
                second: Box::new(self.ty(second)),
            },
            FieldType::Group(tys) => FieldType::Group(self.between(tys)),
            // Each way of a choice settles its own trivia, which is what the
            // sequence around the choice cannot do for it.
            FieldType::Alts(tys) => FieldType::Alts(tys.iter().map(|ty| self.ty(ty)).collect()),
            FieldType::Punctuated { item, sep } => FieldType::Punctuated {
                item: Box::new(self.ty(item)),
                sep: Box::new(self.ty(sep)),
            },
            FieldType::Delimited { open, body, close } => FieldType::Delimited {
                open: Box::new(self.ty(open)),
                body: self.between(body),
                close: Box::new(self.ty(close)),
            },
        }
    }

    /// Settles one operand of a run, which is where the trivia between two of its
    /// repetitions is kept.
    ///
    /// A run has no neighbour of its own to lean on between its repetitions: the
    /// sequence around it keeps only the trivia before the first, so what stands
    /// between one repetition and the next belongs to the repetition itself. It
    /// is a gap at the front of what each takes, except where a repetition already
    /// reaches the trivia on one of its own ends.
    fn run(&self, item: &FieldType) -> FieldType {
        let settled = self.ty(item);

        if self.whitespace.is_none() || self.eats_leading(&settled) || self.eats_trailing(&settled)
        {
            return settled;
        }

        match settled {
            FieldType::Group(mut tys) => {
                tys.insert(0, FieldType::Gap);

                FieldType::Group(tys)
            }
            settled => FieldType::Group(vec![FieldType::Gap, settled]),
        }
    }

    /// Folds each `(T P)* T?` a sequence holds into one punctuated list, which is
    /// the shape the backend keeps the runs of trivia in for it.
    ///
    /// The pair has to be written as a group of exactly two operands, since the
    /// trivia between them would have made it three, and the tail has to be the
    /// very same type as the items, since anything else is a sequence that only
    /// looks like a list.
    fn punctuates(&self, tys: &mut Vec<FieldType>) {
        let mut index = 0;

        while index + 1 < tys.len() {
            let pair = match &tys[index] {
                FieldType::Many { item } => match item.as_ref() {
                    FieldType::Group(tys) if tys.len() == 2 => {
                        Some((tys[0].clone(), tys[1].clone()))
                    }
                    _ => None,
                },
                _ => None,
            };

            let Some((item, sep)) = pair else {
                index += 1;

                continue;
            };

            let tail = match &tys[index + 1] {
                FieldType::Opt(tail) => **tail == item,
                _ => false,
            };

            if !tail || self.needs_gap(&sep, &item) {
                index += 1;

                continue;
            }

            tys[index] = FieldType::Punctuated {
                item: Box::new(item),
                sep: Box::new(sep),
            };
            tys.remove(index + 1);
            index += 1;
        }
    }

    /// Folds each run two terminals hold into one delimited run, which is the
    /// shape the backend keeps the trivia of both ends in for it.
    ///
    /// The run folded is the **narrowest** one in the sequence rather than the
    /// first one the leftmost opener can reach, because reaching is exactly what
    /// spoils a nesting: `'~' '[' … ']'` offers `]` both to the `~` and to the `[`,
    /// and taking it for the `~` would file `~` and `]` as the two ends of one run,
    /// which is a pairing nothing in the production says. The narrowest window is
    /// the `[ … ]` the production does say, and the `~` is left standing outside it.
    /// Two runs written beside each other are likewise folded as two, and a run
    /// never holds a terminal of the closing end's own text.
    fn closes(&self, tys: &mut Vec<FieldType>) {
        let Some((open, close)) = self.narrowest(tys) else {
            return;
        };

        let ends = tys.remove(close);
        let mut body: Vec<FieldType> = tys.drain((open + 1)..close).collect();
        let starts = tys.remove(open);

        // The operands of the body are settled already, so the run folds from
        // what they are: settling them again would ask a gap to stand where
        // one of them already keeps the trivia.
        self.folded(&mut body);

        tys.insert(
            open,
            FieldType::Delimited {
                open: Box::new(starts),
                body,
                close: Box::new(ends),
            },
        );

        // What is left of the sequence may hold runs of its own, and a run that
        // held the one just folded is narrower now that the fold is one field.
        self.closes(tys);
    }

    /// The ends of the narrowest run of the sequence, the leftmost when two are
    /// equally narrow.
    fn narrowest(&self, tys: &[FieldType]) -> Option<(usize, usize)> {
        (0..tys.len())
            .filter_map(|open| self.closed_at(tys, open).map(|close| (open, close)))
            .min_by_key(|&(open, close)| (close - open, open))
    }

    /// The end of the shortest run that opens at `open`, if it has one.
    fn closed_at(&self, tys: &[FieldType], open: usize) -> Option<usize> {
        let starts = self.texts(&tys[open])?;

        (open + 2..tys.len()).find(|&close| {
            self.texts(&tys[close]).is_some_and(|ends| {
                !overlaps(starts, ends)
                    && !tys[open + 1..close]
                        .iter()
                        .any(|inner| self.texts(inner).is_some_and(|held| overlaps(held, ends)))
            })
        })
    }

    /// Puts a gap where neither of two neighbours keeps the run between them.
    fn gaps(&self, tys: &mut Vec<FieldType>) {
        let written = std::mem::take(tys);
        let mut previous: Option<&FieldType> = None;

        for ty in &written {
            if let Some(before) = previous
                && self.needs_gap(before, ty)
            {
                tys.push(FieldType::Gap);
            }

            previous = Some(ty);
            tys.push(ty.clone());
        }
    }
}

/// Returns `true` when two terminals could both match one of the strings the
/// other matches, which is when neither can open a run the other closes.
fn overlaps(a: &[String], b: &[String]) -> bool {
    a.iter().any(|text| b.iter().any(|other| other == text))
}

/// What each operand of one alternative is called, in the order the rule wrote
/// them, which is the shape the alternative is told apart from its siblings by.
///
/// These are descriptors and not types: an operand written out is counted where
/// it stands even where the settling folds it away, since it is the rule as it
/// was written that says which alternative this is.
fn operand_names<I: UnsynInput>(seq: &ExprNoTopAlts<I>) -> Vec<String> {
    seq.elements().map(operand_name).collect()
}

/// What one operand is called among the operands of its alternative.
///
/// The names are the ones the generated file has always carried, so they outlive
/// the shape they were read off: the view of an operand says `Complement` here
/// and the type it names is still the one written for a `~`.
fn operand_name<I: UnsynInput>(expr: &ExprNoTopAlt<I>) -> String {
    match expr {
        ExprNoTopAlt::WithoutSuffix(expr) => match Operand::of(expr) {
            // The word or the token an operand is built on is what it is called.
            Operand::Reference(ident) if ident.0.as_str() != WILDCARD => {
                keyword_name(ident.0.as_str())
            }
            // The wildcard is a token of any one character, which is what it is
            // named for; the underscore it is written with names no type.
            Operand::Reference(_) => "Any".to_owned(),
            Operand::Str(lit) => {
                let (text, _) = crate::semantics::str_value(lit);

                literal_name(&text)
            }
            Operand::Unicode(lit) => match crate::semantics::unicode_value(lit) {
                Ok(c) => punct_name(&c.to_string()),
                // A code point that denotes nothing denotes no character to be
                // named for, so it is named for what it is written with.
                Err(_) => "Unicode".to_owned(),
            },
            Operand::Complement(_) => "Tilde".to_owned(),
            Operand::Set(_) => "Set".to_owned(),
            Operand::Group(_) => "Group".to_owned(),
            // An external call is named for what it calls, which the rule around
            // it does not say.
            Operand::External(angle) => keyword_name(angle.body.0.as_str()),
        },
        ExprNoTopAlt::WithSuffix(expr) => match Suffixed::of(expr) {
            Suffixed::Concat { .. } => "Concat".to_owned(),
            Suffixed::Followed { .. } => "Followed".to_owned(),
            Suffixed::Except { .. } => "Except".to_owned(),
            // A group of operators is one alternative, but the operator it picked is
            // what tells two alternatives of a rule apart, so the operand is still
            // named for the operator rather than for the group.
            Suffixed::Repeated { op, .. } => match op {
                SuffixOp::Star(_) => "Star",
                SuffixOp::Question(_) => "Question",
                SuffixOp::Plus(_) => "Plus",
                SuffixOp::Repeat(_) => "Repeat",
            }
            .to_owned(),
        },
    }
}

/// The name the shape of one alternative gives it among its siblings.
///
/// An alternative is named for the first operand none of its siblings has at the
/// same place, which is what divides it: of `A 'concat' B | A 'except' B` it is
/// the operator in the middle and not the operand both of them start with. The
/// name is read off the shape rather than off the place it holds so that writing
/// the alternatives of a rule in another order does not rename them.
fn variant_name(shapes: &[Vec<String>], index: usize) -> String {
    let here = &shapes[index];

    here.iter()
        .enumerate()
        .find(|(place, name)| {
            shapes
                .iter()
                .enumerate()
                .all(|(other, sibling)| other == index || sibling.get(*place) != Some(*name))
        })
        .map(|(_, name)| name.clone())
        // Nothing at any place divides this alternative from its siblings, which
        // leaves what it ends with: of `A B | A` the longer is already named for
        // the operand that makes it longer, so the shorter keeps the one they
        // share and the two are still told apart.
        .or_else(|| here.last().cloned())
        .expect("an alternative always has a first operand to be named for")
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
        let name = builder.counted_type(FieldType::Named("X".to_owned()), 2, Some(4), "Owner");

        assert_eq!(name, "Owner1");
        assert_eq!(builder.pending.len(), 1);

        match &builder.pending[0].kind {
            TypeDefKind::Bounds { item, min, max } => {
                assert_eq!(item, &FieldType::Named("X".to_owned()));
                assert_eq!((*min, *max), (2, Some(4)));
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
        }
    }

    #[test]
    fn a_type_of_nothing_to_take_says_so_and_what_it_was() {
        let empty: HashSet<String> = ["Space".to_owned()].into_iter().collect();
        let stuck = FieldType::Many {
            item: Box::new(FieldType::Named("Space".to_owned())),
        };

        assert!(stuck.takes_nothing(&empty));
        assert_eq!(stuck.describe(), "Space*");

        // The shapes the settling folds out of a sequence can hold nothing
        // between them as well: a list may be empty, a gap may be nothing, and
        // `concat` sticks only when both of its operands do.
        assert!(
            FieldType::Punctuated {
                item: Box::new(FieldType::Named("Space".to_owned())),
                sep: Box::new(FieldType::Named("Comma".to_owned())),
            }
            .takes_nothing(&empty)
        );
        assert!(FieldType::Gap.takes_nothing(&empty));
        assert!(
            !FieldType::Concat {
                first: Box::new(FieldType::Named("Space".to_owned())),
                second: Box::new(FieldType::Named("X".to_owned())),
            }
            .takes_nothing(&empty)
        );
    }

    #[test]
    fn a_fold_reports_the_shape_it_recognized() {
        assert_eq!(
            FieldType::Punctuated {
                item: Box::new(FieldType::Named("Item".to_owned())),
                sep: Box::new(FieldType::Named("Comma".to_owned())),
            }
            .describe(),
            "Item separated by Comma"
        );

        assert_eq!(
            FieldType::Delimited {
                open: Box::new(FieldType::Named("Open".to_owned())),
                body: vec![FieldType::Named("Item".to_owned())],
                close: Box::new(FieldType::Named("Close".to_owned())),
            }
            .describe(),
            "Item held between Open and Close"
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

    /// A table over a file that declares `WHITESPACE` and lets each of `owning`
    /// keep the trivia on each side of the text it matches.
    fn trivia(owning: &[(&str, &[&str])]) -> Trivia {
        let owning = owning
            .iter()
            .map(|(name, texts)| {
                (
                    (*name).to_owned(),
                    texts.iter().map(|text| (*text).to_owned()).collect(),
                )
            })
            .collect();

        Trivia::new(Some("WHITESPACE".to_owned()), owning)
    }

    /// The fields a rule of the operands `tys` is given once the runs of trivia
    /// of `trivia` are settled onto it.
    fn settled(trivia: &Trivia, tys: Vec<FieldType>) -> Vec<FieldType> {
        let mut defs = vec![TypeDef {
            name: "Rule".to_owned(),
            kind: TypeDefKind::Struct {
                fields: tys.into_iter().map(|ty| Field { ty }).collect(),
            },
        }];

        trivia.settle(&mut defs);

        let TypeDefKind::Struct { fields } = &defs[0].kind else {
            unreachable!("a rule of one alternative is a struct");
        };

        fields.iter().map(|field| field.ty.clone()).collect()
    }

    /// The type of a named operand.
    fn named(name: &str) -> FieldType {
        FieldType::Named(name.to_owned())
    }

    /// A run of what a test names.
    fn many(item: FieldType) -> FieldType {
        FieldType::Many {
            item: Box::new(item),
        }
    }

    #[test]
    fn a_repeated_pair_with_a_tail_is_one_list() {
        // `(Item Comma)* Item?` is what a list of items is, and the separator
        // reaching the trivia on each side of itself is what lets the runs be
        // kept in the list rather than between its fields.
        let tys = settled(
            &trivia(&[("Comma", &[","])]),
            vec![
                many(FieldType::Group(vec![named("Item"), named("Comma")])),
                FieldType::Opt(Box::new(named("Item"))),
            ],
        );

        assert_eq!(
            tys,
            vec![FieldType::Punctuated {
                item: Box::new(named("Item")),
                sep: Box::new(named("Comma")),
            }]
        );
    }

    #[test]
    fn a_pair_with_a_run_between_its_two_operands_is_not_a_list() {
        // Neither operand keeps the trivia, so the pair is a sequence of three
        // fields once the gap between them stands, and a list is not what three
        // fields are.
        let tys = settled(
            &trivia(&[]),
            vec![
                many(FieldType::Group(vec![named("A"), named("B")])),
                FieldType::Opt(Box::new(named("A"))),
            ],
        );

        assert_eq!(
            tys,
            vec![
                many(FieldType::Group(vec![
                    FieldType::Gap,
                    named("A"),
                    FieldType::Gap,
                    named("B"),
                ])),
                FieldType::Gap,
                FieldType::Opt(Box::new(named("A"))),
            ]
        );
    }

    #[test]
    fn a_pair_with_a_different_tail_is_not_a_list() {
        let tys = settled(
            &trivia(&[("Comma", &[","])]),
            vec![
                many(FieldType::Group(vec![named("Item"), named("Comma")])),
                FieldType::Opt(Box::new(named("Other"))),
            ],
        );

        assert_eq!(tys.len(), 3, "a gap stands where the list broke apart");
    }

    #[test]
    fn two_terminals_hold_the_run_between_them() {
        let tys = settled(
            &trivia(&[("Open", &["["]), ("Close", &["]"])]),
            vec![named("Open"), named("A"), named("Close")],
        );

        assert_eq!(
            tys,
            vec![FieldType::Delimited {
                open: Box::new(named("Open")),
                body: vec![named("A")],
                close: Box::new(named("Close")),
            }]
        );
    }

    #[test]
    fn two_runs_beside_each_other_are_folded_as_two() {
        let tys = settled(
            &trivia(&[("Open", &["("]), ("Close", &[")"])]),
            vec![
                named("Open"),
                named("A"),
                named("Close"),
                named("Open"),
                named("B"),
                named("Close"),
            ],
        );

        assert_eq!(tys.len(), 2, "the shortest run is the one that folds");

        assert_eq!(
            tys[0],
            FieldType::Delimited {
                open: Box::new(named("Open")),
                body: vec![named("A")],
                close: Box::new(named("Close")),
            }
        );
    }

    #[test]
    fn a_run_is_not_folded_over_a_terminal_of_its_closing_text() {
        // The middle `Close` closes nothing the outer run could hold, so the
        // window that would have had to hold it is not a run at all.
        let tys = settled(
            &trivia(&[("Open", &["["]), ("Close", &["]"])]),
            vec![named("Open"), named("Close"), named("A"), named("Close")],
        );

        assert_eq!(tys.len(), 4, "nothing was folded");
    }

    #[test]
    fn one_terminal_cannot_hold_a_run_it_closes_itself() {
        // `QUOTE … QUOTE` is two terminals with the same text, not a pair of
        // brackets, and telling them apart is the whole question.
        let tys = settled(
            &trivia(&[("Quote", &["\""])]),
            vec![named("Quote"), named("Chars"), named("Quote")],
        );

        assert_eq!(tys, vec![named("Quote"), named("Chars"), named("Quote")]);
    }

    #[test]
    fn a_list_held_by_two_terminals_is_folded_within_the_run() {
        let tys = settled(
            &trivia(&[("Open", &["["]), ("Close", &["]"]), ("Comma", &[","])]),
            vec![
                named("Open"),
                many(FieldType::Group(vec![named("Item"), named("Comma")])),
                FieldType::Opt(Box::new(named("Item"))),
                named("Close"),
            ],
        );

        assert_eq!(
            tys,
            vec![FieldType::Delimited {
                open: Box::new(named("Open")),
                body: vec![FieldType::Punctuated {
                    item: Box::new(named("Item")),
                    sep: Box::new(named("Comma")),
                }],
                close: Box::new(named("Close")),
            }]
        );
    }

    #[test]
    fn a_gap_stands_only_where_neither_neighbour_reaches() {
        let kept = settled(
            &trivia(&[("Dot", &["."])]),
            vec![named("Dot"), named("A"), named("Dot")],
        );

        assert_eq!(kept, vec![named("Dot"), named("A"), named("Dot")]);

        let lost = settled(&trivia(&[]), vec![named("A"), named("B"), named("C")]);

        assert_eq!(
            lost,
            vec![
                named("A"),
                FieldType::Gap,
                named("B"),
                FieldType::Gap,
                named("C")
            ]
        );
    }

    #[test]
    fn a_run_of_one_owns_its_trivia_from_the_front() {
        // The first item is reached over the sequence's own gap, which a
        // repetition cannot keep on its own, while each item after the first is
        // only reached over the run the repetition keeps for it.
        let tys = settled(
            &trivia(&[]),
            vec![FieldType::Concat {
                first: Box::new(named("X")),
                second: Box::new(many(named("X"))),
            }],
        );

        assert_eq!(
            tys,
            vec![FieldType::Concat {
                first: Box::new(named("X")),
                second: Box::new(many(FieldType::Group(vec![FieldType::Gap, named("X")]))),
            }]
        );
    }

    #[test]
    fn a_counted_run_is_settled_as_a_run_is() {
        let mut defs = vec![TypeDef {
            name: "Rule1".to_owned(),
            kind: TypeDefKind::Bounds {
                item: named("X"),
                min: 2,
                max: None,
            },
        }];

        trivia(&[]).settle(&mut defs);

        let TypeDefKind::Bounds { item, .. } = &defs[0].kind else {
            unreachable!("a counted run is bounds");
        };

        assert_eq!(item, &FieldType::Group(vec![FieldType::Gap, named("X")]));
    }

    #[test]
    fn a_file_with_no_trivia_keeps_the_operands_as_written() {
        let empty = Trivia::new(None, HashMap::new());
        let tys = settled(
            &empty,
            vec![
                named("A"),
                many(FieldType::Group(vec![named("A"), named("B")])),
                FieldType::Opt(Box::new(named("A"))),
            ],
        );

        // Nothing is anybody's to keep where there is no trivia at all, so the
        // pair that looks like a list is one whatever the operands hold.
        assert_eq!(
            tys,
            vec![
                named("A"),
                FieldType::Punctuated {
                    item: Box::new(named("A")),
                    sep: Box::new(named("B")),
                }
            ]
        );
        assert!(!empty.owns("A"));
        assert!(!empty.is_trivia("A"));
    }

    #[test]
    fn the_shapes_a_fold_makes_answer_for_the_trivia_beside_them() {
        let table = trivia(&[("Open", &["["]), ("Close", &["]"]), ("Comma", &[","])]);

        let list = FieldType::Punctuated {
            item: Box::new(named("Item")),
            sep: Box::new(named("Comma")),
        };
        let run = FieldType::Delimited {
            open: Box::new(named("Open")),
            body: vec![named("A")],
            close: Box::new(named("Close")),
        };

        assert!(!table.eats_leading(&list));
        assert!(!table.eats_trailing(&list));
        assert!(table.eats_leading(&run));
        assert!(table.eats_trailing(&run));

        let mut names = Vec::new();
        run.names(&mut names);

        assert_eq!(names, ["Open", "A", "Close"]);
    }
}
