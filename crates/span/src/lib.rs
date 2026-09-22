//! Source code regions ([`Span`]) shared by the `parserc` toolchain.
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{
    cmp,
    ops::{self, Range, RangeFrom, RangeFull, RangeTo},
};

/// A region of source code.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Span<Idx> {
    /// No value.
    None,
    /// A (half-open) range bounded inclusively below and exclusively above (start..end in a future edition).
    Range(Range<Idx>),
    /// A range only bounded inclusively below (start..).
    RangeFrom(RangeFrom<Idx>),
    /// A range only bounded exclusively above (..end).
    RangeTo(RangeTo<Idx>),
    /// Match the whole source code.
    RangeFull,
}

impl<Idx> From<Range<Idx>> for Span<Idx> {
    fn from(value: Range<Idx>) -> Self {
        Self::Range(value)
    }
}

impl<Idx> From<RangeFrom<Idx>> for Span<Idx> {
    fn from(value: RangeFrom<Idx>) -> Self {
        Self::RangeFrom(value)
    }
}

impl<Idx> From<RangeTo<Idx>> for Span<Idx> {
    fn from(value: RangeTo<Idx>) -> Self {
        Self::RangeTo(value)
    }
}

impl<Idx> From<RangeFull> for Span<Idx> {
    fn from(_: RangeFull) -> Self {
        Self::RangeFull
    }
}

impl<Idx> Span<Idx>
where
    Idx: Ord + Copy,
{
    /// Returns the region lying between the two spans.
    ///
    /// Returns [`Span::None`] when the two spans intersect.
    #[inline]
    pub fn between(&self, other: &Self) -> Span<Idx> {
        match (self, other) {
            (Span::None, Span::None) => Span::None,
            (Span::None, Span::Range(_)) => Span::None,
            (Span::None, Span::RangeFrom(_)) => Span::None,
            (Span::None, Span::RangeTo(_)) => Span::None,
            (Span::Range(_), Span::None) => Span::None,
            (Span::Range(range), Span::Range(other_range)) => {
                if range.end < other_range.start {
                    Span::Range(range.end..other_range.start)
                } else if other_range.end < range.start {
                    Span::Range(other_range.end..range.start)
                } else {
                    Span::None
                }
            }
            (Span::Range(range), Span::RangeFrom(range_from)) => {
                if range.end < range_from.start {
                    Span::Range(range.end..range_from.start)
                } else {
                    Span::None
                }
            }
            (Span::Range(range), Span::RangeTo(range_to)) => {
                if range_to.end < range.start {
                    Span::Range(range_to.end..range.start)
                } else {
                    Span::None
                }
            }
            (Span::RangeFrom(_), Span::None) => Span::None,
            (Span::RangeFrom(range_from), Span::Range(range)) => {
                if range.end < range_from.start {
                    Span::Range(range.end..range_from.start)
                } else {
                    Span::None
                }
            }
            (Span::RangeFrom(_), Span::RangeFrom(_)) => Span::None,
            (Span::RangeFrom(range_from), Span::RangeTo(range_to)) => {
                if range_to.end < range_from.start {
                    Span::Range(range_to.end..range_from.start)
                } else {
                    Span::None
                }
            }
            (Span::RangeTo(_), Span::None) => Span::None,
            (Span::RangeTo(range_to), Span::Range(range)) => {
                if range_to.end < range.start {
                    Span::Range(range_to.end..range.start)
                } else {
                    Span::None
                }
            }
            (Span::RangeTo(range_to), Span::RangeFrom(range_from)) => {
                if range_to.end < range_from.start {
                    Span::Range(range_to.end..range_from.start)
                } else {
                    Span::None
                }
            }
            (Span::RangeTo(_), Span::RangeTo(_)) => Span::None,
            (Span::None, Span::RangeFull) => Span::None,
            (Span::Range(_), Span::RangeFull) => Span::None,
            (Span::RangeFrom(_), Span::RangeFull) => Span::None,
            (Span::RangeTo(_), Span::RangeFull) => Span::None,
            (Span::RangeFull, Span::None) => Span::None,
            (Span::RangeFull, Span::Range(_)) => Span::None,
            (Span::RangeFull, Span::RangeFrom(_)) => Span::None,
            (Span::RangeFull, Span::RangeTo(_)) => Span::None,
            (Span::RangeFull, Span::RangeFull) => Span::None,
        }
    }

    /// Merges the two spans into the smallest span covering both.
    #[inline]
    pub fn union(&self, other: &Self) -> Self {
        match (self, other) {
            (Span::None, Span::None) => Span::None,
            (Span::None, Span::Range(_)) => other.clone(),
            (Span::None, Span::RangeFrom(_)) => other.clone(),
            (Span::None, Span::RangeTo(_)) => other.clone(),
            (Span::Range(_), Span::None) => self.clone(),
            (Span::Range(range), Span::Range(other_range)) => {
                let start = cmp::min(range.start, other_range.start);
                let end = cmp::max(range.end, other_range.end);

                Span::Range(start..end)
            }
            (Span::Range(range), Span::RangeFrom(range_from)) => {
                let start = cmp::min(range.start, range_from.start);

                Span::RangeFrom(start..)
            }
            (Span::Range(range), Span::RangeTo(range_to)) => {
                let end = cmp::max(range.end, range_to.end);

                Span::RangeTo(..end)
            }
            (Span::RangeFrom(_), Span::None) => self.clone(),
            (Span::RangeFrom(range_from), Span::Range(range)) => {
                if range_from.start > range.end {
                    Span::None
                } else {
                    Span::Range(range_from.start..range.end)
                }
            }
            (Span::RangeFrom(range_from), Span::RangeFrom(other_range_from)) => {
                let start = cmp::min(range_from.start, other_range_from.start);
                Span::RangeFrom(start..)
            }
            (Span::RangeFrom(_), Span::RangeTo(_)) => Span::RangeFull,
            (Span::RangeTo(_), Span::None) => self.clone(),
            (Span::RangeTo(range_to), Span::Range(range)) => {
                let end = cmp::max(range_to.end, range.end);

                Span::RangeTo(..end)
            }
            (Span::RangeTo(range_to), Span::RangeFrom(range_from)) => {
                if range_to.end < range_from.start {
                    return Span::None;
                }

                return Span::RangeFull;
            }
            (Span::RangeTo(range_to), Span::RangeTo(other_range_to)) => {
                let end = cmp::max(range_to.end, other_range_to.end);

                Span::RangeTo(..end)
            }
            (Span::None, Span::RangeFull) => Span::RangeFull,
            (Span::Range(_), Span::RangeFull) => Span::RangeFull,
            (Span::RangeFrom(_), Span::RangeFull) => Span::RangeFull,
            (Span::RangeTo(_), Span::RangeFull) => Span::RangeFull,
            (Span::RangeFull, Span::None) => Span::RangeFull,
            (Span::RangeFull, Span::Range(_)) => Span::RangeFull,
            (Span::RangeFull, Span::RangeFrom(_)) => Span::RangeFull,
            (Span::RangeFull, Span::RangeTo(_)) => Span::RangeFull,
            (Span::RangeFull, Span::RangeFull) => Span::RangeFull,
        }
    }
}

