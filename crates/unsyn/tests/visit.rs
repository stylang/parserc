//! The walk of a tree, checked from both ends.
//!
//! [`unsyn::visit::Visit`] is the interface a pass is written against, and it has
//! to be right in two separate ways, which is why this file has two halves.
//!
//! The first half runs a walk over a file and asks what it saw. A walk owes a
//! pass every token of the source, once, in the order the source has them; and
//! every node the language can build must be reachable through the method named
//! for it, because a method no file can reach is a method a pass would write for
//! nothing.
//!
//! The second half asks a question no run can answer: whether the interface is
//! *the* one, rather than one that happens to fit the files tried so far. A
//! bootstrapped parser is a second tree of the same language, built from types
//! this crate writes rather than types it hand-wrote, and the two share no Rust
//! type — so the only thing that can make one pass read both is an interface that
//! follows from the tree and says nothing else. That is what these tests measure,
//! straight out of the source: `tests/bootstrap.rs` reads the same file the same
//! way for the same reason.
//!
//! [`unsyn::visit::Visit`]: unsyn::visit::Visit

use std::{collections::BTreeSet, fs, path::PathBuf};

use parserc::{AsStr, Span, syntax::Syntax};
use unsyn::{
    input::{TokenStream, UnsynInput},
    lexical::{
        S,
        comments::{InnerBlockDoc, InnerLineDoc, OuterBlockDoc, OuterLineDoc},
        ident::Ident,
        keyword::{Concat, Except, Followed, Lexer, Syntax as SyntaxKeyword, Whitespace},
        lit::{AsciiEscape, LitDec, LitStr, LitUnicode, QuoteEscape, StrSegment, UnicodeEscape},
        punct::{
            ArrowRight, BraceEnd, BraceStart, BracketEnd, BracketStart, Comma, Gt, Lt, Minus, Or,
            ParenEnd, ParenStart, Plus, Question, Semi, Star, Tilde,
        },
    },
    syntax::{
        Crate, Expr, ExprNoTopAlt, ExprNoTopAlts, ExprWithSuffix, ExprWithoutSuffix, InnerDoc,
        Item, LeadingKey, OuterDoc, Range, Repeat, SetItem, Stmt,
    },
    visit::{Visit, walk},
};

// ## A walk sees the whole file

/// A file in every form the language has, so that a walk of it reaches every node
/// the interface names.
///
/// The rules are the ones `bootstrap.uns` writes, kept short and about one thing
/// each: what matters here is that every kind of operand, every form of set
/// member, every suffix, every escape and every kind of document stands in the
/// file at least once.
const EVERY_FORM: &str = r#"//! The document that opens the file.
/*! A second one, written as a block. */
/// The document of the statement that follows it.
/** A block document, and the last of them. */
whitespace S -> [U+0009-U+000D, U+0020];
lexer NAME_START -> <XID_Start> | '_' | <XID_Continue>*;
lexer TEXT -> 'a\tA\x41\u{42}\'' '\n' '\r' '\t' '\\' '\0' '\x7F' 'é';
lexer ONE_OF -> ~['x', '\n'] | ['a', U+0041, 'é', TEXT, '0'-'9', U+0041-U+005A];
lexer RUNS -> 'a'? 'b'* 'c'+ NAME_START{2} TEXT{,3} 'd'{1,} 'e'{1,2};
lexer UNTIL -> LineComment followed [';'];
lexer CUT -> LineComment except [';'];
lexer JOINED -> 'a' concat 'b';
syntax GROUP -> ( RUNS | ONE_OF ) except [' '];
"#;

/// Collects the text of every token a walk reaches, in the order it reaches them.
///
/// This is all a pass that only wants the tokens has to write, which makes it a
/// fair test of the defaults: the walk down to the leaves is the trait's, not the
/// pass's.
#[derive(Default)]
struct Tokens {
    texts: Vec<String>,
    spans: Vec<Span>,
}

