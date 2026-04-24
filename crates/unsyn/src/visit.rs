//! visit-api for `unsyn` CST.

use parserc::syntax::Punctuated;

use crate::{
    errors::UnsynError,
    input::UnsynInput,
    syntax::{
        Expr, ExprNoTopAlt, ExprNoTopAlts, ExprWithSuffix, ExprWithoutSuffix, File,
        ModuleDeclaration, OuterDoc, Path, PathSegment, Range, Repeat, SetItem, Stmt,
        UseDeclaration, UseTree,
    },
    token::{
        S,
        comments::{OuterBlockDoc, OuterLineDoc},
        delimiter::{Angle, Brace, Bracket, Paren},
        ident::Ident,
        keyword::{
            As, Concat, Crate, Except, Followed, Lexer, Mod, Super, Syntax, This, Use, Whitespace,
        },
        lit::{LitDec, LitStr, LitUnicode},
        punct::{ArrowRight, Comma, DotDot, Minus, PathSep, Plus, Question, Semi, Star, Tilde},
    },
};

/// V `CST` node processable by the **semantic visitor**.
pub trait Visit<I>
where
    I: UnsynInput,
{
    fn analyze<V>(&self, visitor: &mut V) -> Result<(), UnsynError>
    where
        V: Visitor<I>;
}

