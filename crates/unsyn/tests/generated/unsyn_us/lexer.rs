//! The terminals of `unsyn.us`: its `lexer` rules, and the `whitespace` rule
//! beside them, since a separator is a terminal like any other. Each is one run
//! of text, and the reading a pattern asked for stands beside the rule that
//! asked for it.
//!
//! This file is generated: see `tests/unsyn_us.rs`. Do not edit it.
#![allow(
    dead_code,
    clippy::manual_is_ascii_check,
    clippy::needless_borrow,
    clippy::type_complexity,
    clippy::enum_variant_names
)]
use super::input::UnsynInput;
#[inline]
fn is_s(c: char) -> bool {
    matches ! (c , '\t' ..= '\r' | ' ' | '\u{85}' | '\u{a0}' | '\u{1680}' | '\u{2000}' ..= '\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}')
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
# [parserc (take_while_with = (1 .. , is_s))]
pub struct S<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(next = '\n')]
pub struct LF<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(next = '\r')]
pub struct CR<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(crate::scan_ident_or_keyword)]
pub struct IdentOrKeyword<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct RawIdent<I>
where
    I: UnsynInput,
{
    #[parserc(keyword = "r#")]
    pub f1: I,
    pub non_keyword_ident: NonKeywordIdent<I>,
}
fn read_non_keyword_ident<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
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
                return Err(::unsyn::errors::UnsynError::Syntax(
                    ::unsyn::errors::SyntaxKind::NonKeywordIdent,
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
#[parserc(read_non_keyword_ident)]
pub struct NonKeywordIdent<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum Ident<I>
where
    I: UnsynInput,
{
    RawIdent(RawIdent<I>),
    NonKeywordIdent(NonKeywordIdent<I>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum Keywords<I>
where
    I: UnsynInput,
{
    LeadingKeywords(LeadingKeywords<I>),
    OperatorKeywords(OperatorKeywords<I>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum LeadingKeywords<I>
where
    I: UnsynInput,
{
    Lexer(#[parserc(keyword = "lexer")] I),
    Syntax(#[parserc(keyword = "syntax")] I),
    Whitespace(#[parserc(keyword = "whitespace")] I),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum OperatorKeywords<I>
where
    I: UnsynInput,
{
    Concat(#[parserc(keyword = "concat")] I),
    Except(#[parserc(keyword = "except")] I),
}
fn read_lit_str_1<I>(input: &mut I) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
        while ::parserc::next_if(|c: char| !(matches!(c, '\'' | '\\' | '\r')))
            .or(<QuoteEscape<I> as ::parserc::syntax::Syntax<I>>::into_parser().map(|node| node.0))
            .or(::parserc::text_of(
                <AsciiEscape<I> as ::parserc::syntax::Syntax<I>>::into_parser(),
            ))
            .or(::parserc::text_of(
                <UnicodeEscape<I> as ::parserc::syntax::Syntax<I>>::into_parser(),
            ))
            .ok()
            .parse(input)?
            .is_some()
        {}
        Ok(())
    })
    .parse(input)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct LitStr<I>(
    #[parserc(next = '\'')] pub I,
    #[parserc(read_lit_str_1)] pub I,
    #[parserc(next = '\'')] pub I,
)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(keyword = "\\'")]
pub struct QuoteEscape<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum AsciiEscape<I>
where
    I: UnsynInput,
{
    AsciiHexEscape(AsciiHexEscape<I>),
    AsciiNewline(AsciiNewline<I>),
    AsciiCarriageReturn(AsciiCarriageReturn<I>),
    AsciiTab(AsciiTab<I>),
    AsciiBackslash(AsciiBackslash<I>),
    AsciiNull(AsciiNull<I>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(keyword = "\\n")]
pub struct AsciiNewline<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(keyword = "\\r")]
pub struct AsciiCarriageReturn<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(keyword = "\\t")]
pub struct AsciiTab<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(keyword = "\\\\")]
pub struct AsciiBackslash<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(keyword = "\\0")]
pub struct AsciiNull<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct AsciiHexEscape<I>
where
    I: UnsynInput,
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
    I: UnsynInput,
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
        return Err(::unsyn::errors::UnsynError::Syntax(
            ::unsyn::errors::SyntaxKind::UnicodeEscape,
            ::parserc::ControlFlow::Recovable,
            input.to_span(),
        ));
    }
    Ok(taken)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct UnicodeEscape<I>
where
    I: UnsynInput,
{
    #[parserc(keyword = "\\u{")]
    pub f1: I,
    #[parserc(read_unicode_escape_1)]
    pub hex_digit: ::std::vec::Vec<HexDigit<I>>,
    #[parserc(next = '}')]
    pub f2: I,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
# [parserc (next_if = | c : char | matches ! (c , '0' ..= '7'))]
pub struct OctDigit<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
# [parserc (next_if = | c : char | matches ! (c , '0' ..= '9'))]
pub struct DecDigit<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
# [parserc (next_if = | c : char | matches ! (c , '0' ..= '9' | 'a' ..= 'f' | 'A' ..= 'F'))]
pub struct HexDigit<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct LitDec<I>
where
    I: UnsynInput,
{
    pub dec_digit: DecDigit<I>,
    # [parserc (take_while = | c : char | matches ! (c , '0' ..= '9' | '_'))]
    pub f1: I,
}
fn read_lit_unicode_1<I>(
    input: &mut I,
) -> ::core::result::Result<::std::vec::Vec<HexDigit<I>>, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
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
        return Err(::unsyn::errors::UnsynError::Syntax(
            ::unsyn::errors::SyntaxKind::LitUnicode,
            ::parserc::ControlFlow::Recovable,
            input.to_span(),
        ));
    }
    Ok(taken)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct LitUnicode<I>
where
    I: UnsynInput,
{
    #[parserc(keyword = "U+")]
    pub f1: I,
    #[parserc(read_lit_unicode_1)]
    pub hex_digit: ::std::vec::Vec<HexDigit<I>>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum BlockComment<I>
where
    I: UnsynInput,
{
    BlockCommentBody(BlockCommentBody<I>),
    EmptyBlockComment(EmptyBlockComment<I>),
    StarredEmptyBlockComment(StarredEmptyBlockComment<I>),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(keyword = "/**/")]
pub struct EmptyBlockComment<I>(pub I)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
#[parserc(keyword = "/***/")]
pub struct StarredEmptyBlockComment<I>(pub I)
where
    I: UnsynInput;
fn read_block_comment_body_1<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
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
        Ok(())
    })
    .parse(input)
}
fn read_block_comment_body_2<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
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
        Ok(())
    })
    .parse(input)
}
fn read_block_comment_body_3<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
        while ::parserc::text_of(
            <BlockCommentOrDoc<I> as ::parserc::syntax::Syntax<I>>::into_parser(),
        )
        .or(read_block_comment_body_2)
        .ok()
        .parse(input)?
        .is_some()
        {}
        Ok(())
    })
    .parse(input)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct BlockCommentBody<I>(
    #[parserc(keyword = "/*")] pub I,
    # [parserc (parser = read_block_comment_body_1 . or (:: parserc :: keyword ("**")) . or (:: parserc :: text_of (< BlockCommentOrDoc < I > as :: parserc :: syntax :: Syntax < I >> :: into_parser ())))]
    pub I,
    #[parserc(read_block_comment_body_3)] pub I,
    #[parserc(keyword = "*/")] pub I,
)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct InnerLineDoc<I>(
    #[parserc(keyword = "//!")]
    pub I,
    # [parserc (take_while = | c : char | ! (matches ! (c , '\n' | '\r')))] pub I,
)
where
    I: UnsynInput;
