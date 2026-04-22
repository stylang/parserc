//！parser `combinators` for rust language.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod error;
pub use error::*;

mod parser;
pub use parser::*;

#[cfg(feature = "combinators")]
#[cfg_attr(docsrs, doc(cfg(feature = "syntax")))]
#[path = "comb.rs"]
pub mod combinators;

#[cfg(feature = "syntax")]
#[cfg_attr(docsrs, doc(cfg(feature = "syntax")))]
pub mod syntax;
