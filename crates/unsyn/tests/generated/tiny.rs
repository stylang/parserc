//! The code of a grammar of another language, as a build writes it when
//! it is told nothing.
//!
//! The stream trait, the error and the family of rules a refusal is
//! named after are all written here, because no caller said where they
//! stand — which is the half of a build that the file of `unsyn.us`
//! leaves untested, that file being kept inside the crate whose error it
//! reaches.
//!
//! This file is generated: see `tests/unsyn_us.rs`. Do not edit it.
#![allow(dead_code, clippy::enum_variant_names)]
pub trait UnsynInput:
    ::parserc::Input<Item = char, Error = UnsynError>
    + ::parserc::AsBytes
    + ::parserc::AsStr
    + ::parserc::StartWith<&'static str>
    + ::parserc::StartWith<&'static [u8]>
    + ::parserc::Find<&'static str>
    + ::parserc::Find<&'static [u8]>
    + ::core::clone::Clone
    + ::core::fmt::Debug
    + ::core::cmp::PartialEq
{
}
pub type TokenStream<'a> = ::parserc::chars::TokenStream<'a, UnsynError>;
impl<'a> UnsynInput for TokenStream<'a> {}
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SyntaxKind {
    Word,
    Run,
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum UnsynError {
    Kind(::parserc::Kind),
    Syntax(SyntaxKind, ::parserc::ControlFlow, ::parserc::Span),
}
impl ::core::convert::From<::parserc::Kind> for UnsynError {
    #[inline]
    fn from(kind: ::parserc::Kind) -> Self {
        Self::Kind(kind)
    }
}
impl ::parserc::ParseError for UnsynError {
    #[inline]
    fn to_span(&self) -> ::parserc::Span {
        match self {
            UnsynError::Kind(kind) => kind.to_span(),
            UnsynError::Syntax(_, _, span) => span.clone(),
        }
    }
    #[inline]
    fn control_flow(&self) -> ::parserc::ControlFlow {
        match self {
            UnsynError::Kind(kind) => kind.control_flow(),
            UnsynError::Syntax(_, control_flow, _) => *control_flow,
        }
    }
    #[inline]
    fn into_fatal(self) -> Self {
        match self {
            UnsynError::Kind(kind) => Self::Kind(kind.into_fatal()),
            UnsynError::Syntax(kind, _, span) => {
                Self::Syntax(kind, ::parserc::ControlFlow::Fatal, span)
            }
        }
    }
}
#[inline]
fn is_w(c: char) -> bool {
    matches!(c, ' ')
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
# [parserc (take_while_with = (1 .. , is_w))]
pub struct W<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(next = 'a')]
pub struct A<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(next = 'b')]
pub struct B<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct Inner<I>
where
    I: UnsynInput,
{
    pub a: A<I>,
    pub b: B<I>,
}
fn read_word<I>(input: &mut I) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
        let taken = ::parserc::text_of(<Inner<I> as ::parserc::syntax::Syntax<I>>::into_parser())
            .parse(input)?;
        {
            let mut rest = taken.clone();
            if <A<I> as ::parserc::syntax::Syntax<I>>::into_parser()
                .map(|node| node.0)
                .ok()
                .parse(&mut rest)?
                .is_some()
                && rest.is_empty()
            {
                return Err(UnsynError::Syntax(
                    SyntaxKind::Word,
                    ::parserc::ControlFlow::Fatal,
                    taken.to_span(),
                ));
            }
        }
        Ok(())
    })
    .parse(input)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(read_word)]
pub struct Word<I>(pub I)
where
    I: UnsynInput;
fn read_run_1<I>(
    input: &mut I,
) -> ::core::result::Result<::std::vec::Vec<Inner<I>>, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    let mut taken = ::std::vec::Vec::new();
    while taken.len() < 2usize {
        let Some(one) = <Inner<I> as ::parserc::syntax::Syntax<I>>::into_parser()
            .ok()
            .parse(input)?
        else {
            break;
        };
        taken.push(one);
    }
    if taken.len() < 2usize {
        return Err(UnsynError::Syntax(
            SyntaxKind::Run,
            ::parserc::ControlFlow::Recovable,
            input.to_span(),
        ));
    }
    Ok(taken)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct Run<I>