impl<'ast, I: UnsynInput> Visit<'ast, I> for Tokens {
    fn visit_input(&mut self, input: &'ast I) {
        self.texts.push(input.as_str().to_owned());
        self.spans.push(input.to_span());
    }
}

/// Parses [`EVERY_FORM`] into a tree.
fn every_form() -> Crate<TokenStream<'static>> {
    let mut stream = TokenStream::from(EVERY_FORM);

    Crate::parse(&mut stream).expect("a file in every form of the language should parse")
}

#[test]
fn a_walk_of_a_file_reads_every_token_once_and_in_order() {
    let file = every_form();

    let mut tokens = Tokens::default();
    tokens.visit_crate(&file);

    // Every character of the file is in one of the tokens the walk reached and in
    // no other, which is what it means for the walk to have entered them all and
    // none of them twice: the concatenation is the file.
    assert_eq!(tokens.texts.concat(), EVERY_FORM);

    // The places the tokens came from agree with their text, follow one another,
    // and leave nothing between them for the walk to have skipped.
    let mut offset = 0;

    for (span, text) in tokens.spans.iter().zip(&tokens.texts) {
        let Span::Range(range) = span else {
            panic!("a token read from a file has a range, not {span:?}");
        };

        assert_eq!(
            range.end - range.start,
            text.len(),
            "{text:?} is not as long as its span"
        );
        assert_eq!(
            &EVERY_FORM[range.clone()],
            text,
            "a token is not the text at its own place"
        );
        assert_eq!(
            range.start, offset,
            "the tokens the walk reached are not in the order the file has them"
        );

        offset = range.end;
    }

    assert_eq!(
        offset,
        EVERY_FORM.len(),
        "the walk stopped short of the end of the file"
    );
}

// ## Every method is reached, and hands on

/// The same walk, recording which methods it entered.
#[derive(Default)]
struct Spy {
    entered: Vec<&'static str>,
    tokens: Tokens,
}

/// Writes the whole of [`Spy`]'s implementation from one list, so that the list
/// below is the only place the interface's methods are named in this file.
///
/// Each entry gives the method, the `walk_*` it hands on to, and the node it is
/// about. Handing on is what makes the recording mean anything: a method that
/// noted itself and stopped would prove nothing about the nodes beneath it, which
/// is why the test that reads this also asks for the tokens.
macro_rules! spy {
    ($($method:ident = $walk:ident : $ty:ident),* $(,)?) => {
        impl<'ast, I: UnsynInput> Visit<'ast, I> for Spy {
            $(
                fn $method(&mut self, node: &'ast $ty<I>) {
                    self.entered.push(stringify!($method));
                    walk::$walk(self, node);
                }
            )*

            fn visit_input(&mut self, input: &'ast I) {
                self.entered.push("visit_input");
                self.tokens.visit_input(input);
            }
        }

        /// Every method of the interface: the leaf, then the ones the list above
        /// hands on.
        const METHODS: &[&str] = &["visit_input", $( stringify!($method), )*];
    };
}

