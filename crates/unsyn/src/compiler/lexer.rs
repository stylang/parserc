//! The terminals of the DSL: the `whitespace` rule and the `lexer` rules that
//! `unsyn.us` beside this crate writes.
//!
//! One production, one type: every type here is named for the left side of its
//! production, and its right side is spelled out with the nodes and combinators
//! of [`parserc`], so that the declaration of a type is the declaration of the
//! rule. A sequence of the right side is a field apiece of the struct; a choice
//! that is the whole of the right side is an enum whose variants are the
//! alternatives.
//!
//! A choice written inside a group belongs to no left side, and the
//! [analyzer](super::semantic) says so of every file that writes one. Where the
//! choice is what a *pattern* has to answer — which of the quantity operators
//! stood on an operand, which of the three forms spelled a bound — the group is
//! a rule of the spec and a node of the tree, because a reader of a pattern is
//! matching on it either way. Where it is only how a token is spelled, the group
//! earns no name and no node either: the token is read as one run of text and
//! [`decode`] says what the run holds. What the spec answers from outside
//! itself — `<IdentOrKeyword>` and its like — is read whole for the same
//! reason.
//!
//! [`decode`]: super::decode
//!
//! # Where the trivia goes
//!
//! No node of this module keeps a run of the trivia. A `lexer` token is read
//! out of the text it is handed, and a run taken there would end up inside that
//! text. The trivia on each side of such a token is therefore the sequence's
//! own: the `syntax` rule that reaches the token holds the run in a field laid
//! beside it, which is why the terminals a `syntax` rule is written with — the
//! puncts and the fixed words — live in [`super::syntax`] instead. See the
//! [crate documentation](crate) for how a run is filed away exactly once.
//!
//! The one consequence worth stating is the order of the alternatives of
//! [`Ident`]. A semantics error is fatal, so once a rule has refused text for
//! meaning nothing no enclosing choice may try another way — and `NonKeywordIdent`
//! reads `r#foo` as the name `r`, which is a name and is not fatal; [`RawIdent`]
//! therefore comes first, and takes the sigil and the name whole. Its own
//! refusal is the same [`Keyword`](SemanticsKind::Keyword) one and fatal too,
//! which is what keeps `r#lexer` from falling through to be read as anything.

use parserc::syntax::Syntax;
use parserc::{ControlFlow, Input, Kind, Parser, ToSpan, keyword, next, next_if, take_while};
use unicode_ident::{is_xid_continue, is_xid_start};

use crate::compiler::errors::{KeywordKind, PunctKind, SemanticsKind, SyntaxKind, UnsynError};
use crate::compiler::input::UnsynInput;

// ---------------------------------------------------------------------------
// `whitespace S -> [ U+0009-U+000D, U+0020, U+0085, U+00A0, U+1680,`
// `                  U+2000-U+200A, U+2028, U+2029, U+202F, U+205F, U+3000 ];`
// ---------------------------------------------------------------------------

/// The trivia of the language: one run of the characters the `whitespace` rule
/// lists, taken as greedily as the text offers.
///
/// A `syntax` rule is separated by this node and never names it, so a run of
/// trivia is owned by exactly one of the two terminals it lies between.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(take_while_with = (1.., is_trivia))]
pub struct S<I>(pub I)
where
    I: UnsynInput;

/// Whether a character is one of the eleven runs the `whitespace` rule counts
/// out.
#[inline]
fn is_trivia(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    )
}

// ---------------------------------------------------------------------------
// `lexer LF -> U+000A;`
// `lexer CR -> U+000D;`
// ---------------------------------------------------------------------------

/// A line feed, the end of a line of text.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(next = '\u{000A}')]
pub struct LF<I>(pub I)
where
    I: UnsynInput;

/// A carriage return, which a line document has to end at as much as a line feed.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(next = '\u{000D}')]
pub struct CR<I>(pub I)
where
    I: UnsynInput;

// ---------------------------------------------------------------------------
// `lexer IdentOrKeyword -> <IdentOrKeyword>;`
// ---------------------------------------------------------------------------

/// A name before it is known whether it is a keyword.
///
/// `<IdentOrKeyword>` is a link the caller of a generated parser binds; the
/// hand tree binds it to the same scan the name has always been: a start —
/// [`is_xid_start`] or `_` — and then the run of characters a name continues
/// with, in one greedy piece. The rule was once written a character at a time,
/// over externals of the Unicode properties; the spec says the name is one
/// token and the scan reads it whole, which is what a name is worth: the word
/// it spells and nothing below it.
///
/// [`is_xid_start`]: unicode_ident::is_xid_start
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(read_ident_or_keyword)]
pub struct IdentOrKeyword<I>(pub I)
where
    I: UnsynInput;

/// The scan the external of [`IdentOrKeyword`] is bound to: the start of a
/// name, of either of its two shapes, and then the run that continues it.
fn read_ident_or_keyword<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    next_if(|c: char| is_xid_start(c) || c == '_')
        .parse(input)
        .map_err(SyntaxKind::Ident.map())?;

    take_while(is_xid_continue).parse(input)?;

    Ok(whole.split_to(before - input.len()))
}

// ---------------------------------------------------------------------------
// `lexer RawIdent -> 'r#' NonKeywordIdent;`
// `lexer NonKeywordIdent -> IdentOrKeyword except Keywords;`
// `lexer Ident -> RawIdent | NonKeywordIdent;` — the hand tree reads the raw
// form first, which is what the order of a choice decides and not what the
// spec means to say about either form.
// ---------------------------------------------------------------------------

/// A name written with the `r#` sigil.
///
/// The sigil borrows a word of **Rust**, not one of this language: what it
/// marks is still a name that is no word of the DSL, since the rule reaches
/// [`NonKeywordIdent`] and carries that rule's refusal with it. A name written
/// `r#lexer` is therefore no name at all — the sigil is not a way around the
/// words of the language, only around the reserved words of the language the
/// tree is written in.
///
/// The two sub-expressions of the rule are the two fields of the node: the
/// sigil, and behind it the name. The sigil is what tells the two forms of a
/// name apart, so what stands where it was written is not this name, and no
/// other reading of the rule begins there either.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RawIdent<I>
where
    I: UnsynInput,
{
    /// The `r#` that opens the name.
    #[parserc(keyword = "r#", map_err = SyntaxKind::Ident.map())]
    pub f1: I,
    /// The name the sigil marks, read by the rule that refuses a word of the
    /// language — which is what keeps `r#` for the words of Rust alone.
    pub non_keyword_ident: NonKeywordIdent<I>,
}