/// Semantic visitor for the `unsyn` language.
pub trait Visitor<I>
where
    I: UnsynInput,
{
    /// visit the `File` node.
    #[inline(always)]
    fn visit_file(&mut self, node: &mut File<I>) {
        visit_file(self, node);
    }

    /// visit the `OuterDoc` node.
    #[inline(always)]
    fn visit_item_outer_doc(&mut self, node: &mut OuterDoc<I>) {
        visit_item_outer_doc(self, node);
    }

    /// visit the `S` node.
    #[inline(always)]
    fn visit_item_s(&mut self, node: &mut S<I>) {
        self.visit_token_s(node);
    }

    /// visit the `S` node.
    #[inline(always)]
    fn visit_token_s(&mut self, node: &mut S<I>) {
        let _ = node;
    }

    /// visit the `S` node.
    #[inline(always)]
    fn visit_punct_semi(&mut self, node: &mut Semi<I>) {
        let _ = node;
    }

    /// visit the `Option<S>` node.
    #[inline(always)]
    fn visit_option_s(&mut self, node: &mut Option<S<I>>) {
        if let Some(s) = node {
            self.visit_token_s(s);
        }
    }

    /// visit the `Use` node.
    #[inline(always)]
    fn visit_item_use(&mut self, use_declaration: &mut UseDeclaration<I>, semi: &mut Semi<I>) {
        visit_item_use(self, use_declaration, semi);
    }

    /// visit the `mod` node.
    #[inline(always)]
    fn visit_item_mod(
        &mut self,
        module_declaration: &mut ModuleDeclaration<I>,
        semi: &mut Semi<I>,
    ) {
        visit_item_mod(self, module_declaration, semi);
    }

    /// visit the `stmt` node.
    #[inline(always)]
    fn visit_item_stmt(&mut self, stmt: &mut Stmt<I>) {
        visit_item_stmt(self, stmt);
    }

    /// visit the `OuterBlockDoc` node.
    #[inline(always)]
    fn visit_outer_block_doc(&mut self, node: &mut OuterBlockDoc<I>) {
        let _ = node;
    }

    /// visit the `OuterLineDoc` node.
    #[inline(always)]
    fn visit_outer_line_doc(&mut self, node: &mut OuterLineDoc<I>) {
        let _ = node;
    }

    /// visit the `Use` node.
    #[inline(always)]
    fn visit_use_tree(&mut self, use_tree: &mut UseTree<I>) {
        visit_use_tree(self, use_tree);
    }

    /// visit the `Ident` node.
    #[inline(always)]
    fn visit_token_ident(&mut self, ident: &mut Ident<I>) {
        let _ = ident;
    }

    /// visit the `Ident` node.
    #[inline(always)]
    fn visit_punct_minus(&mut self, punct: &mut Minus<I>) {
        let _ = punct;
    }

    /// visit the `Ident` node.
    #[inline(always)]
    fn visit_punct_arrow_right(&mut self, punct: &mut ArrowRight<I>) {
        let _ = punct;
    }

    /// visit the stmt `Whitespace`.
    #[inline(always)]
    fn visit_stmt_whitespace(
        &mut self,
        keyword: &mut Whitespace<I>,
        ident: &mut Ident<I>,
        arrow_right: &mut ArrowRight<I>,
        expr: &mut Expr<I>,
        semi: &mut Semi<I>,
    ) {
        visit_stmt_whitespace(self, keyword, ident, arrow_right, expr, semi);
    }

    /// visit the stmt `Lexer`.
    #[inline(always)]
    fn visit_stmt_lexer(
        &mut self,
        keyword: &mut Lexer<I>,
        ident: &mut Ident<I>,
        arrow_right: &mut ArrowRight<I>,
        expr: &mut Expr<I>,
        semi: &mut Semi<I>,
    ) {
        visit_stmt_lexer(self, keyword, ident, arrow_right, expr, semi);
    }

    /// visit the stmt `Syntax`.
    #[inline(always)]
    fn visit_stmt_syntax(
        &mut self,
        keyword: &mut Syntax<I>,
        ident: &mut Ident<I>,
        arrow_right: &mut ArrowRight<I>,
        expr: &mut Expr<I>,
        semi: &mut Semi<I>,
    ) {
        visit_stmt_syntax(self, keyword, ident, arrow_right, expr, semi);
    }

    /// visit the `UseTree` star.
    #[inline(always)]
    fn visit_use_tree_star(
        &mut self,
        prefix: &mut Option<(Option<Path<I>>, PathSep<I>)>,
        star: &mut Star<I>,
    ) {
        visit_use_tree_star(self, prefix, star);
    }

    /// visit the `UseTree` group.
    #[inline(always)]
    fn visit_use_tree_path(
        &mut self,
        path: &mut Path<I>,
        as_branch: &mut Option<(As<I>, Ident<I>)>,
    ) {
        visit_use_tree_path(self, path, as_branch);
    }

    /// visit the `UseTree` group.
    #[inline(always)]
    fn visit_use_tree_group(
        &mut self,
        prefix: &mut Option<(Option<Path<I>>, PathSep<I>)>,
        group: &mut Brace<I, Punctuated<UseTree<I>, Comma<I>>>,
    ) {
        visit_use_tree_group(self, prefix, group);
    }

    /// visit the path node.
    #[inline(always)]
    fn visit_path(&mut self, path: &mut Path<I>) {
        visit_path(self, path);
    }

    /// visit the expr node.
    #[inline(always)]
    fn visit_expr(&mut self, expr: &mut Expr<I>) {
        visit_expr(self, expr);
    }

    /// visit the expr node.
    #[inline(always)]
    fn visit_expr_with_suffix(&mut self, expr: &mut ExprWithSuffix<I>) {
        visit_expr_with_suffix(self, expr);
    }

    /// visit the expr node.
    #[inline(always)]
    fn visit_expr_without_suffix(&mut self, expr: &mut ExprWithoutSuffix<I>) {
        visit_expr_without_suffix(self, expr);
    }

    /// visit the `PathSegment` node.
    #[inline(always)]
    fn visit_path_segment(&mut self, segment: &mut PathSegment<I>) {
        visit_path_segment(self, segment);
    }

    /// visit the `ExprNoTopAlts` node.
    #[inline(always)]
    fn visit_expr_no_top_alts(&mut self, node: &mut ExprNoTopAlts<I>) {
        visit_expr_no_top_alts(self, node);
    }

    /// visit the `ExprNoTopAlts` node.
    #[inline(always)]
    fn visit_expr_no_top_alt(&mut self, node: &mut ExprNoTopAlt<I>) {
        visit_expr_no_top_alt(self, node);
    }

    /// visit the pathsegment `This`.
    #[inline(always)]
    fn visit_pathsegment_this(&mut self, node: &mut This<I>) {
        visit_pathsegment_this(self, node);
    }

    /// visit the pathsegment `super`.
    #[inline(always)]
    fn visit_pathsegment_super(&mut self, node: &mut Super<I>) {
        visit_pathsegment_super(self, node);
    }

    /// visit the pathsegment `crate`.
    #[inline(always)]
    fn visit_pathsegment_crate(&mut self, node: &mut Crate<I>) {
        visit_pathsegment_crate(self, node);
    }

    /// visit the pathsegment `crate`.
    #[inline(always)]
    fn visit_pathsegment_ident(&mut self, node: &mut Ident<I>) {
        self.visit_token_ident(node);
    }

    /// visit expr with `star`
    #[inline(always)]
    fn visit_expr_with_star(
        &mut self,
        expr_without_suffix: &mut ExprWithoutSuffix<I>,
        star: &mut Star<I>,
    ) {
        visit_expr_with_star(self, expr_without_suffix, star);
    }

    /// visit expr with `star`
    #[inline(always)]
    fn visit_expr_with_plus(
        &mut self,
        expr_without_suffix: &mut ExprWithoutSuffix<I>,
        suffix: &mut Plus<I>,
    ) {
        visit_expr_with_plus(self, expr_without_suffix, suffix);
    }

    /// visit expr with `?`
    #[inline(always)]
    fn visit_expr_with_question(
        &mut self,
        expr_without_suffix: &mut ExprWithoutSuffix<I>,
        suffix: &mut Question<I>,
    ) {
        visit_expr_with_question(self, expr_without_suffix, suffix);
    }

    /// visit expr with `Repeat`
    #[inline(always)]
    fn visit_expr_with_repeat(
        &mut self,
        expr_without_suffix: &mut ExprWithoutSuffix<I>,
        suffix: &mut Brace<I, Repeat<I>>,
    ) {
        visit_expr_with_repeat(self, expr_without_suffix, suffix);
    }

    /// visit expr with `Repeat`
    #[inline(always)]
    fn visit_expr_concat(
        &mut self,
        target: &mut ExprWithoutSuffix<I>,
        s: &mut Option<S<I>>,
        keyword: &mut Concat<I>,
        suffix: &mut Box<ExprNoTopAlt<I>>,
    ) {
        visit_expr_concat(self, target, s, keyword, suffix);
    }

    /// visit expr with `followed`
    #[inline(always)]
    fn visit_expr_with_followed(
        &mut self,
        target: &mut ExprWithoutSuffix<I>,
        s: &mut Option<S<I>>,
        keyword: &mut Followed<I>,
        suffix: &mut Box<ExprNoTopAlt<I>>,
    ) {
        visit_expr_with_followed(self, target, s, keyword, suffix);
    }

    /// visit expr with `followed`
    #[inline(always)]
    fn visit_expr_with_except(
        &mut self,
        target: &mut ExprWithoutSuffix<I>,
        s: &mut Option<S<I>>,
        keyword: &mut Except<I>,
        suffix: &mut ExprWithoutSuffix<I>,
    ) {
        visit_expr_with_except(self, target, s, keyword, suffix);
    }

    /// visit expr `Repeat`
    #[inline(always)]
    fn visit_expr_repeat(&mut self, expr: &mut Repeat<I>) {
        visit_expr_repeat(self, expr);
    }

    /// tilde expr.
    #[inline(always)]
    fn visit_expr_tilde(&mut self, keyword: &mut Tilde<I>, suffix: &mut Box<ExprWithoutSuffix<I>>) {
        visit_expr_tilde(self, keyword, suffix);
    }

    #[inline(always)]
    fn visit_expr_call(&mut self, expr: &mut Angle<I, Ident<I>>) {
        visit_expr_call(self, expr);
    }

    /// paren expr `(T)`
    #[inline(always)]
    fn visit_expr_paren(&mut self, expr: &mut Paren<I, Box<Expr<I>>>) {
        visit_expr_paren(self, expr);
    }

    /// a set expression,
    #[inline(always)]
    fn visit_expr_set(&mut self, expr: &mut Bracket<I, Punctuated<SetItem<I>, Comma<I>>>) {
        visit_expr_set(self, expr);
    }

    /// A literal range expr.
    #[inline(always)]
    fn visit_expr_range(&mut self, expr: &mut Range<I>) {
        visit_expr_range(self, expr);
    }

    /// A literal range expr.
    #[inline(always)]
    fn visit_expr_range_unicode(
        &mut self,
        from: &mut LitUnicode<I>,
        minus: &mut Minus<I>,
        to: &mut LitUnicode<I>,
    ) {
        visit_expr_range_unicode(self, from, minus, to);
    }

    /// A literal range expr.
    #[inline(always)]
    fn visit_expr_range_char(
        &mut self,
        from: &mut LitStr<I>,
        minus: &mut Minus<I>,
        to: &mut LitStr<I>,
    ) {
        visit_expr_range_char(self, from, minus, to);
    }

    /// A literal string expr.
    #[inline(always)]
    fn visit_expr_str(&mut self, expr: &mut LitStr<I>) {
        visit_expr_str(self, expr);
    }

    /// A literal unicode expr.
    #[inline(always)]
    fn visit_expr_unicode(&mut self, expr: &mut LitUnicode<I>) {
        visit_expr_unicode(self, expr);
    }

    /// A path expression.
    #[inline(always)]
    fn visit_expr_path(&mut self, expr: &mut Path<I>) {
        visit_expr_path(self, expr);
    }

    /// A set item expression.
    #[inline(always)]
    fn visit_expr_set_item(&mut self, expr: &mut SetItem<I>) {
        visit_expr_set_item(self, expr);
    }

    #[inline(always)]
    fn visit_expr_repeat_range_to(&mut self, dotdot: &mut DotDot<I>, dec: &mut LitDec<I>) {
        visit_expr_repeat_range_to(self, dotdot, dec)
    }

    #[inline(always)]
    fn visit_expr_repeat_range(
        &mut self,
        from: &mut LitDec<I>,
        dotdot: &mut DotDot<I>,
        to: &mut LitDec<I>,
    ) {
        visit_expr_repeat_range(self, from, dotdot, to);
    }

    #[inline(always)]
    fn visit_expr_repeat_range_from(&mut self, from: &mut LitDec<I>, dotdot: &mut DotDot<I>) {
        visit_expr_repeat_range_from(self, from, dotdot);
    }

    #[inline(always)]
    fn visit_expr_repeat_count(&mut self, dec: &mut LitDec<I>) {
        visit_expr_repeat_count(self, dec)
    }

    #[inline(always)]
    fn visit_lit_unicode(&mut self, lit: &mut LitUnicode<I>) {
        let _ = lit;
    }

    #[inline(always)]
    fn visit_lit_dec(&mut self, lit: &mut LitDec<I>) {
        let _ = lit;
    }

    #[inline(always)]
    fn visit_lit_str(&mut self, lit: &mut LitStr<I>) {
        let _ = lit;
    }

    #[inline(always)]
    fn visit_punct_star(&mut self, punct: &mut Star<I>) {
        let _ = punct;
    }

    #[inline(always)]
    fn visit_keyword_this(&mut self, keyword: &mut This<I>) {
        let _ = keyword;
    }

    #[inline(always)]
    fn visit_keyword_concat(&mut self, keyword: &mut Concat<I>) {
        let _ = keyword;
    }

    #[inline(always)]
    fn visit_keyword_followed(&mut self, keyword: &mut Followed<I>) {
        let _ = keyword;
    }

    #[inline(always)]
    fn visit_keyword_super(&mut self, keyword: &mut Super<I>) {
        let _ = keyword;
    }

    #[inline(always)]
    fn visit_keyword_whitespace(&mut self, keyword: &mut Whitespace<I>) {
        let _ = keyword;
    }

    #[inline(always)]
    fn visit_keyword_crate(&mut self, keyword: &mut Crate<I>) {
        let _ = keyword;
    }

    #[inline(always)]
    fn visit_keyword_except(&mut self, keyword: &mut Except<I>) {
        let _ = keyword;
    }

    #[inline(always)]
    fn visit_keyword_mod(&mut self, keyword: &mut Mod<I>) {
        let _ = keyword;
    }

    #[inline(always)]
    fn visit_keyword_use(&mut self, keyword: &mut Use<I>) {
        let _ = keyword;
    }

    #[inline(always)]
    fn visit_punct_tilde(&mut self, keyword: &mut Tilde<I>) {
        let _ = keyword;
    }
}

