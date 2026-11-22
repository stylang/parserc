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
                    Span::None
                } else {
                    Span::RangeFull
                }
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

/// The bounds of a region as the tightest of the two, where a missing bound
/// means the region runs on without one rather than that it holds nothing.
///
/// `Option`'s own ordering would make the missing bound the *smaller*, which is
/// right for a lower bound and backwards for an upper one.
fn tighter<Idx: Ord>(lower: Option<Idx>, upper: Option<Idx>) -> Option<Idx> {
    match (lower, upper) {
        (None, held) | (held, None) => held,
        (Some(one), Some(other)) => Some(cmp::min(one, other)),
    }
}

/// The bounds of a region as the loosest of the two, a missing bound meaning
/// the region runs on without one.
fn looser<Idx: Ord>(lower: Option<Idx>, upper: Option<Idx>) -> Option<Idx> {
    match (lower, upper) {
        (None, held) | (held, None) => held,
        (Some(one), Some(other)) => Some(cmp::max(one, other)),
    }
}

impl<Idx> Span<Idx>
where
    Idx: Ord + Copy,
{
    /// The lowest offset the region holds.
    ///
    /// `None` says the region has no lower bound — which is what
    /// [`Span::RangeFull`] and [`Span::RangeTo`] mean — and not that it holds
    /// nothing; [`Span::None`] answers `None` here too, so asking for a bound is
    /// not how to tell an unbounded region from an empty one. Use
    /// [`is_empty`](Span::is_empty) for that.
    #[inline]
    pub fn start(&self) -> Option<Idx> {
        match self {
            Span::Range(range) => Some(range.start),
            Span::RangeFrom(range) => Some(range.start),
            Span::None | Span::RangeTo(_) | Span::RangeFull => None,
        }
    }

    /// The offset one past the highest the region holds.
    ///
    /// See [`start`](Span::start) for what `None` means.
    #[inline]
    pub fn end(&self) -> Option<Idx> {
        match self {
            Span::Range(range) => Some(range.end),
            Span::RangeTo(range) => Some(range.end),
            Span::None | Span::RangeFrom(_) | Span::RangeFull => None,
        }
    }

    /// Whether the region holds nothing.
    ///
    /// A region open at one end is never empty, since however far it is
    /// bounded it still reaches on.
    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            Span::None => true,
            Span::Range(range) => range.start >= range.end,
            Span::RangeFrom(_) | Span::RangeTo(_) | Span::RangeFull => false,
        }
    }

    /// The region as a plain range.
    ///
    /// `None` for an open region and for [`Span::None`], which is what a caller
    /// that means to index by it wants: neither has both ends to index with.
    #[inline]
    pub fn as_range(&self) -> Option<Range<Idx>>
    where
        Idx: Clone,
    {
        match self {
            Span::Range(range) => Some(range.clone()),
            _ => None,
        }
    }

    /// The region the two hold in common.
    ///
    /// [`Span::None`] takes part in nothing, so it is what an intersection with
    /// it answers with. Any other pair meets in the region bounded by the
    /// tighter of each end, and open ends simply do not tighten anything.
    #[inline]
    pub fn intersect(&self, other: &Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return Span::None;
        }

        let start = looser(self.start(), other.start());
        let end = tighter(self.end(), other.end());

        match (start, end) {
            (Some(one), Some(other_end)) => {
                if one < other_end {
                    Span::Range(one..other_end)
                } else {
                    Span::None
                }
            }
            (Some(one), None) => Span::RangeFrom(one..),
            (None, Some(other_end)) => Span::RangeTo(..other_end),
            (None, None) => Span::RangeFull,
        }
    }

    /// Whether `other` lies wholly inside this region.
    ///
    /// An empty `other` is contained anywhere, including in nothing; this is the
    /// same answer `Range` gives for an empty range, and keeps
    /// `a.contains(&b) == a.intersect(&b) == b` true for bounded regions.
    #[inline]
    pub fn contains(&self, other: &Self) -> bool {
        let below = match (self.start(), other.start()) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(here), Some(there)) => here <= there,
        };

        let above = match (self.end(), other.end()) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(here), Some(there)) => here >= there,
        };

        below && above
    }
}

impl<Idx> Span<Idx>
where
    Idx: Ord + Copy + ops::Sub<Output = Idx> + ops::Add<Output = Idx>,
{
    /// How much the region holds.
    ///
    /// `None` when it is open at either end, since no subtraction reaches past
    /// a bound that was never written.
    #[inline]
    pub fn width(&self) -> Option<Idx> {
        let (start, end) = (self.start()?, self.end()?);

        (start < end).then(|| end - start)
    }

    /// The region recounted from `from`, which becomes its zero.
    ///
    /// This is what turns an offset into the whole source into an offset into
    /// some piece of it: narrow a file to a token's start and every span below
    /// it is measured from there instead.
    ///
    /// `None` when the region begins before `from` — those bytes lie behind the
    /// piece and are not the piece's to name — and for an open-ended region,
    /// which has no start to recount from.
    #[inline]
    pub fn shift(&self, from: Idx) -> Option<Self> {
        let start = self.start()?;

        if start < from {
            return None;
        }

        let start = start - from;

        Some(match self.end() {
            Some(end) => Span::Range(start..end - from),
            None => Span::RangeFrom(start..),
        })
    }

    /// The region moved `by` further along the source.
    #[inline]
    pub fn offset(&self, by: Idx) -> Self {
        match self {
            Span::None => Span::None,
            Span::Range(range) => Span::Range(range.start + by..range.end + by),
            Span::RangeFrom(range) => Span::RangeFrom(range.start + by..),
            Span::RangeTo(range) => Span::RangeTo(..range.end + by),
            Span::RangeFull => Span::RangeFull,
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
