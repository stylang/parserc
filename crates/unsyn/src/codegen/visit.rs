//! A visitor over the CST, in the shape [`syn`] gives its own visitors.
//!
//! [`syn`]: https://docs.rs/syn/latest/syn/visit/index.html
//!
//! The interface is mechanical, and deliberately so: it is the shape a generator
//! must be able to write out of a grammar without knowing anything else about it.
//! Every rule of the model therefore gets one method, and every method a walk:
//!
//! - one `visit_*` per node type, named by the left side of the production that
//!   declares it — [`Visit::visit_stmt`] for `syntax Stmt -> …`,
//!   [`Visit::visit_lit_str`] for `lexer LitStr -> …` — plus one per terminal,
//!   since a literal of the pattern level is a node type too;
//! - the default body of each is `walk::walk_*`, which hands every child of the
//!   node to its own method, in the order the children are written in the source;
//! - [`Visit::visit_text`] is the one hook below the nodes: the text a token
//!   holds, and the only argument a visitor receives that says nothing about
//!   which node it came from. A visitor that wants every character of a file
//!   overrides that one method and gets them, in order.
//!
//! Two things are not methods, because nothing in the grammar names them: the
//! groups a right side writes inline — `X+`, `('[' … ']')`, the choice of the four
//! quantity operators — and the shape a repetition has. Their children are walked
//! by the private helpers of [`walk`], which is why a walk reads like the
//! production it comes from.
//!
//! Inside [`walk`] the 44 nodes that hold one token and nothing else — every
//! terminal and every lexer rule — are written by two local macros, since their
//! walks share one body. That is how this module is laid out and not how the
//! interface is: the methods a caller implements and the order they are called in
//! are the same as they would be written out, and the test that reassembles a file
//! from the text a visitor was handed is what pins that down.
//!
//! Where `syn` funnels every piece of punctuation into a single `visit_tokens`,
//! each terminal here is its own type and so its own method: `';'` and `'|'` are
//! different nodes of this tree, and the one thing a generator cannot recover from
//! a grammar is which of its literals stand where. Two other differences follow
//! from the same decision: the node types are generic over the input they were read
//! from, so the trait carries `I` beside `'ast`, and a lexer rule holds the text it
//! matched rather than the tokens it was written out of, which is why its walk ends
//! at [`Visit::visit_text`] instead of descending.
//!
//! A visit begins wherever the tree does, which for a file is
//! `visitor.visit_crate(&crate)`; every other node reaches the visitor through the
//! walk of the node that holds it. To visit a tree, implement the methods that
//! matter and call the walk to keep going:
//!
//! ```
//! use parserc::AsStr;
//! use unsyn::codegen::input::TokenStream;
//! use unsyn::codegen::syntax::Stmt;
//! use unsyn::codegen::visit::{Visit, walk};
//!
//! #[derive(Default)]
//! pub struct Definitions<'ast> {
//!     /// The name of every rule the file defines, in the order it defined them.
//!     pub names: Vec<&'ast str>,
//! }
//!
//! impl<'ast> Visit<'ast, TokenStream<'ast>> for Definitions<'ast> {
//!     fn visit_stmt(&mut self, node: &'ast Stmt<TokenStream<'ast>>) {
//!         self.names.push(node.ident.text().as_str());
//!         walk::walk_stmt(self, node);
//!     }
//! }
//! ```
//!
//! Not calling the walk is how a visitor stops there: the children of that node
//! then go unvisited, which is a thing to want when a rule is asked about its own
//! pattern and not about the pattern of everything below it.

use super::lexer;
use super::syntax;
use crate::codegen::input::UnsynInput;

