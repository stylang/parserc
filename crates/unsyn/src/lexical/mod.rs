//! lexical tokens for `unsyn` language.

mod s;
pub use s::*;

pub mod comments;
pub mod delimiter;
pub mod ident;
pub mod keyword;
pub mod lit;
pub mod punct;
