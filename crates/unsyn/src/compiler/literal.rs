//! What the literals of a file spell: the value the text of a `'…'`, `U+…` or
//! `123` token stands for.
//!
//! Two passes read these. The [analyzer](super::semantic) says whether the
//! values a file spells make sense together; the [generator](super::codegen)
//! writes the values into the code of a parser. Each reports what it finds in
//! its own channel — a diagnostic for the one, a
//! [`BuildError`](super::codegen::BuildError) for the other — but the *reading*
//! of a literal is one rule, stated once here, so that neither pass can drift
//! into a language of its own.
//!
//! A literal is only ever read out of a node that was read out of text, so the
//! shapes here are the shapes the reader already refused anything else than: a
//! backslash that opens no escape, or a `U+` of the wrong count of digits, never
//! reaches this code from a file that parsed.

use super::input::UnsynInput;
use super::lexer;

/// An escape that spelled a number rather than a character.
///
/// `\u{D800}` is a shape the reader accepts, since four to six hex digits is a
/// number, and a meaning only a code point can be asked for. The pass that
/// reads a literal is therefore handed a place it has to refuse again, and this
/// is where it says which place was refused.
pub(crate) struct Escape {
    /// Where the backslash begins, counted in bytes from the first character of
    /// the content of the literal — that is, from one byte after the token
    /// begins.
    pub offset: usize,
    /// How many bytes of the content the escape takes, backslash and all.
    pub len: usize,
}

/// The characters a `'…'` literal spells, and the escapes of it that spelled a
/// number no character is.
///
/// The two quotes bound the literal and carry no value, so only what lies
/// between them is read. An escape of no character yields no character: what
/// the literal spells is the value of the escapes that did, which is what the
/// caller asked for, and the refusals travel beside it rather than stopping the
/// read.
pub(crate) fn chars_of<I>(lit: &lexer::LitStr<I>) -> (Vec<char>, Vec<Escape>)
where
    I: UnsynInput,
{
    // The reader keeps both quotes, so stripping them is a no-op on any literal
    // that was read out of a file that parsed.
    let content = lit
        .0
        .as_str()
        .strip_prefix('\'')
        .and_then(|body| body.strip_suffix('\''))
        .unwrap_or(lit.0.as_str());

    let mut chars = Vec::new();
    let mut escapes = Vec::new();
    let mut from = 0;

    while from < content.len() {
        if !content[from..].starts_with('\\') {
            let c = content[from..]
                .chars()
                .next()
                .expect("a byte left in the content of a literal");

            chars.push(c);
            from += c.len_utf8();
            continue;
        }

        let (code, len) = escape(&content[from + 1..]);

        match code.and_then(char::from_u32) {
            // A character, which is what the escape was written for.
            Some(c) => chars.push(c),
            // A number that is no character. An escape that spells no number at
            // all takes no character either, and is left unremarked: the reader
            // refuses a literal it finds one in, so a tree that reached here
            // was never read out of a file that parsed.
            None => {
                if code.is_some() {
                    escapes.push(Escape { offset: from, len });
                }
            }
        }

        from += 1 + len;
    }

    (chars, escapes)
}

/// The code point a `U+…` literal spells, when it spells a character.
///
/// Four hex digits is the shape the rule asks for and the shape the reader
/// counted, so the only thing left to answer is whether the number of them is a
/// character, which `D800` through `DFFF` are not.
pub(crate) fn code_of<I>(lit: &lexer::LitUnicode<I>) -> Option<char>
where
    I: UnsynInput,
{
    let digits = lit.0.as_str().strip_prefix("U+").unwrap_or(lit.0.as_str());

    u32::from_str_radix(digits, 16)
        .ok()
        .and_then(char::from_u32)
}

/// The number a `123` literal spells, with the underscores the rule allows
/// between its digits dropped.
///
/// `None` says the number is past what a count of repetitions can be kept to,
/// which no reading of the digits can turn into a count.
pub(crate) fn count_of<I>(lit: &lexer::LitDec<I>) -> Option<u32>
where
    I: UnsynInput,
{
    lit.0.as_str().replace("_", "").parse::<u32>().ok()
}