where
    I: UnsynInput,
{
    #[parserc(next = 'x')]
    pub f1: I,
    #[parserc(read_run_1)]
    pub inner: ::std::vec::Vec<Inner<I>>,
}
#[doc = r" The visitor of the file's nodes, in the shape of `syn`'s: one"]
#[doc = r" method per rule descending through the walk of it unless a"]
#[doc = r" visitor says otherwise, and one method for the text that every"]
#[doc = r" walk ends with."]
#[doc = r""]
#[doc = r" [`syn`]: https://docs.rs/syn/latest/syn/visit/index.html"]
pub trait Visit<'ast, I>
where
    I: UnsynInput,
{
    #[doc = r" The text of one token, which is the leaf every descent"]
    #[doc = r" hands over."]
    fn visit_text(&mut self, _text: &'ast I) {}
    # [doc = concat ! ("The node of the rule `" , stringify ! (W) , "`.")]
    fn visit_w(&mut self, node: &'ast W<I>) {
        walk::walk_w(self, node)
    }
    # [doc = concat ! ("The node of the rule `" , stringify ! (A) , "`.")]
    fn visit_a(&mut self, node: &'ast A<I>) {
        walk::walk_a(self, node)
    }
    # [doc = concat ! ("The node of the rule `" , stringify ! (B) , "`.")]
    fn visit_b(&mut self, node: &'ast B<I>) {
        walk::walk_b(self, node)
    }
    # [doc = concat ! ("The node of the rule `" , stringify ! (Inner) , "`.")]
    fn visit_inner(&mut self, node: &'ast Inner<I>) {
        walk::walk_inner(self, node)
    }
    # [doc = concat ! ("The node of the rule `" , stringify ! (Word) , "`.")]
    fn visit_word(&mut self, node: &'ast Word<I>) {
        walk::walk_word(self, node)
    }
    # [doc = concat ! ("The node of the rule `" , stringify ! (Run) , "`.")]
    fn visit_run(&mut self, node: &'ast Run<I>) {
        walk::walk_run(self, node)
    }
}
#[doc = r" The descents of the visitor, one per rule, public so that a"]
#[doc = r" method which does its own work can still hand the children to"]
#[doc = r" the visitor below it."]
#[allow(clippy::needless_borrow, clippy::possible_missing_else)]
pub mod walk {
    # [rustfmt :: skip] use super :: { Visit , UnsynInput , W , A , B , Inner , Word , Run } ;
    # [doc = concat ! ("Descend through the node of `" , stringify ! (W) , "`.")]
    pub fn walk_w<'ast, I, V>(v: &mut V, node: &'ast W<I>)
    where
        I: UnsynInput,
        V: ?Sized + Visit<'ast, I>,
    {
        v.visit_text(&node.0);
    }
    # [doc = concat ! ("Descend through the node of `" , stringify ! (A) , "`.")]
    pub fn walk_a<'ast, I, V>(v: &mut V, node: &'ast A<I>)
    where
        I: UnsynInput,
        V: ?Sized + Visit<'ast, I>,
    {
        v.visit_text(&node.0);
    }
    # [doc = concat ! ("Descend through the node of `" , stringify ! (B) , "`.")]
    pub fn walk_b<'ast, I, V>(v: &mut V, node: &'ast B<I>)
    where
        I: UnsynInput,
        V: ?Sized + Visit<'ast, I>,
    {
        v.visit_text(&node.0);
    }
    # [doc = concat ! ("Descend through the node of `" , stringify ! (Inner) , "`.")]
    pub fn walk_inner<'ast, I, V>(v: &mut V, node: &'ast Inner<I>)
    where
        I: UnsynInput,
        V: ?Sized + Visit<'ast, I>,
    {
        v.visit_a(&node.a);
        v.visit_b(&node.b);
    }
    # [doc = concat ! ("Descend through the node of `" , stringify ! (Word) , "`.")]
    pub fn walk_word<'ast, I, V>(v: &mut V, node: &'ast Word<I>)
    where
        I: UnsynInput,
        V: ?Sized + Visit<'ast, I>,
    {
        v.visit_text(&node.0);
    }
    # [doc = concat ! ("Descend through the node of `" , stringify ! (Run) , "`.")]
    pub fn walk_run<'ast, I, V>(v: &mut V, node: &'ast Run<I>)
    where
        I: UnsynInput,
        V: ?Sized + Visit<'ast, I>,
    {
        v.visit_text(&node.f1);
        for inner in &node.inner {
            v.visit_inner(inner);
        }
    }
}
