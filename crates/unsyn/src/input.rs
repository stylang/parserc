//! input type for `unsyn` parser.

use parserc::sourceinput::{self, CharsInput, SplitTo};

use crate::errors::UnsynError;

/// Source code segement for `unsyn` parser.
pub type Chars<'a> = sourceinput::Chars<'a, UnsynError>;

/// `input` type for `unsyn` parser.
pub trait UnsynInput: CharsInput<UnsynError> + SplitTo {}

/// impl [`UnsynInput`] for `Chars<'a, UnsynError>`
impl<'a> UnsynInput for Chars<'a> {}
