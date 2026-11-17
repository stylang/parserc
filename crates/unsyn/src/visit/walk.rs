//! The descent each method of [`Visit`] answers for by default.
//!
//! These functions are the walk on its own, so a pass can call one from inside
//! an override to carry on where it chose to intervene, which is how `syn` uses
//! its own `walk_*`. They are also the whole of the knowledge this module has of
//! the tree's shape: what each node holds, and the order its parts stand in the
//! file.
//!
//! One function per method, with the same name and a `walk_` prefix in place of
//! the `visit_`, so that the pair (method, function) is everything a walk knows
//! about a node. A generator that can name a type can therefore emit both
//! halves, which is how the interface of a bootstrapped tree is kept the same as
//! this one rather than rewritten by hand.

use parserc::syntax::{Delimiter, Or as Choice, Punctuated};

use crate::{
    input::UnsynInput,
    lexical::{
        S,
        comments::{InnerBlockDoc, InnerLineDoc, OuterBlockDoc, OuterLineDoc},
        delimiter::Bracket,
        ident::Ident,
        keyword::{Concat, Except, Followed, Lexer, Syntax, Whitespace},
        lit::{AsciiEscape, LitDec, LitStr, LitUnicode, QuoteEscape, StrSegment, UnicodeEscape},
        punct::{
            ArrowRight, BraceEnd, BraceStart, BracketEnd, BracketStart, Comma, Gt, Lt, Minus, Or,
            ParenEnd, ParenStart, Plus, Question, Semi, Star, Tilde,
        },
    },
    syntax::{
        Crate, Expr, ExprNoTopAlt, ExprNoTopAlts, ExprWithSuffix, ExprWithoutSuffix, InnerDoc,
        Item, LeadingKey, OuterDoc, Range, Repeat, RepeatGroup, SetItem, Stmt, SuffixGroup,
    },
    visit::Visit,
};

/// The two kinds of token the language has, as the parts they hold.
///
/// A punct owns the whitespace on both sides of it and a keyword only the
/// whitespace after it, which is two shapes rather than one; each is written
/// here once and used by the per-type functions below, so that the order of the
/// three calls is stated in exactly one place.
enum Parts<'ast, I>
where
    I: UnsynInput,
{
    /// A punct: the run before it, its text, and the run after it.
    Punct(Option<&'ast S<I>>, &'ast I, Option<&'ast S<I>>),
    /// A keyword: its text and the run after it.
    Keyword(&'ast I, Option<&'ast S<I>>),
}

/// Enters the parts of one token, in the order they stand.
fn walk_token<'ast, I, V>(v: &mut V, parts: Parts<'ast, I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match parts {
        Parts::Punct(before, text, after) => {
            if let Some(before) = before {
                v.visit_s(before);
            }

            v.visit_input(text);

            if let Some(after) = after {
                v.visit_s(after);
            }
        }
        Parts::Keyword(text, after) => {
            v.visit_input(text);

            if let Some(after) = after {
                v.visit_s(after);
            }
        }
    }
}

/// The three parts of a punct, read off the token itself.
macro_rules! punct {
    ($node:expr) => {
        Parts::Punct($node.0.as_ref(), &$node.1, $node.2.as_ref())
    };
}

/// The two parts of a keyword, read off the token itself.
macro_rules! keyword {
    ($node:expr) => {
        Parts::Keyword(&$node.0, $node.1.as_ref())
    };
}

/// The two ends and the body of a statement's definition, entered in the order
/// the file writes them: `->`, the pattern, `;` — which is not the order of the
/// three fields of [`Delimiter`].
fn walk_definition<'ast, I, V>(v: &mut V, node: &'ast Delimiter<ArrowRight<I>, Semi<I>, Expr<I>>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_arrow_right(&node.start);
    v.visit_expr(&node.body);
    v.visit_semi(&node.end);
}

// ## The file

pub fn walk_crate<'ast, I, V>(v: &mut V, node: &'ast Crate<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    for inner_doc in &node.inner_docs {
        v.visit_inner_doc(inner_doc);
    }

    for item in &node.items {
        v.visit_item(item);
    }
}

pub fn walk_item<'ast, I, V>(v: &mut V, node: &'ast Item<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match node {
        Item::OuterDoc(outer_doc) => v.visit_outer_doc(outer_doc),
        Item::S(s) => v.visit_s(s),
        Item::Stmt(stmt) => v.visit_stmt(stmt),
    }
}