spy! {
    visit_crate = walk_crate: Crate,
    visit_item = walk_item: Item,
    visit_inner_doc = walk_inner_doc: InnerDoc,
    visit_outer_doc = walk_outer_doc: OuterDoc,
    visit_stmt = walk_stmt: Stmt,
    visit_leading_key = walk_leading_key: LeadingKey,
    visit_expr = walk_expr: Expr,
    visit_expr_no_top_alts = walk_expr_no_top_alts: ExprNoTopAlts,
    visit_expr_no_top_alt = walk_expr_no_top_alt: ExprNoTopAlt,
    visit_expr_with_suffix = walk_expr_with_suffix: ExprWithSuffix,
    visit_expr_without_suffix = walk_expr_without_suffix: ExprWithoutSuffix,
    visit_set_item = walk_set_item: SetItem,
    visit_range = walk_range: Range,
    visit_repeat = walk_repeat: Repeat,
    visit_ident = walk_ident: Ident,
    visit_s = walk_s: S,
    visit_lit_str = walk_lit_str: LitStr,
    visit_str_segment = walk_str_segment: StrSegment,
    visit_ascii_escape = walk_ascii_escape: AsciiEscape,
    visit_unicode_escape = walk_unicode_escape: UnicodeEscape,
    visit_quote_escape = walk_quote_escape: QuoteEscape,
    visit_lit_unicode = walk_lit_unicode: LitUnicode,
    visit_lit_dec = walk_lit_dec: LitDec,
    visit_inner_line_doc = walk_inner_line_doc: InnerLineDoc,
    visit_inner_block_doc = walk_inner_block_doc: InnerBlockDoc,
    visit_outer_line_doc = walk_outer_line_doc: OuterLineDoc,
    visit_outer_block_doc = walk_outer_block_doc: OuterBlockDoc,
    visit_plus = walk_plus: Plus,
    visit_minus = walk_minus: Minus,
    visit_star = walk_star: Star,
    visit_question = walk_question: Question,
    visit_tilde = walk_tilde: Tilde,
    visit_or = walk_or: Or,
    visit_paren_start = walk_paren_start: ParenStart,
    visit_paren_end = walk_paren_end: ParenEnd,
    visit_bracket_start = walk_bracket_start: BracketStart,
    visit_bracket_end = walk_bracket_end: BracketEnd,
    visit_brace_start = walk_brace_start: BraceStart,
    visit_brace_end = walk_brace_end: BraceEnd,
    visit_lt = walk_lt: Lt,
    visit_gt = walk_gt: Gt,
    visit_comma = walk_comma: Comma,
    visit_arrow_right = walk_arrow_right: ArrowRight,
    visit_semi = walk_semi: Semi,
    visit_whitespace = walk_whitespace: Whitespace,
    visit_lexer = walk_lexer: Lexer,
    visit_syntax = walk_syntax: SyntaxKeyword,
    visit_followed = walk_followed: Followed,
    visit_concat = walk_concat: Concat,
    visit_except = walk_except: Except,
}

#[test]
fn every_method_of_the_interface_is_reached_by_a_file_of_the_language() {
    let file = every_form();

    let mut spy = Spy::default();
    spy.visit_crate(&file);

    // Nothing in the interface is decoration. A method no file of the language can
    // reach is a method a generated tree would have to invent an answer for, and a
    // pass written against one would run on nothing at all.
    let mut unreached = METHODS
        .iter()
        .filter(|method| !spy.entered.contains(method))
        .copied()
        .collect::<Vec<_>>();

    unreached.sort_unstable();

    assert!(
        unreached.is_empty(),
        "no file of the language reaches {unreached:?}, so the interface names a \
         node the language does not have"
    );

    // The list is a set, which is the other way this test could pass by accident:
    // a list that named one method twice would hide a method that is never called.
    let distinct = METHODS.iter().collect::<BTreeSet<_>>();

    assert_eq!(
        distinct.len(),
        METHODS.len(),
        "the list of methods hands one of them to the walk twice"
    );

    // And every method handed on, since the leaves the walk reported are the whole
    // file.
    assert_eq!(
        spy.tokens.texts.concat(),
        EVERY_FORM,
        "a method recorded its node but did not carry on down to the tokens"
    );
}

// ## The interface follows from the tree, and only from the tree

/// Reads one file of this crate, so that the tests below can ask it what it has
/// rather than being told by a list kept beside them.
///
/// `tests/bootstrap.rs` reads the Rust the crate builds in order to ask a question
/// about the language; these read the interface the same way for the same reason,
/// because whether this walk is the one a generated tree would answer to is not a
/// thing a running program can be asked.
fn read(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);

    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// One method of [`Visit`]: its name, the `walk_*` its default body hands the node
/// to, and whether that body is nothing but the handing on.
struct Method {
    name: String,
    has_body: bool,
    hands_on_to: Option<String>,
    only_the_walk: bool,
}