/// The methods a visitor of the CST has, one for every node of it.
///
/// Every body written here walks its node, so an implementation only writes the
/// methods it cares about. See the [module document](self) for the rule the names
/// follow and for how the walk of a node is laid out.
pub trait Visit<'ast, I>
where
    I: UnsynInput,
{
    // -----------------------------------------------------------------------
    // Below the nodes
    // -----------------------------------------------------------------------

    /// The text of a token: the last argument a walk passes, and the one that
    /// names no node. The text of `lexer LF -> U+000A;` is the whole of that node,
    /// and the `-` of a `concat`ed range is text with no node of its own at all;
    /// both of them reach a visitor here.
    ///
    /// The default body does nothing, which is why a visitor that records spans or
    /// text overrides this method rather than one method per token.
    fn visit_text(&mut self, _text: &'ast I) {}

    // -----------------------------------------------------------------------
    // The nodes a `syntax` production names
    // -----------------------------------------------------------------------

    /// `syntax Crate -> InnerDoc* Item*;`
    fn visit_crate(&mut self, node: &'ast syntax::Crate<I>) {
        walk::walk_crate(self, node);
    }

    /// `syntax Item -> OuterDoc | S | Stmt;`
    fn visit_item(&mut self, node: &'ast syntax::Item<I>) {
        walk::walk_item(self, node);
    }

    /// `syntax Stmt -> LeadingKeywords Ident '->' Expr ';';`
    fn visit_stmt(&mut self, node: &'ast syntax::Stmt<I>) {
        walk::walk_stmt(self, node);
    }

    /// `syntax OuterDoc -> OuterBlockDoc | OuterLineDoc;`
    fn visit_outer_doc(&mut self, node: &'ast syntax::OuterDoc<I>) {
        walk::walk_outer_doc(self, node);
    }

    /// `syntax InnerDoc -> InnerBlockDoc | InnerLineDoc;`
    fn visit_inner_doc(&mut self, node: &'ast syntax::InnerDoc<I>) {
        walk::walk_inner_doc(self, node);
    }

    /// `syntax Expr -> ExprNoTopAlt+ ( '|' ExprNoTopAlt+ )*;`
    fn visit_expr(&mut self, node: &'ast syntax::Expr<I>) {
        walk::walk_expr(self, node);
    }

    /// `syntax ExprNoTopAlt -> ExprWithSuffix | ExprWithoutSuffix;`
    fn visit_expr_no_top_alt(&mut self, node: &'ast syntax::ExprNoTopAlt<I>) {
        walk::walk_expr_no_top_alt(self, node);
    }

    /// `syntax ExprWithoutSuffix -> '~' […] | '<' Ident '>' | …`
    fn visit_expr_without_suffix(&mut self, node: &'ast syntax::ExprWithoutSuffix<I>) {
        walk::walk_expr_without_suffix(self, node);
    }

    /// `syntax ExprWithSuffix -> ExprWithoutSuffix ( '*' | … ) | …`
    fn visit_expr_with_suffix(&mut self, node: &'ast syntax::ExprWithSuffix<I>) {
        walk::walk_expr_with_suffix(self, node);
    }

    /// `syntax Repeat -> '{' ( ',' LitDec | LitDec ',' LitDec? | LitDec ) '}';`
    fn visit_repeat(&mut self, node: &'ast syntax::Repeat<I>) {
        walk::walk_repeat(self, node);
    }

    /// The group of bounds the [`syntax::Repeat`] rule holds: the one right side
    /// that has
    /// a type of its own, and so a method of its own.
    fn visit_repeat_bound(&mut self, node: &'ast syntax::RepeatBound<I>) {
        walk::walk_repeat_bound(self, node);
    }

    /// `syntax Range -> LitUnicode '-' LitUnicode | LitStr concat '-' concat LitStr;`
    fn visit_range(&mut self, node: &'ast syntax::Range<I>) {
        walk::walk_range(self, node);
    }

    /// `syntax SetItem -> Range | LitStr | LitUnicode | Ident;`
    fn visit_set_item(&mut self, node: &'ast syntax::SetItem<I>) {
        walk::walk_set_item(self, node);
    }

    // -----------------------------------------------------------------------
    // The terminals a `syntax` production is written with
    //
    // Each owns the run of the trivia on both sides of its text, so each walk
    // begins and ends with `visit_s`. See the crate document for the rule that
    // gives a run to exactly one of the two neighbours it lies between.
    // -----------------------------------------------------------------------

    /// `';'`, which ends a definition.
    fn visit_semi(&mut self, node: &'ast syntax::Semi<I>) {
        walk::walk_semi(self, node);
    }

    /// `'+'`, the suffix that asks for one operand or more.
    fn visit_plus(&mut self, node: &'ast syntax::Plus<I>) {
        walk::walk_plus(self, node);
    }

    /// `'*'`, the suffix that asks for an operand any number of times.
    fn visit_star(&mut self, node: &'ast syntax::Star<I>) {
        walk::walk_star(self, node);
    }

    /// `'?'`, the suffix that asks for an operand at most once.
    fn visit_question(&mut self, node: &'ast syntax::Question<I>) {
        walk::walk_question(self, node);
    }

    /// `'~'`, the prefix that asks for the one character a set does not denote.
    fn visit_tilde(&mut self, node: &'ast syntax::Tilde<I>) {
        walk::walk_tilde(self, node);
    }

    /// `'|'`, which joins two alternatives.
    fn visit_or(&mut self, node: &'ast syntax::Or<I>) {
        walk::walk_or(self, node);
    }

    /// `'('`, which opens a group.
    fn visit_paren_start(&mut self, node: &'ast syntax::ParenStart<I>) {
        walk::walk_paren_start(self, node);
    }

    /// `')'`, which closes a group.
    fn visit_paren_end(&mut self, node: &'ast syntax::ParenEnd<I>) {
        walk::walk_paren_end(self, node);
    }

    /// `'['`, which opens a set.
    fn visit_bracket_start(&mut self, node: &'ast syntax::BracketStart<I>) {
        walk::walk_bracket_start(self, node);
    }

    /// `']'`, which closes a set.
    fn visit_bracket_end(&mut self, node: &'ast syntax::BracketEnd<I>) {
        walk::walk_bracket_end(self, node);
    }

    /// `'{'`, which opens the bound of a repetition.
    fn visit_brace_start(&mut self, node: &'ast syntax::BraceStart<I>) {
        walk::walk_brace_start(self, node);
    }

    /// `'}'`, which closes the bound of a repetition.
    fn visit_brace_end(&mut self, node: &'ast syntax::BraceEnd<I>) {
        walk::walk_brace_end(self, node);
    }

    /// `'<'`, which opens a call of an external.
    fn visit_lt(&mut self, node: &'ast syntax::Lt<I>) {
        walk::walk_lt(self, node);
    }

    /// `'>'`, which closes a call of an external.
    fn visit_gt(&mut self, node: &'ast syntax::Gt<I>) {
        walk::walk_gt(self, node);
    }

    /// `','`, which separates two members of a set or the two bounds of a
    /// repetition.
    fn visit_comma(&mut self, node: &'ast syntax::Comma<I>) {
        walk::walk_comma(self, node);
    }

    /// `'->'`, which divides the name of a node from the pattern it denotes.
    fn visit_arrow_right(&mut self, node: &'ast syntax::ArrowRight<I>) {
        walk::walk_arrow_right(self, node);
    }

    /// `'-'`, which divides the two ends of a range of code points.
    fn visit_minus(&mut self, node: &'ast syntax::Minus<I>) {
        walk::walk_minus(self, node);
    }

    /// `'concat'`, which asks that what two operands matched be joined.
    fn visit_concat(&mut self, node: &'ast syntax::Concat<I>) {
        walk::walk_concat(self, node);
    }

    /// `'followed'`, which asks that an operand come next.
    fn visit_followed(&mut self, node: &'ast syntax::Followed<I>) {
        walk::walk_followed(self, node);
    }

    /// `'except'`, which removes what one operand matches from another.
    fn visit_except(&mut self, node: &'ast syntax::Except<I>) {
        walk::walk_except(self, node);
    }

    // -----------------------------------------------------------------------
    // The tokens a `lexer` production names
    //
    // A lexer rule keeps the text it matched and not the tokens its right side
    // was written out of, so every walk here is a call to `visit_text` and
    // nothing else — except the three rules that are nothing but a choice of
    // other rules, which are enums and so are descended into.
    // -----------------------------------------------------------------------

    /// `whitespace S -> […]`, one run of the trivia of the language.
    fn visit_s(&mut self, node: &'ast lexer::S<I>) {
        walk::walk_s(self, node);
    }

    /// `lexer LF -> U+000A;`
    fn visit_lf(&mut self, node: &'ast lexer::LF<I>) {
        walk::walk_lf(self, node);
    }

    /// `lexer CR -> U+000D;`
    fn visit_cr(&mut self, node: &'ast lexer::CR<I>) {
        walk::walk_cr(self, node);
    }

    /// `lexer IdentOrKeyword -> ( XID_Start | '_' ) XID_Continue*;`
    fn visit_ident_or_keyword(&mut self, node: &'ast lexer::IdentOrKeyword<I>) {
        walk::walk_ident_or_keyword(self, node);
    }

    /// `lexer XID_Start -> <XID_Start>;`
    ///
    /// The text of an external is whatever the caller's function took, which is
    /// why this walk has nothing to say about how long it was.
    fn visit_xid_start(&mut self, node: &'ast lexer::XID_Start<I>) {
        walk::walk_xid_start(self, node);
    }

    /// `lexer XID_Continue -> <XID_Continue>;`
    fn visit_xid_continue(&mut self, node: &'ast lexer::XID_Continue<I>) {
        walk::walk_xid_continue(self, node);
    }

    /// `lexer RawIdent -> 'r#' IdentOrKeyword;`
    fn visit_raw_ident(&mut self, node: &'ast lexer::RawIdent<I>) {
        walk::walk_raw_ident(self, node);
    }

    /// `lexer NonKeywordIdent -> IdentOrKeyword except Keywords;`
    fn visit_non_keyword_ident(&mut self, node: &'ast lexer::NonKeywordIdent<I>) {
        walk::walk_non_keyword_ident(self, node);
    }

    /// `lexer Ident -> RawIdent | NonKeywordIdent;`
    fn visit_ident(&mut self, node: &'ast lexer::Ident<I>) {
        walk::walk_ident(self, node);
    }

    /// `lexer Keywords -> LeadingKeywords | FollowingKeywords;`
    fn visit_keywords(&mut self, node: &'ast lexer::Keywords<I>) {
        walk::walk_keywords(self, node);
    }

    /// `lexer LeadingKeywords -> 'lexer' | 'syntax' | 'whitespace';`
    fn visit_leading_keywords(&mut self, node: &'ast lexer::LeadingKeywords<I>) {
        walk::walk_leading_keywords(self, node);
    }

    /// `lexer FollowingKeywords -> 'concat' | 'except' | 'followed';`
    fn visit_following_keywords(&mut self, node: &'ast lexer::FollowingKeywords<I>) {
        walk::walk_following_keywords(self, node);
    }

    /// `lexer LitStr -> '\'' … '\'';`
    fn visit_lit_str(&mut self, node: &'ast lexer::LitStr<I>) {
        walk::walk_lit_str(self, node);
    }

    /// `lexer QuoteEscape -> '\\'';`
    fn visit_quote_escape(&mut self, node: &'ast lexer::QuoteEscape<I>) {
        walk::walk_quote_escape(self, node);
    }

    /// `lexer AsciiEscape -> '\\x' OctDigit HexDigit | …`
    fn visit_ascii_escape(&mut self, node: &'ast lexer::AsciiEscape<I>) {
        walk::walk_ascii_escape(self, node);
    }

    /// `lexer UnicodeEscape -> '\\u{' ( HexDigit '_'* ){1, 6} '}';`
    fn visit_unicode_escape(&mut self, node: &'ast lexer::UnicodeEscape<I>) {
        walk::walk_unicode_escape(self, node);
    }

    /// `lexer OctDigit -> ['0'-'7'];`
    fn visit_oct_digit(&mut self, node: &'ast lexer::OctDigit<I>) {
        walk::walk_oct_digit(self, node);
    }

    /// `lexer DecDigit -> ['0'-'9'];`
    fn visit_dec_digit(&mut self, node: &'ast lexer::DecDigit<I>) {
        walk::walk_dec_digit(self, node);
    }

    /// `lexer HexDigit -> ['0'-'9','a'-'f','A'-'F'];`
    fn visit_hex_digit(&mut self, node: &'ast lexer::HexDigit<I>) {
        walk::walk_hex_digit(self, node);
    }

    /// `lexer LitDec -> DecDigit ( DecDigit | '_' )*;`
    fn visit_lit_dec(&mut self, node: &'ast lexer::LitDec<I>) {
        walk::walk_lit_dec(self, node);
    }

    /// `lexer LitUnicode -> 'U+' concat HexDigit{4};`
    fn visit_lit_unicode(&mut self, node: &'ast lexer::LitUnicode<I>) {
        walk::walk_lit_unicode(self, node);
    }

    /// `lexer BlockComment -> '/*' … '*/' | '/**/' | '/***/';`
    fn visit_block_comment(&mut self, node: &'ast lexer::BlockComment<I>) {
        walk::walk_block_comment(self, node);
    }

    /// `lexer BlockCommentOrDoc -> BlockComment | OuterBlockDoc | InnerBlockDoc;`
    fn visit_block_comment_or_doc(&mut self, node: &'ast lexer::BlockCommentOrDoc<I>) {
        walk::walk_block_comment_or_doc(self, node);
    }

    /// `lexer InnerLineDoc -> '//!' ~[LF,CR]*;`
    fn visit_inner_line_doc(&mut self, node: &'ast lexer::InnerLineDoc<I>) {
        walk::walk_inner_line_doc(self, node);
    }

    /// `lexer InnerBlockDoc -> '/*!' … '*/';`
    fn visit_inner_block_doc(&mut self, node: &'ast lexer::InnerBlockDoc<I>) {
        walk::walk_inner_block_doc(self, node);
    }

    /// `lexer OuterLineDoc -> '///' ( ~['/'] ~[LF,CR]* )?;`
    fn visit_outer_line_doc(&mut self, node: &'ast lexer::OuterLineDoc<I>) {
        walk::walk_outer_line_doc(self, node);
    }

    /// `lexer OuterBlockDoc -> '/**' … '*/';`
    fn visit_outer_block_doc(&mut self, node: &'ast lexer::OuterBlockDoc<I>) {
        walk::walk_outer_block_doc(self, node);
    }
}

/// The walks the default bodies of [`Visit`] call.
///
/// They are public so that a method which does its own work can still hand the
/// children to the visitor below it, which is what `walk_*` is for in [`syn`].
///
/// [`syn`]: https://docs.rs/syn/latest/syn/visit/index.html
pub mod walk {
    use parserc::syntax::Or;

    use super::{lexer, syntax};
    use crate::codegen::input::UnsynInput;
    use crate::codegen::visit::Visit;

    // -----------------------------------------------------------------------
    // The nodes a `syntax` production names
    // -----------------------------------------------------------------------

    /// `InnerDoc* Item*`: every document of the file with the run before it, then
    /// every item.
    pub fn walk_crate<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::Crate<I>,
    ) {
        for (s, inner_doc) in &node.inner_docs {
            if let Some(s) = s {
                v.visit_s(s);
            }

            v.visit_inner_doc(inner_doc);
        }

        for item in &node.items {
            v.visit_item(item);
        }
    }

    pub fn walk_item<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::Item<I>,
    ) {
        match node {
            syntax::Item::OuterDoc(outer_doc) => v.visit_outer_doc(outer_doc),
            syntax::Item::S(s) => v.visit_s(s),
            syntax::Item::Stmt(stmt) => v.visit_stmt(stmt),
        }
    }

    pub fn walk_stmt<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::Stmt<I>,
    ) {
        v.visit_leading_keywords(&node.keyword);

        if let Some(s) = &node.s {
            v.visit_s(s);
        }

        v.visit_ident(&node.ident);
        v.visit_arrow_right(&node.definition.start);
        v.visit_expr(&node.definition.body);
        v.visit_semi(&node.definition.end);
    }

    pub fn walk_outer_doc<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::OuterDoc<I>,
    ) {
        match node {
            syntax::OuterDoc::Block(doc) => v.visit_outer_block_doc(doc),
            syntax::OuterDoc::Line(doc) => v.visit_outer_line_doc(doc),
        }
    }

    pub fn walk_inner_doc<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::InnerDoc<I>,
    ) {
        match node {
            syntax::InnerDoc::Block(doc) => v.visit_inner_block_doc(doc),
            syntax::InnerDoc::Line(doc) => v.visit_inner_line_doc(doc),
        }
    }

    pub fn walk_expr<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::Expr<I>,
    ) {
        walk_one_or_more(v, &node.first);

        for (or, alternative) in &node.rest {
            v.visit_or(or);
            walk_one_or_more(v, alternative);
        }
    }

    pub fn walk_expr_no_top_alt<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::ExprNoTopAlt<I>,
    ) {
        match node {
            syntax::ExprNoTopAlt::WithSuffix(suffix) => v.visit_expr_with_suffix(suffix),
            syntax::ExprNoTopAlt::WithoutSuffix(operand) => v.visit_expr_without_suffix(operand),
        }
    }

    pub fn walk_expr_without_suffix<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::ExprWithoutSuffix<I>,
    ) {
        match node {
            syntax::ExprWithoutSuffix::Tilde(tilde, set) => {
                v.visit_tilde(tilde);
                walk_set(v, set);
            }
            syntax::ExprWithoutSuffix::Call(external) => {
                v.visit_lt(&external.start);
                v.visit_ident(&external.body);
                v.visit_gt(&external.end);
            }
            syntax::ExprWithoutSuffix::Paren(group) => {
                v.visit_paren_start(&group.start);
                v.visit_expr(&group.body);
                v.visit_paren_end(&group.end);
            }
            syntax::ExprWithoutSuffix::Set(set) => walk_set(v, set),
            syntax::ExprWithoutSuffix::Str(lit) => v.visit_lit_str(lit),
            syntax::ExprWithoutSuffix::Unicode(lit) => v.visit_lit_unicode(lit),
            syntax::ExprWithoutSuffix::Reference(ident) => v.visit_ident(ident),
        }
    }

    pub fn walk_expr_with_suffix<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::ExprWithSuffix<I>,
    ) {
        match node {
            syntax::ExprWithSuffix::Grouped { target, suffix } => {
                v.visit_expr_without_suffix(target);
                walk_suffix_group(v, suffix);
            }
            syntax::ExprWithSuffix::Concat {
                target,
                keyword,
                suffix,
            } => {
                v.visit_expr_without_suffix(target);
                v.visit_concat(keyword);
                v.visit_expr_no_top_alt(suffix);
            }
            syntax::ExprWithSuffix::Followed {
                target,
                keyword,
                suffix,
            } => {
                v.visit_expr_without_suffix(target);
                v.visit_followed(keyword);
                v.visit_expr_no_top_alt(suffix);
            }
            syntax::ExprWithSuffix::Except {
                target,
                keyword,
                tokens,
            } => {
                v.visit_expr_without_suffix(target);
                v.visit_except(keyword);
                v.visit_expr_without_suffix(tokens);
            }
        }
    }

    pub fn walk_repeat<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::Repeat<I>,
    ) {
        v.visit_brace_start(&node.0.start);
        v.visit_repeat_bound(&node.0.body);
        v.visit_brace_end(&node.0.end);
    }

    pub fn walk_repeat_bound<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::RepeatBound<I>,
    ) {
        match node {
            syntax::RepeatBound::RangeTo(comma, end) => {
                v.visit_comma(comma);
                v.visit_lit_dec(end);
            }
            syntax::RepeatBound::RangeFrom(start, comma, end) => {
                v.visit_lit_dec(start);
                v.visit_comma(comma);

                if let Some(end) = end {
                    v.visit_lit_dec(end);
                }
            }
            syntax::RepeatBound::Count(count) => v.visit_lit_dec(count),
        }
    }

    pub fn walk_range<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::Range<I>,
    ) {
        match node {
            syntax::Range::Unicode(start, dash, end) => {
                v.visit_lit_unicode(start);
                v.visit_minus(dash);
                v.visit_lit_unicode(end);
            }
            // The `-` of this form is joined to both of its neighbours by the
            // `concat` the rule asks for, so it is the text of the node rather
            // than a terminal of it, and reaches the visitor as text.
            syntax::Range::Str(start, dash, end) => {
                v.visit_lit_str(start);
                v.visit_text(dash);
                v.visit_lit_str(end);
            }
        }
    }

    pub fn walk_set_item<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::SetItem<I>,
    ) {
        match node {
            syntax::SetItem::Range(range) => v.visit_range(range),
            syntax::SetItem::Str(lit) => v.visit_lit_str(lit),
            syntax::SetItem::Unicode(lit) => v.visit_lit_unicode(lit),
            syntax::SetItem::Reference(ident) => v.visit_ident(ident),
        }
    }

    // -----------------------------------------------------------------------
    // The groups no production names
    // -----------------------------------------------------------------------

    /// `X+` over the trivia of the language: the operand, then every operand the
    /// text offers after it, each with the run that separates it.
    ///
    /// The grammar repeats one kind of operand only, so this is the one helper for
    /// the shape rather than a helper per rule that uses it.
    fn walk_one_or_more<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::OneOrMore<I, syntax::ExprNoTopAlt<I>>,
    ) {
        v.visit_expr_no_top_alt(&node.first);

        for (s, operand) in &node.rest {
            if let Some(s) = s {
                v.visit_s(s);
            }

            v.visit_expr_no_top_alt(operand);
        }
    }

    /// `'[' ( SetItem ',' )* SetItem? ']'`, which [`Punctuated`] reads.
    ///
    /// [`Punctuated`]: parserc::syntax::Punctuated
    fn walk_set<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::Set<I>,
    ) {
        v.visit_bracket_start(&node.start);

        for (item, comma) in &node.body.pairs {
            v.visit_set_item(item);
            v.visit_comma(comma);
        }

        if let Some(tail) = &node.body.tail {
            v.visit_set_item(tail);
        }

        v.visit_bracket_end(&node.end);
    }

    /// `('*' | '?' | '+' | Repeat)`, the four quantity operators the production
    /// writes as one group.
    fn walk_suffix_group<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast syntax::SuffixGroup<I>,
    ) {
        match node {
            Or::First(star) => v.visit_star(star),
            Or::Second(Or::First(question)) => v.visit_question(question),
            Or::Second(Or::Second(Or::First(plus))) => v.visit_plus(plus),
            Or::Second(Or::Second(Or::Second(repeat))) => v.visit_repeat(repeat),
        }
    }

    // -----------------------------------------------------------------------
    // The terminals a `syntax` production is written with
    //
    // Every one of them is the same three fields: the run before, the word, the
    // run after. A generator writes one walk per terminal for the same reason it
    // writes one type per terminal.
    // -----------------------------------------------------------------------

    macro_rules! walk_terminal {
        ($($method:ident, $type:ident);* $(;)?) => {
            $(
                #[doc = concat!("The `", stringify!($type), "` punct, with the runs of the trivia on each side of it.")]
                pub fn $method<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
                    v: &mut V,
                    node: &'ast syntax::$type<I>,
                ) {
                    if let Some(s) = &node.0 {
                        v.visit_s(s);
                    }

                    v.visit_text(&node.1);

                    if let Some(s) = &node.2 {
                        v.visit_s(s);
                    }
                }
            )*
        };
    }

    walk_terminal! {
        walk_semi, Semi;
        walk_plus, Plus;
        walk_star, Star;
        walk_question, Question;
        walk_tilde, Tilde;
        walk_or, Or;
        walk_paren_start, ParenStart;
        walk_paren_end, ParenEnd;
        walk_bracket_start, BracketStart;
        walk_bracket_end, BracketEnd;
        walk_brace_start, BraceStart;
        walk_brace_end, BraceEnd;
        walk_lt, Lt;
        walk_gt, Gt;
        walk_comma, Comma;
        walk_arrow_right, ArrowRight;
        walk_minus, Minus;
        walk_concat, Concat;
        walk_followed, Followed;
        walk_except, Except;
    }

    // -----------------------------------------------------------------------
    // The tokens a `lexer` production names
    // -----------------------------------------------------------------------

    /// The tokens that hold their own text: one field of `I`, and nothing below it
    /// for a walk to reach.
    macro_rules! walk_token {
        ($($method:ident, $type:ident);* $(;)?) => {
            $(
                #[doc = concat!("The text of `", stringify!($type), "`, which is the whole of the node.")]
                pub fn $method<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
                    v: &mut V,
                    node: &'ast lexer::$type<I>,
                ) {
                    v.visit_text(&node.0);
                }
            )*
        };
    }

    walk_token! {
        walk_s, S;
        walk_lf, LF;
        walk_cr, CR;
        walk_ident_or_keyword, IdentOrKeyword;
        walk_xid_start, XID_Start;
        walk_xid_continue, XID_Continue;
        walk_raw_ident, RawIdent;
        walk_non_keyword_ident, NonKeywordIdent;
        walk_leading_keywords, LeadingKeywords;
        walk_following_keywords, FollowingKeywords;
        walk_lit_str, LitStr;
        walk_quote_escape, QuoteEscape;
        walk_ascii_escape, AsciiEscape;
        walk_unicode_escape, UnicodeEscape;
        walk_oct_digit, OctDigit;
        walk_dec_digit, DecDigit;
        walk_hex_digit, HexDigit;
        walk_lit_dec, LitDec;
        walk_lit_unicode, LitUnicode;
        walk_block_comment, BlockComment;
        walk_inner_line_doc, InnerLineDoc;
        walk_inner_block_doc, InnerBlockDoc;
        walk_outer_line_doc, OuterLineDoc;
        walk_outer_block_doc, OuterBlockDoc;
    }

    // The three rules that are nothing but a choice of other rules keep the rules
    // they chose rather than the text of them, so these are descended into.

    pub fn walk_ident<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast lexer::Ident<I>,
    ) {
        match node {
            lexer::Ident::Raw(raw) => v.visit_raw_ident(raw),
            lexer::Ident::NonKeyword(ident) => v.visit_non_keyword_ident(ident),
        }
    }

    pub fn walk_keywords<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast lexer::Keywords<I>,
    ) {
        match node {
            lexer::Keywords::Leading(keywords) => v.visit_leading_keywords(keywords),
            lexer::Keywords::Following(keywords) => v.visit_following_keywords(keywords),
        }
    }

    pub fn walk_block_comment_or_doc<'ast, I: UnsynInput, V: ?Sized + Visit<'ast, I>>(
        v: &mut V,
        node: &'ast lexer::BlockCommentOrDoc<I>,
    ) {
        match node {
            lexer::BlockCommentOrDoc::Block(comment) => v.visit_block_comment(comment),
            lexer::BlockCommentOrDoc::Outer(doc) => v.visit_outer_block_doc(doc),
            lexer::BlockCommentOrDoc::Inner(doc) => v.visit_inner_block_doc(doc),
        }
    }
}