pub fn walk_inner_doc<'ast, I, V>(v: &mut V, node: &'ast InnerDoc<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match node {
        InnerDoc::Block(s, doc) => {
            if let Some(s) = s {
                v.visit_s(s);
            }

            v.visit_inner_block_doc(doc);
        }
        InnerDoc::Line(s, doc) => {
            if let Some(s) = s {
                v.visit_s(s);
            }

            v.visit_inner_line_doc(doc);
        }
    }
}

pub fn walk_outer_doc<'ast, I, V>(v: &mut V, node: &'ast OuterDoc<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match node {
        OuterDoc::Block(s, doc) => {
            if let Some(s) = s {
                v.visit_s(s);
            }

            v.visit_outer_block_doc(doc);
        }
        OuterDoc::Line(s, doc) => {
            if let Some(s) = s {
                v.visit_s(s);
            }

            v.visit_outer_line_doc(doc);
        }
    }
}

pub fn walk_inner_line_doc<'ast, I, V>(v: &mut V, node: &'ast InnerLineDoc<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_input(&node.0);
}

pub fn walk_inner_block_doc<'ast, I, V>(v: &mut V, node: &'ast InnerBlockDoc<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_input(&node.0);
}

pub fn walk_outer_line_doc<'ast, I, V>(v: &mut V, node: &'ast OuterLineDoc<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_input(&node.0);
}

pub fn walk_outer_block_doc<'ast, I, V>(v: &mut V, node: &'ast OuterBlockDoc<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_input(&node.0);
}

// ## A definition

pub fn walk_stmt<'ast, I, V>(v: &mut V, node: &'ast Stmt<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_leading_key(&node.keyword);
    v.visit_ident(&node.ident);

    // The definition is the `->` … `;` run with the pattern between them.
    walk_definition(v, &node.definition);
}

pub fn walk_leading_key<'ast, I, V>(v: &mut V, node: &'ast LeadingKey<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match node {
        LeadingKey::Whitespace(keyword) => v.visit_whitespace(keyword),
        LeadingKey::Lexer(keyword) => v.visit_lexer(keyword),
        LeadingKey::Syntax(keyword) => v.visit_syntax(keyword),
    }
}

// ## An expression

pub fn walk_expr<'ast, I, V>(v: &mut V, node: &'ast Expr<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_expr_no_top_alts(&node.first);

    for (or, alternatives) in &node.rest {
        // The `|` stands before the alternative it introduced.
        v.visit_or(or);
        v.visit_expr_no_top_alts(alternatives);
    }
}

pub fn walk_expr_no_top_alts<'ast, I, V>(v: &mut V, node: &'ast ExprNoTopAlts<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_expr_no_top_alt(&node.first);

    for (s, operand) in &node.rest {
        // A sequence has no operator of its own, so the whitespace between two
        // operands is what says they are two.
        if let Some(s) = s {
            v.visit_s(s);
        }

        v.visit_expr_no_top_alt(operand);
    }
}

pub fn walk_expr_no_top_alt<'ast, I, V>(v: &mut V, node: &'ast ExprNoTopAlt<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match node {
        ExprNoTopAlt::WithSuffix(operand) => v.visit_expr_with_suffix(operand),
        ExprNoTopAlt::WithoutSuffix(operand) => v.visit_expr_without_suffix(operand),
    }
}

pub fn walk_expr_with_suffix<'ast, I, V>(v: &mut V, node: &'ast ExprWithSuffix<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match node {
        ExprWithSuffix::Grouped { target, suffix } => {
            v.visit_expr_without_suffix(target);
            walk_suffix_group(v, suffix);
        }
        ExprWithSuffix::Concat {
            target,
            s,
            keyword,
            suffix,
        } => {
            v.visit_expr_without_suffix(target);

            if let Some(s) = s {
                v.visit_s(s);
            }

            v.visit_concat(keyword);
            v.visit_expr_no_top_alt(suffix);
        }
        ExprWithSuffix::Followed {
            target,
            s,
            keyword,
            suffix,
        } => {
            v.visit_expr_without_suffix(target);

            if let Some(s) = s {
                v.visit_s(s);
            }

            v.visit_followed(keyword);
            v.visit_expr_no_top_alt(suffix);
        }
        ExprWithSuffix::Except {
            target,
            s,
            keyword,
            tokens,
        } => {
            v.visit_expr_without_suffix(target);

            if let Some(s) = s {
                v.visit_s(s);
            }

            v.visit_except(keyword);
            v.visit_expr_without_suffix(tokens);
        }
    }
}

