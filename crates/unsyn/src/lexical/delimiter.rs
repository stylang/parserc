//! token `brackets`

use parserc::syntax::Delimiter;

use super::punct::*;

/// Curly braces
pub type Brace<I, T> = Delimiter<BraceStart<I>, BraceEnd<I>, T>;
/// Square brackets
pub type Bracket<I, T> = Delimiter<BracketStart<I>, BracketEnd<I>, T>;
/// Parentheses
pub type Paren<I, T> = Delimiter<ParenStart<I>, ParenEnd<I>, T>;
/// Angle brackets
pub type Angle<I, T> = Delimiter<Lt<I>, Gt<I>, T>;