/// Call this function in [`visit_file`](Analyzer::visit_file) to recurse into child nodes.
#[inline]
pub fn visit_file<V, I>(visitor: &mut V, node: &mut File<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    for item in &mut node.items {
        match item {
            crate::syntax::Item::OuterDoc(outer_doc) => visitor.visit_item_outer_doc(outer_doc),
            crate::syntax::Item::S(s) => visitor.visit_item_s(s),
            crate::syntax::Item::Use(use_declaration, semi) => {
                visitor.visit_item_use(use_declaration, semi);
            }
            crate::syntax::Item::Mod(module_declaration, semi) => {
                visitor.visit_item_mod(module_declaration, semi);
            }
            crate::syntax::Item::Stmt(stmt) => visitor.visit_item_stmt(stmt),
        }
    }
}

/// Call this function in [`visit_item_outer_doc`](Analyzer::visit_item_outer_doc) to recurse into child nodes.
#[inline]
pub fn visit_item_outer_doc<V, I>(visitor: &mut V, outer_doc: &mut OuterDoc<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    match outer_doc {
        OuterDoc::Block(s, outer_block_doc) => {
            visitor.visit_option_s(s);
            visitor.visit_outer_block_doc(outer_block_doc);
        }
        OuterDoc::Line(s, outer_line_doc) => {
            visitor.visit_option_s(s);
            visitor.visit_outer_line_doc(outer_line_doc);
        }
    }
}