fn read_inner_block_doc_1<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
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
        Ok(())
    })
    .parse(input)
}
fn read_inner_block_doc_2<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
        while ::parserc::text_of(
            <BlockCommentOrDoc<I> as ::parserc::syntax::Syntax<I>>::into_parser(),
        )
        .or(read_inner_block_doc_1)
        .ok()
        .parse(input)?
        .is_some()
        {}
        Ok(())
    })
    .parse(input)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct InnerBlockDoc<I>(
    #[parserc(keyword = "/*!")] pub I,
    #[parserc(read_inner_block_doc_2)] pub I,
    #[parserc(keyword = "*/")] pub I,
)
where
    I: UnsynInput;
fn read_outer_line_doc_1<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
        ::parserc::next_if(|c: char| !(matches!(c, '/'))).parse(input)?;
        ::parserc::take_while(|c: char| !(matches!(c, '\n' | '\r'))).parse(input)?;
        Ok(())
    })
    .parse(input)
}
fn read_outer_line_doc_2<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
        let _ = read_outer_line_doc_1.ok().parse(input)?;
        Ok(())
    })
    .parse(input)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct OuterLineDoc<I>(
    #[parserc(keyword = "///")]
    pub I,
    #[parserc(read_outer_line_doc_2)] pub I,
)
where
    I: UnsynInput;
fn read_outer_block_doc_1<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
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
        Ok(())
    })
    .parse(input)
}
fn read_outer_block_doc_2<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
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
        Ok(())
    })
    .parse(input)
}
fn read_outer_block_doc_3<I>(
    input: &mut I,
) -> ::core::result::Result<I, <I as ::parserc::Input>::Error>
where
    I: UnsynInput,
{
    use ::parserc::Parser as _;
    ::parserc::text_of(|input: &mut I| {
        while ::parserc::text_of(
            <BlockCommentOrDoc<I> as ::parserc::syntax::Syntax<I>>::into_parser(),
        )
        .or(read_outer_block_doc_2)
        .ok()
        .parse(input)?
        .is_some()
        {}
        Ok(())
    })
    .parse(input)
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub struct OuterBlockDoc<I>(
    #[parserc(keyword = "/**")] pub I,
    # [parserc (parser = read_outer_block_doc_1 . or (:: parserc :: text_of (< BlockCommentOrDoc < I > as :: parserc :: syntax :: Syntax < I >> :: into_parser ())))]
    pub I,
    #[parserc(read_outer_block_doc_3)] pub I,
    #[parserc(keyword = "*/")] pub I,
)
where
    I: UnsynInput;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, :: parserc :: syntax :: Syntax)]
pub enum BlockCommentOrDoc<I>
where
    I: UnsynInput,
{
    BlockComment(BlockComment<I>),
    OuterBlockDoc(OuterBlockDoc<I>),
    InnerBlockDoc(InnerBlockDoc<I>),
}
