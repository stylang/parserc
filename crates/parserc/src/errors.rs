use crate::Span;

/// Controls how an error affects the parsing process.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ControlFlow {
    /// The parsing process cannot continue; backtracking combinators propagate
    /// this error instead of trying alternatives.
    Fatal,
    /// The parser did not match; an alternative parser may still succeed.
    ///
    /// (The variant name keeps the crate's historical spelling of "recoverable".)
    Recovable,
    /// The input stream ended before the parser could finish.
    Incomplete,
}

/// Error kind returned by the built-in parser combinators.
///
/// Every variant carries its [`ControlFlow`] code and the [`Span`] it points to.
#[derive(thiserror::Error, Debug, PartialEq, Eq, Hash, Clone)]
pub enum Kind {
    #[error("Error from `next` combinator")]
    Next(ControlFlow, Span),
    #[error("Error from `next_if` combinator")]
    NextIf(ControlFlow, Span),
    #[error("Error from `keyword` combinator")]
    Keyword(ControlFlow, Span),
    #[error("Error from parsing syntax `{0}`")]
    Syntax(&'static str, ControlFlow, Span),
    #[error("Error from parsing token `{0}`")]
    Token(&'static str, ControlFlow, Span),
    #[error("Error from parsing syntax `LimitsTo`")]
    LimitsTo(ControlFlow, Span),
    #[error("Error from parsing syntax `Limits`")]
    Limits(ControlFlow, Span),
    #[error("Error from parsing syntax `LimitsFrom`")]
    LimitsFrom(ControlFlow, Span),
    #[error("Error from `take_until`")]
    TakeUntil(ControlFlow, Span),
    #[error("Error from `take_while_range`")]
    TakeWhileRange(ControlFlow, Span),
    #[error("Error from `take_while_from`")]
    TakeWhileFrom(ControlFlow, Span),
    #[error("Error from `take_while_to`")]
    TakeWhileTo(ControlFlow, Span),
    #[error("Detected `left recursion`")]
    LeftRecursion(ControlFlow, Span),
}

/// Error type returned by parser combinators.
pub trait ParseError: From<Kind> {
    /// Returns the [`Span`] this error points to.
    fn to_span(&self) -> Span;
    /// Returns the [`ControlFlow`] code of this error.
    fn control_flow(&self) -> ControlFlow;
    /// Converts this error into a [`ControlFlow::Fatal`] one.
    fn into_fatal(self) -> Self;

    /// Returns `true` when `control_flow() == ControlFlow::Fatal`.
    #[inline]
    fn is_fatal(&self) -> bool {
        self.control_flow() == ControlFlow::Fatal
    }
}

impl ParseError for Kind {
    fn control_flow(&self) -> ControlFlow {
        match self {
            Kind::Next(control_flow, _) => *control_flow,
            Kind::NextIf(control_flow, _) => *control_flow,
            Kind::Keyword(control_flow, _) => *control_flow,
            Kind::Syntax(_, control_flow, _) => *control_flow,
            Kind::LimitsTo(control_flow, _) => *control_flow,
            Kind::Limits(control_flow, _) => *control_flow,
            Kind::LimitsFrom(control_flow, _) => *control_flow,
            Kind::TakeUntil(control_flow, _) => *control_flow,
            Kind::Token(_, control_flow, _) => *control_flow,
            Kind::TakeWhileRange(control_flow, _) => *control_flow,
            Kind::TakeWhileFrom(control_flow, _) => *control_flow,
            Kind::TakeWhileTo(control_flow, _) => *control_flow,
            Kind::LeftRecursion(control_flow, _) => *control_flow,
        }
    }

    fn into_fatal(self) -> Self {
        match self {
            Kind::Next(_, span) => Kind::Next(ControlFlow::Fatal, span),
            Kind::NextIf(_, span) => Kind::NextIf(ControlFlow::Fatal, span),
            Kind::Keyword(_, span) => Kind::Keyword(ControlFlow::Fatal, span),
            Kind::TakeUntil(_, span) => Kind::TakeUntil(ControlFlow::Fatal, span),
            Kind::TakeWhileRange(_, span) => Kind::TakeWhileRange(ControlFlow::Fatal, span),
            Kind::TakeWhileFrom(_, span) => Kind::TakeWhileFrom(ControlFlow::Fatal, span),
            Kind::TakeWhileTo(_, span) => Kind::TakeWhileTo(ControlFlow::Fatal, span),
            Kind::Syntax(name, _, span) => Kind::Syntax(name, ControlFlow::Fatal, span),
            Kind::Token(name, _, span) => Kind::Token(name, ControlFlow::Fatal, span),
            Kind::LimitsTo(_, span) => Kind::LimitsTo(ControlFlow::Fatal, span),
            Kind::Limits(_, span) => Kind::Limits(ControlFlow::Fatal, span),
            Kind::LimitsFrom(_, span) => Kind::LimitsFrom(ControlFlow::Fatal, span),
            Kind::LeftRecursion(_, span) => Kind::LeftRecursion(ControlFlow::Fatal, span),
        }
    }

    fn to_span(&self) -> Span {
        match self {
            Kind::Next(_, span) => span.clone(),
            Kind::NextIf(_, span) => span.clone(),
            Kind::Keyword(_, span) => span.clone(),
            Kind::Syntax(_, _, span) => span.clone(),
            Kind::Token(_, _, span) => span.clone(),
            Kind::LimitsTo(_, span) => span.clone(),
            Kind::Limits(_, span) => span.clone(),
            Kind::TakeUntil(_, span) => span.clone(),
            Kind::TakeWhileRange(_, span) => span.clone(),
            Kind::TakeWhileFrom(_, span) => span.clone(),
            Kind::TakeWhileTo(_, span) => span.clone(),
            Kind::LimitsFrom(_, span) => span.clone(),
            Kind::LeftRecursion(_, span) => span.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLOWS: [ControlFlow; 3] = [
        ControlFlow::Fatal,
        ControlFlow::Recovable,
        ControlFlow::Incomplete,
    ];

    fn span() -> Span {
        Span::Range(3..7)
    }

    /// Builds one `Kind` per enum case, all carrying `flow` and `span()`.
    fn all_kinds(flow: ControlFlow) -> Vec<Kind> {
        vec![
            Kind::Next(flow, span()),
            Kind::NextIf(flow, span()),
            Kind::Keyword(flow, span()),
            Kind::Syntax("node", flow, span()),
            Kind::Token("token", flow, span()),
            Kind::LimitsTo(flow, span()),
            Kind::Limits(flow, span()),
            Kind::LimitsFrom(flow, span()),
            Kind::TakeUntil(flow, span()),
            Kind::TakeWhileRange(flow, span()),
            Kind::TakeWhileFrom(flow, span()),
            Kind::TakeWhileTo(flow, span()),
            Kind::LeftRecursion(flow, span()),
        ]
    }

    #[test]
    fn control_flow_returns_the_attached_code() {
        for flow in FLOWS {
            for kind in all_kinds(flow) {
                assert_eq!(kind.control_flow(), flow);
            }
        }
    }

    #[test]
    fn to_span_returns_the_attached_span() {
        for flow in FLOWS {
            for kind in all_kinds(flow) {
                assert_eq!(kind.to_span(), span());
            }
        }
    }

    #[test]
    fn into_fatal_promotes_every_kind() {
        for flow in FLOWS {
            for kind in all_kinds(flow) {
                let fatal = kind.into_fatal();

                assert!(fatal.is_fatal());
                assert_eq!(fatal.to_span(), span());
            }
        }
    }

    #[test]
    fn is_fatal_matches_the_fatal_flow() {
        assert!(Kind::Next(ControlFlow::Fatal, span()).is_fatal());
        assert!(!Kind::Next(ControlFlow::Recovable, span()).is_fatal());
        assert!(!Kind::Next(ControlFlow::Incomplete, span()).is_fatal());
    }
}