/// Call this function in [`visit_item_use`](Analyzer::visit_item_use) to recurse into child nodes.
#[inline]
pub fn visit_item_use<V, I>(visitor: &mut V, node: &mut UseDeclaration<I>, semi: &mut Semi<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_keyword_use(&mut node.keyword);
    visitor.visit_use_tree(&mut node.use_tree);
    visitor.visit_punct_semi(semi);
}

/// Call this function in [`visit_item_mod`](Analyzer::visit_item_mod) to recurse into child nodes.
#[inline]
pub fn visit_item_mod<V, I>(visitor: &mut V, node: &mut ModuleDeclaration<I>, semi: &mut Semi<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_keyword_mod(&mut node.keyword);
    visitor.visit_token_ident(&mut node.ident);
    visitor.visit_punct_semi(semi);
}

/// Call this function in [`visit_item_outer_doc`](Analyzer::visit_item_outer_doc) to recurse into child nodes.
#[inline]
pub fn visit_item_stmt<V, I>(visitor: &mut V, stmt: &mut Stmt<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    match stmt {
        Stmt::Whitespace {
            keyword,
            ident,
            arrow_right,
            expr,
            semi,
        } => visitor.visit_stmt_whitespace(keyword, ident, arrow_right, expr, semi),
        Stmt::Lexer {
            keyword,
            ident,
            arrow_right,
            expr,
            semi,
        } => visitor.visit_stmt_lexer(keyword, ident, arrow_right, expr, semi),
        Stmt::Syntax {
            keyword,
            ident,
            arrow_right,
            expr,
            semi,
        } => visitor.visit_stmt_syntax(keyword, ident, arrow_right, expr, semi),
    }
}

