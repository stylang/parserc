//! The reports the generator makes when a definition has no code to run.
//!
//! Codegen runs after [`analyze`](crate::semantics::analyze), so a definition it
//! is given is well formed: its names resolve, its values decode and its
//! recursion is already bounded. What is left to go wrong is that the generator
//! does not know how to emit something *yet* — a pattern shape whose meaning
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
    /// A pattern shape the generator does not emit yet.
    #[error("unsupported pattern: {0}")]
    Unsupported(&'static str),
    /// A name used where codegen needs to know what it denotes.
    #[error("unresolved name `{0}`")]
    Unresolved(String),
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
#[error("{kind}, at {span:?}")]
pub struct CodegenError {
    /// What the generator could not do.
    pub kind: CodegenKind,
    /// The region of the source file the construct was written in.
    pub span: Span,
}

impl CodegenError {
    /// Reports a pattern shape stage 1 does not emit.
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
        let error = CodegenError::unsupported("nested matcher call", Span::Range(4..9));

        assert_eq!(
            error.to_string(),
            "unsupported pattern: nested matcher call, at Range(4..9)"
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
}
