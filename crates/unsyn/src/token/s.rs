use std::num::NonZeroUsize;

use parserc::{combinators::take_while_at_least_n, syntax::Syntax};

use crate::input::UnsynInput;

/// whitespace characters: `\r,\n,...`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct S<I>(
    #[parserc(parser = take_while_at_least_n(NonZeroUsize::new(1).unwrap(),|c: char| c.is_whitespace()))]
    pub I,
)
where
    I: UnsynInput;