/// Call this function in [`visit_use_tree`](Analyzer::visit_use_tree) to recurse into child nodes.
#[inline]
pub fn visit_use_tree<V, I>(visitor: &mut V, use_tree: &mut UseTree<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    match use_tree {
        UseTree::Star { prefix, star } => visitor.visit_use_tree_star(prefix, star),
        UseTree::Group { prefix, group } => visitor.visit_use_tree_group(prefix, group),
        UseTree::Path(path, as_branch) => {
            visitor.visit_path(path);

            if let Some((_, ident)) = as_branch {
                visitor.visit_token_ident(ident);
            }
        }
    }
}

/// Call this function in [`visit_stmt_whitespace`](Analyzer::visit_stmt_whitespace) to recurse into child nodes.
#[inline(always)]
pub fn visit_stmt_whitespace<V, I>(
    visitor: &mut V,
    keyword: &mut Whitespace<I>,
    ident: &mut Ident<I>,
    arrow_right: &mut ArrowRight<I>,
    expr: &mut Expr<I>,
    semi: &mut Semi<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_keyword_whitespace(keyword);
    visitor.visit_token_ident(ident);
    visitor.visit_punct_arrow_right(arrow_right);
    visitor.visit_expr(expr);
    visitor.visit_punct_semi(semi);
}

/// Call this function in [`visit_stmt_lexer`](Analyzer::visit_stmt_lexer) to recurse into child nodes.
#[inline(always)]
#[allow(unused)]
pub fn visit_stmt_lexer<V, I>(
    visitor: &mut V,
    keyword: &mut Lexer<I>,
    ident: &mut Ident<I>,
    arrow_right: &mut ArrowRight<I>,
    expr: &mut Expr<I>,
    semi: &mut Semi<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_token_ident(ident);
    visitor.visit_expr(expr);
}

/// Call this function in [`visit_stmt_syntax`](Analyzer::visit_stmt_syntax) to recurse into child nodes.
#[inline(always)]
#[allow(unused)]
pub fn visit_stmt_syntax<V, I>(
    visitor: &mut V,
    keyword: &mut Syntax<I>,
    ident: &mut Ident<I>,
    arrow_right: &mut ArrowRight<I>,
    expr: &mut Expr<I>,
    semi: &mut Semi<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_token_ident(ident);
    visitor.visit_expr(expr);
}

/// Call this function in [`visit_use_tree_star`](Analyzer::visit_use_tree_star) to recurse into child nodes.
#[inline(always)]
pub fn visit_use_tree_star<V, I>(
    visitor: &mut V,
    prefix: &mut Option<(Option<Path<I>>, PathSep<I>)>,
    star: &mut Star<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    if let Some((Some(path), _)) = prefix {
        visitor.visit_path(path);
    }

    visitor.visit_punct_star(star);
}

/// Call this function in [`visit_use_tree_path`](Analyzer::visit_use_tree_path) to recurse into child nodes.
#[inline(always)]
pub fn visit_use_tree_path<V, I>(
    visitor: &mut V,
    path: &mut Path<I>,
    as_branch: &mut Option<(As<I>, Ident<I>)>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_path(path);

    if let Some((_, ident)) = as_branch {
        visitor.visit_token_ident(ident);
    }
}

/// Call this function in [`visit_use_tree_group`](Analyzer::visit_use_tree_group) to recurse into child nodes.
#[inline(always)]
pub fn visit_use_tree_group<V, I>(
    visitor: &mut V,
    prefix: &mut Option<(Option<Path<I>>, PathSep<I>)>,
    brace: &mut Brace<I, Punctuated<UseTree<I>, Comma<I>>>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    if let Some((Some(path), _)) = prefix {
        visitor.visit_path(path);
    }

    for use_tree in &mut brace.body {
        visitor.visit_use_tree(use_tree);
    }
}

