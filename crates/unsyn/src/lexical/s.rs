use parserc::{syntax::Syntax, take_while_with};

use crate::input::UnsynInput;

/// whitespace characters: `\r,\n,...`
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Syntax)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct S<I>(#[parserc(parser = take_while_with(1..,|c: char| c.is_whitespace()) )] pub I)
where
    I: UnsynInput;
