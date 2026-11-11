//! Structural validation of node definition bodies.
//!
//! This module checks everything a body expression says about *values*: the
//! literals it uses, the character sets it enumerates, the ranges it draws and
//! the repetitions it counts. Names are not resolved here: that happens in the
//! link phase, once every definition of the crate is known.

use parserc::syntax::{Punctuated, Syntax};

use crate::{
    errors::{SemanticsKind, UnsynError},
    input::UnsynInput,
    lexical::{delimiter::Bracket, lit::LitDec, punct::Comma},
    semantics::value::{dec_usize, dec_value, str_chars, str_value, unicode_value},
    syntax::{
        Expr, ExprNoTopAlt, ExprNoTopAlts, ExprWithSuffix, ExprWithoutSuffix, Range, Repeat,
        SetItem,
    },
};

/// Validates the literals, sets, ranges and repetitions of a node body.
///
/// Every problem found is pushed to `errors`, so one call reports all
/// value-level diagnostics of one definition.
pub fn validate_expr<I: UnsynInput>(expr: &Expr<I>, errors: &mut Vec<UnsynError>) {
    validate_alts(&expr.first, errors);

    for (_, alts) in expr.rest.iter() {
        validate_alts(alts, errors);
    }
}

/// Iterates the items of a punctuated sequence.
pub(crate) fn items<T, P>(punctuated: &Punctuated<T, P>) -> impl Iterator<Item = &T> {
    punctuated
        .pairs
        .iter()
        .map(|(item, _)| item)
        .chain(punctuated.tail.iter().map(|tail| &**tail))
}

fn validate_alts<I: UnsynInput>(alts: &ExprNoTopAlts<I>, errors: &mut Vec<UnsynError>) {
    validate_alt(&alts.first, errors);

    for (_, expr) in alts.rest.iter() {
        validate_alt(expr, errors);
    }
}

fn validate_alt<I: UnsynInput>(expr: &ExprNoTopAlt<I>, errors: &mut Vec<UnsynError>) {
    match expr {
        ExprNoTopAlt::WithSuffix(expr) => validate_with_suffix(expr, errors),
        ExprNoTopAlt::WithoutSuffix(expr) => validate_without_suffix(expr, errors),
    }
}

fn validate_with_suffix<I: UnsynInput>(expr: &ExprWithSuffix<I>, errors: &mut Vec<UnsynError>) {
    match expr {
        ExprWithSuffix::Star(target, _)
        | ExprWithSuffix::Question(target, _)
        | ExprWithSuffix::Plus(target, _) => validate_without_suffix(target, errors),
        ExprWithSuffix::Repeat { target, suffix } => {
            validate_without_suffix(target, errors);
            validate_repeat(&suffix.body, errors);
        }
        ExprWithSuffix::Concat { target, suffix, .. }
        | ExprWithSuffix::Followed { target, suffix, .. } => {
            validate_without_suffix(target, errors);
            validate_alt(suffix, errors);
        }
        ExprWithSuffix::Except { target, tokens, .. } => {
            validate_without_suffix(target, errors);
            validate_without_suffix(tokens, errors);
        }
    }
}

fn validate_without_suffix<I: UnsynInput>(
    expr: &ExprWithoutSuffix<I>,
    errors: &mut Vec<UnsynError>,
) {
    match expr {
        ExprWithoutSuffix::Tilde(_, target) => validate_without_suffix(target, errors),
        // The target of a call is resolved by the linker, not here.
        ExprWithoutSuffix::Call(_) => {}
        // A reference carries no value to evaluate; see [`crate::semantics::Analyzer`].
        ExprWithoutSuffix::Ident(_) => {}
        ExprWithoutSuffix::Paren(paren) => validate_expr(&paren.body, errors),
        ExprWithoutSuffix::Set(bracket) => validate_set(bracket, errors),
        ExprWithoutSuffix::Str(lit) => {
            if let Some(error) = str_value(lit).1 {
                errors.push(error);
            }
        }
        ExprWithoutSuffix::Unicode(lit) => {
            if let Err(error) = unicode_value(lit) {
                errors.push(error);
            }
        }
    }
}

fn validate_set<I: UnsynInput>(
    bracket: &Bracket<I, Punctuated<SetItem<I>, Comma<I>>>,
    errors: &mut Vec<UnsynError>,
) {
    if bracket.body.is_empty() {
        errors.push(UnsynError::Semantics(
            SemanticsKind::EmptySet,
            bracket.to_span(),
        ));
        return;
    }

    for item in items(&bracket.body) {
        validate_set_item(item, errors);
    }
}

fn validate_set_item<I: UnsynInput>(item: &SetItem<I>, errors: &mut Vec<UnsynError>) {
    match item {
        SetItem::Range(range) => validate_range(range, errors),
        SetItem::Str(lit) => {
            // `'…'` is a string as well as a character, and a set is a union of
            // strings, so a member of any length is meaningful here.
            if let Some(error) = str_value(lit).1 {
                errors.push(error);
            }
        }
        SetItem::Unicode(lit) => {
            if let Err(error) = unicode_value(lit) {
                errors.push(error);
            }
        }
        // The shape of a referenced set member can not be known here.
        SetItem::Ident(_) => {}
    }
}

