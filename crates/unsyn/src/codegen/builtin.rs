//! The matcher names a generated file may call without a backend, which is a
//! thing the generator has to know and the generated file does not.
//!
//! A `<NAME>` in a pattern stands for text this file's rules do not describe, so
//! the analysis of a spec collects the names it expects from outside and the
//! generator emits each as a call of the `matcher` path it is given. Two names
//! are excepted: the two character classes of `unicode_ident`, which this crate
//! writes in `runtime::matcher` for a generated file to call, so a grammar that
//! only needs an identifier can be generated before its backend exists.
//!
//! The two cases do not generate the same thing. A name that is already provided
//! is the same program wherever it is called, so one scanner the whole file
//! shares serves it; a name the backend must give means whatever that backend
//! says it means, so two rules may mean two things by one name and each gets a
//! type of its own. The fork is taken in `shape`, which is the only reader of
//! the list here.
//!
//! Keeping the list on this side of the split is what keeps the phases apart:
//! the generator need not compile anything of the runtime to know which names it
//! has already covered, so `codegen` asks for no `runtime`. What that costs is
//! that the list lives away from the functions it names, which
//! `every_listed_name_is_a_matcher_runtime_gives` holds together: a spelling
//! this list keeps while the runtime drops it stops compiling here rather than
//! failing quietly inside someone else's generated file.

/// The names `runtime::matcher` gives a generated file without a backend.
pub(super) const BUILTINS: [&str; 2] = ["XID_Start", "XID_Continue"];

/// Returns `true` when the matcher `name` is one of the [`BUILTINS`], which is
/// what lets a grammar of identifiers be generated and compiled before the
/// project it describes has written a matcher of its own.
///
/// A backend that gives a matcher under one of these two names has to point the
/// generated file at it by the `matcher` path of [`Paths`](super::Paths) rather
/// than by `runtime::matcher`, since the generator treats these as already
/// provided and never asks for them.
#[inline]
pub(super) fn is_builtin(name: &str) -> bool {
    BUILTINS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_listed_name_is_provided_and_any_other_name_is_not() {
        // The generator treats a call of one of these as already given, which is
        // what lets a grammar of identifiers be written before a backend is.
        assert!(is_builtin("XID_Start"));
        assert!(is_builtin("XID_Continue"));

        // Anything else is the project's own matcher, called at the path the
        // generator is given for this module.
        assert!(!is_builtin("Casing"));
        assert!(!is_builtin(""));
    }

    #[cfg(feature = "runtime")]
    #[test]
    fn every_listed_name_is_a_matcher_runtime_gives() {
        // Naming the functions and not only their spellings is the point of this
        // test, since the list and the matchers live on opposite sides of the
        // split and a name kept here past its removal there would otherwise
        // reach a generated file as a call nothing defines.
        let provided = |name: &str| {
            let mut input = parserc::chars::TokenStream::<parserc::Kind>::from("a");

            match name {
                "XID_Start" => crate::runtime::matcher::XID_Start(&mut input),
                "XID_Continue" => crate::runtime::matcher::XID_Continue(&mut input),
                _ => panic!("{name} is listed here but called by no matcher"),
            }
            .is_ok()
        };

        for name in BUILTINS {
            assert!(provided(name), "{name} matches nothing it should");
        }
    }
}
