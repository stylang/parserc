//! The code of `unsyn.us`, as the generator writes it.
//!
//! This file is generated: see `tests/unsyn_us.rs`. Do not edit it.
#![allow(dead_code, clippy::manual_is_ascii_check)]
#[inline]
fn is_s(c: char) -> bool {
    matches ! (c , '\t' ..= '\r' | ' ' | '\u{85}' | '\u{a0}' | '\u{1680}' | '\u{2000}' ..= '\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
# [parserc (take_while = is_s)]
pub struct S<I>(pub I)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct LF<I>(# [parserc (parser = :: parserc :: next ('\n'))] pub I)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct CR<I>(# [parserc (parser = :: parserc :: next ('\r'))] pub I)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct IdentOrKeyword<I>(# [parserc (parser = crate :: scan_ident_or_keyword)] pub I)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct RawIdent<I>
where
    I: ::parserc::chars::CharsInput,
{
    # [parserc (parser = :: parserc :: keyword ("r#"))]
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
pub struct NonKeywordIdent<I>(# [parserc (parser = read_non_keyword_ident)] pub I)
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
pub struct LeadingKeywords<I>(
    # [parserc (parser = :: parserc :: keyword ("lexer") . or (:: parserc :: keyword ("syntax")) . or (:: parserc :: keyword ("whitespace")))]
    pub I,
)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct FollowingKeywords<I>(
    # [parserc (parser = :: parserc :: keyword ("concat") . or (:: parserc :: keyword ("except")) . or (:: parserc :: keyword ("followed")))]
    pub I,
)
where
    I: ::parserc::chars::CharsInput;
fn read_lit_str_1<I>(input: &mut I) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    while ::parserc::next_if(|c: char| !(matches!(c, '\'' | '\\' | '\r')))
        .or(<QuoteEscape<I> as ::parserc::syntax::Syntax<I>>::into_parser().map(|node| node.0))
        .or(<AsciiEscape<I> as ::parserc::syntax::Syntax<I>>::into_parser().map(|node| node.0))
        .or(<UnicodeEscape<I> as ::parserc::syntax::Syntax<I>>::into_parser().map(|node| node.0))
        .ok()
        .parse(input)?
        .is_some()
    {}
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct LitStr<I>(
    # [parserc (parser = :: parserc :: next ('\''))] pub I,
    # [parserc (parser = read_lit_str_1)] pub I,
    # [parserc (parser = :: parserc :: next ('\''))] pub I,
)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct QuoteEscape<I>(# [parserc (parser = :: parserc :: keyword ("\\'"))] pub I)
where
    I: ::parserc::chars::CharsInput;
fn read_ascii_escape_1<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    ::parserc::keyword("\\x").parse(input)?;
    <OctDigit<I> as ::parserc::syntax::Syntax<I>>::into_parser()
        .map(|node| node.0)
        .parse(input)?;
    <HexDigit<I> as ::parserc::syntax::Syntax<I>>::into_parser()
        .map(|node| node.0)
        .parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct AsciiEscape<I>(
    # [parserc (parser = read_ascii_escape_1 . or (:: parserc :: keyword ("\\n")) . or (:: parserc :: keyword ("\\r")) . or (:: parserc :: keyword ("\\t")) . or (:: parserc :: keyword ("\\\\")) . or (:: parserc :: keyword ("\\0")))]
    pub I,
)
where
    I: ::parserc::chars::CharsInput;
fn read_unicode_escape_1<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    <HexDigit<I> as ::parserc::syntax::Syntax<I>>::into_parser()
        .map(|node| node.0)
        .parse(input)?;
    ::parserc::take_while(|c: char| matches!(c, '_')).parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
fn read_unicode_escape<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    ::parserc::keyword("\\u{").parse(input)?;
    <HexDigit<I> as ::parserc::syntax::Syntax<I>>::into_parser()
        .map(|node| node.0)
        .parse(input)?;
    ::parserc::take_while(|c: char| matches!(c, '_')).parse(input)?;
    for _ in 1..6 {
        if read_unicode_escape_1.ok().parse(input)?.is_none() {
            break;
        }
    }
    ::parserc::next('}').parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct UnicodeEscape<I>(# [parserc (parser = read_unicode_escape)] pub I)
where
    I: ::parserc::chars::CharsInput;
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
    # [parserc (parser = :: parserc :: take_while (| c : char | matches ! (c , '0' ..= '9' | '_')))]
    pub f1: I,
}
fn read_lit_unicode<I>(input: &mut I) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    ::parserc::keyword("U+").parse(input)?;
    ::parserc::take_while_with(
        4..=4,
        |c: char| matches ! (c , '0' ..= '9' | 'a' ..= 'f' | 'A' ..= 'F'),
    )
    .parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct LitUnicode<I>(# [parserc (parser = read_lit_unicode)] pub I)
where
    I: ::parserc::chars::CharsInput;
fn read_block_comment_1<I>(
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
fn read_block_comment_2<I>(
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
fn read_block_comment_3<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    ::parserc::keyword("/*").parse(input)?;
    read_block_comment_1
        .or(::parserc::keyword("**"))
        .or(
            <BlockCommentOrDoc<I> as ::parserc::syntax::Syntax<I>>::into_parser()
                .map(|node| node.0),
        )
        .parse(input)?;
    while <BlockCommentOrDoc<I> as ::parserc::syntax::Syntax<I>>::into_parser()
        .map(|node| node.0)
        .or(read_block_comment_2)
        .ok()
        .parse(input)?
        .is_some()
    {}
    ::parserc::keyword("*/").parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct BlockComment<I>(
    # [parserc (parser = read_block_comment_3 . or (:: parserc :: keyword ("/**/")) . or (:: parserc :: keyword ("/***/")))]
    pub I,
)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct InnerLineDoc<I>(
    # [parserc (parser = :: parserc :: keyword ("//!"))]
    pub I,
    # [parserc (parser = :: parserc :: take_while (| c : char | ! (matches ! (c , '\n' | '\r'))))]
    pub I,
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
fn read_inner_block_doc<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    ::parserc::keyword("/*!").parse(input)?;
    while <BlockCommentOrDoc<I> as ::parserc::syntax::Syntax<I>>::into_parser()
        .map(|node| node.0)
        .or(read_inner_block_doc_1)
        .ok()
        .parse(input)?
        .is_some()
    {}
    ::parserc::keyword("*/").parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct InnerBlockDoc<I>(# [parserc (parser = read_inner_block_doc)] pub I)
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
    # [parserc (parser = :: parserc :: keyword ("///"))]
    pub I,
    # [parserc (parser = read_outer_line_doc_2)] pub I,
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
fn read_outer_block_doc<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: ::parserc::chars::CharsInput,
{
    use ::parserc::Parser as _;
    let mut whole = input.clone();
    let before = input.len();
    ::parserc::keyword("/**").parse(input)?;
    read_outer_block_doc_1
        .or(
            <BlockCommentOrDoc<I> as ::parserc::syntax::Syntax<I>>::into_parser()
                .map(|node| node.0),
        )
        .parse(input)?;
    while <BlockCommentOrDoc<I> as ::parserc::syntax::Syntax<I>>::into_parser()
        .map(|node| node.0)
        .or(read_outer_block_doc_2)
        .ok()
        .parse(input)?
        .is_some()
    {}
    ::parserc::keyword("*/").parse(input)?;
    Ok(whole.split_to(before - input.len()))
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct OuterBlockDoc<I>(# [parserc (parser = read_outer_block_doc)] pub I)
where
    I: ::parserc::chars::CharsInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct BlockCommentOrDoc<I>(
    # [parserc (parser = < BlockComment < I > as :: parserc :: syntax :: Syntax < I >> :: into_parser () . map (| node | node . 0) . or (< OuterBlockDoc < I > as :: parserc :: syntax :: Syntax < I >> :: into_parser () . map (| node | node . 0)) . or (< InnerBlockDoc < I > as :: parserc :: syntax :: Syntax < I >> :: into_parser () . map (| node | node . 0)))]
    pub I,
)
where
    I: ::parserc::chars::CharsInput;
