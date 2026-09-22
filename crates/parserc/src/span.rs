/// A region of source code, indexed by byte offsets.
pub type Span = sourcespan::Span<usize>;

/// Extension trait for the span that precedes another span.
pub trait BeforeSpan {
    /// Returns `self` moved one byte to the left, clamped at offset `0`.
    ///
    /// [`Span::Range`] collapses to a zero-width span just before its start,
    /// which makes a handy "expected … here" error position;
    /// [`Span::RangeFrom`] is shifted one byte left, other variants are
    /// returned unchanged.
    fn before(&self) -> Self;
}

impl BeforeSpan for Span {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn before_collapses_a_range_to_its_predecessor() {
        assert_eq!(Span::Range(5..7).before(), Span::Range(4..4));
    }

    #[test]
    fn before_clamps_a_range_at_the_origin() {
        assert_eq!(Span::Range(0..7).before(), Span::Range(0..0));
    }

    #[test]
    fn before_shifts_a_range_from_left() {
        assert_eq!(Span::RangeFrom(5..).before(), Span::RangeFrom(4..));
        assert_eq!(Span::RangeFrom(0..).before(), Span::RangeFrom(0..));
    }

    #[test]
    fn before_leaves_other_span_kinds_unchanged() {
        assert_eq!(Span::RangeTo(..4).before(), Span::RangeTo(..4));
        assert_eq!(Span::RangeFull.before(), Span::RangeFull);
        assert_eq!(Span::None.before(), Span::None);
    }
}
