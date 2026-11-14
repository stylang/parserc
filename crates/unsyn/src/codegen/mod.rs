//! Rust code for the grammar a definition file gives.
//!
//! The last stage of the crate: it takes the [`Module`]
//! the analysis produced and writes the Rust of the tree it describes — one type
//! per definition, each implementing
//! [`Syntax`](parserc::syntax::Syntax) over an
//! [`Input`](parserc::Input) stream, which is the shape a concrete syntax tree
//! of [`parserc`] is made of. The generated types *are* the CST nodes of the
//! described language, and their `parse` functions are its parser.
//!
//! The code is built from [`proc_macro2::TokenStream`] with
//! [`quote`](quote::quote!), the same technique a proc macro uses, so that the
//! derive macro of the backend and this generator can share one way of writing a
//! node.
//!
//! # The stages it stands on
//!
//! Generating is the last of four passes over a source file, each one dependent
//! on the previous one:
//!
//! - [`input`](crate::input) holds the [`Input`](parserc::Input) the parsers share.
//! - [`lexical`](crate::lexical) recognises the tokens: identifiers, keywords,
//!   punctuation, delimiters, literals and the several comment and documentation
//!   forms.
//! - [`syntax`](crate::syntax) applies the context-free grammar of the language
//!   and produces a [`Crate`](crate::syntax::Crate), the CST of the source file.
//! - [`semantics`](crate::semantics) gives that CST meaning: it resolves the
//!   references between nodes, validates their patterns, rejects the definitions
//!   that can start with themselves, and reports what a definition says twice as
//!   a lint.
//!
//! ```
//! use parserc::syntax::Syntax;
//! use unsyn::{input::TokenStream, semantics::analyze, syntax::Crate};
//!
//! let source = r#"
//!     whitespace W -> U+0020 | U+0009;
//!     lexer XID_Start -> <XID_Start>;
//!     lexer XID_Continue -> <XID_Continue>;
//!     lexer IDENTIFIER -> ( XID_Start | '_' ) XID_Continue*;
//!     syntax PROGRAM -> IDENTIFIER;
//! "#;
//!
//! let mut input = TokenStream::from(source);
//! let file = Crate::parse(&mut input).expect("the source should parse");
//!
//! let module = analyze(&file).expect("the source should be well-formed");
//!
//! assert_eq!(module.symbols().len(), 5);
//! assert_eq!(module.definition("IDENTIFIER").unwrap().kind().keyword(), "lexer");
//!
//! // `<Name>` is a call of a matcher the backend provides, not a node.
//! assert_eq!(module.matchers(), ["XID_Start", "XID_Continue"]);
//!
//! // A set whose members say nothing twice earns no lint.
//! assert!(module.lints().is_empty());
//! ```
//!
//! What those stages do one after the other, over `bootstrap.uns` and a small
//! spec written inline, is what `examples/pipeline.rs` runs and reports.
//!
//! # Two programs, one plan
//!
//! A `lexer` body reads text, so it is first compiled to a program over
//! characters, and that program is then run; a `syntax` body reads nodes, so it
//! is compiled to a *shape*: the fields of the type, and the order its `parse`
//! takes them in. The two are kept apart because they answer different questions
//! — how far along the text a token gets, and how many nodes a field holds — and
//! each has its own shortcuts.
//!
//! # The shortcuts
//!
//! Where a program is only a question about single characters it is a *class*,
//! and a repetition of one is scanned by the combinators of [`parserc`] in a
//! pass over the text rather than by looping over a matcher:
//! [`take_while`](parserc::take_while) for `X*`, [`take_till`](parserc::take_till)
//! for `~ […]*`, [`keyword`](parserc::keyword) for a literal of more than one
//! character, and [`take_until`](parserc::take_until) for `~ ['lit']*` where
//! the text after it has to be the literal. A node rule has shortcuts of its
//! own: the sequence of a group becomes a tuple, a repetition a [`Vec`] and an
//! optional operand an [`Option`], each of which the backend already implements
//! as syntax.
//!
//! # Names
//!
//! A definition's name is the type's name, and the literals and groups written
//! inside a `syntax` body are given types of their own: shared types for the
//! literals, since `'if'` in one rule and `'if'` in another are the same token,
//! and anonymous types for the groups of a rule, named after the rule they
//! belong to. Every name is decided before anything is written, so no two types
//! in a file collide.
//!
//! # Recursion
//!
//! A rule that names itself, or that names its way back to itself, cannot hold
//! what it names in a field of its own: the type would have no size. Those edges
//! are found over the shapes that were built, and the field is written as a
//! [`Box`], which the backend implements as syntax like everything else. The
//! analysis has already refused the recursion that comes back to the same
//! position without taking anything, so every cycle left is one a tree can hold.

