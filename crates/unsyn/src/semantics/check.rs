//! Structural validation of node definition bodies.
//!
//! This module checks everything a body expression says about *values*: the
//! literals it uses, the character sets it enumerates, the ranges it draws and
//! the repetitions it counts. Names are not resolved here: that happens in the
//! link phase, once every definition of the crate is known.
//!
//! What is found goes to the two channels of a [`Diagnostics`]. A value no
//! definition can mean — an escape that decodes to nothing, a range drawn
//! backwards — is an error. A value that another member of the same set already
//! means is a lint: the set still denotes one language, it merely says part of
//! it twice.

use parserc::syntax::{Punctuated, Syntax};

use crate::{
    errors::{Diagnostics, LintKind, SemanticsKind, UnsynError},
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
/// Every problem found is reported to `report`, so one call states all the
/// value-level diagnostics of one definition.
pub fn validate_expr<I: UnsynInput>(expr: &Expr<I>, report: &mut Diagnostics) {
    validate_alts(&expr.first, report);

    for (_, alts) in expr.rest.iter() {
        validate_alts(alts, report);
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

fn validate_alts<I: UnsynInput>(alts: &ExprNoTopAlts<I>, report: &mut Diagnostics) {
    validate_alt(&alts.first, report);

    for (_, expr) in alts.rest.iter() {
        validate_alt(expr, report);
    }
}

fn validate_alt<I: UnsynInput>(expr: &ExprNoTopAlt<I>, report: &mut Diagnostics) {
    match expr {
        ExprNoTopAlt::WithSuffix(expr) => validate_with_suffix(expr, report),
        ExprNoTopAlt::WithoutSuffix(expr) => validate_without_suffix(expr, report),
    }
}

fn validate_with_suffix<I: UnsynInput>(expr: &ExprWithSuffix<I>, report: &mut Diagnostics) {
    match expr {
        ExprWithSuffix::Star(target, _)
        | ExprWithSuffix::Question(target, _)
        | ExprWithSuffix::Plus(target, _) => validate_without_suffix(target, report),
        ExprWithSuffix::Repeat { target, suffix } => {
            validate_without_suffix(target, report);
            validate_repeat(&suffix.body, report);
        }
        ExprWithSuffix::Concat { target, suffix, .. }
        | ExprWithSuffix::Followed { target, suffix, .. } => {
            validate_without_suffix(target, report);
            validate_alt(suffix, report);
        }
        ExprWithSuffix::Except { target, tokens, .. } => {
            validate_without_suffix(target, report);
            validate_without_suffix(tokens, report);
        }
    }
}

fn validate_without_suffix<I: UnsynInput>(expr: &ExprWithoutSuffix<I>, report: &mut Diagnostics) {
    match expr {
        // A complement refuses the members of a set, which is what a set is.
        ExprWithoutSuffix::Tilde(_, bracket) => validate_set(bracket, report),
        // The target of a call is resolved by the linker, not here.
        ExprWithoutSuffix::Call(_) => {}
        // A reference carries no value to evaluate; see [`crate::semantics::Analyzer`].
        ExprWithoutSuffix::Ident(_) => {}
        ExprWithoutSuffix::Paren(paren) => validate_expr(&paren.body, report),
        ExprWithoutSuffix::Set(bracket) => validate_set(bracket, report),
        ExprWithoutSuffix::Str(lit) => {
            if let Some(error) = str_value(lit).1 {
                report.push(error);
            }
        }
        ExprWithoutSuffix::Unicode(lit) => {
            if let Err(error) = unicode_value(lit) {
                report.push(error);
            }
        }
    }
}

/// Validates the members of a set and reports the ones the set already says.
///
/// A set is a union, so a member whose values another member already denotes
/// adds nothing to what the pattern matches. That is redundant rather than
/// meaningless, and so it belongs to the lint channel. A member is reported at
/// most once, against the first earlier member it conflicts with: what it says
/// twice is one fact, not the several ways it could be phrased.
fn validate_set<I: UnsynInput>(
    bracket: &Bracket<I, Punctuated<SetItem<I>, Comma<I>>>,
    report: &mut Diagnostics,
) {
    if bracket.body.is_empty() {
        report.push(UnsynError::Semantics(
            SemanticsKind::EmptySet,
            bracket.to_span(),
        ));
        return;
    }

    let mut members: Vec<Member> = vec![];
    let mut names: Vec<&str> = vec![];

    for item in items(&bracket.body) {
        // The same name written twice is the same member written twice, however
        // little of that node one file knows: only whether two *different*
        // names denote values in common is the question the file can not answer
        // (see `validate_set_item`).
        if let SetItem::Ident(ident) = item {
            let name = ident.0.as_str();

            if names.contains(&name) {
                report.lint(LintKind::DuplicateMember(name.to_owned()), item.to_span());
            } else {
                names.push(name);
            }

            continue;
        }

        // A member that denotes no value at all carries its own error, and a
        // member that denotes an unknown one is not a value of this set yet.
        let Some(member) = validate_set_item(item, report) else {
            continue;
        };

        if let Some(kind) = members
            .iter()
            .find_map(|earlier| conflict(earlier, &member))
        {
            report.lint(kind, item.to_span());
        }

        members.push(member);
    }
}

/// Validates one member of a set and returns the value it denotes.
///
/// `None` when the member was rejected — its diagnostic is the report's to make,
/// and a broken value is not worth comparing with anything — or when the member
/// is a name, which is compared with the names of the set beside it rather than
/// with the values here: one file can not know what the node it names denotes.
fn validate_set_item<I: UnsynInput>(item: &SetItem<I>, report: &mut Diagnostics) -> Option<Member> {
    match item {
        SetItem::Range(range) => {
            validate_range(range, report).map(|(start, end)| Member::of_range(start, end))
        }
        SetItem::Str(lit) => {
            // `'…'` is a string as well as a character, and a set is a union of
            // strings, so a member of any length is meaningful here.
            let (value, error) = str_value(lit);

            if let Some(error) = error {
                report.push(error);
                return None;
            }

            Some(Member::of_value(value))
        }
        SetItem::Unicode(lit) => match unicode_value(lit) {
            Ok(value) => Some(Member::Char(value)),
            Err(error) => {
                report.push(error);
                None
            }
        },
        // A repeated name is linted by `validate_set`, which is where the names
        // of a set are kept together.
        SetItem::Ident(_) => None,
    }
}

/// Validates a range and returns the two bounds it draws.
///
/// `None` when a bound can not be evaluated or the bounds stand in the wrong
/// order: such a range denotes no code point at all.
fn validate_range<I: UnsynInput>(
    range: &Range<I>,
    report: &mut Diagnostics,
) -> Option<(char, char)> {
    let mut sides: Vec<char> = vec![];

    match range {
        Range::Unicode(start, _, end) => {
            for lit in [start, end] {
                match unicode_value(lit) {
                    Ok(value) => sides.push(value),
                    Err(error) => report.push(error),
                }
            }
        }
        Range::Str(start, _, end) => {
            for lit in [start, end] {
                if let Some(error) = str_value(lit).1 {
                    report.push(error);
                    continue;
                }

                let chars = str_chars(lit);

                if chars.len() != 1 {
                    report.push(UnsynError::Semantics(
                        SemanticsKind::NotAChar,
                        lit.to_span(),
                    ));
                    continue;
                }

                sides.push(chars[0]);
            }
        }
    }

    // A bound that was rejected already carries its own diagnostic, so the
    // comparison is only made when both sides could be evaluated.
    let [start, end]: [char; 2] = sides.try_into().ok()?;

    if start > end {
        report.push(UnsynError::Semantics(
            SemanticsKind::InvalidRange,
            range.to_span(),
        ));
        return None;
    }

    Some((start, end))
}

/// Evaluates a repetition bound, reporting a diagnostic when it can not be
/// evaluated.
fn bound<I: UnsynInput>(lit: &LitDec<I>, report: &mut Diagnostics) -> Option<usize> {
    match dec_value(lit) {
        Ok(value) => Some(value),
        Err(error) => {
            report.push(error);
            None
        }
    }
}

fn validate_repeat<I: UnsynInput>(repeat: &Repeat<I>, report: &mut Diagnostics) {
    let span = repeat.to_span();

    match repeat {
        // `{, N}` and `{N}` matching nothing at all is never intended.
        Repeat::RangeTo(_, end) | Repeat::Count(end) => {
            if let Some(0) = bound(end, report) {
                report.push(UnsynError::Semantics(SemanticsKind::InvalidRepeat, span));
            }
        }
        Repeat::RangeFrom(start, _, end) => {
            let Some(start) = bound(start, report) else {
                return;
            };

            if let Some(end) = end {
                let Some(end) = bound(end, report) else {
                    return;
                };

                if end < start {
                    report.push(UnsynError::Semantics(SemanticsKind::InvalidRepeat, span));
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

/// The value one member of a set adds to it.
///
/// Two members are compared by what they *denote* rather than by how they are
/// spelled, so `'a'`, `U+0061` and `'\u{61}'` are one member written three
/// times, and `'a'-'a'` is the character `'a'`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Member {
    /// A single character.
    Char(char),
    /// A sequence of two or more characters, which no range can reach.
    Str(String),
    /// Every code point between two bounds that differ, inclusive.
    Range(char, char),
}

impl Member {
    /// The member a decoded string literal denotes.
    fn of_value(value: String) -> Self {
        let mut chars = value.chars();

        match (chars.next(), chars.next()) {
            (Some(one), None) => Member::Char(one),
            _ => Member::Str(value),
        }
    }

    /// The member the interval between two characters denotes.
    fn of_range(start: char, end: char) -> Self {
        if start == end {
            Member::Char(start)
        } else {
            Member::Range(start, end)
        }
    }

    /// The code points this member denotes, or `None` when it denotes a
    /// sequence of them.
    ///
    /// Only single-character values live in one space, where one member can
    /// contain another: a range covers no `'/*'`, because it denotes none.
    fn code_points(&self) -> Option<(u32, u32)> {
        match self {
            Member::Char(c) => Some((*c as u32, *c as u32)),
            Member::Range(start, end) => Some((*start as u32, *end as u32)),
            Member::Str(_) => None,
        }
    }

    /// This member's value, written the way the DSL would write it.
    ///
    /// The source text of a literal is not kept by the model, and two spellings
    /// of one character are one fact to report, so the value is re-rendered.
    fn text(&self) -> String {
        match self {
            Member::Char(c) => format!("'{}'", c.escape_default()),
            Member::Str(value) => {
                let escaped: String = value.chars().flat_map(char::escape_default).collect();

                format!("'{escaped}'")
            }
            Member::Range(start, end) => {
                format!("'{}'-'{}'", start.escape_default(), end.escape_default())
            }
        }
    }
}

/// Diagnoses `later` against `earlier`, a member the same set already carries.
///
/// `None` when the two denote different things, either because they share no
/// value or because what they share leaves each of them still worth writing —
/// which is the case for two sequences of characters of different length.
fn conflict(earlier: &Member, later: &Member) -> Option<LintKind> {
    if earlier == later {
        return Some(LintKind::DuplicateMember(later.text()));
    }

    // Two members of different lengths are not in the same space at all.
    let (Some((first, last)), Some((next, end))) = (earlier.code_points(), later.code_points())
    else {
        return None;
    };

    if last < next || end < first {
        return None;
    }

    // Both share a value and are not equal, so one of three relations is left.
    if next <= first && last <= end {
        Some(LintKind::CoversMember(later.text(), earlier.text()))
    } else if first <= next && end <= last {
        Some(LintKind::CoveredMember(later.text(), earlier.text()))
    } else {
        Some(LintKind::OverlappingMember(later.text(), earlier.text()))
    }
}

#[cfg(test)]
mod tests {
    use parserc::syntax::SyntaxInput;

    use crate::input::TokenStream;
    use crate::lexical::delimiter::Brace;
    use crate::syntax::{Expr, Repeat};

    use super::*;

    /// Validates one body expression and collects what it found.
    fn report(body: &str) -> Diagnostics {
        let expr = TokenStream::from(body)
            .parse::<Expr<_>>()
            .expect("the test body should parse");

        let mut report = Diagnostics::default();

        validate_expr(&expr, &mut report);

        report
    }

    /// Renders the errors a body produced.
    fn messages(body: &str) -> Vec<String> {
        report(body)
            .errors
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// Renders the lints a body produced.
    fn lints(body: &str) -> Vec<String> {
        report(body).lints.iter().map(ToString::to_string).collect()
    }

    /// Renders the single lint a body produced, which must have no error.
    fn lint(body: &str) -> String {
        let report = report(body);

        assert!(report.errors.is_empty(), "{body}: {:#?}", report.errors);
        assert_eq!(report.lints.len(), 1, "{body}: {:#?}", report.lints);

        report.lints[0].to_string()
    }

    /// Asserts that `body` earns exactly one lint, and that it reads as `want`.
    fn assert_lint(body: &str, want: &str) {
        let message = lint(body);

        assert!(message.contains(want), "{body}: {message}");
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

    /// A name written twice in one set is one member written twice, whatever the
    /// node it names denotes; whether two *different* names denote values in
    /// common is the question a single file can not answer.
    #[test]
    fn a_reference_the_set_already_names_is_linted() {
        assert_lint("[A,A]", "already declared");
        assert_lint("~[A,A]", "already declared");
        assert_lint("[A,'a',A]", "already declared");

        // Each of the two repeats of a third member says it twice over.
        assert_eq!(lints("[A,A,A]").len(), 2);

        // Two names that may denote the same language are left for the whole
        // program to decide, and a name against a value is not comparable.
        assert!(lints("[A,B]").is_empty());
        assert!(lints("[A,'a']").is_empty());
    }

    #[test]
    fn a_member_the_set_already_says_is_linted() {
        // The values of the members are compared, not their spellings.
        for body in [
            "['a','a']",
            "[ 'a' , 'a' ]",
            "['/*','/*']",
            r"['\u{61}','a']",
            "[U+0061,'a']",
            "['a',U+0061]",
            "['a'-'b','a'-'b']",
            "['a','a'-'a']",
            "~['a','a']",
            "['a','a']{2}",
            "('a' | ['a','a'])",
        ] {
            assert_lint(body, "already declared");
        }

        // Three times the same member is reported twice: each of the two
        // repeats says something an earlier member already said.
        assert_eq!(lints("['a','a','a']").len(), 2);
    }

    #[test]
    fn a_member_the_set_already_covers_is_linted() {
        // The later member adds nothing to the union.
        assert_lint("['a'-'z','c']", "is covered by the earlier member");
        assert_lint("['a'-'z','c'-'d']", "is covered by the earlier member");

        // The later member makes the earlier one the redundant one.
        assert_lint("['c','a'-'z']", "covers the earlier member");
    }

    #[test]
    fn a_member_the_set_partly_says_is_linted() {
        // The member named first in the message is the one being reported, and
        // the span points at it.
        let message = lint("['a'-'c','b'-'d']");

        assert!(
            message.starts_with("lint: member `'b'-'d'` overlaps the earlier member `'a'-'c'`"),
            "{message}"
        );
    }

    #[test]
    fn members_that_say_different_things_are_not_linted() {
        for body in [
            "['a','b']",
            "['a'-'b','c'-'d']",
            "['0'-'9','a'-'f','A'-'F']",
            r"['/*','*/']",
            "~['*','!','/*']",
            "['a','ab']",
            "['ab','abc']",
            "[U+0000,U+000A]",
        ] {
            assert!(report(body).lints.is_empty(), "{body}: {:#?}", report(body));
        }
    }

    #[test]
    fn a_name_is_compared_only_with_itself() {
        // `LF` and `LF` are one member written twice, which this pass can see
        // without knowing what `LF` denotes; whether `LF` and `CR` reach the
        // same character is a question about the definitions they denote, and
        // this pass does not know them.
        assert_lint("[LF, LF]", "already declared");

        for body in ["[LF, CR]", "[LF, 'a']"] {
            assert!(report(body).lints.is_empty(), "{body}: {:#?}", report(body));
        }
    }

    #[test]
    fn an_alternation_is_not_a_set() {
        // A body that repeats an alternative repeats a pattern, not a member of
        // a union of characters, and is out of the scope of these lints.
        for body in ["'a' | 'a'", "('a' | 'a')", "'a' | 'a' | 'a'"] {
            assert!(report(body).lints.is_empty(), "{body}: {:#?}", report(body));
        }
    }

    #[test]
    fn a_member_that_denotes_nothing_is_not_compared() {
        // The error says what is wrong with the range; calling it a lint as well
        // would report one problem under two names.
        let broken = report("['z'-'a','z'-'a']");

        assert_eq!(broken.errors.len(), 2, "{broken:#?}");
        assert!(broken.lints.is_empty(), "{broken:#?}");

        // A bound that decodes to no character has the same excuse.
        let escaped = report("['a','a'-'\\u{D800}']");

        assert!(
            escaped
                .errors
                .iter()
                .map(ToString::to_string)
                .any(|m| m.contains("not a valid unicode code point")),
            "{escaped:#?}"
        );
        assert!(escaped.lints.is_empty(), "{escaped:#?}");
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
            "'a'{3, 1}",
            "'a'{, 0}",
            "'a'{99999999999999999999999}",
        ] {
            assert!(
                messages(body)
                    .iter()
                    .any(|m| m.contains("invalid repetition range")),
                "{body}"
            );
        }

        for body in ["'a'{0, 3}", "'a'{1, 3}", "'a'{3}", "'a'{, 3}"] {
            assert!(messages(body).is_empty(), "{body}");
        }
    }

    #[test]
    fn the_lower_bound_of_a_repetition_is_evaluated() {
        for (body, expected) in [
            ("{0, 3}", 0),
            ("{2, 3}", 2),
            ("{3}", 3),
            ("{, 3}", 0),
            ("{1_0, 20}", 10),
        ] {
            let repeat = TokenStream::from(body)
                .parse::<Brace<_, Repeat<_>>>()
                .expect("the test repetition should parse");

            assert_eq!(repeat_lower(&repeat.body), expected, "{body}");
        }
    }
}
