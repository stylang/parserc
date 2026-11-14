//! The names of the types the generator emits.
//!
//! A `.uns` file already names every node it defines, and those names are the
//! names of the Rust types: `lexer IDENTIFIER` becomes `IDENTIFIER<I>`. What
//! needs a name that the file does not give is everything the pattern only
//! *implies* — the token for a punctuation mark, the node for a parenthesized
//! alternation, the scanner for a set used inside a rule — and those are named
//! from what they match, in the style [`crate::lexical`] names its own.
//!
//! Two rules keep the emitted file compiling:
//!
//! * A name is never reused. [`Names`] hands out `Foo`, then `Foo_2`, then
//!   `Foo_3`, so a node called `Or` and the `'|'` token called `Or` can live in
//!   one file.
//! * A name derived from text is made into something Rust accepts: a word keeps
//!   its spelling with a leading capital, and the characters that are not words
//!   are spelled out (`;` is `Semi`, `->` is `ArrowRight`, anything the table
//!   does not know becomes `U003B`).

use proc_macro2::Ident;
use unicode_ident::{is_xid_continue, is_xid_start};

/// The punctuation whose name is worth remembering, rather than spelling out.
const PUNCT_WORDS: &[(&str, &str)] = &[
    ("!", "Bang"),
    ("\"", "DoubleQuote"),
    ("#", "Sharp"),
    ("$", "Dollar"),
    ("%", "Percent"),
    ("&", "Amp"),
    ("'", "Quote"),
    ("(", "ParenStart"),
    (")", "ParenEnd"),
    ("*", "Star"),
    ("+", "Plus"),
    (",", "Comma"),
    ("-", "Minus"),
    (".", "Dot"),
    ("/", "Slash"),
    ("::", "ColonColon"),
    (":", "Colon"),
    (";", "Semi"),
    ("<", "Lt"),
    ("=", "Eq"),
    (">", "Gt"),
    ("?", "Question"),
    ("@", "At"),
    ("[", "BracketStart"),
    ("]", "BracketEnd"),
    ("^", "Caret"),
    ("_", "Underscore"),
    ("`", "Backtick"),
    ("|", "Or"),
    ("}", "BraceEnd"),
    ("{", "BraceStart"),
    ("~", "Tilde"),
];

/// The multi-character punctuation that is read as one name, not two.
const PUNCT_PAIRS: &[(&str, &str)] = &[
    ("->", "ArrowRight"),
    ("<-", "ArrowLeft"),
    ("=>", "FatArrow"),
    ("==", "EqEq"),
    ("!=", "NotEq"),
    ("..", "DotDot"),
    ("...", "DotDotDot"),
    ("&&", "AmpAmp"),
    ("||", "OrOr"),
    ("**", "StarStar"),
    ("//", "SlashSlash"),
];

/// Returns `true` when `text` reads as a word: it could be an identifier of the
/// language the file describes.
///
/// A word matched in a pattern is a keyword, which is different from punctuation
/// in more than its shape: it may not be followed by characters that would
/// continue an identifier, and it owns the whitespace after it rather than the
/// whitespace on both sides of it.
pub(crate) fn is_word(text: &str) -> bool {
    let mut chars = text.chars();

    let Some(first) = chars.next() else {
        return false;
    };

    (first == '_' || is_xid_start(first)) && chars.all(is_xid_continue)
}

/// Returns `true` when `name` can be the name of a Rust type as it stands.
///
/// An underscore is allowed to begin a name and to be all of one only apart:
/// `_` is the hole a pattern leaves, not a name, and a type called that is no
/// type at all. A keyword of Rust is no name either.
pub(crate) fn is_type_name(name: &str) -> bool {
    let mut chars = name.chars();

    let Some(first) = chars.next() else {
        return false;
    };

    name != "_"
        && (first == '_' || is_xid_start(first))
        && chars.all(is_xid_continue)
        && !is_keyword(name)
}

/// The words Rust spends on its own grammar, which no type of a generated file
/// may be called. Anything else that begins with an underscore or a letter and
/// is made of such characters is a name.
const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while", "yield",
];

/// Returns `true` when `name` is a keyword of Rust, which is the one case the
/// shape of an identifier does not decide.
pub(crate) fn is_keyword(name: &str) -> bool {
    KEYWORDS.contains(&name)
}

