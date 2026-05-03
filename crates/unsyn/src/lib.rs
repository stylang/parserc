//! A domain-specific language for building Concrete Syntax Trees, developed by `parserc`.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod errors;
pub mod input;
pub mod semantics;
pub mod syntax;
pub mod token;
pub mod visit;