/// The methods of [`Visit`] and the functions of [`unsyn::visit::walk`], read out of
/// the source of the module they live in.
///
/// Read as Rust and not as text, so that a method renamed in one place and not the
/// other is caught here rather than at the point where a pass written against the
/// old name quietly stops running.
///
/// [`unsyn::visit::walk`]: unsyn::visit::walk
fn interface() -> (Vec<Method>, BTreeSet<String>) {
    let file = syn::parse_file(&read("src/visit/mod.rs")).expect("the visit module is Rust");
    let walks = walk_functions();

    let mut methods = Vec::new();

    for item in &file.items {
        let syn::Item::Trait(trait_) = item else {
            continue;
        };

        assert_eq!(
            trait_.ident, "Visit",
            "the module has a trait beside `Visit`"
        );

        for item in &trait_.items {
            let syn::TraitItem::Fn(method) = item else {
                continue;
            };

            let name = method.sig.ident.to_string();

            // The leaf has no default body at all: a pass that wants the tokens of a
            // file answers for it, and nothing above it can answer on its behalf.
            let Some(body) = &method.default else {
                methods.push(Method {
                    name,
                    has_body: false,
                    hands_on_to: None,
                    only_the_walk: false,
                });

                continue;
            };

            // Nor may a default body do anything but hand the node to the walk named
            // for it. That rule is what keeps the knowledge of a node in one
            // function, which is in turn what lets a generator emit both halves of an
            // interface out of a tree without writing that knowledge twice.
            let only_the_walk = body.stmts.len() == 1;

            let hands_on_to = match &body.stmts[..] {
                [syn::Stmt::Expr(syn::Expr::Call(call), _)] => match &*call.func {
                    syn::Expr::Path(target) => Some(
                        target
                            .path
                            .segments
                            .last()
                            .expect("a call of a walk names one")
                            .ident
                            .to_string(),
                    ),
                    _ => None,
                },
                _ => None,
            };

            methods.push(Method {
                name,
                has_body: true,
                hands_on_to,
                only_the_walk,
            });
        }
    }

    (methods, walks)
}

/// The public functions of the walk module, which are read from its own file: the
/// module is declared as `pub mod walk;`, so there is no body here for `syn` to see.
///
/// Only the public ones count. A private function of that module is how the parts of
/// one node are reached — the group a suffix applies to, the members of a set, the
/// ends and body of a definition — and names no node of the tree, which is precisely
/// what a generator would have to invent a name for and so does not have one.
fn walk_functions() -> BTreeSet<String> {
    let file = syn::parse_file(&read("src/visit/walk.rs")).expect("the walk module is Rust");

    file.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(function)
                if matches!(function.vis, syn::Visibility::Public(_))
                    && function.sig.ident.to_string().starts_with("walk_") =>
            {
                Some(function.sig.ident.to_string())
            }
            _ => None,
        })
        .collect()
}

/// Turns the name of a node type into the name of its method, which is the only
/// naming rule the interface follows: the type in snake case, after `visit_`.
///
/// No such rule exists in the crate yet — nothing in [`unsyn::codegen`] snake-cases,
/// since the names it writes are the ones the tree already has — so it is spelled out
/// here until P6 makes the two share one. It needs no handling of acronyms to reach
/// the names the language has, and a name this rule cannot reach fails the test below
/// rather than passing quietly.
///
/// [`unsyn::codegen`]: unsyn::codegen
fn method_for(ty: &str) -> String {
    let mut name = String::from("visit_");

    for (index, c) in ty.char_indices() {
        if c.is_uppercase() && index > 0 {
            name.push('_');
        }

        name.extend(c.to_lowercase());
    }

    name
}

