//! The stream a build of `unsyn.us` reads, and the alias it is read through.
//!
//! This file is generated: see `tests/unsyn_us.rs`. Do not edit it.
#![allow(
    dead_code,
    clippy::manual_is_ascii_check,
    clippy::needless_borrow,
    clippy::type_complexity,
    clippy::enum_variant_names
)]
pub trait UnsynInput:
    ::parserc::Input<Item = char, Error = ::unsyn::errors::UnsynError>
    + ::parserc::AsBytes
    + ::parserc::AsStr
    + ::parserc::StartWith<&'static str>
    + ::parserc::StartWith<&'static [u8]>
    + ::parserc::Find<&'static str>
    + ::parserc::Find<&'static [u8]>
    + ::core::clone::Clone
    + ::core::fmt::Debug
    + ::core::cmp::PartialEq
{
}
pub type TokenStream<'a> = ::parserc::chars::TokenStream<'a, ::unsyn::errors::UnsynError>;
impl<'a> UnsynInput for TokenStream<'a> {}