fn validate_range<I: UnsynInput>(range: &Range<I>, errors: &mut Vec<UnsynError>) {
    let mut sides: Vec<u32> = vec![];

    match range {
        Range::Unicode(start, _, end) => {
            for lit in [start, end] {
                match unicode_value(lit) {
                    Ok(value) => sides.push(value as u32),
                    Err(error) => errors.push(error),
                }
            }
        }
        Range::Str(start, _, end) => {
            for lit in [start, end] {
                if let Some(error) = str_value(lit).1 {
                    errors.push(error);
                    continue;
                }

                let chars = str_chars(lit);

                if chars.len() != 1 {
                    errors.push(UnsynError::Semantics(
                        SemanticsKind::NotAChar,
                        lit.to_span(),
                    ));
                    continue;
                }

                sides.push(chars[0] as u32);
            }
        }
    }

    // A bound that was rejected already carries its own diagnostic, so the
    // comparison is only made when both sides could be evaluated.
    if sides.len() == 2 && sides[0] > sides[1] {
        errors.push(UnsynError::Semantics(
            SemanticsKind::InvalidRange,
            range.to_span(),
        ));
    }
}

/// Evaluates a repetition bound, reporting a diagnostic when it can not be
/// evaluated.
fn bound<I: UnsynInput>(lit: &LitDec<I>, errors: &mut Vec<UnsynError>) -> Option<usize> {
    match dec_value(lit) {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(error);
            None
        }
    }
}

fn validate_repeat<I: UnsynInput>(repeat: &Repeat<I>, errors: &mut Vec<UnsynError>) {
    let span = repeat.to_span();

    match repeat {
        // `{..N}` and `{N}` matching nothing at all is never intended.
        Repeat::RangeTo(_, end) | Repeat::Count(end) => {
            if let Some(0) = bound(end, errors) {
                errors.push(UnsynError::Semantics(SemanticsKind::InvalidRepeat, span));
            }
        }
        Repeat::RangeFrom(start, _, end) => {
            let Some(start) = bound(start, errors) else {
                return;
            };

            if let Some(end) = end {
                let Some(end) = bound(end, errors) else {
                    return;
                };

                if end < start {
                    errors.push(UnsynError::Semantics(SemanticsKind::InvalidRepeat, span));
                }
            }
        }
    }
}

/// Returns the lowest number of repetitions `repeat` matches.
///
/// A bound that can not be evaluated is treated as `0`, which keeps the
/// recursion analysis conservative instead of reporting the same literal twice.
pub(crate) fn repeat_lower<I: UnsynInput>(repeat: &Repeat<I>) -> usize {
    match repeat {
        Repeat::RangeTo(..) => 0,
        Repeat::RangeFrom(start, _, _) | Repeat::Count(start) => dec_usize(start).unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use parserc::syntax::SyntaxInput;

    use crate::input::TokenStream;
    use crate::lexical::delimiter::Brace;
    use crate::syntax::{Expr, Repeat};

    use super::*;

    /// Validates one body expression and renders what it found.
    fn messages(body: &str) -> Vec<String> {
        let expr = TokenStream::from(body)
            .parse::<Expr<_>>()
            .expect("the test body should parse");

        let mut errors = vec![];

        validate_expr(&expr, &mut errors);

        errors.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn every_alternation_of_the_body_is_checked() {
        let messages = messages("'a' | ['z'-'a'] | []");

        assert_eq!(messages.len(), 2, "{messages:#?}");
    }

    #[test]
    fn a_reference_carries_no_value_to_check() {
        assert!(messages("MISSING").is_empty());
        assert!(messages("[MISSING]").is_empty());
    }

    #[test]
    fn operands_of_every_suffix_are_visited() {
        for body in [
            "~['z'-'a']",
            "A except ['z'-'a']",
            "A concat ['z'-'a']",
            "A followed ['z'-'a']",
            "['z'-'a']{2}",
            "['z'-'a']?",
            "('a' | ['z'-'a'])",
        ] {
            assert!(
                messages(body).iter().any(|m| m.contains("invalid range")),
                "{body}"
            );
        }
    }

    #[test]
    fn invalid_repetition_bounds_are_reported() {
        for body in [
            "'a'{0}",
            "'a'{3..1}",
            "'a'{..0}",
            "'a'{99999999999999999999999}",
        ] {
            assert!(
                messages(body)
                    .iter()
                    .any(|m| m.contains("invalid repetition range")),
                "{body}"
            );
        }

        for body in ["'a'{0..3}", "'a'{1..3}", "'a'{3}", "'a'{..3}"] {
            assert!(messages(body).is_empty(), "{body}");
        }
    }

    #[test]
    fn the_lower_bound_of_a_repetition_is_evaluated() {
        for (body, expected) in [
            ("{0..3}", 0),
            ("{2..3}", 2),
            ("{3}", 3),
            ("{..3}", 0),
            ("{1_0..20}", 10),
        ] {
            let repeat = TokenStream::from(body)
                .parse::<Brace<_, Repeat<_>>>()
                .expect("the test repetition should parse");

            assert_eq!(repeat_lower(&repeat.body), expected, "{body}");
        }
    }
}