/// The node types of the hand-written tree.
///
/// Every type of [`unsyn::lexical`] and [`unsyn::syntax`] a node of [`Crate`] can be
/// built out of, read off the source of those two modules: a `pub struct` or `pub
/// enum` generic over the input alone, plus the punctuation and the keywords, which a
/// macro call declares rather than writes out, so that the call is the only place
/// their names stand.
///
/// [`unsyn::lexical`]: unsyn::lexical
/// [`unsyn::syntax`]: unsyn::syntax
fn node_types() -> BTreeSet<String> {
    // The kinds of comment a delimiter skips over. They are types of the lexical
    // module but not nodes of the tree: a comment reaches a file only inside the
    // whitespace run that covers it, which is why the text of a document goes to
    // `visit_input` rather than to a method of the comment's own. Everything else
    // those two modules define over the input alone is a node, and has a method.
    const NOT_A_NODE: &[&str] = &["BlockComment", "LineComment"];

    let mut types = BTreeSet::new();

    for path in [
        "src/lexical/s.rs",
        "src/lexical/ident.rs",
        "src/lexical/punct.rs",
        "src/lexical/keyword.rs",
        "src/lexical/lit.rs",
        "src/lexical/comments.rs",
        "src/lexical/delimiter.rs",
        "src/syntax/mod.rs",
        "src/syntax/item.rs",
        "src/syntax/expr.rs",
    ] {
        let file = syn::parse_file(&read(path)).expect("the tree is Rust");

        for item in &file.items {
            match item {
                // A node is generic over the input and nothing else. A view over a
                // node takes a lifetime as well, which is how `SuffixOp` and
                // `RepeatBound` are told apart from the nodes they read; a shape that
                // holds a node takes the input and its contents, which is how the
                // delimiter aliases are told apart from the nodes they wrap.
                syn::Item::Struct(item) if is_a_node(&item.ident, &item.generics, NOT_A_NODE) => {
                    types.insert(item.ident.to_string());
                }
                syn::Item::Enum(item) if is_a_node(&item.ident, &item.generics, NOT_A_NODE) => {
                    types.insert(item.ident.to_string());
                }
                // One node is a name for a shape rather than a shape of its own, and
                // the distinction is there in the declaration: an alias of a group
                // bounded by a pair of delimiters is stored as a node, because the
                // walk has to enter the braces it stands between, while an alias of a
                // choice is not, because the walk enters whichever of the choices the
                // file picked. `Repeat` is the first and `SuffixGroup` the second.
                //
                // The rule is a proxy for "the spec writes this on the left of a
                // production", which is the real reason, and it holds only while no
                // delimiter-bounded group of the grammar is both unnamed on its left
                // and named here for convenience: the `[` … `]` of a set is such a
                // group, so aliasing it in the tree would ask this test for a
                // `visit_set` the spec gives no node. A module that wants the long
                // spelling shorter aliases it for itself, as `codegen` does.
                syn::Item::Type(item)
                    if is_a_node(&item.ident, &item.generics, NOT_A_NODE)
                        && bounds_a_group(&item.ty) =>
                {
                    types.insert(item.ident.to_string());
                }
                syn::Item::Macro(item) => {
                    // A `macro_rules!` definition has no path in front of its name;
                    // only a line that invokes one of the two declaring macros lists a
                    // type, and it may list several.
                    let Some(last) = item.mac.path.segments.last() else {
                        continue;
                    };

                    if last.ident == "define_punct" || last.ident == "define_keyword" {
                        // One type per call, named first and followed by the text it
                        // stands for — which is itself full of the `,` and `;` that
                        // would split it again, so only the name is taken.
                        let name = item
                            .mac
                            .tokens
                            .to_string()
                            .split(',')
                            .next()
                            .expect("a punct is declared by a name and its text")
                            .trim()
                            .to_owned();

                        types.insert(name);
                    }
                }
                _ => {}
            }
        }
    }

    types
}

/// Whether a `pub type` alias names a node because it stands for a group bounded by
/// a pair of delimiters, which is the only thing an alias can name and still be
/// walked into.
fn bounds_a_group(ty: &syn::Type) -> bool {
    let syn::Type::Path(path) = ty else {
        return false;
    };

    let Some(last) = path.path.segments.last() else {
        return false;
    };

    matches!(
        last.ident.to_string().as_str(),
        "Brace" | "Bracket" | "Paren" | "Angle" | "Delimiter"
    )
}