/// A name that is not a word of the language.
///
/// The name is one token of text, as the generated code reads it: what the
/// `except` asks is answered while the name is read and leaves nothing of its
/// own in the tree.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(read_non_keyword_ident)]
pub struct NonKeywordIdent<I>(pub I)
where
    I: UnsynInput;

/// Reads the `IdentOrKeyword except Keywords` of [`NonKeywordIdent`] as one
/// token of text.
///
/// `except` asks its question of the text the operand to its left read and of
/// nothing else: the removal runs over that text on its own, and only a removal
/// that takes all of it refuses the name. `lexerX` is therefore a name of its
/// own, although `lexer` fits the head of it.
fn read_non_keyword_ident<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let name = IdentOrKeyword::into_parser().parse(input)?;

    let mut rest = name.0.clone();

    if Keywords::into_parser().ok().parse(&mut rest)?.is_some() && rest.is_empty() {
        return Err(UnsynError::Semantics(
            SemanticsKind::Keyword,
            name.to_span(),
        ));
    }

    Ok(name.0)
}

/// A name of a node: either one that borrows a word of Rust under the `r#`
/// sigil, or one that is plain and no word of either language.
///
/// The spec writes the raw form second, where it has to stand first: a name is
/// read by the first alternative that fits, and `NonKeywordIdent` fits the `r` of
/// `r#foo` and leaves `#foo` behind, so the raw form would never be reached.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Ident<I>
where
    I: UnsynInput,
{
    /// A name written raw, as `r#where`.
    RawIdent(RawIdent<I>),
    /// A name that is no word of the language.
    NonKeywordIdent(NonKeywordIdent<I>),
}

// ---------------------------------------------------------------------------
// `lexer Keywords -> LeadingKeywords | OperatorKeywords;`
// `lexer LeadingKeywords -> 'lexer' | 'syntax' | 'whitespace';`
// `lexer OperatorKeywords -> 'concat' | 'except';`
// ---------------------------------------------------------------------------

/// A word of the language, of either of the two places one may stand.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Keywords<I>
where
    I: UnsynInput,
{
    /// A word that opens a definition.
    LeadingKeywords(LeadingKeywords<I>),
    /// A word that joins two operands of a pattern.
    OperatorKeywords(OperatorKeywords<I>),
}