/// The type name for the keyword `text`, as `lexer` becomes `Lexer`.
pub(crate) fn keyword_name(text: &str) -> String {
    let mut out = String::with_capacity(text.len());

    for (index, c) in text.chars().enumerate() {
        if index == 0 {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
    }

    out
}

/// The type name for the punctuation `text`, as `;` becomes `Semi`.
///
/// A character the table does not know is spelled as its code point, so that two
/// different punctuations never share a name.
pub(crate) fn punct_name(text: &str) -> String {
    if let Some((_, name)) = PUNCT_PAIRS.iter().find(|(punct, _)| *punct == text) {
        return (*name).to_owned();
    }

    let mut out = String::new();

    for c in text.chars() {
        match PUNCT_WORDS.iter().find(|(punct, _)| punct.chars().eq([c])) {
            Some((_, name)) => out.push_str(name),
            None => out.push_str(&format!("U{:04X}", c as u32)),
        }
    }

    out
}

/// The type name for the literal `text` used as an operand of a pattern: a
/// keyword if it reads as a word, punctuation otherwise.
///
/// A word that is not a name — `_` reads as a word but is no type's name — is
/// spelled as punctuation is, since the alternative would be an identifier Rust
/// does not have.
pub(crate) fn literal_name(text: &str) -> String {
    if is_word(text) {
        let word = keyword_name(text);

        if is_type_name(&word) {
            return word;
        }
    }

    punct_name(text)
}

/// The names handed out for the types codegen emits, with no duplicates.
#[derive(Debug, Default)]
pub(crate) struct Names {
    /// The names already handed out, so that a later one can be renamed.
    used: std::collections::HashSet<String>,
}

impl Names {
    /// Creates a registry that already reserves the names a file gives its own
    /// definitions, which are the ones codegen may not overwrite.
    pub(crate) fn with_reserved<'a>(reserved: impl IntoIterator<Item = &'a str>) -> Self {
        let mut names = Self::default();

        for name in reserved {
            names.reserve(name);
        }

        names
    }

    /// Marks `name` as taken, without asking for a usable variant of it.
    pub(crate) fn reserve(&mut self, name: &str) {
        self.used.insert(name.to_owned());
    }

    /// Returns `base`, renamed only if a type already claimed that name.
    ///
    /// A definition's own name goes through this too, so a file that names two
    /// nodes alike is caught here rather than emitting a duplicate type.
    ///
    /// Returns `None` when `base` is no name of Rust at all, which is what
    /// leaves the choice of what to report about it to the caller.
    pub(crate) fn register(&mut self, base: &str) -> Option<String> {
        if !is_type_name(base) {
            return None;
        }
        if self.used.contains(base) {
            let mut next = 2;

            loop {
                let candidate = format!("{base}_{next}");

                if !self.used.contains(&candidate) {
                    self.used.insert(candidate.clone());
                    return Some(candidate);
                }

                next += 1;
            }
        } else {
            self.used.insert(base.to_owned());
            Some(base.to_owned())
        }
    }

    /// Returns `base` for a type codegen names itself, which is allowed to step
    /// aside: `Foo` becomes `Foo_2` rather than reporting a collision, because
    /// nothing in the source asked for either name.
    pub(crate) fn register_anonymous(&mut self, base: &str) -> String {
        if let Some(name) = self.register(base) {
            return name;
        }

        // A base Rust would not accept is given a head that it is a name of a
        // type, since what it stands for is still needed by the file: nothing
        // in the source asked for this either, and a type left out would be.
        let prefixed = format!("Node{base}");

        self.register(&prefixed)
            .expect("a name with a head is a name Rust takes")
    }
}

/// The identifier for the type named `name`.
pub(crate) fn type_ident(name: &str) -> Ident {
    Ident::new(name, proc_macro2::Span::call_site())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_is_read_as_a_word() {
        assert!(is_word("lexer"));
        assert!(is_word("_"));
        assert!(is_word("x_1"));
        assert!(is_word("复制"));

        // Punctuation, and a word with punctuation in it, are not words.
        assert!(!is_word("->"));
        assert!(!is_word("a-b"));
        assert!(!is_word("1abc"));
        assert!(!is_word(""));
    }

    #[test]
    fn a_name_is_a_type_name_only_when_rust_agrees() {
        assert!(is_type_name("IDENTIFIER"));
        assert!(is_type_name("_Private"));
        assert!(!is_type_name("1NCORRECT"));
        assert!(!is_type_name("a-b"));
        assert!(!is_type_name(""));
    }

    #[test]
    fn punctuation_is_named_by_what_it_is() {
        assert_eq!(punct_name(";"), "Semi");
        assert_eq!(punct_name("->"), "ArrowRight");
        assert_eq!(punct_name("**"), "StarStar");

        // An unknown character is spelled as its code point, so that the name is
        // never accidentally the name of another punctuation.
        assert_eq!(punct_name("\u{0001}"), "U0001");

        // A word is a keyword, and keeps its own spelling with a capital head.
        assert_eq!(keyword_name("concat"), "Concat");
        assert_eq!(literal_name("lexer"), "Lexer");
        assert_eq!(literal_name("("), "ParenStart");
    }

    #[test]
    fn a_taken_name_is_numbered_instead_of_reused() {
        let mut names = Names::with_reserved(["Or"]);

        assert_eq!(names.register("Expr").as_deref(), Some("Expr"));
        assert_eq!(names.register("Or").as_deref(), Some("Or_2"));
        assert_eq!(names.register("Or").as_deref(), Some("Or_3"));
        assert_eq!(names.register_anonymous("Expr"), "Expr_2");
    }

    #[test]
    fn the_reserved_names_are_the_ones_the_language_already_has() {
        // The table covers the punctuation `bootstrap.uns` itself is written
        // with, which is the set of names a generated file is most likely to
        // want to read well.
        for punct in [
            "~", "(", ")", "[", "]", "<", ">", "'", "{", "}", ",", "-", ";", "|", "*",
        ] {
            assert_ne!(punct_name(punct), "", "{punct}");
            assert!(
                is_type_name(&punct_name(punct)),
                "{} names a type",
                punct_name(punct)
            );
        }
    }
}