/// Whether a type one of those two modules declares is a node of the tree: named
/// over the input alone, and not one of the kinds of token the tree keeps no node for.
fn is_a_node(name: &syn::Ident, generics: &syn::Generics, not_a_node: &[&str]) -> bool {
    let text = name.to_string();

    if not_a_node.contains(&text.as_str()) {
        return false;
    }

    let params = generics.params.iter().collect::<Vec<_>>();

    let [syn::GenericParam::Type(param)] = &params[..] else {
        return false;
    };

    param.ident == "I"
}

#[test]
fn each_method_of_the_interface_hands_its_node_to_the_walk_named_for_it() {
    let (methods, walks) = interface();

    for method in &methods {
        if !method.has_body {
            assert_eq!(
                method.name, "visit_input",
                "{} has no default body, so every pass would have to answer for it",
                method.name
            );

            continue;
        }

        let Some(walk) = &method.hands_on_to else {
            panic!("{}'s default body is not one call to a walk", method.name);
        };

        assert!(
            method.only_the_walk,
            "{}'s default body is more than the call to {walk}",
            method.name
        );

        assert!(
            walks.contains(walk),
            "{} hands its node to {walk}, which the walk module does not define",
            method.name
        );

        assert_eq!(
            *walk,
            format!("walk_{}", method.name.trim_start_matches("visit_")),
            "{} does not hand its node to the walk named for the same type",
            method.name
        );
    }

    // The other direction: the walk module knows exactly the nodes the trait has
    // methods for, so neither half is a list the other has to be told about.
    let handed_on = methods
        .iter()
        .filter_map(|method| method.hands_on_to.clone())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        handed_on, walks,
        "the trait and the walk module disagree about which nodes a walk knows"
    );
}

#[test]
fn the_interface_names_exactly_the_nodes_of_the_tree() {
    let (methods, _) = interface();

    let named = methods
        .iter()
        .map(|method| method.name.clone())
        .filter(|name| name != "visit_input")
        .collect::<BTreeSet<_>>();

    let expected = node_types()
        .iter()
        .map(|ty| method_for(ty))
        .collect::<BTreeSet<_>>();

    let no_such_node = expected.difference(&named).collect::<Vec<_>>();
    let no_such_method = named.difference(&expected).collect::<Vec<_>>();

    assert!(
        no_such_node.is_empty() && no_such_method.is_empty(),
        "the interface and the tree are not one list: {no_such_node:?} are methods for \
         no node type, and {no_such_method:?} are node types with no method"
    );

    // Which is also the list a walk of a real file was checked against above, so the
    // two halves of this file cannot drift apart from one another either.
    assert_eq!(
        METHODS.len(),
        methods.len(),
        "the file of the language is walked over a different set of methods than the \
         interface has"
    );
}

/// The node types the language's own definition of itself names: the left-hand side
/// of every `syntax` statement of `bootstrap.uns`.
///
/// The weaker half of the contract, and meant to be read that way. The spec describes
/// the patterns a definition's body is written with, and not the file those
/// definitions stand in — `Crate`, `Item`, `InnerDoc` and `OuterDoc` are on the left of
/// no production of it, which is the hole `BOOTSTRAP_PLAN.md` keeps open as P6 — so the
/// spec can say that a method is missing, and cannot say that one is spare.
#[test]
fn every_node_the_spec_defines_has_a_method() {
    let content = read("bootstrap.uns");
    let mut stream = TokenStream::from(content.as_str());

    let spec = Crate::parse(&mut stream).expect("the spec of the language parses");

    let defined = spec
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Stmt(stmt) if matches!(stmt.keyword, LeadingKey::Syntax(_)) => {
                Some(stmt.ident.0.as_str().to_owned())
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    assert!(
        !defined.is_empty(),
        "the spec defines no syntax node, so this test would prove nothing"
    );

    let (methods, _) = interface();
    let named = methods
        .iter()
        .map(|method| method.name.clone())
        .collect::<BTreeSet<_>>();

    let missing = defined
        .iter()
        .filter(|name| !named.contains(&method_for(name)))
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "the language defines {missing:?} and the walk has no method named for them, so \
         a pass written against the interface would not see them in a bootstrapped tree"
    );
}
