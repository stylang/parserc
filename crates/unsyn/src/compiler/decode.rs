//! Reading the characters a `lexer` rule spells, from the text it was read as.
//!
//! A token such as a string literal is one run of text in the tree: the hand
//! parser and the generated one both read it that way, because taking the run
//! and saying what it holds beats asking a character stream for its parts one
//! by one. What the tree therefore does not hold is the *value* of the literal,
//! and the passes that need one — `'a'` as a member of a set, `'\n'` as the end
//! of a range, `\u{…}` as a code point, `{12, 34}` as a count — read it here.
//!
//! These are free functions and not methods of the node types on purpose. The
//! tree is what a build of [`unsyn`](crate) writes, and a build writes types,
//! parsers and a visitor and no inherent method of its own: a `fn count` beside
//! a decimal literal would be a hand-written patch on a generated file, gone the
//! moment the spec compiles itself.
//!
//! Decoding is the mirror of the reading, so it follows the escape rules the
//! spec writes and no others. The parser has already refused a literal whose
//! escapes are not among those, so the only thing left to report is a number of
//! digits that spells no character, which `\u{D800}` and its like are. Spans
//! come back as ranges of the source the literal was read from, so a diagnostic
//! points at the escape and not at the whole literal.

use crate::compiler::{
    input::UnsynInput,
    lexer::{LitDec, LitStr, LitUnicode},
};
use parserc::Span;

/// What the body of a string literal spells.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Body {
    /// The characters the body holds, with the escapes resolved.
    ///
    /// An escape of no character adds nothing here: the refusal in `invalid` is
    /// what says so, and the pass that wants the value of the literal is the
    /// one to act on it.
    pub chars: Vec<char>,
    /// The span of every `\u{…}` that spells no character, as `D800` does.
    pub invalid: Vec<Span>,
}

/// The number a decimal literal spells, with the underscores the rule allows
/// between its digits dropped.
///
/// `None` says the number is past what a count of repetitions can be kept to,
/// which no reading of the digits can turn into a count.
pub fn decimal<I: UnsynInput>(lit: &LitDec<I>) -> Option<u32> {
    let mut digits = String::from(lit.dec_digit.0.as_str());

    digits.extend(lit.f1.as_str().chars().filter(|c| *c != '_'));

    digits.parse::<u32>().ok()
}

/// The code point a `U+` literal spells, when it spells a character.
///
/// Four hex digits is the shape the reader counted, so the only thing left to
/// answer is whether the number of them is a character, which `D800` through
/// `DFFF` are not.
pub fn code_point<I: UnsynInput>(lit: &LitUnicode<I>) -> Option<char> {
    let digits: String = lit.hex_digit.iter().map(|digit| digit.0.as_str()).collect();

    u32::from_str_radix(&digits, 16)
        .ok()
        .and_then(char::from_u32)
}

/// What the body of a string literal spells: its characters, and the span of
/// every escape among them that spells no character.
///
/// The two quotes bound the literal and carry no value, so only the body is
/// read, and the offsets the refusals carry are the source's because the body
/// says where in the source it began.
pub fn literal<I: UnsynInput>(lit: &LitStr<I>) -> Body {
    string_body(lit.1.as_str(), lit.1.start())
}

/// The characters a string literal spells.
///
/// An escape of no character yields no character: the refusal it is worth is in
/// [`literal`], and the pass that reads a value is the one to say so from that
/// span.
pub fn literal_chars<I: UnsynInput>(lit: &LitStr<I>) -> Vec<char> {
    literal(lit).chars
}

/// Reads the body of a string literal: the text between the two quotes, which
/// the rule fills with plain characters and with the four escapes.
///
/// `at` is where the body begins in the source, which is what the spans of the
/// refusals are measured from.
pub fn string_body(body: &str, at: usize) -> Body {
    let mut read = Body::default();
    // The text left to read, and the source offset it begins at. A `&str` is
    // measured in bytes and so is a span of the source, so the two agree.
    let mut rest = body;
    let mut offset = at;

    while !rest.is_empty() {
        // Where this piece of the body begins, which is what a refusal about an
        // escape points at.
        let here = offset;

        let (c, after) = match rest.chars().next() {
            Some(c) => (c, &rest[c.len_utf8()..]),
            None => break,
        };

        // The characters the rule allows to stand bare. Everything it refuses —
        // the quote that would close the literal, the backslash that opens an
        // escape, and the `CR` no escape writes — has been taken by the parser,
        // so what is left here is one of them and the end of the body.
        if c != '\\' {
            read.chars.push(c);
            offset += c.len_utf8();
            rest = after;
            continue;
        }

        // The backslash itself.
        offset += 1;
        rest = after;

        // The escapes of one character: the quote, the four ASCII controls the
        // spec writes with a letter, and the backslash.
        if let Some((value, after)) = single_escape(rest) {
            read.chars.push(value);
            offset += rest.len() - after.len();
            rest = after;
            continue;
        }

        // `AsciiEscape -> '\\x' OctDigit HexDigit`: seven bits in two digits,
        // both of them ASCII, which is why the count of them is their length.
        if let Some(after) = rest.strip_prefix('x') {
            let digits = take_chars(after, 2);

            if let Some(value) = u32::from_str_radix(digits, 16)
                .ok()
                .and_then(char::from_u32)
            {
                read.chars.push(value);
            }

            offset += 1 + digits.len();
            rest = &after[digits.len()..];
            continue;
        }

        // `UnicodeEscape -> '\\u{' ( HexDigit '_'* ){1, 6} '}';`. The underscores
        // carry no value; the braces bound the digits.
        if let Some(after) = rest.strip_prefix('u') {
            let (digits, closed) = braced(after);
            let spelled: String = digits.chars().filter(|c| *c != '_').collect();

            match u32::from_str_radix(&spelled, 16)
                .ok()
                .and_then(char::from_u32)
            {
                Some(value) => read.chars.push(value),
                None => read.invalid.push(Span::Range(here..offset + 1 + closed)),
            }

            offset += 1 + closed;
            rest = &after[closed..];
            continue;
        }

        // A backslash with none of the four escapes after it. No parser of this
        // rule leaves one here, so nothing more is claimed of the body.
        break;
    }

    read
}

/// The value of a `\`-escape of exactly one character, and what follows it.
/// The value and the rest of a `\`-escape of exactly one character.
fn single_escape(rest: &str) -> Option<(char, &str)> {
    let (c, after) = rest.chars().next().map(|c| (c, &rest[c.len_utf8()..]))?;

    let value = match c {
        '\'' => '\'',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '\\' => '\\',
        '0' => '\0',
        _ => return None,
    };

    Some((value, after))
}

/// The digits inside the braces of a `\u{…}` escape, and how many bytes the
/// escape's own text — the braces and all — runs to.
///
/// An escape without a closing brace is not a thing the parser leaves behind;
/// the whole of what is here is then taken as its digits.
fn braced(escape: &str) -> (&str, usize) {
    let inside = match escape.strip_prefix('{') {
        Some(inside) => inside,
        None => return (escape, escape.len()),
    };

    match inside.find('}') {
        Some(end) => (&inside[..end], escape.len() - inside.len() + end + 1),
        None => (inside, escape.len()),
    }
}

/// The first `count` characters of `text`, which is how many digits an escape
/// asks for.
fn take_chars(text: &str, count: usize) -> &str {
    text.char_indices()
        .nth(count)
        .map_or(text, |(index, _)| &text[..index])
}