/// The escape that begins at `rest`, which is the text just after a backslash.
///
/// It returns the code the escape spells and the number of bytes of `rest` it
/// takes, backslash not counted. A code that is no character is handed back
/// rather than refused, since the reader has already said whether the escape is
/// one of the shapes the spec writes, and only the pass that reads a value knows
/// whether the number of it is a character.
fn escape(rest: &str) -> (Option<u32>, usize) {
    let one = |c: char| (Some(c as u32), 1);

    match rest.chars().next() {
        Some('\'') => one('\''),
        Some('\\') => one('\\'),
        Some('n') => one('\n'),
        Some('r') => one('\r'),
        Some('t') => one('\t'),
        Some('0') => one('\0'),
        // `\x` OctDigit HexDigit: the two digits of a code of seven bits.
        Some('x') => {
            let digits: String = rest[1..].chars().take(2).collect();

            match u32::from_str_radix(&digits, 16) {
                Ok(code) => (Some(code), 1 + digits.len()),
                Err(_) => (None, 1),
            }
        }
        // `\u{` digits `}`: from one digit to six, which underscores may separate.
        Some('u') => {
            let Some(body) = rest[1..].strip_prefix('{') else {
                return (None, 1);
            };

            let Some(ends) = body.find('}') else {
                return (None, 1);
            };

            let digits: String = body[..ends].chars().filter(|c| *c != '_').collect();

            match u32::from_str_radix(&digits, 16) {
                Ok(code) => (Some(code), 2 + ends + 1),
                Err(_) => (None, 1),
            }
        }
        // A backslash that opens none of the escapes the spec writes.
        _ => (None, 1),
    }
}

#[cfg(test)]
mod tests {
    use parserc::syntax::SyntaxInput;

    use super::*;
    use crate::compiler::input::TokenStream;

    /// Reads `'…'` out of a file and then spells what the literal means.
    ///
    /// The second list is how many bytes of the content each refusal took,
    /// which is all the passes that read a literal ask of one.
    fn escaped(body: &str) -> (Vec<char>, Vec<usize>) {
        let mut input = TokenStream::from(body);

        let lit = input
            .parse::<lexer::LitStr<TokenStream>>()
            .expect("the literal should read");

        let (chars, escapes) = chars_of(&lit);

        (chars, escapes.into_iter().map(|e| e.len).collect())
    }

    #[test]
    fn a_literal_spells_the_characters_it_holds() {
        assert_eq!(escaped("'ab'").0, vec!['a', 'b']);
        assert_eq!(escaped("'\\\\'").0, vec!['\\']);
        assert_eq!(escaped("'\\''").0, vec!['\'']);
        assert_eq!(escaped("'\\n\\r\\t\\0'").0, vec!['\n', '\r', '\t', '\0']);
    }

    #[test]
    fn an_escape_spells_a_number_of_its_own() {
        // `\x` takes two hex digits, and `\u{}` from one to six.
        assert_eq!(escaped("'\\x7f'").0, vec!['\u{7f}']);
        assert_eq!(escaped("'\\u{1F600}'").0, vec!['\u{1F600}']);
        assert_eq!(escaped("'\\u{7FF}_*'").0, vec!['\u{7FF}', '_', '*']);

        // A number that is no character is handed back as a refusal, and leaves
        // no character behind.
        assert_eq!(escaped("'\\u{D800}a'"), (vec!['a'], vec![7]));
    }

    #[test]
    fn a_code_point_literal_is_a_character_or_nothing() {
        let mut input = TokenStream::from("U+0009");
        let lit = input
            .parse::<lexer::LitUnicode<TokenStream>>()
            .expect("the literal should read");
        assert_eq!(code_of(&lit), Some('\u{0009}'));

        let mut input = TokenStream::from("U+D800");
        let lit = input
            .parse::<lexer::LitUnicode<TokenStream>>()
            .expect("the literal should read");
        assert_eq!(code_of(&lit), None);
    }

    #[test]
    fn a_count_of_repetitions_drops_its_underscores() {
        let mut input = TokenStream::from("1_000");
        let lit = input
            .parse::<lexer::LitDec<TokenStream>>()
            .expect("the count should read");
        assert_eq!(count_of(&lit), Some(1_000));

        // Past what a count can be kept to, and so no count at all.
        let mut input = TokenStream::from("4294967296");
        let lit = input
            .parse::<lexer::LitDec<TokenStream>>()
            .expect("the count should read");
        assert_eq!(count_of(&lit), None);
    }
}