#[cfg(test)]
mod tests {
    use parserc::{AsStr, Input, Span, syntax::SyntaxInput};

    use super::{Visit, walk};
    use crate::codegen::input::TokenStream;
    use crate::codegen::lexer;
    use crate::codegen::syntax::{self, Crate, Stmt};

    /// Parses `source` as a whole file.
    fn file(source: &str) -> Crate<TokenStream<'_>> {
        let mut input = TokenStream::from(source);
        let node = input
            .parse::<Crate<_>>()
            .expect("the source should be a file of the language");
        assert!(input.is_empty(), "the file left text unread");
        node
    }

    /// The text every token reported, in the order the visitor reached them.
    #[derive(Default)]
    struct Echo<'ast> {
        text: String,
        texts: Vec<(&'ast str, Span)>,
    }

    impl<'ast> Visit<'ast, TokenStream<'ast>> for Echo<'ast> {
        fn visit_text(&mut self, text: &'ast TokenStream<'ast>) {
            let held = text.as_str();
            self.text.push_str(held);
            self.texts.push((held, text.to_span()));
        }
    }

    #[test]
    fn a_visitor_that_only_records_text_rebuilds_the_file() {
        let source = include_str!("../../unsyn.us");
        let tree = file(source);

        let mut echo = Echo::default();
        echo.visit_crate(&tree);

        // Every character of the source is held by exactly one token of the tree,
        // and the walk reaches the tokens in the order they were written: the
        // runs of the trivia, which the terminals file away on each side of their
        // own text, are never given twice and never left out.
        assert_eq!(echo.text, source);

        let mut covered = 0;
        for (held, span) in &echo.texts {
            match span {
                Span::Range(range) => {
                    assert_eq!(range.start, covered, "a gap before {held:?}");
                    assert_eq!(&source[range.clone()], *held, "{held:?}");
                    covered = range.end;
                }
                span => panic!("a token that reports {span:?} rather than a range"),
            }
        }

        assert_eq!(covered, source.len(), "the text no token reached");
    }

    #[test]
    fn a_visitor_reaches_every_definition_of_a_file() {
        #[derive(Default)]
        struct Definitions<'ast> {
            keywords: Vec<&'ast str>,
            names: Vec<&'ast str>,
            patterns: usize,
        }

        impl<'ast> Visit<'ast, TokenStream<'ast>> for Definitions<'ast> {
            fn visit_stmt(&mut self, node: &'ast Stmt<TokenStream<'ast>>) {
                self.keywords.push(node.keyword.0.as_str());
                self.names.push(node.ident.text().as_str());
                walk::walk_stmt(self, node);
            }

            fn visit_expr(&mut self, _node: &'ast syntax::Expr<TokenStream<'ast>>) {
                self.patterns += 1;
            }
        }

        // A rule of a file has exactly one pattern, and a visitor that stops at
        // the pattern never reaches the tokens inside it.
        let source = "lexer LF -> U+000A;\nwhitespace S -> [' '];\nsyntax Item -> LF | S;\n";
        let tree = file(source);
        let mut seen = Definitions::default();
        seen.visit_crate(&tree);

        assert_eq!(seen.keywords, ["lexer", "whitespace", "syntax"]);
        assert_eq!(seen.names, ["LF", "S", "Item"]);
        assert_eq!(seen.patterns, 3);
    }

    #[test]
    fn a_visitor_reaches_the_whole_of_a_pattern() {
        #[derive(Default)]
        struct Tokens(Vec<String>);

        impl<'ast> Visit<'ast, TokenStream<'ast>> for Tokens {
            fn visit_text(&mut self, text: &'ast TokenStream<'ast>) {
                self.0.push(text.as_str().to_string());
            }
        }

        let source = "syntax Repeat -> '{' ',' concat LitDec '}';";
        let tree = file(source);
        let mut tokens = Tokens::default();
        tokens.visit_crate(&tree);

        assert_eq!(
            tokens.0,
            [
                "syntax", " ", "Repeat", " ", "->", " ", "'{'", " ", "','", " ", "concat", " ",
                "LitDec", " ", "'}'", ";",
            ],
            "the words of a definition, in the order a walk hands them over"
        );
    }

    /// Every walk hands a child to the visitor's *method* for it, and never to the
    /// walk of it, which is what lets an implementation stop a descent at any node
    /// by writing one method.
    #[test]
    fn a_walk_hands_a_child_to_the_method_and_not_to_the_walk() {
        #[derive(Default)]
        struct StopsAtBounds {
            bounds: usize,
            decimals: usize,
        }

        impl<'ast> Visit<'ast, TokenStream<'ast>> for StopsAtBounds {
            fn visit_repeat_bound(&mut self, _node: &'ast syntax::RepeatBound<TokenStream<'ast>>) {
                // The walk of the bounds is deliberately not called.
                self.bounds += 1;
            }

            fn visit_lit_dec(&mut self, _node: &'ast lexer::LitDec<TokenStream<'ast>>) {
                self.decimals += 1;
            }
        }

        let tree = file("syntax S -> X{1, 2};");
        let mut seen = StopsAtBounds::default();
        seen.visit_crate(&tree);

        assert_eq!(seen.bounds, 1);
        assert_eq!(
            seen.decimals, 0,
            "the bounds were walked past the override that refused to"
        );
    }
}