/// The group `('*' | '?' | '+' | Repeat)`, entered in the order the four
/// alternatives stand in the file.
///
/// The group names no node, so it is descended through rather than reported: a
/// generated tree has the same four choices in the same order, and what it
/// reports is whichever one of them the file wrote.
fn walk_suffix_group<'ast, I, V>(v: &mut V, group: &'ast SuffixGroup<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match group {
        Choice::First(star) => v.visit_star(star),
        Choice::Second(Choice::First(question)) => v.visit_question(question),
        Choice::Second(Choice::Second(Choice::First(plus))) => v.visit_plus(plus),
        Choice::Second(Choice::Second(Choice::Second(repeat))) => v.visit_repeat(repeat),
    }
}

pub fn walk_expr_without_suffix<'ast, I, V>(v: &mut V, node: &'ast ExprWithoutSuffix<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match node {
        ExprWithoutSuffix::Tilde(tilde, bracket) => {
            v.visit_tilde(tilde);
            walk_set(v, bracket);
        }
        ExprWithoutSuffix::Call(angle) => {
            v.visit_lt(&angle.start);
            v.visit_ident(&angle.body);
            v.visit_gt(&angle.end);
        }
        ExprWithoutSuffix::Paren(paren) => {
            v.visit_paren_start(&paren.start);
            v.visit_expr(&paren.body);
            v.visit_paren_end(&paren.end);
        }
        ExprWithoutSuffix::Set(bracket) => walk_set(v, bracket),
        ExprWithoutSuffix::Str(lit) => v.visit_lit_str(lit),
        ExprWithoutSuffix::Unicode(lit) => v.visit_lit_unicode(lit),
        ExprWithoutSuffix::Ident(ident) => v.visit_ident(ident),
    }
}

/// The `[` … `]` of a set, with the members it lists and the commas between
/// them.
///
/// A set is a node in a generated tree and an anonymous group in this one, so
/// the descent has no function of its own to name and no method to report: what
/// both trees owe the walk is the members, in order, which is what this does.
fn walk_set<'ast, I, V>(v: &mut V, node: &'ast Bracket<I, Punctuated<SetItem<I>, Comma<I>>>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_bracket_start(&node.start);

    for (item, comma) in &node.body.pairs {
        v.visit_set_item(item);
        v.visit_comma(comma);
    }

    if let Some(item) = &node.body.tail {
        v.visit_set_item(item);
    }

    v.visit_bracket_end(&node.end);
}

pub fn walk_set_item<'ast, I, V>(v: &mut V, node: &'ast SetItem<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match node {
        SetItem::Range(range) => v.visit_range(range),
        SetItem::Str(lit) => v.visit_lit_str(lit),
        SetItem::Unicode(lit) => v.visit_lit_unicode(lit),
        SetItem::Ident(ident) => v.visit_ident(ident),
    }
}

pub fn walk_range<'ast, I, V>(v: &mut V, node: &'ast Range<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match node {
        Range::Unicode(start, dash, end) => {
            v.visit_lit_unicode(start);
            v.visit_minus(dash);
            v.visit_lit_unicode(end);
        }
        Range::Str(start, dash, end) => {
            v.visit_lit_str(start);
            v.visit_minus(dash);
            v.visit_lit_str(end);
        }
    }
}

pub fn walk_repeat<'ast, I, V>(v: &mut V, node: &'ast Repeat<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_brace_start(&node.start);
    walk_repeat_group(v, &node.body);
    v.visit_brace_end(&node.end);
}

/// The group of forms the braces hold, entered in the order they stand in the
/// file, and descended through because it names no node of its own.
fn walk_repeat_group<'ast, I, V>(v: &mut V, group: &'ast RepeatGroup<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match group {
        Choice::First((comma, end)) => {
            v.visit_comma(comma);
            v.visit_lit_dec(end);
        }
        Choice::Second(Choice::First((start, comma, end))) => {
            v.visit_lit_dec(start);
            v.visit_comma(comma);

            if let Some(end) = end {
                v.visit_lit_dec(end);
            }
        }
        Choice::Second(Choice::Second(count)) => v.visit_lit_dec(count),
    }
}

// ## The tokens a definition is written from

pub fn walk_ident<'ast, I, V>(v: &mut V, node: &'ast Ident<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_input(&node.0);
}

pub fn walk_s<'ast, I, V>(v: &mut V, node: &'ast S<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_input(&node.0);
}