/// Call this function in [`visit_path`](Analyzer::visit_path) to recurse into child nodes.
#[inline(always)]
pub fn visit_path<V, I>(visitor: &mut V, path: &mut Path<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_path_segment(&mut path.first);

    for (_, segment) in &mut path.rest {
        visitor.visit_path_segment(segment);
    }
}

/// Call this function in [`visit_expr`](Analyzer::visit_expr) to recurse into child nodes.
#[inline(always)]
pub fn visit_expr<V, I>(visitor: &mut V, expr: &mut Expr<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_expr_no_top_alts(&mut expr.first);

    for (_, next) in &mut expr.rest {
        visitor.visit_expr_no_top_alts(next);
    }
}

/// Call this function in [`visit_path_segment`](Analyzer::visit_path_segment) to recurse into child nodes.
#[inline(always)]
pub fn visit_path_segment<V, I>(visitor: &mut V, segment: &mut PathSegment<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    match segment {
        PathSegment::This(node) => visitor.visit_pathsegment_this(node),
        PathSegment::Super(node) => visitor.visit_pathsegment_super(node),
        PathSegment::Crate(node) => visitor.visit_pathsegment_crate(node),
        PathSegment::Ident(node) => visitor.visit_pathsegment_ident(node),
    }
}

/// Call this function in [`visit_expr_no_top_alts`](Analyzer::visit_expr_no_top_alts) to recurse into child nodes.
#[inline(always)]
pub fn visit_expr_no_top_alts<V, I>(visitor: &mut V, node: &mut ExprNoTopAlts<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_expr_no_top_alt(&mut node.first);

    for (_, next) in &mut node.rest {
        visitor.visit_expr_no_top_alt(next);
    }
}

/// Call this function in [`visit_expr_no_top_alt`](Analyzer::visit_expr_no_top_alt) to recurse into child nodes.
#[inline(always)]
pub fn visit_expr_no_top_alt<V, I>(visitor: &mut V, node: &mut ExprNoTopAlt<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    match node {
        ExprNoTopAlt::WithSuffix(expr) => visitor.visit_expr_with_suffix(expr),
        ExprNoTopAlt::WithoutSuffix(expr) => visitor.visit_expr_without_suffix(expr),
    }
}

/// Call this function in [`visit_pathsegment_this`](Analyzer::visit_pathsegment_this) to recurse into child nodes.
#[inline(always)]
pub fn visit_pathsegment_this<V, I>(visitor: &mut V, node: &mut This<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_keyword_this(node);
}

/// Call this function in [`visit_pathsegment_this`](Analyzer::visit_pathsegment_this) to recurse into child nodes.
#[inline(always)]
pub fn visit_pathsegment_super<V, I>(visitor: &mut V, node: &mut Super<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_keyword_super(node);
}

/// Call this function in [`visit_pathsegment_this`](Analyzer::visit_pathsegment_this) to recurse into child nodes.
#[inline(always)]
pub fn visit_pathsegment_crate<V, I>(visitor: &mut V, node: &mut Crate<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_keyword_crate(node);
}

/// Call this function in [`visit_expr_with_suffix`](Analyzer::visit_expr_with_suffix) to recurse into child nodes.
#[inline(always)]
pub fn visit_expr_with_suffix<V, I>(visitor: &mut V, node: &mut ExprWithSuffix<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    match node {
        ExprWithSuffix::Star(expr_without_suffix, star) => {
            visitor.visit_expr_with_star(expr_without_suffix, star)
        }
        ExprWithSuffix::Question(expr_without_suffix, question) => {
            visitor.visit_expr_with_question(expr_without_suffix, question)
        }
        ExprWithSuffix::Plus(expr_without_suffix, plus) => {
            visitor.visit_expr_with_plus(expr_without_suffix, plus)
        }
        ExprWithSuffix::Repeat { target, suffix } => visitor.visit_expr_with_repeat(target, suffix),
        ExprWithSuffix::Concat {
            target,
            s,
            keyword,
            suffix,
        } => {
            visitor.visit_expr_concat(target, s, keyword, suffix);
        }
        ExprWithSuffix::Followed {
            target,
            s,
            keyword,
            suffix,
        } => {
            visitor.visit_expr_with_followed(target, s, keyword, suffix);
        }
        ExprWithSuffix::Except {
            target,
            s,
            keyword,
            tokens,
        } => visitor.visit_expr_with_except(target, s, keyword, tokens),
    }
}