/// The word a definition opens with.
///
/// Which of the three it was is the only thing that tells a `whitespace`
/// declaration from a `lexer` one and from a `syntax` one, and the enum says it
/// as a variant rather than as text to be read: the words are the alternatives
/// the rule writes, and the variant is the one that was read.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = SyntaxKind::LeadingKeywords.map())]
pub enum LeadingKeywords<I>
where
    I: UnsynInput,
{
    /// `lexer`, which defines a terminal.
    Lexer(#[parserc(keyword = "lexer", map_err = KeywordKind::Lexer.map())] I),
    /// `syntax`, which defines a node of the tree.
    Syntax(#[parserc(keyword = "syntax", map_err = KeywordKind::Syntax.map())] I),
    /// `whitespace`, which declares the trivia of the language.
    Whitespace(#[parserc(keyword = "whitespace", map_err = KeywordKind::Whitespace.map())] I),
}

/// The word that joins two operands of a pattern into one construct.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(map_err = SyntaxKind::OperatorKeywords.map())]
pub enum OperatorKeywords<I>
where
    I: UnsynInput,
{
    /// `concat`, which joins what two operands matched into one token.
    Concat(#[parserc(keyword = "concat", map_err = KeywordKind::Concat.map())] I),
    /// `except`, which removes what one operand matches from another.
    Except(#[parserc(keyword = "except", map_err = KeywordKind::Except.map())] I),
}

// ---------------------------------------------------------------------------
// `lexer OctDigit -> ['0'-'7'];`
// `lexer DecDigit -> ['0'-'9'];`
// `lexer HexDigit -> ['0'-'9','a'-'f','A'-'F'];`
// ---------------------------------------------------------------------------

/// An octal digit, the first of the pair that spells a seven-bit escape.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(next_if = |c: char| matches!(c, '0'..='7'))]
pub struct OctDigit<I>(pub I)
where
    I: UnsynInput;

/// A decimal digit.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(next_if = |c: char| c.is_ascii_digit())]
pub struct DecDigit<I>(pub I)
where
    I: UnsynInput;

/// A hexadecimal digit.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(next_if = |c: char| c.is_ascii_hexdigit())]
pub struct HexDigit<I>(pub I)
where
    I: UnsynInput;

// ---------------------------------------------------------------------------
// `lexer LitDec -> DecDigit ( DecDigit | '_' )*;`
// ---------------------------------------------------------------------------

/// A decimal number, which is how the bound of a repetition is written.
///
/// The first digit is a node because the rule names it; every character after
/// it is one run of text, which is how the generated code reads the same
/// production — `take_while` over the two characters the rule allows, and the
/// fastest reading of a number a grammar can name. An underscore is part of the
/// token and carries no value, so the count is taken from the text with the
/// separators dropped, which is what [`decimal`](super::decode::decimal) does.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitDec<I>
where
    I: UnsynInput,
{
    /// The digit the number begins with.
    pub dec_digit: DecDigit<I>,
    /// Every digit and separator after it, as one run.
    #[parserc(take_while = |c: char| matches!(c, '0'..='9' | '_'))]
    pub f1: I,
}

// ---------------------------------------------------------------------------
// `lexer LitUnicode -> 'U+' HexDigit{4};`
// ---------------------------------------------------------------------------

/// A code point written in a string, as `U+0009` is.
///
/// The two sub-expressions of the rule are the two fields of the node: the `U+`
/// the spec writes first, and the four digits behind it. A terminal lays no run
/// between its operands, so a fifth digit is not part of this token — it belongs
/// to whatever operand comes next, which is what the generated code does with
/// the same production, and the two readings have to accept the same text for
/// one spec to compile itself.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitUnicode<I>
where
    I: UnsynInput,
{
    /// The `U+` that opens the literal.
    #[parserc(keyword = "U+", map_err = SyntaxKind::Unicode.map())]
    pub f1: I,
    /// The four digits that spell the code point.
    #[parserc(read_lit_unicode_digits)]
    pub hex_digit: Vec<HexDigit<I>>,
}

/// Reads the `HexDigit{4}` of [`LitUnicode`].
///
/// Four digits and no more are taken, exactly as the bound says. Fewer than
/// four is a code point written short, which no enclosing choice can put right,
/// so the refusal names the literal and covers the `U+` and every digit that
/// was read rather than the one that stopped the count.
fn read_lit_unicode_digits<I>(input: &mut I) -> Result<Vec<HexDigit<I>>, UnsynError>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    let mut digits = Vec::new();

    while digits.len() < 4 {
        let Some(digit) = HexDigit::into_parser().ok().parse(input)? else {
            break;
        };

        digits.push(digit);
    }

    if digits.len() < 4 {
        return Err(UnsynError::Semantics(
            SemanticsKind::Unicode,
            whole.split_to(before - input.len()).to_span(),
        ));
    }

    Ok(digits)
}

// ---------------------------------------------------------------------------
// `lexer QuoteEscape -> '\\'';`
// `lexer AsciiEscape -> AsciiHexEscape | AsciiNewline | AsciiCarriageReturn`
// `                   | AsciiTab | AsciiBackslash | AsciiNull;`
// `lexer AsciiHexEscape -> '\\x' OctDigit HexDigit;`
// `lexer AsciiNewline -> '\\n';`
// `lexer AsciiCarriageReturn -> '\\r';`
// `lexer AsciiTab -> '\\t';`
// `lexer AsciiBackslash -> '\\\\';`
// `lexer AsciiNull -> '\\0';`
// `lexer UnicodeEscape -> '\\u{' ( HexDigit '_'* ){1, 6} '}';`
// ---------------------------------------------------------------------------

/// An escaped quote, which is how a `'` is written inside a `'` delimited
/// literal.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(keyword = "\\'", map_err = SyntaxKind::QuoteEscape.map())]
pub struct QuoteEscape<I>(pub I)
where
    I: UnsynInput;

/// An escape for one of the characters ASCII writes with a backslash.
///
/// The rule is a choice of six named rules, so each alternative is a node of its
/// own and the walk reaches it through that node's hook.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AsciiEscape<I>
where
    I: UnsynInput,
{
    /// `\x` and the two digits of a code of seven bits.
    AsciiHexEscape(AsciiHexEscape<I>),
    /// `\n`, a line feed.
    AsciiNewline(AsciiNewline<I>),
    /// `\r`, a carriage return.
    AsciiCarriageReturn(AsciiCarriageReturn<I>),
    /// `\t`, a tab.
    AsciiTab(AsciiTab<I>),
    /// `\\`, the backslash itself.
    AsciiBackslash(AsciiBackslash<I>),
    /// `\0`, the nul character.
    AsciiNull(AsciiNull<I>),
}

/// `lexer AsciiNewline -> '\\n';`: an escaped line feed.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(keyword = "\\n", map_err = SyntaxKind::AsciiEscape.map())]
pub struct AsciiNewline<I>(pub I)
where
    I: UnsynInput;

/// `lexer AsciiCarriageReturn -> '\\r';`: an escaped carriage return.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(keyword = "\\r", map_err = SyntaxKind::AsciiEscape.map())]
pub struct AsciiCarriageReturn<I>(pub I)
where
    I: UnsynInput;

/// `lexer AsciiTab -> '\\t';`: an escaped tab.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(keyword = "\\t", map_err = SyntaxKind::AsciiEscape.map())]
pub struct AsciiTab<I>(pub I)
where
    I: UnsynInput;

/// `lexer AsciiBackslash -> '\\\\';`: an escaped backslash.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(keyword = "\\\\", map_err = SyntaxKind::AsciiEscape.map())]
pub struct AsciiBackslash<I>(pub I)
where
    I: UnsynInput;

/// `lexer AsciiNull -> '\\0';`: an escaped nul.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(keyword = "\\0", map_err = SyntaxKind::AsciiEscape.map())]
pub struct AsciiNull<I>(pub I)
where
    I: UnsynInput;

/// `lexer AsciiHexEscape -> '\\x' OctDigit HexDigit;`: the escape of a code of
/// seven bits, spelled by the two hex digits after its marker.
///
/// The three operands the rule writes are the three fields the node holds. What
/// is read past the marker is a written escape going wrong, and no enclosing
/// choice can mean anything else there — which is why the marker is `crucial`.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AsciiHexEscape<I>
where
    I: UnsynInput,
{
    /// The `\x` that opens the escape.
    #[parserc(crucial, keyword = "\\x", map_err = SyntaxKind::AsciiEscape.map())]
    pub f1: I,
    /// The leading digit, bounded by [`OctDigit`] — which is why the first of
    /// them is refused above `7`.
    pub oct_digit: OctDigit<I>,
    /// The second digit of the pair.
    pub hex_digit: HexDigit<I>,
}

/// An escape of a code point, as `\u{1F600}` is.
///
/// The three sub-expressions the rule writes are the three fields the node
/// holds: the marker, the digits and the brace that closes it — the same shape
/// the generated code gives the rule. The bound is of the digits, so they are
/// held one apiece and counted as they are read.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnicodeEscape<I>
where
    I: UnsynInput,
{
    /// The `\u{` that opens the escape.
    #[parserc(keyword = "\\u{", map_err = SyntaxKind::UnicodeEscape.map())]
    pub f1: I,
    /// The one to six digits the braces hold.
    #[parserc(read_escape_digits)]
    pub hex_digit: Vec<HexDigit<I>>,
    /// The `}` that closes the escape.
    #[parserc(keyword = "}", map_err = PunctKind::BraceEnd.map())]
    pub f2: I,
}

/// Reads the `HexDigit{1, 6}` of [`UnicodeEscape`].
///
/// The bound counts digits, so they are taken one at a time and at most six of
/// them: a seventh is not the bound speaking but the `}` after it, and the
/// closing of the escape is the one to say that it did not arrive. Fewer than
/// one is an escape written empty, which no enclosing choice can put right, so
/// the refusal names the escape rather than the count that met it.
fn read_escape_digits<I>(input: &mut I) -> Result<Vec<HexDigit<I>>, UnsynError>
where
    I: UnsynInput,
{
    let mut digits = Vec::new();

    while digits.len() < 6 {
        let Some(digit) = HexDigit::into_parser().ok().parse(input)? else {
            break;
        };

        digits.push(digit);
    }

    if digits.is_empty() {
        return Err(UnsynError::Semantics(
            SemanticsKind::UnicodeEscape,
            input.to_span_at(1),
        ));
    }

    Ok(digits)
}

// ---------------------------------------------------------------------------
// `lexer LitStr -> '\'' ( ~['"','\\',CR] | QuoteEscape | AsciiEscape`
// `                   | UnicodeEscape )* '\'';`
// ---------------------------------------------------------------------------

/// A string literal: any run of characters between two `'`, in which a `'`, a
/// `\` and a `CR` can only be written by an escape.
///
/// The body is one run of text, which is the shape the generated code gives the
/// same production: the pieces are read as tokens only so that the run stops
/// where a piece does not begin, and what the tree keeps is the text they made
/// up. What the literal spells is read back from that text by [`decode`],
/// because a member of a set and the end of a range are both asked what a
/// literal *spells* and never how it was spelled.
///
/// [`decode`]: super::decode
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LitStr<I>(
    /// The quote that opens the literal.
    #[parserc(keyword = "'", map_err = PunctKind::SingleQuote.map())]
    pub I,
    /// The body of the literal, pieces of plain text and escapes together.
    ///
    /// The repetition ends where no piece begins, which is the closing quote
    /// wherever one stands and whatever the text after the pieces otherwise
    /// is; the quote that has to come next says what was expected there.
    #[parserc(read_lit_str_body)]
    pub I,
    /// The quote that closes the literal.
    #[parserc(keyword = "'", map_err = PunctKind::SingleQuote.map())]
    pub I,
)
where
    I: UnsynInput;

/// Reads the body of a string literal as one run of text.
///
/// The four pieces the rule lists are tried one after another, and each is
/// thrown away once it has been read: they bound the run and say nothing the
/// tree does not already hold. A piece that does not begin is the end of the
/// body, so the loop stops there with the input untouched and the closing
/// quote — or whatever else stands there — is left for the rule above.
fn read_lit_str_body<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let mut taken = 0;

    loop {
        let here = input.len();

        let piece = next_if(|c: char| c != '\'' && c != '\\' && c != '\r')
            .map(|_| ())
            .or(<QuoteEscape<I>>::into_parser().map(|_| ()))
            .or(<AsciiEscape<I>>::into_parser().map(|_| ()))
            .or(<UnicodeEscape<I>>::into_parser().map(|_| ()))
            .ok();

        if piece.parse(input)?.is_none() {
            break;
        }

        taken += here - input.len();
    }

    Ok(whole.split_to(taken))
}

// ---------------------------------------------------------------------------
// `lexer InnerLineDoc -> '//!' ~[LF,CR]*;`
// `lexer OuterLineDoc -> '///' ( ~['/'] ~[LF,CR]* )?;`
// ---------------------------------------------------------------------------

/// Whether a character leaves the line a document is written on.
#[inline]
fn stays_on_the_line(c: char) -> bool {
    c != '\n' && c != '\r'
}

/// A line document that describes the file it is written in.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InnerLineDoc<I>(
    /// The `//!` that opens the document.
    #[parserc(keyword = "//!", map_err = SyntaxKind::InnerLineDoc.map())]
    pub I,
    /// The text to the end of the line, which the set of the rule refuses to
    /// cross. The run is taken in one piece: that is what a repetition of one
    /// refused character arrives at, and nothing below it says more.
    #[parserc(take_while = stays_on_the_line)]
    pub I,
)
where
    I: UnsynInput;

/// A line document that describes the definition after it.
///
/// The body is one run of text, as the generated code reads it: the rule asks
/// for it at most once, and what it holds is a character that is no slash
/// followed by the rest of the line — a run the tree keeps whole, because a
/// document is trivia and nothing asks it for its pieces.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OuterLineDoc<I>(
    /// The `///` that opens the document.
    #[parserc(keyword = "///", map_err = SyntaxKind::OuterLineDoc.map())]
    pub I,
    /// The body, empty when the rule's one optional group is not filled: the
    /// fourth `/` is refused by what the group asks first, which leaves the
    /// body unfilled, and what stands after the document is for the file to
    /// say.
    #[parserc(read_outer_line_doc_body)]
    pub I,
)
where
    I: UnsynInput;

/// Reads the one optional group of [`OuterLineDoc`] as a run of text.
///
/// The group is optional as a whole, so a body that does not begin is no error
/// and the run it would have made is empty.
fn read_outer_line_doc_body<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    let body = |input: &mut I| {
        next_if(|c: char| c != '/').parse(input)?;
        take_while(stays_on_the_line).parse(input)?;
        Ok(())
    };

    _ = body.ok().parse(input)?;

    Ok(whole.split_to(before - input.len()))
}