pub fn walk_lit_str<'ast, I, V>(v: &mut V, node: &'ast LitStr<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    // The quotes are the token the literal was read from and not a punct of the
    // grammar, so they are reported as text.
    v.visit_input(&node.delimiter_start);

    for segment in &node.content {
        v.visit_str_segment(segment);
    }

    v.visit_input(&node.delimiter_end);
}

pub fn walk_str_segment<'ast, I, V>(v: &mut V, node: &'ast StrSegment<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    match node {
        StrSegment::QuoteEscape(escape) => v.visit_quote_escape(escape),
        StrSegment::AsciiEscape(escape) => v.visit_ascii_escape(escape),
        StrSegment::UnicodeEscape(escape) => v.visit_unicode_escape(escape),
        StrSegment::CharWithException(text) => v.visit_input(text),
    }
}

pub fn walk_ascii_escape<'ast, I, V>(v: &mut V, node: &'ast AsciiEscape<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    // Each form of escape is one token, however many characters it was written
    // with, so the walk hands over its text and stops.
    match node {
        AsciiEscape::LF(text)
        | AsciiEscape::CR(text)
        | AsciiEscape::Tab(text)
        | AsciiEscape::BlackSlash(text)
        | AsciiEscape::Null(text)
        | AsciiEscape::Char(text) => v.visit_input(text),
    }
}

pub fn walk_unicode_escape<'ast, I, V>(v: &mut V, node: &'ast UnicodeEscape<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_input(&node.delimiter_start);
    v.visit_input(&node.digits);
    v.visit_input(&node.delimiter_end);
}

pub fn walk_quote_escape<'ast, I, V>(v: &mut V, node: &'ast QuoteEscape<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_input(&node.0);
}

pub fn walk_lit_unicode<'ast, I, V>(v: &mut V, node: &'ast LitUnicode<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_input(&node.0);
}

pub fn walk_lit_dec<'ast, I, V>(v: &mut V, node: &'ast LitDec<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    v.visit_input(&node.0);
}

// ## The punctuation, one function per kind

pub fn walk_plus<'ast, I, V>(v: &mut V, node: &'ast Plus<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_minus<'ast, I, V>(v: &mut V, node: &'ast Minus<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_star<'ast, I, V>(v: &mut V, node: &'ast Star<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_question<'ast, I, V>(v: &mut V, node: &'ast Question<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_tilde<'ast, I, V>(v: &mut V, node: &'ast Tilde<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_or<'ast, I, V>(v: &mut V, node: &'ast Or<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_paren_start<'ast, I, V>(v: &mut V, node: &'ast ParenStart<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_paren_end<'ast, I, V>(v: &mut V, node: &'ast ParenEnd<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_bracket_start<'ast, I, V>(v: &mut V, node: &'ast BracketStart<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_bracket_end<'ast, I, V>(v: &mut V, node: &'ast BracketEnd<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_brace_start<'ast, I, V>(v: &mut V, node: &'ast BraceStart<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_brace_end<'ast, I, V>(v: &mut V, node: &'ast BraceEnd<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_lt<'ast, I, V>(v: &mut V, node: &'ast Lt<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_gt<'ast, I, V>(v: &mut V, node: &'ast Gt<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_comma<'ast, I, V>(v: &mut V, node: &'ast Comma<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_arrow_right<'ast, I, V>(v: &mut V, node: &'ast ArrowRight<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

pub fn walk_semi<'ast, I, V>(v: &mut V, node: &'ast Semi<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, punct!(node));
}

// ## The keywords, one function per kind

pub fn walk_whitespace<'ast, I, V>(v: &mut V, node: &'ast Whitespace<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, keyword!(node));
}

pub fn walk_lexer<'ast, I, V>(v: &mut V, node: &'ast Lexer<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, keyword!(node));
}

pub fn walk_syntax<'ast, I, V>(v: &mut V, node: &'ast Syntax<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, keyword!(node));
}

pub fn walk_followed<'ast, I, V>(v: &mut V, node: &'ast Followed<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, keyword!(node));
}

pub fn walk_concat<'ast, I, V>(v: &mut V, node: &'ast Concat<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, keyword!(node));
}

pub fn walk_except<'ast, I, V>(v: &mut V, node: &'ast Except<I>)
where
    I: UnsynInput,
    V: Visit<'ast, I> + ?Sized,
{
    walk_token(v, keyword!(node));
}