/// Call this function in [`visit_expr_without_suffix`](Analyzer::visit_expr_without_suffix) to recurse into child nodes.
#[inline(always)]
pub fn visit_expr_without_suffix<V, I>(visitor: &mut V, node: &mut ExprWithoutSuffix<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    match node {
        ExprWithoutSuffix::Tilde(tilde, expr_without_suffix) => {
            visitor.visit_expr_tilde(tilde, expr_without_suffix)
        }
        ExprWithoutSuffix::Call(delimiter) => visitor.visit_expr_call(delimiter),
        ExprWithoutSuffix::Paren(delimiter) => visitor.visit_expr_paren(delimiter),
        ExprWithoutSuffix::Set(delimiter) => visitor.visit_expr_set(delimiter),
        ExprWithoutSuffix::Str(lit_str) => visitor.visit_expr_str(lit_str),
        ExprWithoutSuffix::Unicode(lit_unicode) => visitor.visit_expr_unicode(lit_unicode),
        ExprWithoutSuffix::Path(path) => visitor.visit_expr_path(path),
    }
}

/// Call this function in [`visit_expr_without_suffix`](Analyzer::visit_expr_without_suffix) to recurse into child nodes.
#[inline(always)]
pub fn visit_expr_with_star<V, I>(
    visitor: &mut V,
    expr_without_suffix: &mut ExprWithoutSuffix<I>,
    star: &mut Star<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_expr_without_suffix(expr_without_suffix);
    visitor.visit_punct_star(star);
}

/// Call this function in [`visit_expr_without_suffix`](Analyzer::visit_expr_without_suffix) to recurse into child nodes.
#[inline(always)]
pub fn visit_expr_with_plus<V, I>(
    visitor: &mut V,
    expr_without_suffix: &mut ExprWithoutSuffix<I>,
    suffix: &mut Plus<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_expr_without_suffix(expr_without_suffix);
    let _ = suffix;
}

/// Call this function in [`visit_expr_without_suffix`](Analyzer::visit_expr_without_suffix) to recurse into child nodes.
#[inline(always)]
pub fn visit_expr_with_question<V, I>(
    visitor: &mut V,
    expr_without_suffix: &mut ExprWithoutSuffix<I>,
    suffix: &mut Question<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_expr_without_suffix(expr_without_suffix);
    let _ = suffix;
}

/// Call this function in [`visit_expr_without_suffix`](Analyzer::visit_expr_without_suffix) to recurse into child nodes.
#[inline(always)]
pub fn visit_expr_with_repeat<V, I>(
    visitor: &mut V,
    expr_without_suffix: &mut ExprWithoutSuffix<I>,
    suffix: &mut Brace<I, Repeat<I>>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_expr_without_suffix(expr_without_suffix);
    visitor.visit_expr_repeat(&mut suffix.body);
}

/// Call this function in [`visit_expr_without_suffix`](Analyzer::visit_expr_without_suffix) to recurse into child nodes.
#[inline(always)]
pub fn visit_expr_concat<V, I>(
    visitor: &mut V,
    target: &mut ExprWithoutSuffix<I>,
    s: &mut Option<S<I>>,
    keyword: &mut Concat<I>,
    suffix: &mut Box<ExprNoTopAlt<I>>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_expr_without_suffix(target);
    visitor.visit_option_s(s);
    visitor.visit_keyword_concat(keyword);
    visitor.visit_expr_no_top_alt(suffix.as_mut());
}

/// Call this function in [`visit_expr_without_suffix`](Analyzer::visit_expr_without_suffix) to recurse into child nodes.
#[inline(always)]
pub fn visit_expr_with_followed<V, I>(
    visitor: &mut V,
    target: &mut ExprWithoutSuffix<I>,
    s: &mut Option<S<I>>,
    keyword: &mut Followed<I>,
    suffix: &mut Box<ExprNoTopAlt<I>>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_expr_without_suffix(target);

    visitor.visit_option_s(s);
    visitor.visit_keyword_followed(keyword);

    visitor.visit_expr_no_top_alt(suffix.as_mut());
}

/// Call this function in [`visit_expr_without_suffix`](Analyzer::visit_expr_without_suffix) to recurse into child nodes.
#[inline(always)]
pub fn visit_expr_with_except<V, I>(
    visitor: &mut V,
    target: &mut ExprWithoutSuffix<I>,
    s: &mut Option<S<I>>,
    keyword: &mut Except<I>,
    suffix: &mut ExprWithoutSuffix<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_expr_without_suffix(target);

    visitor.visit_option_s(s);
    visitor.visit_keyword_except(keyword);

    visitor.visit_expr_without_suffix(suffix);
}

