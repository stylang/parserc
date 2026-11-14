//! The reports the generator makes when a definition has no code to run.
//!
//! Codegen runs after [`analyze`](crate::semantics::analyze), so a definition it
//! is given is well formed: its names resolve, its values decode and its
//! recursion is already bounded. What is left to go wrong is that the generator
//! does not know how to emit something *yet* — a pattern whose meaning
//! needs machinery stage 1 does not carry.
//!
//! Those are reported rather than guessed at: a generated file that quietly
//! dropped a rule would be worse than no file at all, because its parsers would
//! accept less than the source says while still looking like it. Every report
//! names the construct and points at the span it was written at.

use parserc::Span;
use thiserror::Error;

/// Why one construct could not be turned into code.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CodegenKind {
    /// A pattern the generator does not emit yet.
    #[error("unsupported pattern: {0}")]
    Unsupported(&'static str),
    /// A name used where codegen needs to know what it denotes.
    #[error("unresolved name `{0}`")]
    Unresolved(String),
    /// An external call no table gives a function to run.
    #[error("nothing is mapped to `<{0}>`: say what it calls with `Codegen::with_externals`")]
    Unmapped(String),
    /// A rule that has to be a token, reading text, but is written as a node.
    #[error("`{0}` is not a token rule: {1}")]
    NotAToken(String, String),
    /// A node name that cannot be the name of a Rust type.
    #[error("`{0}` cannot be used as a Rust identifier")]
    InvalidName(String),
    /// A repetition bound that decodes to nothing.
    #[error("repetition bound that cannot be evaluated")]
    InvalidRepeat,
    /// A pattern that would loop forever without consuming input.
    #[error("repetition of `{0}`, which can match nothing, would loop forever")]
    EmptyRepetition(String),
}

/// One construct that could not be generated, where it was written.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub struct CodegenError {
    /// What the generator could not do.
    pub kind: CodegenKind,
    /// The region of the source file the construct was written in.
    pub span: Span,
}

/// A report says what could not be done, and where it was written when the
/// generator has a place to point at.
///
/// The `Emitter` of the `scan` module works from a compiled pattern rather than
/// from the source, so what it refuses beyond a `<NAME>` has no span of its
/// own; saying `at None` would read as a location the generator gave up on
/// rather than one it never had.
impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.span {
            Span::None => write!(f, "{}", self.kind),
            span => write!(f, "{}, at {span:?}", self.kind),
        }
    }
}

impl CodegenError {
    /// Reports a pattern stage 1 does not emit.
    #[inline]
    pub fn unsupported(what: &'static str, span: Span) -> Self {
        Self {
            kind: CodegenKind::Unsupported(what),
            span,
        }
    }

    /// Reports a name that denotes nothing codegen can reach.
    #[inline]
    pub fn unresolved(name: &str, span: Span) -> Self {
        Self {
            kind: CodegenKind::Unresolved(name.to_owned()),
            span,
        }
    }

    /// Reports an external call the table of the run gives no function to.
    #[inline]
    pub fn unmapped(name: &str, span: Span) -> Self {
        Self {
            kind: CodegenKind::Unmapped(name.to_owned()),
            span,
        }
    }

    /// Reports that one of the `lexer` rules is written against nodes.
    #[inline]
    pub fn not_a_token(name: &str, why: &str, span: Span) -> Self {
        Self {
            kind: CodegenKind::NotAToken(name.to_owned(), why.to_owned()),
            span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_names_the_construct_and_where_it_is() {
        let error = CodegenError::unsupported("nested external call", Span::Range(4..9));

        assert_eq!(
            error.to_string(),
            "unsupported pattern: nested external call, at Range(4..9)"
        );
    }

    #[test]
    fn a_stuck_repetition_says_what_loops() {
        let error = CodegenError {
            kind: CodegenKind::EmptyRepetition("SPACE".to_owned()),
            span: Span::Range(0..9),
        };

        assert_eq!(
            error.to_string(),
            "repetition of `SPACE`, which can match nothing, would loop forever, at Range(0..9)"
        );
    }

    #[test]
    fn a_report_without_a_place_in_the_file_stops_at_what_it_could_not_do() {
        let error = CodegenError::unmapped("WORD_CARRY_ON", Span::None);

        assert_eq!(
            error.to_string(),
            "nothing is mapped to `<WORD_CARRY_ON>`: say what it calls with `Codegen::with_externals`"
        );
    }
}