mod builtin;
mod error;
mod item;
mod name;
mod scan;
mod shape;
mod token;

use std::collections::{HashMap, HashSet};

use parserc::syntax::Punctuated;
use proc_macro2::TokenStream;
use quote::quote;

pub use error::{CodegenError, CodegenKind};

use crate::input::UnsynInput;
use crate::lexical::delimiter::Bracket;
use crate::lexical::lit::{LitStr, LitUnicode};
use crate::lexical::punct::Comma;
use crate::semantics::{Module, NodeKind, nullable_nodes};
use crate::syntax::SetItem;

use item::Renderer;
use name::is_type_name;
use shape::{Shape, Shaper};
use token::{Builder as Program, LexRefs, Scanner};

/// A set of characters written inside a rule of nodes.
pub(crate) type SetOf<I> = Bracket<I, Punctuated<SetItem<I>, Comma<I>>>;

/// A literal string, as a pattern writes it.
pub(crate) type StrOf<I> = LitStr<I>;

/// A code point literal, as a pattern writes it.
pub(crate) type UnicodeOf<I> = LitUnicode<I>;

/// The members of a punctuated list, without their separators.
pub(crate) fn set_items<T, P>(punctuated: &Punctuated<T, P>) -> impl Iterator<Item = &T> {
    punctuated
        .pairs
        .iter()
        .map(|(item, _)| item)
        .chain(punctuated.tail.iter().map(|boxed| &**boxed))
}

/// The paths a generated file reaches the crates it needs by.
///
/// A generated file is a fragment of items, which cannot know where the two
/// crates it leans on live in the crate that includes it. It reaches them by
/// `::parserc` and `::unsyn` unless the project keeps them elsewhere, in which
/// case the paths are given here.
///
/// [`GeneratedInput`]: crate::runtime::GeneratedInput
#[derive(Debug, Clone)]
pub struct Paths {
    /// Where [`GeneratedInput`](crate::runtime::GeneratedInput) is, which every
    /// generated type is written over.
    pub unsyn: TokenStream,
    /// Where the combinators and [`Syntax`](parserc::syntax::Syntax) are.
    pub parserc: TokenStream,
    /// Where the [`matcher`](crate::runtime::matcher) module is, which is what
    /// `<NAME>` calls.
    pub matcher: TokenStream,
}

impl Default for Paths {
    fn default() -> Self {
        Self {
            unsyn: quote!(::unsyn),
            parserc: quote!(::parserc),
            matcher: quote!(::unsyn::runtime::matcher),
        }
    }
}

/// The generator: how to write a file, and nothing of the grammar in it.
///
/// One generator writes a whole [`Module`] and is
/// reusable across files, since the state of a generation lives in the plan it
/// builds and not here.
///
/// [`Module`]: crate::semantics::Module
#[derive(Debug, Clone)]
pub struct Codegen {
    /// The paths the generated file reaches the crates by.
    paths: Paths,
    /// The visibility of every generated type.
    visibility: TokenStream,
    /// The derives every generated type is given.
    derives: Vec<String>,
}