// ---------------------------------------------------------------------------
// `lexer BlockComment -> '/*' ( ~['*','!','/*'] | '**' | BlockCommentOrDoc )`
// `                         ( BlockCommentOrDoc | ~['/*','*/'] )* '*/'`
// `                       | '/**/' | '/***/';`
// `lexer InnerBlockDoc -> '/*!' ( BlockCommentOrDoc | ~['/*','*/'] )* '*/';`
// `lexer OuterBlockDoc -> '/**' ( ~['*','/*'] | BlockCommentOrDoc )`
// `                        ( BlockCommentOrDoc | ~['/*','*/'] )* '*/';`
// `lexer BlockCommentOrDoc -> BlockComment | OuterBlockDoc | InnerBlockDoc;`
// ---------------------------------------------------------------------------

/// Takes one character when it is none of `singles` and opens none of the two
/// character `pairs`.
///
/// This is what the negated set of a block rule asks of a character: the members
/// of the set are read as much from a `/*` as from a `*`, so the window of two
/// characters is what a plain character of a body is refused by. The refusal is
/// [`Recovable`](ControlFlow::Recovable): what stands where a plain character is
/// asked for is the alternative after it, or the end of the body.
fn plain_refusing<I>(
    input: &mut I,
    singles: &[char],
    pairs: &[&'static str],
) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    let refused =
        |input: &mut I| UnsynError::Kind(Kind::NextIf(ControlFlow::Recovable, input.to_span_at(1)));

    let Some(c) = input.iter().next() else {
        return Err(UnsynError::Kind(Kind::NextIf(
            ControlFlow::Incomplete,
            input.to_span_at(1),
        )));
    };

    if singles.contains(&c) || pairs.iter().any(|pair| input.starts_with(*pair).is_some()) {
        return Err(refused(input));
    }

    next(c).parse(input)
}

/// The one character of the text of a [`BlockComment`] body, which is none of
/// `*` and `!` and opens no nested group.
fn head_plain<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    plain_refusing(input, &['*', '!'], &["/*"])
}

