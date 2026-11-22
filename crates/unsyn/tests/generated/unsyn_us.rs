//! The code of `unsyn.us`, as the generator writes it.
//!
//! This file is generated: see `tests/unsyn_us.rs`. Do not edit it.
#![allow(
    dead_code,
    clippy::manual_is_ascii_check,
    clippy::needless_borrow,
    clippy::type_complexity,
    clippy::enum_variant_names
)]
fn text_of<P, I>(
    parser: P,
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
    P: ::parserc::Parser<I>,
{
    let mut whole = input.clone();
    let before = input.len();
    parser.parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
#[inline]
fn is_s(c: char) -> bool {
    matches ! (c , '\t' ..= '\r' | ' ' | '\u{85}' | '\u{a0}' | '\u{1680}' | '\u{2000}' ..= '\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
# [parserc (take_while_with = (1 .. , is_s))]
pub struct S<I>(pub I)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct LF<I>(#[parserc(next = '\n')] pub I)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct CR<I>(#[parserc(next = '\r')] pub I)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct IdentOrKeyword<I>(#[parserc(crate::scan_ident_or_keyword)] pub I)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct RawIdent<I>
where
    I: ::parserc::chars::CharsInput,
{
    #[parserc(keyword = "r#")]
    pub f1: I,
    pub ident_or_keyword: IdentOrKeyword<I>,
}
fn read_non_keyword_ident<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    let taken = <IdentOrKeyword<I> as ::parserc::syntax::Syntax<I>>::into_parser()
        .map(|node| node.0)
        .parse(input)?;
    {
        let mut rest = taken.clone();
        if <Keywords<I> as ::parserc::syntax::Syntax<I>>::into_parser()
            .ok()
            .parse(&mut rest)?
            .is_some()
            && rest.is_empty()
        {
            return Err(::parserc::Kind::Syntax(
                "NonKeywordIdent",
                ::parserc::ControlFlow::Fatal,
                taken.to_span(),
            )
            .into());
        }
    }
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct NonKeywordIdent<I>(#[parserc(read_non_keyword_ident)] pub I)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum Ident<I>
where
    I: ::parserc::chars::CharsInput,
{
    RawIdent(RawIdent<I>),
    NonKeywordIdent(NonKeywordIdent<I>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum Keywords<I>
where
    I: ::parserc::chars::CharsInput,
{
    LeadingKeywords(LeadingKeywords<I>),
    FollowingKeywords(FollowingKeywords<I>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum LeadingKeywords<I>
where
    I: ::parserc::chars::CharsInput,
{
    Lexer(#[parserc(keyword = "lexer")] I),
    Syntax(#[parserc(keyword = "syntax")] I),
    Whitespace(#[parserc(keyword = "whitespace")] I),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum FollowingKeywords<I>
where
    I: ::parserc::chars::CharsInput,
{
    Concat(#[parserc(keyword = "concat")] I),
    Except(#[parserc(keyword = "except")] I),
    Followed(#[parserc(keyword = "followed")] I),
}
fn read_lit_str_1<I>(input: &mut I) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    while ::parserc::next_if(|c: char| !(matches!(c, '\'' | '\\' | '\r')))
        .or(<QuoteEscape<I> as ::parserc::syntax::Syntax<I>>::into_parser().map(|node| node.0))
        .or(|input: &mut I| {
            text_of(
                <AsciiEscape<I> as ::parserc::syntax::Syntax<I>>::into_parser(),
                input,
            )
        })
        .or(|input: &mut I| {
            text_of(
                <UnicodeEscape<I> as ::parserc::syntax::Syntax<I>>::into_parser(),
                input,
            )
        })
        .ok()
        .parse(input)?
        .is_some()
    {}
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct LitStr<I>(
    #[parserc(next = '\'')] pub I,
    #[parserc(read_lit_str_1)] pub I,
    #[parserc(next = '\'')] pub I,
)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct QuoteEscape<I>(#[parserc(keyword = "\\'")] pub I)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum AsciiEscape<I>
where
    I: ::parserc::chars::CharsInput,
{
    AsciiHexEscape(AsciiHexEscape<I>),
    F2(#[parserc(keyword = "\\n")] I),
    F3(#[parserc(keyword = "\\r")] I),
    F4(#[parserc(keyword = "\\t")] I),
    F5(#[parserc(keyword = "\\\\")] I),
    F6(#[parserc(keyword = "\\0")] I),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct AsciiHexEscape<I>
where
    I: ::parserc::chars::CharsInput,
{
    #[parserc(keyword = "\\x")]
    pub f1: I,
    pub oct_digit: OctDigit<I>,
    pub hex_digit: HexDigit<I>,
}
fn read_unicode_escape_1<I>(
    input: &mut I,
) -> ::core::result::Result<::std::vec::Vec<HexDigit<I>>, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut taken = ::std::vec::Vec::new();
    while taken.len() < 6usize {
        let Some(one) = <HexDigit<I> as ::parserc::syntax::Syntax<I>>::into_parser()
            .ok()
            .parse(input)?
        else {
            break;
        };
        taken.push(one);
    }
    if taken.is_empty() {
        return Err(::parserc::Kind::Syntax(
            "UnicodeEscape",
            ::parserc::ControlFlow::Recovable,
            input.to_span(),
        )
        .into());
    }
    Ok(taken)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct UnicodeEscape<I>
where
    I: ::parserc::chars::CharsInput,
{
    #[parserc(keyword = "\\u{")]
    pub f1: I,
    #[parserc(read_unicode_escape_1)]
    pub hex_digit: ::std::vec::Vec<HexDigit<I>>,
    #[parserc(next = '}')]
    pub f2: I,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct OctDigit<I>(
    # [parserc (parser = :: parserc :: next_if (| c : char | matches ! (c , '0' ..= '7')))] pub I,
)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct DecDigit<I>(
    # [parserc (parser = :: parserc :: next_if (| c : char | matches ! (c , '0' ..= '9')))] pub I,
)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct HexDigit<I>(
    # [parserc (parser = :: parserc :: next_if (| c : char | matches ! (c , '0' ..= '9' | 'a' ..= 'f' | 'A' ..= 'F')))]
    pub I,
)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct LitDec<I>
where
    I: ::parserc::chars::CharsInput,
{
    pub dec_digit: DecDigit<I>,
    # [parserc (take_while = | c : char | matches ! (c , '0' ..= '9' | '_'))]
    pub f1: I,
}
fn read_lit_unicode_1<I>(
    input: &mut I,
) -> ::core::result::Result<::std::vec::Vec<HexDigit<I>>, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut taken = ::std::vec::Vec::new();
    while taken.len() < 4usize {
        let Some(one) = <HexDigit<I> as ::parserc::syntax::Syntax<I>>::into_parser()
            .ok()
            .parse(input)?
        else {
            break;
        };
        taken.push(one);
    }
    if taken.len() < 4usize {
        return Err(::parserc::Kind::Syntax(
            "LitUnicode",
            ::parserc::ControlFlow::Recovable,
            input.to_span(),
        )
        .into());
    }
    Ok(taken)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct LitUnicode<I>
where
    I: ::parserc::chars::CharsInput,
{
    #[parserc(keyword = "U+")]
    pub f1: I,
    #[parserc(read_lit_unicode_1)]
    pub hex_digit: ::std::vec::Vec<HexDigit<I>>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum BlockComment<I>
where
    I: ::parserc::chars::CharsInput,
{
    BlockCommentBody(BlockCommentBody<I>),
    F2(#[parserc(keyword = "/**/")] I),
    F3(#[parserc(keyword = "/***/")] I),
}
fn read_block_comment_body_1<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    {
        let mut rest = input.clone();
        if ::parserc::next('*')
            .or(::parserc::next('!'))
            .or(::parserc::keyword("/*"))
            .ok()
            .parse(&mut rest)?
            .is_some()
        {
            return Err(::parserc::Kind::NextIf(
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into());
        }
    }
    ::parserc::next_if(|_c: char| true).parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
fn read_block_comment_body_2<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    {
        let mut rest = input.clone();
        if ::parserc::keyword("/*")
            .or(::parserc::keyword("*/"))
            .ok()
            .parse(&mut rest)?
            .is_some()
        {
            return Err(::parserc::Kind::NextIf(
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into());
        }
    }
    ::parserc::next_if(|_c: char| true).parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
fn read_block_comment_body_3<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    while (|input: &mut I| {
        text_of(
            <BlockCommentOrDoc<I> as ::parserc::syntax::Syntax<I>>::into_parser(),
            input,
        )
    })
    .or(read_block_comment_body_2)
    .ok()
    .parse(input)?
    .is_some()
    {}
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct BlockCommentBody<I>(
    #[parserc(keyword = "/*")] pub I,
    # [parserc (parser = read_block_comment_body_1 . or (:: parserc :: keyword ("**")) . or (| input : & mut I | { text_of (< BlockCommentOrDoc < I > as :: parserc :: syntax :: Syntax < I >> :: into_parser () , input) }))]
    pub I,
    #[parserc(read_block_comment_body_3)] pub I,
    #[parserc(keyword = "*/")] pub I,
)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct InnerLineDoc<I>(
    #[parserc(keyword = "//!")]
    pub I,
    # [parserc (take_while = | c : char | ! (matches ! (c , '\n' | '\r')))] pub I,
)
where
    I: ::parserc::chars::CharsInput;
fn read_inner_block_doc_1<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    {
        let mut rest = input.clone();
        if ::parserc::keyword("/*")
            .or(::parserc::keyword("*/"))
            .ok()
            .parse(&mut rest)?
            .is_some()
        {
            return Err(::parserc::Kind::NextIf(
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into());
        }
    }
    ::parserc::next_if(|_c: char| true).parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
fn read_inner_block_doc_2<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    while (|input: &mut I| {
        text_of(
            <BlockCommentOrDoc<I> as ::parserc::syntax::Syntax<I>>::into_parser(),
            input,
        )
    })
    .or(read_inner_block_doc_1)
    .ok()
    .parse(input)?
    .is_some()
    {}
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct InnerBlockDoc<I>(
    #[parserc(keyword = "/*!")] pub I,
    #[parserc(read_inner_block_doc_2)] pub I,
    #[parserc(keyword = "*/")] pub I,
)
where
    I: ::parserc::chars::CharsInput;
fn read_outer_line_doc_1<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    ::parserc::next_if(|c: char| !(matches!(c, '/'))).parse(input)?;
    ::parserc::take_while(|c: char| !(matches!(c, '\n' | '\r'))).parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
fn read_outer_line_doc_2<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    let _ = read_outer_line_doc_1.ok().parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct OuterLineDoc<I>(
    #[parserc(keyword = "///")]
    pub I,
    #[parserc(read_outer_line_doc_2)] pub I,
)
where
    I: ::parserc::chars::CharsInput;
fn read_outer_block_doc_1<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    {
        let mut rest = input.clone();
        if ::parserc::next('*')
            .or(::parserc::keyword("/*"))
            .ok()
            .parse(&mut rest)?
            .is_some()
        {
            return Err(::parserc::Kind::NextIf(
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into());
        }
    }
    ::parserc::next_if(|_c: char| true).parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
fn read_outer_block_doc_2<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    {
        let mut rest = input.clone();
        if ::parserc::keyword("/*")
            .or(::parserc::keyword("*/"))
            .ok()
            .parse(&mut rest)?
            .is_some()
        {
            return Err(::parserc::Kind::NextIf(
                ::parserc::ControlFlow::Recovable,
                input.to_span_at(1),
            )
            .into());
        }
    }
    ::parserc::next_if(|_c: char| true).parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
fn read_outer_block_doc_3<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    while (|input: &mut I| {
        text_of(
            <BlockCommentOrDoc<I> as ::parserc::syntax::Syntax<I>>::into_parser(),
            input,
        )
    })
    .or(read_outer_block_doc_2)
    .ok()
    .parse(input)?
    .is_some()
    {}
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct OuterBlockDoc<I>(
    #[parserc(keyword = "/**")] pub I,
    # [parserc (parser = read_outer_block_doc_1 . or (| input : & mut I | { text_of (< BlockCommentOrDoc < I > as :: parserc :: syntax :: Syntax < I >> :: into_parser () , input) }))]
    pub I,
    #[parserc(read_outer_block_doc_3)] pub I,
    #[parserc(keyword = "*/")] pub I,
)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum BlockCommentOrDoc<I>
where
    I: ::parserc::chars::CharsInput,
{
    BlockComment(BlockComment<I>),
    OuterBlockDoc(OuterBlockDoc<I>),
    InnerBlockDoc(InnerBlockDoc<I>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct Expr<I>(
    pub  (
        Box<ExprNoTopAlt<I>>,
        ::std::vec::Vec<(::core::option::Option<S<I>>, Box<ExprNoTopAlt<I>>)>,
    ),
    pub  ::std::vec::Vec<(
        ::core::option::Option<S<I>>,
        ::parserc::syntax::Char<I, '|'>,
        ::core::option::Option<S<I>>,
        (
            Box<ExprNoTopAlt<I>>,
            ::std::vec::Vec<(::core::option::Option<S<I>>, Box<ExprNoTopAlt<I>>)>,
        ),
    )>,
)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum ExprNoTopAlt<I>
where
    I: ::parserc::chars::CharsInput,
{
    ExprWithSuffix(Box<ExprWithSuffix<I>>),
    ExprWithoutSuffix(Box<ExprWithoutSuffix<I>>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct Set<I>(
    pub ::parserc::syntax::Char<I, '['>,
    pub  ::std::vec::Vec<(
        ::core::option::Option<S<I>>,
        Box<SetItem<I>>,
        ::core::option::Option<S<I>>,
        ::parserc::syntax::Char<I, ','>,
    )>,
    pub ::core::option::Option<(::core::option::Option<S<I>>, Box<SetItem<I>>)>,
    pub ::core::option::Option<S<I>>,
    pub ::parserc::syntax::Char<I, ']'>,
)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum ExprWithoutSuffix<I>
where
    I: ::parserc::chars::CharsInput,
{
    NegatedSet(Box<NegatedSet<I>>),
    Call(Box<Call<I>>),
    Paren(Box<Paren<I>>),
    Set(Box<Set<I>>),
    LitStr(LitStr<I>),
    LitUnicode(LitUnicode<I>),
    Ident(Ident<I>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct NegatedSet<I>
where
    I: ::parserc::chars::CharsInput,
{
    pub f1: ::parserc::syntax::Char<I, '~'>,
    pub f2: ::core::option::Option<S<I>>,
    pub set: Box<Set<I>>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct Call<I>
where
    I: ::parserc::chars::CharsInput,
{
    pub f1: ::parserc::syntax::Char<I, '<'>,
    pub f2: ::core::option::Option<S<I>>,
    pub ident: Ident<I>,
    pub f3: ::core::option::Option<S<I>>,
    pub f4: ::parserc::syntax::Char<I, '>'>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct Paren<I>
where
    I: ::parserc::chars::CharsInput,
{
    pub f1: ::parserc::syntax::Char<I, '('>,
    pub f2: ::core::option::Option<S<I>>,
    pub expr: Box<Expr<I>>,
    pub f3: ::core::option::Option<S<I>>,
    pub f4: ::parserc::syntax::Char<I, ')'>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum Suffix<I>
where
    I: ::parserc::chars::CharsInput,
{
    F1(::parserc::syntax::Char<I, '*'>),
    F2(::parserc::syntax::Char<I, '?'>),
    F3(::parserc::syntax::Char<I, '+'>),
    Repeat(Box<Repeat<I>>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum ExprWithSuffix<I>
where
    I: ::parserc::chars::CharsInput,
{
    Suffixed(Box<Suffixed<I>>),
    Concat {
        expr_without_suffix: Box<ExprWithoutSuffix<I>>,
        f1: ::core::option::Option<S<I>>,
        #[parserc(keyword = "concat")]
        f2: I,
        f3: ::core::option::Option<S<I>>,
        expr_no_top_alt: Box<ExprNoTopAlt<I>>,
    },
    Followed {
        expr_without_suffix: Box<ExprWithoutSuffix<I>>,
        f1: ::core::option::Option<S<I>>,
        #[parserc(keyword = "followed")]
        f2: I,
        f3: ::core::option::Option<S<I>>,
        expr_no_top_alt: Box<ExprNoTopAlt<I>>,
    },
    Except {
        expr_without_suffix: Box<ExprWithoutSuffix<I>>,
        f1: ::core::option::Option<S<I>>,
        #[parserc(keyword = "except")]
        f2: I,
        f3: ::core::option::Option<S<I>>,
        f4: Box<ExprWithoutSuffix<I>>,
    },
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct Suffixed<I>
where
    I: ::parserc::chars::CharsInput,
{
    pub expr_without_suffix: Box<ExprWithoutSuffix<I>>,
    pub f1: ::core::option::Option<S<I>>,
    pub suffix: Box<Suffix<I>>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum RepeatBound<I>
where
    I: ::parserc::chars::CharsInput,
{
    RangeTo(Box<RangeTo<I>>),
    RangeFrom(Box<RangeFrom<I>>),
    LitDec(LitDec<I>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct RangeTo<I>
where
    I: ::parserc::chars::CharsInput,
{
    pub f1: ::parserc::syntax::Char<I, ','>,
    pub f2: ::core::option::Option<S<I>>,
    pub lit_dec: LitDec<I>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct RangeFrom<I>
where
    I: ::parserc::chars::CharsInput,
{
    pub lit_dec: LitDec<I>,
    pub f1: ::core::option::Option<S<I>>,
    pub f2: ::parserc::syntax::Char<I, ','>,
    pub f3: ::core::option::Option<(::core::option::Option<S<I>>, LitDec<I>)>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct Repeat<I>
where
    I: ::parserc::chars::CharsInput,
{
    pub f1: ::parserc::syntax::Char<I, '{'>,
    pub f2: ::core::option::Option<S<I>>,
    pub repeat_bound: Box<RepeatBound<I>>,
    pub f3: ::core::option::Option<S<I>>,
    pub f4: ::parserc::syntax::Char<I, '}'>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum Range<I>
where
    I: ::parserc::chars::CharsInput,
{
    UnicodeRange(Box<UnicodeRange<I>>),
    StrRange(Box<StrRange<I>>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct UnicodeRange<I>
where
    I: ::parserc::chars::CharsInput,
{
    pub lit_unicode: LitUnicode<I>,
    pub f1: ::parserc::syntax::Char<I, '-'>,
    pub f2: LitUnicode<I>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct StrRange<I>
where
    I: ::parserc::chars::CharsInput,
{
    pub lit_str: LitStr<I>,
    pub f1: ::parserc::syntax::Char<I, '-'>,
    pub f2: LitStr<I>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum SetItem<I>
where
    I: ::parserc::chars::CharsInput,
{
    Range(Box<Range<I>>),
    LitStr(LitStr<I>),
    LitUnicode(LitUnicode<I>),
    Ident(Ident<I>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct Stmt<I>
where
    I: ::parserc::chars::CharsInput,
{
    pub leading_keywords: LeadingKeywords<I>,
    pub f1: ::core::option::Option<S<I>>,
    pub ident: Ident<I>,
    pub f2: ::core::option::Option<S<I>>,
    #[parserc(keyword = "->")]
    pub f3: I,
    pub f4: ::core::option::Option<S<I>>,
    pub expr: Box<Expr<I>>,
    pub f5: ::core::option::Option<S<I>>,
    pub f6: ::parserc::syntax::Char<I, ';'>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum Item<I>
where
    I: ::parserc::chars::CharsInput,
{
    OuterDoc(Box<OuterDoc<I>>),
    S(S<I>),
    Stmt(Box<Stmt<I>>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum OuterDoc<I>
where
    I: ::parserc::chars::CharsInput,
{
    OuterBlockDoc(OuterBlockDoc<I>),
    OuterLineDoc(OuterLineDoc<I>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum InnerDoc<I>
where
    I: ::parserc::chars::CharsInput,
{
    InnerBlockDoc(InnerBlockDoc<I>),
    InnerLineDoc(InnerLineDoc<I>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct Crate<I>(
    pub ::std::vec::Vec<(::core::option::Option<S<I>>, Box<InnerDoc<I>>)>,
    pub ::std::vec::Vec<Box<Item<I>>>,
)
where
    I: ::parserc::chars::CharsInput;
# [doc = r" The visitor of the file's nodes, in the shape of `syn`'s: one"] # [doc = r" method per rule descending through the walk of it unless a"] # [doc = r" visitor says otherwise, and one method for the text that every"] # [doc = r" walk ends with."] # [doc = r""] # [doc = r" [`syn`]: https://docs.rs/syn/latest/syn/visit/index.html"] # [rustfmt :: skip] pub trait Visit < 'ast , I > where I : :: parserc :: chars :: CharsInput , { # [doc = r" The text of one token, which is the leaf every descent"] # [doc = r" hands over."] fn visit_text (& mut self , _text : & 'ast I) { } # [doc = concat ! ("The node of the rule `" , stringify ! (S) , "`.")] fn visit_s (& mut self , node : & 'ast S < I >) { walk :: walk_s (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (LF) , "`.")] fn visit_lf (& mut self , node : & 'ast LF < I >) { walk :: walk_lf (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (CR) , "`.")] fn visit_cr (& mut self , node : & 'ast CR < I >) { walk :: walk_cr (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (IdentOrKeyword) , "`.")] fn visit_ident_or_keyword (& mut self , node : & 'ast IdentOrKeyword < I >) { walk :: walk_ident_or_keyword (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (RawIdent) , "`.")] fn visit_raw_ident (& mut self , node : & 'ast RawIdent < I >) { walk :: walk_raw_ident (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (NonKeywordIdent) , "`.")] fn visit_non_keyword_ident (& mut self , node : & 'ast NonKeywordIdent < I >) { walk :: walk_non_keyword_ident (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Ident) , "`.")] fn visit_ident (& mut self , node : & 'ast Ident < I >) { walk :: walk_ident (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Keywords) , "`.")] fn visit_keywords (& mut self , node : & 'ast Keywords < I >) { walk :: walk_keywords (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (LeadingKeywords) , "`.")] fn visit_leading_keywords (& mut self , node : & 'ast LeadingKeywords < I >) { walk :: walk_leading_keywords (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (FollowingKeywords) , "`.")] fn visit_following_keywords (& mut self , node : & 'ast FollowingKeywords < I >) { walk :: walk_following_keywords (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (LitStr) , "`.")] fn visit_lit_str (& mut self , node : & 'ast LitStr < I >) { walk :: walk_lit_str (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (QuoteEscape) , "`.")] fn visit_quote_escape (& mut self , node : & 'ast QuoteEscape < I >) { walk :: walk_quote_escape (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (AsciiEscape) , "`.")] fn visit_ascii_escape (& mut self , node : & 'ast AsciiEscape < I >) { walk :: walk_ascii_escape (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (AsciiHexEscape) , "`.")] fn visit_ascii_hex_escape (& mut self , node : & 'ast AsciiHexEscape < I >) { walk :: walk_ascii_hex_escape (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (UnicodeEscape) , "`.")] fn visit_unicode_escape (& mut self , node : & 'ast UnicodeEscape < I >) { walk :: walk_unicode_escape (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (OctDigit) , "`.")] fn visit_oct_digit (& mut self , node : & 'ast OctDigit < I >) { walk :: walk_oct_digit (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (DecDigit) , "`.")] fn visit_dec_digit (& mut self , node : & 'ast DecDigit < I >) { walk :: walk_dec_digit (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (HexDigit) , "`.")] fn visit_hex_digit (& mut self , node : & 'ast HexDigit < I >) { walk :: walk_hex_digit (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (LitDec) , "`.")] fn visit_lit_dec (& mut self , node : & 'ast LitDec < I >) { walk :: walk_lit_dec (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (LitUnicode) , "`.")] fn visit_lit_unicode (& mut self , node : & 'ast LitUnicode < I >) { walk :: walk_lit_unicode (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (BlockComment) , "`.")] fn visit_block_comment (& mut self , node : & 'ast BlockComment < I >) { walk :: walk_block_comment (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (BlockCommentBody) , "`.")] fn visit_block_comment_body (& mut self , node : & 'ast BlockCommentBody < I >) { walk :: walk_block_comment_body (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (InnerLineDoc) , "`.")] fn visit_inner_line_doc (& mut self , node : & 'ast InnerLineDoc < I >) { walk :: walk_inner_line_doc (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (InnerBlockDoc) , "`.")] fn visit_inner_block_doc (& mut self , node : & 'ast InnerBlockDoc < I >) { walk :: walk_inner_block_doc (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (OuterLineDoc) , "`.")] fn visit_outer_line_doc (& mut self , node : & 'ast OuterLineDoc < I >) { walk :: walk_outer_line_doc (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (OuterBlockDoc) , "`.")] fn visit_outer_block_doc (& mut self , node : & 'ast OuterBlockDoc < I >) { walk :: walk_outer_block_doc (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (BlockCommentOrDoc) , "`.")] fn visit_block_comment_or_doc (& mut self , node : & 'ast BlockCommentOrDoc < I >) { walk :: walk_block_comment_or_doc (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Expr) , "`.")] fn visit_expr (& mut self , node : & 'ast Expr < I >) { walk :: walk_expr (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (ExprNoTopAlt) , "`.")] fn visit_expr_no_top_alt (& mut self , node : & 'ast ExprNoTopAlt < I >) { walk :: walk_expr_no_top_alt (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Set) , "`.")] fn visit_set (& mut self , node : & 'ast Set < I >) { walk :: walk_set (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (ExprWithoutSuffix) , "`.")] fn visit_expr_without_suffix (& mut self , node : & 'ast ExprWithoutSuffix < I >) { walk :: walk_expr_without_suffix (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (NegatedSet) , "`.")] fn visit_negated_set (& mut self , node : & 'ast NegatedSet < I >) { walk :: walk_negated_set (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Call) , "`.")] fn visit_call (& mut self , node : & 'ast Call < I >) { walk :: walk_call (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Paren) , "`.")] fn visit_paren (& mut self , node : & 'ast Paren < I >) { walk :: walk_paren (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Suffix) , "`.")] fn visit_suffix (& mut self , node : & 'ast Suffix < I >) { walk :: walk_suffix (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (ExprWithSuffix) , "`.")] fn visit_expr_with_suffix (& mut self , node : & 'ast ExprWithSuffix < I >) { walk :: walk_expr_with_suffix (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Suffixed) , "`.")] fn visit_suffixed (& mut self , node : & 'ast Suffixed < I >) { walk :: walk_suffixed (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (RepeatBound) , "`.")] fn visit_repeat_bound (& mut self , node : & 'ast RepeatBound < I >) { walk :: walk_repeat_bound (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (RangeTo) , "`.")] fn visit_range_to (& mut self , node : & 'ast RangeTo < I >) { walk :: walk_range_to (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (RangeFrom) , "`.")] fn visit_range_from (& mut self , node : & 'ast RangeFrom < I >) { walk :: walk_range_from (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Repeat) , "`.")] fn visit_repeat (& mut self , node : & 'ast Repeat < I >) { walk :: walk_repeat (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Range) , "`.")] fn visit_range (& mut self , node : & 'ast Range < I >) { walk :: walk_range (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (UnicodeRange) , "`.")] fn visit_unicode_range (& mut self , node : & 'ast UnicodeRange < I >) { walk :: walk_unicode_range (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (StrRange) , "`.")] fn visit_str_range (& mut self , node : & 'ast StrRange < I >) { walk :: walk_str_range (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (SetItem) , "`.")] fn visit_set_item (& mut self , node : & 'ast SetItem < I >) { walk :: walk_set_item (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Stmt) , "`.")] fn visit_stmt (& mut self , node : & 'ast Stmt < I >) { walk :: walk_stmt (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Item) , "`.")] fn visit_item (& mut self , node : & 'ast Item < I >) { walk :: walk_item (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (OuterDoc) , "`.")] fn visit_outer_doc (& mut self , node : & 'ast OuterDoc < I >) { walk :: walk_outer_doc (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (InnerDoc) , "`.")] fn visit_inner_doc (& mut self , node : & 'ast InnerDoc < I >) { walk :: walk_inner_doc (self , node) } # [doc = concat ! ("The node of the rule `" , stringify ! (Crate) , "`.")] fn visit_crate (& mut self , node : & 'ast Crate < I >) { walk :: walk_crate (self , node) } }
# [doc = r" The descents of the visitor, one per rule, public so that a"] # [doc = r" method which does its own work can still hand the children to"] # [doc = r" the visitor below it."] # [doc = r""] # [doc = r" The module is machine-written; the skip keeps the formatter"] # [doc = r" from rewriting the file into tokens the generator would not"] # [doc = r" have written."] # [rustfmt :: skip] # [allow (clippy :: needless_borrow , clippy :: possible_missing_else)] pub mod walk { # [rustfmt :: skip] use super :: { Visit , S , LF , CR , IdentOrKeyword , RawIdent , NonKeywordIdent , Ident , Keywords , LeadingKeywords , FollowingKeywords , LitStr , QuoteEscape , AsciiEscape , AsciiHexEscape , UnicodeEscape , OctDigit , DecDigit , HexDigit , LitDec , LitUnicode , BlockComment , BlockCommentBody , InnerLineDoc , InnerBlockDoc , OuterLineDoc , OuterBlockDoc , BlockCommentOrDoc , Expr , ExprNoTopAlt , Set , ExprWithoutSuffix , NegatedSet , Call , Paren , Suffix , ExprWithSuffix , Suffixed , RepeatBound , RangeTo , RangeFrom , Repeat , Range , UnicodeRange , StrRange , SetItem , Stmt , Item , OuterDoc , InnerDoc , Crate } ; # [doc = concat ! ("Descend through the node of `" , stringify ! (S) , "`.")] pub fn walk_s < 'ast , I , V > (v : & mut V , node : & 'ast S < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (LF) , "`.")] pub fn walk_lf < 'ast , I , V > (v : & mut V , node : & 'ast LF < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (CR) , "`.")] pub fn walk_cr < 'ast , I , V > (v : & mut V , node : & 'ast CR < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (IdentOrKeyword) , "`.")] pub fn walk_ident_or_keyword < 'ast , I , V > (v : & mut V , node : & 'ast IdentOrKeyword < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (RawIdent) , "`.")] pub fn walk_raw_ident < 'ast , I , V > (v : & mut V , node : & 'ast RawIdent < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . f1) ; v . visit_ident_or_keyword (& node . ident_or_keyword) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (NonKeywordIdent) , "`.")] pub fn walk_non_keyword_ident < 'ast , I , V > (v : & mut V , node : & 'ast NonKeywordIdent < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (Ident) , "`.")] pub fn walk_ident < 'ast , I , V > (v : & mut V , node : & 'ast Ident < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { Ident :: RawIdent (part) => { v . visit_raw_ident (part) ; } Ident :: NonKeywordIdent (part) => { v . visit_non_keyword_ident (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (Keywords) , "`.")] pub fn walk_keywords < 'ast , I , V > (v : & mut V , node : & 'ast Keywords < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { Keywords :: LeadingKeywords (part) => { v . visit_leading_keywords (part) ; } Keywords :: FollowingKeywords (part) => { v . visit_following_keywords (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (LeadingKeywords) , "`.")] pub fn walk_leading_keywords < 'ast , I , V > (v : & mut V , node : & 'ast LeadingKeywords < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { LeadingKeywords :: Lexer (part) => { v . visit_text (part) ; } LeadingKeywords :: Syntax (part) => { v . visit_text (part) ; } LeadingKeywords :: Whitespace (part) => { v . visit_text (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (FollowingKeywords) , "`.")] pub fn walk_following_keywords < 'ast , I , V > (v : & mut V , node : & 'ast FollowingKeywords < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { FollowingKeywords :: Concat (part) => { v . visit_text (part) ; } FollowingKeywords :: Except (part) => { v . visit_text (part) ; } FollowingKeywords :: Followed (part) => { v . visit_text (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (LitStr) , "`.")] pub fn walk_lit_str < 'ast , I , V > (v : & mut V , node : & 'ast LitStr < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; v . visit_text (& node . 1) ; v . visit_text (& node . 2) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (QuoteEscape) , "`.")] pub fn walk_quote_escape < 'ast , I , V > (v : & mut V , node : & 'ast QuoteEscape < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (AsciiEscape) , "`.")] pub fn walk_ascii_escape < 'ast , I , V > (v : & mut V , node : & 'ast AsciiEscape < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { AsciiEscape :: AsciiHexEscape (part) => { v . visit_ascii_hex_escape (part) ; } AsciiEscape :: F2 (part) => { v . visit_text (part) ; } AsciiEscape :: F3 (part) => { v . visit_text (part) ; } AsciiEscape :: F4 (part) => { v . visit_text (part) ; } AsciiEscape :: F5 (part) => { v . visit_text (part) ; } AsciiEscape :: F6 (part) => { v . visit_text (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (AsciiHexEscape) , "`.")] pub fn walk_ascii_hex_escape < 'ast , I , V > (v : & mut V , node : & 'ast AsciiHexEscape < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . f1) ; v . visit_oct_digit (& node . oct_digit) ; v . visit_hex_digit (& node . hex_digit) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (UnicodeEscape) , "`.")] pub fn walk_unicode_escape < 'ast , I , V > (v : & mut V , node : & 'ast UnicodeEscape < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . f1) ; for inner in & node . hex_digit { v . visit_hex_digit (inner) ; } v . visit_text (& node . f2) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (OctDigit) , "`.")] pub fn walk_oct_digit < 'ast , I , V > (v : & mut V , node : & 'ast OctDigit < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (DecDigit) , "`.")] pub fn walk_dec_digit < 'ast , I , V > (v : & mut V , node : & 'ast DecDigit < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (HexDigit) , "`.")] pub fn walk_hex_digit < 'ast , I , V > (v : & mut V , node : & 'ast HexDigit < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (LitDec) , "`.")] pub fn walk_lit_dec < 'ast , I , V > (v : & mut V , node : & 'ast LitDec < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_dec_digit (& node . dec_digit) ; v . visit_text (& node . f1) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (LitUnicode) , "`.")] pub fn walk_lit_unicode < 'ast , I , V > (v : & mut V , node : & 'ast LitUnicode < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . f1) ; for inner in & node . hex_digit { v . visit_hex_digit (inner) ; } } # [doc = concat ! ("Descend through the node of `" , stringify ! (BlockComment) , "`.")] pub fn walk_block_comment < 'ast , I , V > (v : & mut V , node : & 'ast BlockComment < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { BlockComment :: BlockCommentBody (part) => { v . visit_block_comment_body (part) ; } BlockComment :: F2 (part) => { v . visit_text (part) ; } BlockComment :: F3 (part) => { v . visit_text (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (BlockCommentBody) , "`.")] pub fn walk_block_comment_body < 'ast , I , V > (v : & mut V , node : & 'ast BlockCommentBody < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; v . visit_text (& node . 1) ; v . visit_text (& node . 2) ; v . visit_text (& node . 3) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (InnerLineDoc) , "`.")] pub fn walk_inner_line_doc < 'ast , I , V > (v : & mut V , node : & 'ast InnerLineDoc < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; v . visit_text (& node . 1) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (InnerBlockDoc) , "`.")] pub fn walk_inner_block_doc < 'ast , I , V > (v : & mut V , node : & 'ast InnerBlockDoc < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; v . visit_text (& node . 1) ; v . visit_text (& node . 2) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (OuterLineDoc) , "`.")] pub fn walk_outer_line_doc < 'ast , I , V > (v : & mut V , node : & 'ast OuterLineDoc < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; v . visit_text (& node . 1) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (OuterBlockDoc) , "`.")] pub fn walk_outer_block_doc < 'ast , I , V > (v : & mut V , node : & 'ast OuterBlockDoc < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& node . 0) ; v . visit_text (& node . 1) ; v . visit_text (& node . 2) ; v . visit_text (& node . 3) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (BlockCommentOrDoc) , "`.")] pub fn walk_block_comment_or_doc < 'ast , I , V > (v : & mut V , node : & 'ast BlockCommentOrDoc < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { BlockCommentOrDoc :: BlockComment (part) => { v . visit_block_comment (part) ; } BlockCommentOrDoc :: OuterBlockDoc (part) => { v . visit_outer_block_doc (part) ; } BlockCommentOrDoc :: InnerBlockDoc (part) => { v . visit_inner_block_doc (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (Expr) , "`.")] pub fn walk_expr < 'ast , I , V > (v : & mut V , node : & 'ast Expr < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_expr_no_top_alt (& (& node . 0) . 0) ; for inner in & (& node . 0) . 1 { if let Some (inner) = & (inner) . 0 { v . visit_s (inner) ; } v . visit_expr_no_top_alt (& (inner) . 1) ; } for inner in & node . 1 { if let Some (inner) = & (inner) . 0 { v . visit_s (inner) ; } v . visit_text (& (& (inner) . 1) . 0) ; if let Some (inner) = & (inner) . 2 { v . visit_s (inner) ; } v . visit_expr_no_top_alt (& (& (inner) . 3) . 0) ; for inner in & (& (inner) . 3) . 1 { if let Some (inner) = & (inner) . 0 { v . visit_s (inner) ; } v . visit_expr_no_top_alt (& (inner) . 1) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (ExprNoTopAlt) , "`.")] pub fn walk_expr_no_top_alt < 'ast , I , V > (v : & mut V , node : & 'ast ExprNoTopAlt < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { ExprNoTopAlt :: ExprWithSuffix (part) => { v . visit_expr_with_suffix (part) ; } ExprNoTopAlt :: ExprWithoutSuffix (part) => { v . visit_expr_without_suffix (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (Set) , "`.")] pub fn walk_set < 'ast , I , V > (v : & mut V , node : & 'ast Set < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& (& node . 0) . 0) ; for inner in & node . 1 { if let Some (inner) = & (inner) . 0 { v . visit_s (inner) ; } v . visit_set_item (& (inner) . 1) ; if let Some (inner) = & (inner) . 2 { v . visit_s (inner) ; } v . visit_text (& (& (inner) . 3) . 0) ; } if let Some (inner) = & node . 2 { if let Some (inner) = & (inner) . 0 { v . visit_s (inner) ; } v . visit_set_item (& (inner) . 1) ; } if let Some (inner) = & node . 3 { v . visit_s (inner) ; } v . visit_text (& (& node . 4) . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (ExprWithoutSuffix) , "`.")] pub fn walk_expr_without_suffix < 'ast , I , V > (v : & mut V , node : & 'ast ExprWithoutSuffix < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { ExprWithoutSuffix :: NegatedSet (part) => { v . visit_negated_set (part) ; } ExprWithoutSuffix :: Call (part) => { v . visit_call (part) ; } ExprWithoutSuffix :: Paren (part) => { v . visit_paren (part) ; } ExprWithoutSuffix :: Set (part) => { v . visit_set (part) ; } ExprWithoutSuffix :: LitStr (part) => { v . visit_lit_str (part) ; } ExprWithoutSuffix :: LitUnicode (part) => { v . visit_lit_unicode (part) ; } ExprWithoutSuffix :: Ident (part) => { v . visit_ident (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (NegatedSet) , "`.")] pub fn walk_negated_set < 'ast , I , V > (v : & mut V , node : & 'ast NegatedSet < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& (& node . f1) . 0) ; if let Some (inner) = & node . f2 { v . visit_s (inner) ; } v . visit_set (& node . set) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (Call) , "`.")] pub fn walk_call < 'ast , I , V > (v : & mut V , node : & 'ast Call < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& (& node . f1) . 0) ; if let Some (inner) = & node . f2 { v . visit_s (inner) ; } v . visit_ident (& node . ident) ; if let Some (inner) = & node . f3 { v . visit_s (inner) ; } v . visit_text (& (& node . f4) . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (Paren) , "`.")] pub fn walk_paren < 'ast , I , V > (v : & mut V , node : & 'ast Paren < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& (& node . f1) . 0) ; if let Some (inner) = & node . f2 { v . visit_s (inner) ; } v . visit_expr (& node . expr) ; if let Some (inner) = & node . f3 { v . visit_s (inner) ; } v . visit_text (& (& node . f4) . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (Suffix) , "`.")] pub fn walk_suffix < 'ast , I , V > (v : & mut V , node : & 'ast Suffix < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { Suffix :: F1 (part) => { v . visit_text (& (part) . 0) ; } Suffix :: F2 (part) => { v . visit_text (& (part) . 0) ; } Suffix :: F3 (part) => { v . visit_text (& (part) . 0) ; } Suffix :: Repeat (part) => { v . visit_repeat (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (ExprWithSuffix) , "`.")] pub fn walk_expr_with_suffix < 'ast , I , V > (v : & mut V , node : & 'ast ExprWithSuffix < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { ExprWithSuffix :: Suffixed (part) => { v . visit_suffixed (part) ; } ExprWithSuffix :: Concat { expr_without_suffix , f1 , f2 , f3 , expr_no_top_alt , .. } => { v . visit_expr_without_suffix (expr_without_suffix) ; if let Some (inner) = f1 { v . visit_s (inner) ; } v . visit_text (f2) ; if let Some (inner) = f3 { v . visit_s (inner) ; } v . visit_expr_no_top_alt (expr_no_top_alt) ; } ExprWithSuffix :: Followed { expr_without_suffix , f1 , f2 , f3 , expr_no_top_alt , .. } => { v . visit_expr_without_suffix (expr_without_suffix) ; if let Some (inner) = f1 { v . visit_s (inner) ; } v . visit_text (f2) ; if let Some (inner) = f3 { v . visit_s (inner) ; } v . visit_expr_no_top_alt (expr_no_top_alt) ; } ExprWithSuffix :: Except { expr_without_suffix , f1 , f2 , f3 , f4 , .. } => { v . visit_expr_without_suffix (expr_without_suffix) ; if let Some (inner) = f1 { v . visit_s (inner) ; } v . visit_text (f2) ; if let Some (inner) = f3 { v . visit_s (inner) ; } v . visit_expr_without_suffix (f4) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (Suffixed) , "`.")] pub fn walk_suffixed < 'ast , I , V > (v : & mut V , node : & 'ast Suffixed < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_expr_without_suffix (& node . expr_without_suffix) ; if let Some (inner) = & node . f1 { v . visit_s (inner) ; } v . visit_suffix (& node . suffix) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (RepeatBound) , "`.")] pub fn walk_repeat_bound < 'ast , I , V > (v : & mut V , node : & 'ast RepeatBound < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { RepeatBound :: RangeTo (part) => { v . visit_range_to (part) ; } RepeatBound :: RangeFrom (part) => { v . visit_range_from (part) ; } RepeatBound :: LitDec (part) => { v . visit_lit_dec (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (RangeTo) , "`.")] pub fn walk_range_to < 'ast , I , V > (v : & mut V , node : & 'ast RangeTo < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& (& node . f1) . 0) ; if let Some (inner) = & node . f2 { v . visit_s (inner) ; } v . visit_lit_dec (& node . lit_dec) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (RangeFrom) , "`.")] pub fn walk_range_from < 'ast , I , V > (v : & mut V , node : & 'ast RangeFrom < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_lit_dec (& node . lit_dec) ; if let Some (inner) = & node . f1 { v . visit_s (inner) ; } v . visit_text (& (& node . f2) . 0) ; if let Some (inner) = & node . f3 { if let Some (inner) = & (inner) . 0 { v . visit_s (inner) ; } v . visit_lit_dec (& (inner) . 1) ; } } # [doc = concat ! ("Descend through the node of `" , stringify ! (Repeat) , "`.")] pub fn walk_repeat < 'ast , I , V > (v : & mut V , node : & 'ast Repeat < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_text (& (& node . f1) . 0) ; if let Some (inner) = & node . f2 { v . visit_s (inner) ; } v . visit_repeat_bound (& node . repeat_bound) ; if let Some (inner) = & node . f3 { v . visit_s (inner) ; } v . visit_text (& (& node . f4) . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (Range) , "`.")] pub fn walk_range < 'ast , I , V > (v : & mut V , node : & 'ast Range < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { Range :: UnicodeRange (part) => { v . visit_unicode_range (part) ; } Range :: StrRange (part) => { v . visit_str_range (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (UnicodeRange) , "`.")] pub fn walk_unicode_range < 'ast , I , V > (v : & mut V , node : & 'ast UnicodeRange < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_lit_unicode (& node . lit_unicode) ; v . visit_text (& (& node . f1) . 0) ; v . visit_lit_unicode (& node . f2) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (StrRange) , "`.")] pub fn walk_str_range < 'ast , I , V > (v : & mut V , node : & 'ast StrRange < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_lit_str (& node . lit_str) ; v . visit_text (& (& node . f1) . 0) ; v . visit_lit_str (& node . f2) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (SetItem) , "`.")] pub fn walk_set_item < 'ast , I , V > (v : & mut V , node : & 'ast SetItem < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { SetItem :: Range (part) => { v . visit_range (part) ; } SetItem :: LitStr (part) => { v . visit_lit_str (part) ; } SetItem :: LitUnicode (part) => { v . visit_lit_unicode (part) ; } SetItem :: Ident (part) => { v . visit_ident (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (Stmt) , "`.")] pub fn walk_stmt < 'ast , I , V > (v : & mut V , node : & 'ast Stmt < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { v . visit_leading_keywords (& node . leading_keywords) ; if let Some (inner) = & node . f1 { v . visit_s (inner) ; } v . visit_ident (& node . ident) ; if let Some (inner) = & node . f2 { v . visit_s (inner) ; } v . visit_text (& node . f3) ; if let Some (inner) = & node . f4 { v . visit_s (inner) ; } v . visit_expr (& node . expr) ; if let Some (inner) = & node . f5 { v . visit_s (inner) ; } v . visit_text (& (& node . f6) . 0) ; } # [doc = concat ! ("Descend through the node of `" , stringify ! (Item) , "`.")] pub fn walk_item < 'ast , I , V > (v : & mut V , node : & 'ast Item < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { Item :: OuterDoc (part) => { v . visit_outer_doc (part) ; } Item :: S (part) => { v . visit_s (part) ; } Item :: Stmt (part) => { v . visit_stmt (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (OuterDoc) , "`.")] pub fn walk_outer_doc < 'ast , I , V > (v : & mut V , node : & 'ast OuterDoc < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { OuterDoc :: OuterBlockDoc (part) => { v . visit_outer_block_doc (part) ; } OuterDoc :: OuterLineDoc (part) => { v . visit_outer_line_doc (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (InnerDoc) , "`.")] pub fn walk_inner_doc < 'ast , I , V > (v : & mut V , node : & 'ast InnerDoc < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { match node { InnerDoc :: InnerBlockDoc (part) => { v . visit_inner_block_doc (part) ; } InnerDoc :: InnerLineDoc (part) => { v . visit_inner_line_doc (part) ; } } } # [doc = concat ! ("Descend through the node of `" , stringify ! (Crate) , "`.")] pub fn walk_crate < 'ast , I , V > (v : & mut V , node : & 'ast Crate < I >) where I : :: parserc :: chars :: CharsInput , V : ? Sized + Visit < 'ast , I > , { for inner in & node . 0 { if let Some (inner) = & (inner) . 0 { v . visit_s (inner) ; } v . visit_inner_doc (& (inner) . 1) ; } for inner in & node . 1 { v . visit_item (inner) ; } } }
