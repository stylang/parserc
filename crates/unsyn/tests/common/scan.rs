//! The scan `<IdentOrKeyword>` is bound to: the start of a name — an
//! XID_Start or an `_` — and then the greedy run of characters a name
//! continues with. A generated file reaches this by the path the caller
//! gave in the binding, which is what an external is: the caller decides
//! what a call means, and here the answer is the same scan the hand lexer
//! of this crate reads.
//!
//! The two test crates that compile the generated file each bring this in
//! and re-export it at their root, which is where the binding points.
//!
//! The bound is the whole of what the scan uses — one character at a time,
//! and a position that can be snapshotted — so that whatever stream a build
//! is written against can be given to it, whether it reports its failures as
//! a `Kind` or as an `UnsynError`.

use parserc::Parser;

pub fn scan_ident_or_keyword<I>(input: &mut I) -> Result<I, <I as parserc::Input>::Error>
where
    I: parserc::Input<Item = char> + Clone,
{
    let mut whole = input.clone();
    let before = input.len();

    parserc::next_if(|c: char| ::unicode_ident::is_xid_start(c) || c == '_').parse(input)?;

    parserc::take_while(::unicode_ident::is_xid_continue).parse(input)?;

    Ok(whole.split_to(before - input.len()))
}
