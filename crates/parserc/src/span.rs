/// Span type used by `parserc`.
pub type Span = sourcespan::Span<usize>;

/// Add `before` fun to `Span` object.
pub trait BeforeSpan {
    fn before(&self) -> Self;
}

impl BeforeSpan for Span {
    /// Returns a span with length `0` and followed by this span.
    #[inline]
    fn before(&self) -> Self {
        match self {
            Span::Range(range) => {
                if range.start > 0 {
                    Span::Range(range.start - 1..range.start - 1)
                } else {
                    Span::Range(range.start..range.start)
                }
            }
            Span::RangeFrom(range) => {
                if range.start > 0 {
                    Span::RangeFrom(range.start - 1..)
                } else {
                    Span::RangeFrom(range.start..)
                }
            }
            _ => self.clone(),
        }
    }
}
