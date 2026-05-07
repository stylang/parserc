//! input type for `unsyn` parser.

use parserc::sourceinput::{self, CharsInput, SplitTo};

use crate::errors::CompileError;

/// Source code segement for `unsyn` parser.
pub type Chars<'a> = sourceinput::Chars<'a, CompileError>;

/// `input` type for `unsyn` parser.
pub trait UnsynInput: CharsInput<CompileError> + SplitTo {}

/// impl [`UnsynInput`] for `Chars<'a, UnsynError>`
impl<'a> UnsynInput for Chars<'a> {}