impl<Idx> ops::Add for Span<Idx>
where
    Idx: Ord + Copy,
{
    type Output = Span<Idx>;

    fn add(self, rhs: Self) -> Self::Output {
        self.union(&rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_between() {
        assert_eq!(
            Span::Range(0..10).between(&Span::Range(12..20)),
            Span::Range(10..12)
        );

        assert_eq!(
            Span::Range(12..20).between(&Span::Range(0..10)),
            Span::Range(10..12)
        );

        assert_eq!(Span::Range(12..20).between(&Span::Range(0..14)), Span::None);

        assert_eq!(
            Span::RangeFrom(15..).between(&Span::Range(0..14)),
            Span::Range(14..15)
        );

        assert_eq!(
            Span::Range(0..14).between(&Span::RangeFrom(15..)),
            Span::Range(14..15)
        );

        assert_eq!(
            Span::RangeFrom(15..).between(&Span::RangeTo(..14)),
            Span::Range(14..15)
        );

        assert_eq!(
            Span::RangeFrom(14..).between(&Span::RangeTo(..15)),
            Span::None
        );

        assert_eq!(
            Span::RangeFrom(14..).between(&Span::RangeFrom(15..)),
            Span::None
        );

        assert_eq!(
            Span::RangeTo(..14).between(&Span::RangeTo(..18)),
            Span::None
        );

        assert_eq!(
            Span::RangeTo(..18).between(&Span::RangeTo(..14)),
            Span::None
        );
    }

    #[test]
    fn test_union() {
        assert_eq!(
            Span::RangeTo(..14).union(&Span::RangeFrom(18..)),
            Span::None
        );

        assert_eq!(
            Span::Range(1..14).union(&Span::Range(13..18)),
            Span::Range(1..18)
        );

        assert_eq!(
            Span::Range(1..14).union(&Span::Range(2..14)),
            Span::Range(1..14)
        );

        assert_eq!(
            Span::Range(4..14).union(&Span::Range(3..4)),
            Span::Range(3..14)
        );

        assert_eq!(
            Span::Range(4..14).union(&Span::Range(1..5)),
            Span::Range(1..14)
        );

        assert_eq!(
            Span::RangeTo(..18).union(&Span::RangeTo(..14)),
            Span::RangeTo(..18)
        );
    }
}