impl Default for Codegen {
    fn default() -> Self {
        Self {
            paths: Paths::default(),
            visibility: quote!(pub),
            derives: ["Debug", "Clone", "PartialEq", "Eq", "PartialOrd", "Ord"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}

/// What to write for one file: every type, in the order they are declared.
///
/// This is the reading half of a generation finished: what was left ambiguous
/// about the grammar is settled here, and nothing about the shape of the Rust
/// file is.
pub(crate) struct Plan {
    /// The name of the whitespace rule's type, which is what every separator
    /// names, or nothing when the file declares no trivia.
    whitespace: Option<String>,
    /// The types that read text: the file's own rules, then the token types the
    /// node shapes needed.
    scanners: Vec<Scanner>,
    /// The types that hold nodes: the file's own rules, then the groups, the
    /// counted runs and the assertions cut out of them.
    shapes: Vec<Shape>,
}

impl Codegen {
    /// A generator that writes a file the way `parserc` and `unsyn` are named
    /// in a crate that depends on both.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the paths the generated file reaches the crates by.
    #[inline]
    pub fn with_paths(mut self, paths: Paths) -> Self {
        self.paths = paths;

        self
    }

    /// Sets the visibility of every generated type, `pub` by default and
    /// `pub(crate)` where the file is private to the crate that includes it.
    #[inline]
    pub fn with_visibility(mut self, visibility: impl Into<TokenStream>) -> Self {
        self.visibility = visibility.into();

        self
    }

    /// Sets the derives every generated type is given.
    ///
    /// Every type the generator writes is built only out of types it writes
    /// itself and out of the stream, so this list is what the described
    /// language's nodes carry: narrowing it is always possible, and widening it
    /// asks the stream type to carry the traits as well.
    #[inline]
    pub fn with_derives(mut self, derives: impl IntoIterator<Item = String>) -> Self {
        self.derives = derives.into_iter().collect();

        self
    }

    /// The Rust of a whole analyzed file, as one stream of tokens.
    ///
    /// Reports every construct it could not write, each with the span it was
    /// written at, rather than leaving it quietly out of the file.
    pub fn generate<I: UnsynInput>(
        &self,
        module: &Module<'_, I>,
    ) -> Result<TokenStream, Vec<CodegenError>> {
        self.render(&self.plan(module)?)
    }

    /// The Rust of a whole analyzed file, as text a caller can write out.
    pub fn generate_str<I: UnsynInput>(
        &self,
        module: &Module<'_, I>,
    ) -> Result<String, Vec<CodegenError>> {
        Ok(self.generate(module)?.to_string())
    }

    /// What to write for a whole analyzed file, without writing it.
    ///
    /// This is the half that reads the grammar: the programs of the rules that
    /// take text, the shapes of the rules that hold nodes, and the types the
    /// shapes needed along the way.
    pub(crate) fn plan<I: UnsynInput>(
        &self,
        module: &Module<'_, I>,
    ) -> Result<Plan, Vec<CodegenError>> {
        let symbols = module.symbols();

        // The names the file itself gives are the names of its types, so they
        // are all claimed before any type is written, and a name Rust would not
        // accept is reported rather than emitted.
        let errors: Vec<CodegenError> = symbols
            .iter()
            .filter(|def| !is_type_name(def.name()))
            .map(|def| CodegenError {
                kind: CodegenKind::InvalidName(def.name().to_owned()),
                span: def.name_span().clone(),
            })
            .collect();

        if !errors.is_empty() {
            return Err(errors);
        }

        let reserved = symbols.iter().map(|def| def.name().to_owned());
        let whitespace = symbols.whitespace().map(|def| def.name().to_owned());

        let refs = LexRefs {
            tokens: names(symbols, false),
            syntaxes: names(symbols, true),
            empty: nullable_nodes(symbols)
                .into_iter()
                .filter(|(_, nullable)| *nullable)
                .map(|(name, _)| name.to_owned())
                .collect(),
        };

        // The programs of the rules that read text come first, since a node
        // shape may name one of them and a token rule may name another.
        let mut scanners = Vec::new();
        let mut errors = Vec::new();

        for def in symbols.iter().filter(|def| def.kind() != NodeKind::Syntax) {
            match Program::new(&refs).expr(def.expr()) {
                Ok(inst) => scanners.push(Scanner::plain(def.name(), inst)),
                Err(error) => errors.push(error),
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        let mut shaper = Shaper::new(&refs, reserved, whitespace.clone());
        let mut shapes = Vec::new();

        for def in symbols.iter().filter(|def| def.kind() == NodeKind::Syntax) {
            match shaper.shape(def.name(), def.expr()) {
                Ok(shape) => shapes.push(shape),
                Err(error) => errors.push(error),
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        let (built, more) = shaper.finish();

        scanners.extend(built);
        shapes.extend(more);

        Ok(Plan {
            whitespace,
            scanners,
            shapes,
        })
    }

    /// The file a plan describes: every type it holds, in the order it holds
    /// them.
    fn render(&self, plan: &Plan) -> Result<TokenStream, Vec<CodegenError>> {
        let classes = plan.classes();
        let renderer = Renderer::new(
            &self.paths,
            &self.visibility,
            &self.derives,
            plan.whitespace.as_deref(),
        );
        let edges = plan.edges();
        let mut items = Vec::new();
        let mut errors = Vec::new();

        for scanner in &plan.scanners {
            match renderer.scanner(scanner, &classes) {
                Ok(item) => items.push(item),
                Err(error) => errors.push(error),
            }
        }

        for shape in &plan.shapes {
            items.push(renderer.node(shape, &plan.pointed(&edges, &shape.name)));
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(quote!(#(#items)*))
    }
}

impl Plan {
    /// The class each rule that reads text takes, which is what makes a
    /// repetition of one character scannable rather than looped.
    fn classes(&self) -> HashMap<String, token::Pred> {
        scan::classes(&self.scanners)
    }

    /// The node types each node type names, which is what a cycle is found in.
    fn edges(&self) -> HashMap<String, HashSet<String>> {
        let nodes: HashSet<&str> = self
            .shapes
            .iter()
            .map(|shape| shape.name.as_str())
            .collect();
        let mut edges: HashMap<String, HashSet<String>> = HashMap::new();

        for shape in &self.shapes {
            let mut named = Vec::new();

            shape.names(&mut named);

            edges.insert(
                shape.name.clone(),
                named
                    .into_iter()
                    .filter(|name| nodes.contains(name.as_str()))
                    .collect(),
            );
        }

        edges
    }

    /// The types a field of `owner` has to hold behind a pointer: its own rule
    /// among them, since a rule that names itself is the plainest cycle there
    /// is.
    fn pointed(&self, edges: &HashMap<String, HashSet<String>>, owner: &str) -> HashSet<String> {
        edges
            .keys()
            .filter(|from| reaches(edges, from, owner, &mut HashSet::new()))
            .cloned()
            .collect()
    }
}

/// Returns `true` when following the edges out of `from` arrives at `target`,
/// which is the cycle a field of that name would close.
fn reaches(
    edges: &HashMap<String, HashSet<String>>,
    from: &str,
    target: &str,
    seen: &mut HashSet<String>,
) -> bool {
    let Some(named) = edges.get(from) else {
        return false;
    };

    for next in named {
        if next == target {
            return true;
        }

        if seen.insert(next.clone()) && reaches(edges, next, target, seen) {
            return true;
        }
    }

    false
}

/// The names of the definitions of one kind: the node rules when `syntax`, and
/// the rules that read text otherwise.
fn names<I: UnsynInput>(
    symbols: &crate::semantics::SymbolTable<'_, I>,
    syntax: bool,
) -> HashSet<String> {
    symbols
        .iter()
        .filter(move |def| (def.kind() == NodeKind::Syntax) == syntax)
        .map(|def| def.name().to_owned())
        .collect()
}