/// visit expr `Repeat`
#[inline(always)]
pub fn visit_expr_repeat<V, I>(visitor: &mut V, expr: &mut Repeat<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    match expr {
        Repeat::RangeTo(dot_dot, lit_dec) => visitor.visit_expr_repeat_range_to(dot_dot, lit_dec),
        Repeat::Range(from, dotdot, to) => visitor.visit_expr_repeat_range(from, dotdot, to),
        Repeat::RangeFrom(lit_dec, dot_dot) => {
            visitor.visit_expr_repeat_range_from(lit_dec, dot_dot)
        }
        Repeat::Times(lit_dec) => visitor.visit_expr_repeat_count(lit_dec),
    }
}

/// tilde expr.
#[inline(always)]
pub fn visit_expr_tilde<V, I>(
    visitor: &mut V,
    keyword: &mut Tilde<I>,
    suffix: &mut Box<ExprWithoutSuffix<I>>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_punct_tilde(keyword);
    visitor.visit_expr_without_suffix(suffix.as_mut());
}

#[inline(always)]
pub fn visit_expr_call<V, I>(visitor: &mut V, expr: &mut Angle<I, Ident<I>>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_token_ident(&mut expr.body);
}

/// paren expr `(T)`
#[inline(always)]
pub fn visit_expr_paren<V, I>(visitor: &mut V, expr: &mut Paren<I, Box<Expr<I>>>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_expr(expr.body.as_mut());
}

/// a set expression,
#[inline(always)]
pub fn visit_expr_set<V, I>(
    visitor: &mut V,
    expr: &mut Bracket<I, Punctuated<SetItem<I>, Comma<I>>>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    for item in &mut expr.body {
        visitor.visit_expr_set_item(item);
    }
}

/// A set item expression.
#[inline(always)]
fn visit_expr_set_item<V, I>(visitor: &mut V, expr: &mut SetItem<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    match expr {
        SetItem::Range(range) => visitor.visit_expr_range(range),
        SetItem::Str(lit_str) => visitor.visit_expr_str(lit_str),
        SetItem::Unicode(lit_unicode) => visitor.visit_expr_unicode(lit_unicode),
        SetItem::Path(path) => visitor.visit_expr_path(path),
    }
}

/// A literal string expr.
#[inline(always)]
pub fn visit_expr_str<V, I>(#[allow(unused)] visitor: &mut V, expr: &mut LitStr<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_lit_str(expr);
}

/// A literal unicode expr.
#[inline(always)]
pub fn visit_expr_unicode<V, I>(visitor: &mut V, expr: &mut LitUnicode<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_lit_unicode(expr);
}

/// A path expression.
#[inline(always)]
pub fn visit_expr_path<V, I>(visitor: &mut V, expr: &mut Path<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_path(expr);
}

#[inline(always)]
#[allow(unused)]
pub fn visit_expr_repeat_range_to<V, I>(
    visitor: &mut V,
    dotdot: &mut DotDot<I>,
    dec: &mut LitDec<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
}

#[inline(always)]
#[allow(unused)]
pub fn visit_expr_repeat_range<V, I>(
    visitor: &mut V,
    from: &mut LitDec<I>,
    dotdot: &mut DotDot<I>,
    to: &mut LitDec<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
}

#[inline(always)]
#[allow(unused)]
pub fn visit_expr_repeat_range_from<V, I>(
    visitor: &mut V,
    from: &mut LitDec<I>,
    dotdot: &mut DotDot<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
}

#[inline(always)]
#[allow(unused)]
pub fn visit_expr_repeat_count<V, I>(visitor: &mut V, dec: &mut LitDec<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
}

#[inline(always)]
fn visit_expr_range<V, I>(visitor: &mut V, expr: &mut Range<I>)
where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    match expr {
        Range::Unicode(from, minus, to) => visitor.visit_expr_range_unicode(from, minus, to),
        Range::Char(from, minus, to) => visitor.visit_expr_range_char(from, minus, to),
    }
}

/// A literal range expr.
#[inline(always)]
fn visit_expr_range_unicode<V, I>(
    visitor: &mut V,
    from: &mut LitUnicode<I>,
    minus: &mut Minus<I>,
    to: &mut LitUnicode<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_expr_unicode(from);
    visitor.visit_punct_minus(minus);
    visitor.visit_expr_unicode(to);
}

/// A literal range expr.
#[inline(always)]
fn visit_expr_range_char<V, I>(
    visitor: &mut V,
    from: &mut LitStr<I>,
    minus: &mut Minus<I>,
    to: &mut LitStr<I>,
) where
    I: UnsynInput,
    V: Visitor<I> + ?Sized,
{
    visitor.visit_expr_str(from);
    visitor.visit_punct_minus(minus);
    visitor.visit_expr_str(to);
}
