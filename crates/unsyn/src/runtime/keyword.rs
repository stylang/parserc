//! Where a keyword of the described language ends.
//!
//! A word-like literal in a pattern — `'let'` — has to be read as a keyword and
//! not as the front of a longer word, so the generated `parse` of one takes the
//! literal and then asks whether the character after it may continue an
//! identifier. The answer is `no` at whitespace, punctuation and the end of the
//! text, and `yes` inside `letter`, which is the case the question exists to
//! refuse.
//!
//! This question belongs to the DSL rather than to any one spec written in it:
//! every spec with a keyword in it asks it, whether or not the spec ever wrote
//! an angle-bracketed call. It is therefore **not** an external. The table a
//! generation is given settles what a `<NAME>` of the spec calls, and a name
//! unsyn asks of itself is no caller's to supply; reaching it instead goes by
//! the `unsyn` path of the generation, the same path that names
//! [`GeneratedInput`](crate::runtime::GeneratedInput). A grammar generated
//! against an empty table has keywords that still end correctly.

/// Whether `c` may continue an identifier of the Unicode standard.
///
/// The generated `parse` of a keyword of more than one character asks this of
/// the character that would come after the keyword, and refuses the keyword
/// when the answer is yes.
pub fn is_xid_continue(c: char) -> bool {
    unicode_ident::is_xid_continue(c)
}

#[cfg(test)]
mod tests {
    use super::is_xid_continue;

    #[test]
    fn a_keyword_ends_where_an_identifier_would_carry_on() {
        // The two answers the generated probe turns on. A character that would
        // carry the word on is refused after a keyword; whitespace,
        // punctuation, and the end of the text are not.
        assert!(is_xid_continue('e') && is_xid_continue('_') && is_xid_continue('词'));
        assert!(!is_xid_continue(' ') && !is_xid_continue('(') && !is_xid_continue('-'));
    }
}