/// The one character of the text of an [`OuterBlockDoc`] body, which is no `*`
/// and opens no nested group.
fn outer_head_plain<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    plain_refusing(input, &['*'], &["/*"])
}

/// The one character of the text of a body that is only text, which opens no
/// nested group and closes no group either.
fn tail_plain<I>(input: &mut I) -> Result<I, UnsynError>
where
    I: UnsynInput,
{
    plain_refusing(input, &[], &["/*", "*/"])
}

/// The text one parser read, cut from where it began.
///
/// A rule that holds no field of its own still has text, and a run of it is what
/// a sequence of the enclosing rule keeps: the parser is asked to read, and what
/// it took is the node, not whatever shape it wore the taking.
fn text_of<P, I>(parser: P, input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
    P: Parser<I>,
{
    let mut whole = input.clone();
    let before = input.len();

    parser.parse(input)?;

    Ok(whole.split_to(before - input.len()))
}

/// Reads the third operand of [`BlockCommentBody`]: as many groups and further
/// runs as the comment takes, until the pair that closes it begins.
fn read_block_comment_body_3<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    loop {
        let part =
            (|input: &mut I| text_of(<BlockCommentOrDoc<I> as Syntax<I>>::into_parser(), input))
                .or(tail_plain)
                .ok()
                .parse(input)?;

        if part.is_none() {
            break;
        }
    }

    Ok(whole.split_to(before - input.len()))
}

/// Reads the second operand of [`InnerBlockDoc`]: the groups and text the body
/// holds, until the pair that closes the document begins.
fn read_inner_block_doc_2<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    loop {
        let part =
            (|input: &mut I| text_of(<BlockCommentOrDoc<I> as Syntax<I>>::into_parser(), input))
                .or(tail_plain)
                .ok()
                .parse(input)?;

        if part.is_none() {
            break;
        }
    }

    Ok(whole.split_to(before - input.len()))
}

/// Reads the third operand of [`OuterBlockDoc`]: as many groups and further runs
/// as the document takes.
fn read_outer_block_doc_3<I>(input: &mut I) -> Result<I, <I as Input>::Error>
where
    I: UnsynInput,
{
    let mut whole = input.clone();
    let before = input.len();

    loop {
        let part =
            (|input: &mut I| text_of(<BlockCommentOrDoc<I> as Syntax<I>>::into_parser(), input))
                .or(tail_plain)
                .ok()
                .parse(input)?;

        if part.is_none() {
            break;
        }
    }

    Ok(whole.split_to(before - input.len()))
}

/// A block comment, which nests and is closed by the `*/` of its own opening.
///
/// The rule is a choice of three named rules, so each form is a node of its own
/// and the walk reaches it through that node's hook.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BlockComment<I>
where
    I: UnsynInput,
{
    /// `/*` … `*/`, a comment that holds something.
    BlockCommentBody(BlockCommentBody<I>),
    /// `/**/`, the comment that closes at once.
    EmptyBlockComment(EmptyBlockComment<I>),
    /// `/***/`, the comment that closes at once after a star.
    StarredEmptyBlockComment(StarredEmptyBlockComment<I>),
}

/// `lexer EmptyBlockComment -> '/**/';`: a block comment with nothing between
/// its pair of stars.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(keyword = "/**/", map_err = SyntaxKind::BlockComment.map())]
pub struct EmptyBlockComment<I>(pub I)
where
    I: UnsynInput;

/// `lexer StarredEmptyBlockComment -> '/***/';`: a block comment whose one
/// content is the star that keeps it from being an outer document.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[parserc(keyword = "/***/", map_err = SyntaxKind::BlockComment.map())]
pub struct StarredEmptyBlockComment<I>(pub I)
where
    I: UnsynInput;

/// `lexer BlockCommentBody -> '/*' ( … ) ( … )* '*/';`: a comment that holds
/// something. The four operands the rule writes are the four fields the node
/// holds, and the nesting the reading has to count lives in the middle two.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockCommentBody<I>(
    /// The `/*` that opens the comment.
    #[parserc(keyword = "/*", map_err = SyntaxKind::BlockComment.map())]
    pub I,
    /// The head: one character that is no star, no bang and opens no group; the
    /// two stars of a comment that closes at once; or a group of its own.
    #[parserc(
        parser = head_plain
            .or(keyword("**").map_err(SyntaxKind::BlockComment.map()))
            .or(|input: &mut I| {
                text_of(<BlockCommentOrDoc<I> as Syntax<I>>::into_parser(), input)
            })
    )]
    pub I,
    /// As many groups and further runs as the comment takes, until the pair that
    /// closes it begins.
    #[parserc(read_block_comment_body_3)]
    pub I,
    /// The `*/` that closes the comment.
    #[parserc(keyword = "*/", map_err = SyntaxKind::BlockComment.map())]
    pub I,
)
where
    I: UnsynInput;

/// `lexer InnerBlockDoc -> '/*!' ( … )* '*/';`: a block document that describes
/// the file it is written in.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InnerBlockDoc<I>(
    /// The `/*!` that opens the document.
    #[parserc(keyword = "/*!", map_err = SyntaxKind::InnerBlockDoc.map())]
    pub I,
    /// The groups and text the body holds.
    #[parserc(read_inner_block_doc_2)]
    pub I,
    /// The `*/` that closes the document.
    #[parserc(keyword = "*/", map_err = SyntaxKind::InnerBlockDoc.map())]
    pub I,
)
where
    I: UnsynInput;

/// `lexer OuterBlockDoc -> '/**' ( … ) ( … )* '*/';`: a block document that
/// describes the definition after it.
///
/// The opening of three refuses what the head set keeps out of the body, which
/// is what leaves `/**/` and `/***/` to the block comment: an opening of three
/// is a comment that closes at once, and not an empty document.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OuterBlockDoc<I>(
    /// The `/**` that opens the document.
    #[parserc(keyword = "/**", map_err = SyntaxKind::OuterBlockDoc.map())]
    pub I,
    /// The head: one character that is no star and opens no group, or a group of
    /// its own.
    #[parserc(
        parser = outer_head_plain
            .or(|input: &mut I| {
                text_of(<BlockCommentOrDoc<I> as Syntax<I>>::into_parser(), input)
            })
    )]
    pub I,
    /// As many groups and further runs as the document takes.
    #[parserc(read_outer_block_doc_3)]
    pub I,
    /// The `*/` that closes the document.
    #[parserc(keyword = "*/", map_err = SyntaxKind::OuterBlockDoc.map())]
    pub I,
)
where
    I: UnsynInput;

