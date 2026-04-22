//！ `span` is a region of source code

#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{
    cmp,
    ops::{self, Range},
};

/// A region of one source file,
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Span<Idx> {
    None,
    Range {
        /// The lower bound of the range (inclusive).
        start: Idx,
        /// The upper bound of the range (exclusive).
        end: Idx,
    },
}

impl<Idx> Default for Span<Idx> {
    #[inline]
    fn default() -> Self {
        Self::None
    }
}

impl<Idx> From<Range<Idx>> for Span<Idx> {
    #[inline]
    fn from(value: Range<Idx>) -> Self {
        Self::Range {
            start: value.start,
            end: value.end,
        }
    }
}

impl<Idx> Span<Idx> {
    /// Create a span with empty `value`.
    #[inline]
    pub const fn none() -> Self {
        Self::None
    }

    /// Create a span with a [`Range`]
    #[inline]
    pub const fn new(start: Idx, end: Idx) -> Self {
        Self::Range { start, end }
    }

    #[inline]
    pub fn join(&self, other: &Span<Idx>) -> Self
    where
        Idx: Ord + Clone,
    {
        match (self, other) {
            (Span::None, Span::None) => Span::None,
            (Span::None, Span::Range { start: _, end: _ }) => other.clone(),
            (Span::Range { start: _, end: _ }, Span::None) => self.clone(),
            (Span::Range { start: s1, end: e1 }, Span::Range { start: s2, end: e2 }) => {
                Span::Range {
                    start: cmp::min(s1.clone(), s2.clone()),
                    end: cmp::max(e1.clone(), e2.clone()),
                }
            }
        }
    }
}

impl<Idx> ops::Add for Span<Idx>
where
    Idx: Ord + Clone,
{
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self.join(&rhs)
    }
}

impl<Idx> ops::AddAssign for Span<Idx>
where
    Idx: Ord + Clone,
{
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = self.join(&rhs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join() {
        assert_eq!(Span::from(0..10) + Span::from(5..20), Span::from(0..20));

        assert_eq!(Span::from(2..10) + Span::from(0..20), Span::from(0..20));

        assert_eq!(Span::from(2..10) + Span::from(15..20), Span::from(2..20));
    }
}
