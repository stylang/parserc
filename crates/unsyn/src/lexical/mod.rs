//! The tokens of the `unsyn` language.
//!
//! Every rule of the DSL that matches source text rather than structure lives
//! here: the whitespace a pattern runs over, the punctuation and keywords that
//! separate it, the identifiers and literals it names, and the comments and
//! delimiters that group or set it aside.
//!
//! Two conventions run through the module, because a token is only useful if the
//! text around it can be found again:
//!
//! * A punctuation owns the whitespace on both sides of itself, and its span
//!   covers the whitespace before it together with the punct. A keyword owns only
//!   the whitespace that follows it, and its span covers it together with that
//!   trailing whitespace. The two are asymmetric on purpose: it is what lets a
//!   sequence of tokens tile the source without a gap or an overlap.
//! * A rule that does not fit reports itself *recovably*, which is what lets the
//!   next alternative of an enum be tried. Something the file must have and does
//!   not — a group that was opened and never closed — is reported *fatally*, so
//!   the parse stops there rather than turning one typo into a missing token at
//!   every level above it.

mod s;
pub use s::*;

pub mod comments;
pub mod delimiter;
pub mod ident;
pub mod keyword;
pub mod lit;
pub mod punct;