/// A block group, which is the one rule that nests: a comment may hold a
/// document, and a document may hold a comment.
///
/// The order of the alternatives is the order of the spec, and it matters only
/// for `/***`, where a comment and an outer document both begin with the three
/// characters: the comment is read first, so the choice settles on the comment.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BlockCommentOrDoc<I>
where
    I: UnsynInput,
{
    /// A comment, which wins the three stars.
    BlockComment(BlockComment<I>),
    /// A document of a definition.
    OuterBlockDoc(OuterBlockDoc<I>),
    /// A document of the file.
    InnerBlockDoc(InnerBlockDoc<I>),
}

#[cfg(test)]
mod tests {

    use parserc::ToSpan;
    use parserc::{AsStr, Span, syntax::SyntaxInput};

    use super::*;
    use crate::compiler::input::TokenStream;
    use parserc::as_str;

    /// Parses `source` as the node `T`.
    fn parse<'a, T>(source: &'a str) -> Result<T, UnsynError>
    where
        T: Syntax<TokenStream<'a>>,
    {
        TokenStream::from(source).parse::<T>()
    }

    /// The text a node read, taken from the source under its span.
    ///
    /// The tests ask this of a node they parse from a literal source, and the
    /// answer comes from the span rather than from the parts the node kept:
    /// that is the property worth pinning, since a node rebuilt from its fields
    /// agrees with its span only while those lie flush, and nothing but a check
    /// like this one says whether they do.
    fn text<'a, T>(source: &'a str) -> String
    where
        T: Syntax<TokenStream<'a>> + ToSpan,
    {
        let whole = TokenStream::from(source);

        as_str(&whole, &parse::<T>(source).expect(source)).to_owned()
    }

    #[test]
    fn the_trivia_is_the_set_of_the_rule() {
        for c in [
            '\u{0009}', '\u{000A}', '\u{000B}', '\u{000C}', '\u{000D}', '\u{0020}', '\u{0085}',
            '\u{00A0}', '\u{1680}', '\u{2000}', '\u{200A}', '\u{2028}', '\u{2029}', '\u{202F}',
            '\u{205F}', '\u{3000}',
        ] {
            assert!(is_trivia(c), "{c:?}");
        }

        for c in [
            '\u{0008}', '\u{000E}', '\u{0021}', '\u{00A1}', '\u{200B}', '\u{3001}',
        ] {
            assert!(!is_trivia(c), "{c:?}");
        }

        // The run is greedy, and it takes the whole of what the text offers.
        assert_eq!(text::<S<_>>(" \n\t "), " \n\t ");
        assert_eq!(text::<S<_>>("  x"), "  ");

        // A run of nothing at all is not a run.
        assert!(parse::<S<_>>("x").is_err());
    }

    #[test]
    fn a_line_ending_is_its_own_token() {
        assert_eq!(text::<LF<_>>("\n"), "\n");
        assert!(parse::<LF<_>>("\r").is_err());
        assert_eq!(text::<CR<_>>("\r"), "\r");
    }

    #[test]
    fn a_name_is_a_start_and_a_run() {
        for source in ["x", "_", "XID_Continue", "a1", "café"] {
            assert_eq!(text::<IdentOrKeyword<_>>(source), source);
        }

        // A name starts where a continuing character cannot.
        assert_eq!(text::<IdentOrKeyword<_>>("_a-b"), "_a");
        assert!(parse::<IdentOrKeyword<_>>("1a").is_err());
        assert!(parse::<IdentOrKeyword<_>>("-a").is_err());
    }

    #[test]
    fn a_keyword_is_a_word_whatever_stands_behind_it() {
        for word in ["lexer", "syntax", "whitespace"] {
            assert_eq!(text::<LeadingKeywords<_>>(word), word);
        }
        for word in ["concat", "except"] {
            assert_eq!(text::<OperatorKeywords<_>>(word), word);
        }

        // The rule writes a word and nothing else, so the word is read wherever
        // its text stands: `lexerX` begins with `lexer`, and it is the rules
        // about it — the `except` that keeps the words out of the names, and the
        // name that has to follow the word — that decide what the rest of the
        // text is.
        assert_eq!(text::<LeadingKeywords<_>>("lexerX"), "lexer");
        assert_eq!(text::<OperatorKeywords<_>>("excepting"), "except");

        // Where no word of the set begins at all, it is the set that the
        // position is missing. Both sets are asked, since each names itself
        // through its own `map_err` on the enum.
        assert_eq!(
            parse::<LeadingKeywords<_>>("| 'a';"),
            Err(UnsynError::Syntax(
                SyntaxKind::LeadingKeywords,
                ControlFlow::Recovable,
                Span::Range(0..1)
            ))
        );

        assert_eq!(
            parse::<OperatorKeywords<_>>("| 'a';"),
            Err(UnsynError::Syntax(
                SyntaxKind::OperatorKeywords,
                ControlFlow::Recovable,
                Span::Range(0..1)
            ))
        );
    }

    #[test]
    fn a_word_is_a_name_except_for_the_words_of_the_language() {
        // The `except` of the spec runs over the text the name read and over
        // nothing else, so a word that is only the head of a name leaves it a
        // name.
        for keyword in ["whitespace", "lexer", "syntax", "except", "concat"] {
            assert_eq!(
                parse::<NonKeywordIdent<_>>(keyword),
                Err(UnsynError::Semantics(
                    SemanticsKind::Keyword,
                    Span::Range(0..keyword.len())
                )),
                "{keyword}"
            );
        }

        for source in ["WHITESPACE_", "_x", "lexerX", "xlet", "syntaxes"] {
            assert_eq!(text::<NonKeywordIdent<_>>(source), source);
        }
    }

    #[test]
    fn a_raw_name_borrows_a_word_of_rust_and_not_one_of_the_language() {
        assert_eq!(text::<RawIdent<_>>("r#where"), "r#where");
        assert_eq!(text::<RawIdent<_>>("r#type"), "r#type");
        assert!(parse::<RawIdent<_>>("r#1").is_err());

        // The sigil reaches the rule that refuses a word of the language, so
        // `r#` opens no way around the words of the DSL: it is for the words of
        // Rust alone.
        assert!(matches!(
            parse::<RawIdent<_>>("r#lexer"),
            Err(UnsynError::Semantics(SemanticsKind::Keyword, _))
        ));

        // The raw form is read first, because `NonKeywordIdent` would settle for
        // the `r` of `r#type` and leave `#type` behind, and because the refusal
        // of `r#lexer` is fatal: read in the other order it would be the name
        // `r` that stands, and `r` is a name.
        assert!(matches!(
            parse::<Ident<_>>("r#type"),
            Ok(Ident::RawIdent(_))
        ));
        assert!(
            matches!(
                parse::<Ident<_>>("r#lexer"),
                Err(UnsynError::Semantics(SemanticsKind::Keyword, _))
            ),
            "a word under the sigil is no name at all, and no other alternative is tried"
        );
        assert!(matches!(
            parse::<Ident<_>>("lexer2"),
            Ok(Ident::NonKeywordIdent(_))
        ));
    }

    #[test]
    fn the_digits_of_the_sets_are_one_character() {
        assert_eq!(text::<OctDigit<_>>("7"), "7");
        assert!(parse::<OctDigit<_>>("8").is_err());
        assert_eq!(text::<DecDigit<_>>("9"), "9");
        assert_eq!(text::<HexDigit<_>>("f"), "f");
        assert!(parse::<HexDigit<_>>("g").is_err());

        // A set is one character, so it takes no second one.
        assert_eq!(
            parse::<OctDigit<_>>("77").map(|d| d.0.as_str().to_string()),
            Ok("7".to_string())
        );
    }

    #[test]
    fn a_decimal_literal_separates_with_underscores() {
        for source in ["0", "12", "1_000", "1_2_3_4"] {
            assert_eq!(text::<LitDec<_>>(source), source);
        }

        // An underscore cannot open the literal, and a run of digits ends where
        // a name would continue.
        assert!(parse::<LitDec<_>>("_1").is_err());
        assert_eq!(text::<LitDec<_>>("12x"), "12");
    }

    #[test]
    fn a_unicode_literal_is_exactly_four_digits() {
        assert_eq!(text::<LitUnicode<_>>("U+2029"), "U+2029");

        // Four digits is the whole token. A fifth is not a miswritten literal
        // but the text that follows it, which is what the bound says and what
        // the generated reading of the same rule does — the two parsers have
        // to accept one language for a spec to compile itself.
        let whole = TokenStream::from("U+11234");
        let mut input = TokenStream::from("U+11234");
        let lit = input.parse::<LitUnicode<_>>().unwrap();
        assert_eq!(lit.to_span(), Span::Range(0..6));
        // The text is the source under the span, read through the stream rather
        // than rebuilt from the prefix and the four digits the node kept.
        assert_eq!(as_str(&whole, &lit), "U+1123");
        assert_eq!(input.as_str(), "4");

        // Fewer than four is the literal written short. The span covers the
        // digits that were read and not the `U+` before them: the bound is a
        // field of its own, and a field's reader cannot see the field that
        // came before it — which is how the generated code is shaped too, so
        // the two refusals point at the same place and differ only in that the
        // hand file names what it expected.
        for (source, span) in [("U+1", 2..3), ("U+12_3", 2..4), ("U+0x29", 2..3)] {
            assert_eq!(
                parse::<LitUnicode<_>>(source),
                Err(UnsynError::Semantics(
                    SemanticsKind::Unicode,
                    Span::Range(span)
                )),
                "{source}"
            );
        }

        // A refusal leaves the input where the reading of the digits stopped,
        // which is what the generated reader of the same bound does too: the
        // digits that were read are gone, and what refused the fourth is still
        // there.
        let mut input = TokenStream::from("U+12_3");
        assert!(input.parse::<LitUnicode<_>>().is_err());
        assert_eq!(input.as_str(), "_3");
    }

    #[test]
    fn an_ascii_escape_is_seven_bits() {
        for source in ["\\n", "\\r", "\\t", "\\\\", "\\0"] {
            assert_eq!(text::<AsciiEscape<_>>(source), source);
        }

        assert_eq!(text::<AsciiEscape<_>>("\\x7F"), "\\x7F");

        // The two digits of the escape are the escape's own once its marker has
        // been read: whatever stands where a digit must is a written escape
        // going wrong, and no enclosing choice can mean anything else there.
        for (source, span) in [("\\x80", 2..3), ("\\xg0", 2..3), ("\\x0", 3..3)] {
            assert_eq!(
                parse::<AsciiEscape<_>>(source),
                Err(UnsynError::Kind(Kind::NextIf(
                    ControlFlow::Fatal,
                    Span::Range(span)
                ))),
                "{source}"
            );
        }

        assert_eq!(text::<QuoteEscape<_>>(r"\'"), r"\'");
    }

    #[test]
    fn a_unicode_escape_is_one_to_six_digits() {
        for source in ["\\u{1}", "\\u{1F600}", "\\u{abcdef}"] {
            assert_eq!(text::<UnicodeEscape<_>>(source), source);
        }

        // No digits at all is the lower bound of the escape refusing its own
        // reading.
        assert!(
            matches!(
                parse::<UnicodeEscape<_>>("\\u{}"),
                Err(UnsynError::Semantics(SemanticsKind::UnicodeEscape, _))
            ),
            "no digits at all"
        );

        // The rule writes `HexDigit{1, 6}` and nothing between the digits, so
        // a separator is not part of an escape: six digits are taken and the
        // brace is then asked for where the `_` stands.
        assert!(
            matches!(
                parse::<UnicodeEscape<_>>("\\u{1_0_0}"),
                Err(UnsynError::Punct(PunctKind::BraceEnd, _, _))
            ),
            "an underscore separates no digits"
        );

        // A seventh digit is not the bound speaking: six are taken and the
        // brace is then asked for where a digit stands. Text that is no digit
        // at all leaves the same question to the same place.
        for source in ["\\u{1234567}", "\\u{abcdefg}", "\\u{0x}"] {
            assert!(
                matches!(
                    parse::<UnicodeEscape<_>>(source),
                    Err(UnsynError::Punct(PunctKind::BraceEnd, _, _))
                ),
                "{source}"
            );
        }

        // The closing brace is not optional.
        assert_eq!(
            parse::<UnicodeEscape<_>>("\\u{1"),
            Err(UnsynError::Punct(
                PunctKind::BraceEnd,
                ControlFlow::Recovable,
                Span::Range(4..4)
            ))
        );
    }

    #[test]
    fn a_string_stops_at_the_quote_that_is_not_escaped() {
        for source in [
            "''",
            "'a'",
            "'\\''",
            "'\\\\x41'",
            "'\\u{41}'",
            "'a\nb'",
            "'U+0041'",
        ] {
            assert_eq!(text::<LitStr<_>>(source), source);
        }

        // A double quote is nothing special here, and a bare apostrophe closes
        // the literal wherever it stands.
        assert_eq!(text::<LitStr<_>>("'\"'"), "'\"'");
        assert_eq!(text::<LitStr<_>>("'a'b'"), "'a'");

        // A body that runs to the end of the input, and a `CR` that is no piece
        // of a body, both end the reading where they stand: what was asked for
        // there is the closing quote, and that is what the refusal says.
        assert_eq!(
            parse::<LitStr<_>>("'a"),
            Err(UnsynError::Punct(
                PunctKind::SingleQuote,
                ControlFlow::Recovable,
                Span::Range(2..2)
            ))
        );
        assert_eq!(
            parse::<LitStr<_>>("'a\rb'"),
            Err(UnsynError::Punct(
                PunctKind::SingleQuote,
                ControlFlow::Recovable,
                Span::Range(2..3)
            ))
        );
        // A `\` that opens no escape is no content, and it is where the closing
        // quote is then asked for and found missing.
        assert_eq!(
            parse::<LitStr<_>>("'\\q'"),
            Err(UnsynError::Punct(
                PunctKind::SingleQuote,
                ControlFlow::Recovable,
                Span::Range(1..2),
            ))
        );
    }

    #[test]
    fn a_line_document_ends_at_either_line_ending() {
        assert_eq!(text::<InnerLineDoc<_>>("//! about\nnext"), "//! about");
        assert_eq!(text::<InnerLineDoc<_>>("//!about\r\nnext"), "//!about");
        assert_eq!(
            text::<InnerLineDoc<_>>("//!"),
            "//!",
            "the body of a line document is optional"
        );

        assert_eq!(text::<OuterLineDoc<_>>("/// about"), "/// about");
        assert_eq!(text::<OuterLineDoc<_>>("///"), "///");

        // The fourth `/` is refused by `~['/']`, which leaves the body of the
        // document unfilled: the document is its three slashes, and whatever
        // stands after them belongs to the file to read.
        assert_eq!(text::<OuterLineDoc<_>>("////"), "///");
    }

    /// The table of the forms a comment takes, which the third character
    /// decides.
    #[test]
    fn a_block_group_is_told_apart_by_how_it_opens() {
        let cases = &[
            ("/* a */", "/* a */"),
            ("/**/ ", "/**/"),
            ("/***/ ", "/***/"),
            ("/* a /* nested */ still */", "/* a /* nested */ still */"),
        ];

        for &(source, expected) in cases {
            assert_eq!(
                text::<BlockComment<_>>(source),
                expected.to_string(),
                "{source}"
            );
            assert!(parse::<BlockCommentOrDoc<_>>(source).is_ok(), "{source}");
        }

        // A document is not a comment, and a comment is not a document.
        assert!(parse::<BlockComment<_>>("/*! a */").is_err());
        assert!(parse::<OuterBlockDoc<_>>("/*! a */").is_err());
        assert!(parse::<InnerBlockDoc<_>>("/* a */").is_err());
        assert!(parse::<OuterBlockDoc<_>>("/**/").is_err());
        assert!(parse::<OuterBlockDoc<_>>("/***/").is_err());

        // `/*/*/ */` opens twice and closes once.
        assert!(parse::<BlockComment<_>>("/*/*/ */").is_err());
        // A lone `/` is only text, so this one does close.
        assert_eq!(text::<BlockComment<_>>("/*/ */"), "/*/ */");
    }

    #[test]
    fn an_unterminated_group_is_answered_by_no_form_of_its_rule() {
        // The comment is a choice of three forms, and none of them is what the
        // text turns out to hold. The reading reports the choice it could not
        // settle, which is the whole of what it had reached for — and this is
        // now the answer the generated tree gives of the same rule, character
        // for character: the hand file kept its own longest-form span only
        // while a hand-written scan read the comment, and the derive does not.
        assert_eq!(
            parse::<BlockComment<_>>("/* a "),
            Err(UnsynError::Kind(Kind::Syntax(
                "BlockComment",
                ControlFlow::Recovable,
                Span::Range(0..1)
            )))
        );

        // The two documents are one form each, so what is missing is exactly
        // the closing the body ran out before.
        assert_eq!(
            parse::<InnerBlockDoc<_>>("/*! a "),
            Err(UnsynError::Syntax(
                SyntaxKind::InnerBlockDoc,
                ControlFlow::Recovable,
                Span::Range(6..6)
            ))
        );
        assert_eq!(
            parse::<OuterBlockDoc<_>>("/** a "),
            Err(UnsynError::Syntax(
                SyntaxKind::OuterBlockDoc,
                ControlFlow::Recovable,
                Span::Range(6..6)
            ))
        );
    }

    #[test]
    fn a_block_group_of_the_three_forms_is_a_block_comment_or_doc() {
        for source in ["/* a */", "/** a */", "/*! a */"] {
            assert!(parse::<BlockCommentOrDoc<_>>(source).is_ok(), "{source}");
        }

        assert!(parse::<BlockCommentOrDoc<_>>("// a").is_err());
    }

    /// What the two document forms do with a group nested in them, pinned here
    /// before the reading of a block group collapses into text: the tree will
    /// not say these parts again, so this is where the language says what it
    /// accepts.
    #[test]
    fn a_document_holds_the_group_it_nests_and_closes_once() {
        for source in [
            "/** outer /* nested */ doc */",
            "/** two /* a */ /* b */ */",
            "/*! inner /* x */ doc */",
            "/*! inner /*! no */ */",
            "/** a /* b /* c */ d */ e */",
        ] {
            assert_eq!(
                text::<BlockCommentOrDoc<_>>(source),
                source.to_string(),
                "{source}"
            );
        }

        // The opening of three stars is the comment's, and the document of the
        // same opening never sees the fourth character.
        assert_eq!(text::<BlockComment<_>>("/***/"), "/***/");
        assert!(parse::<OuterBlockDoc<_>>("/***/").is_err());

        // A nested group that closes the outer one leaves nothing to close the
        // rest with, so no form of the rule holds.
        assert!(parse::<BlockCommentOrDoc<_>>("/** a /* b */").is_err());
    }
}
