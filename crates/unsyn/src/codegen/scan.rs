//! Running a token pattern over the text.
//!
//! [`Emitter`] turns a [`Step`] into the statements of a generated `parse`:
//! every step advances the stream it is handed, and a step that does not fit
//! fails with a recovable [`Kind::Token`](parserc::Kind::Token), which is what
//! lets the caller rewind and try elsewhere. The emitter decides nothing on its
//! own: each step it reads off a pattern says what it costs, and
//! [`Step::char_class`] is what says whether a step is a question about one
//! character.
//!
//! # The shortcuts
//!
//! - A step one character wide is a [`CharClass`], and a repetition of one is scanned
//!   instead of looped over: `X*` is [`take_while`](parserc::take_while),
//!   `~ […]*` is [`take_till`](parserc::take_till), and `X{n, m}` is
//!   [`take_while_with`](parserc::take_while_with), each of which walks the text
//!   once rather than calling a parser per character.
//! - A literal of one character is [`next_if`](parserc::next_if) and of more
//!   [`keyword`](parserc::keyword).
//! - `~ ['lit']*` is [`take_until`](parserc::take_until) only where the step
//!   after it asks for that same literal, because `take_until` fails when the
//!   needle is absent while the loop it stands for reads to the end of the
//!   text. The search is written out where nothing insists on the needle.
//! - An external is opaque. It is never folded into a class and never scanned
//!   ahead, so a repetition of one is a real loop over one call each time.

use std::collections::HashMap;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote};

use super::Paths;
use super::error::CodegenError;
use super::pattern::{CharClass, Step, TokenPattern};

/// The class each token rule takes, for the rules that take one character and
/// nothing else.
///
/// A rule is a class only when every branch of it is one, which where a branch
/// names another rule needs that rule's class: hence the fixed point. A rule
/// that names itself, or that reaches one which does, is never settled, and
/// stays a step of its own.
pub(crate) fn classes(patterns: &[TokenPattern]) -> HashMap<String, CharClass> {
    let mut found: HashMap<String, CharClass> = HashMap::new();
    let mut pending: Vec<&TokenPattern> = patterns.iter().collect();

    loop {
        let mut rest = Vec::new();
        let mut settled = false;

        for pattern in pending {
            match pattern.step.char_class(&found) {
                Some(class) => {
                    found.insert(pattern.name.clone(), class);
                    settled = true;
                }
                None => rest.push(pattern),
            }
        }

        pending = rest;

        if !settled {
            break;
        }
    }

    found
}

/// Writes the statements a token pattern runs.
pub(crate) struct Emitter<'a> {
    /// The paths a generated file reaches the crates by.
    paths: &'a Paths,
    /// Where a `<NAME>` the generator's table settles is called, by name.
    externals: &'a HashMap<String, TokenStream>,
    /// The classes the rules a pattern may name settled out to.
    classes: &'a HashMap<String, CharClass>,
    /// The name written in the error a step of this pattern fails with.
    report: &'a str,
}

impl<'a> Emitter<'a> {
    /// Prepares an emitter that writes the pattern of the rule called `name`.
    pub(crate) fn named(
        paths: &'a Paths,
        externals: &'a HashMap<String, TokenStream>,
        classes: &'a HashMap<String, CharClass>,
        name: &'a str,
    ) -> Self {
        Self {
            paths,
            externals,
            classes,
            report: name,
        }
    }

    /// The body of a generated token's `parse`, which keeps what its pattern
    /// took and nothing else.
    pub(crate) fn parse_body(&self, step: &Step) -> Result<TokenStream, CodegenError> {
        let input = format_ident!("input");
        let stmts = self.stmts(step, &input)?;
        let fail = self.fail(&input);

        Ok(quote! {
            {
                // The pattern runs over a copy that stays where the token
                // started, so what it took can be split off without moving the
                // caller's stream a second time.
                let mut __from = #input.clone();

                #(#stmts)*

                // A token is at least one character wide, and a pattern that
                // got nowhere is not a token.
                if #input.len() == __from.len() {
                    return Err(#fail);
                }

                let __taken = __from.len() - #input.len();

                Ok(Self(__from.split_to(__taken)))
            }
        })
    }

    /// The statements of one step over the stream named `var`.
    fn stmts(&self, step: &Step, var: &Ident) -> Result<Vec<TokenStream>, CodegenError> {
        let parserc = &self.paths.parserc;

        match step {
            Step::Class(class) => {
                let cond = class.closure(self.paths);

                Ok(vec![quote!(#parserc::next_if(#cond).parse(#var)?;)])
            }
            // A step that reaches nowhere is a literal of nothing, which is the
            // one pattern that matches without moving; it is read as a failure
            // rather than as a step that takes nothing.
            Step::Lit(text) if text.is_empty() => Err(CodegenError::unsupported(
                "a pattern that matches nothing",
                parserc::Span::None,
            )),
            Step::Lit(text) => {
                let step = one_char(text)
                    .map(|c| {
                        let c = Literal::character(c);

                        quote!(#parserc::next_if(|c: char| c == #c).parse(#var)?)
                    })
                    .unwrap_or_else(|| {
                        let lit = Literal::string(text);

                        quote!(#parserc::keyword(#lit).parse(#var)?)
                    });

                Ok(vec![quote!(#step;)])
            }
            Step::External(name, span) => {
                let external = self.external(name, span)?;

                Ok(vec![quote!(#external(#var)?;)])
            }
            Step::Ref(name) => {
                let ty = format_ident!("{}", name);

                Ok(vec![quote!(#var.parse::<#ty<I>>()?;)])
            }
            Step::Seq(steps) => self.seq(steps, var),
            Step::Alt(branches) => {
                let mut chain: Option<TokenStream> = None;

                for branch in branches {
                    let closure = self.closure(branch, var)?;

                    chain = Some(match chain {
                        None => closure,
                        Some(previous) => quote!(#previous.or(#closure)),
                    });
                }

                let chain = chain.ok_or_else(|| {
                    CodegenError::unsupported("an alternative of no branches", self.no_span())
                })?;

                Ok(vec![quote!(#chain.parse(#var)?;)])
            }
            Step::Rep { min, max, item } => self.repeat(*min, *max, item, var),
            Step::NotFollowed(next) => {
                let probe = self.probe(next, quote!(#var.clone()))?;
                let fail = self.fail(var);

                Ok(vec![quote!(if #probe { return Err(#fail); })])
            }
            Step::Not(item) => self.complement(item, var).map(|stmt| vec![stmt]),
            Step::Except { item, removed } => {
                let start = format_ident!("__start");
                let mut stmts = vec![quote!(let #start = #var.clone();)];

                stmts.extend(self.stmts(item, var)?);

                let probe = self.probe(removed, quote!(#start.clone()))?;
                let fail = self.fail(var);

                stmts.push(quote!(if #probe { return Err(#fail); }));

                Ok(stmts)
            }
            Step::Followed { item, next } => {
                let mut stmts = self.stmts(item, var)?;

                let probe = self.probe(next, quote!(#var.clone()))?;
                let fail = self.fail(var);

                stmts.push(quote!(if !#probe { return Err(#fail); }));

                Ok(stmts)
            }
        }
    }

    /// The statements of a sequence, which is where a scan that looks ahead for
    /// its own neighbour is decided.
    fn seq(&self, steps: &[Step], var: &Ident) -> Result<Vec<TokenStream>, CodegenError> {
        let parserc = &self.paths.parserc;
        let mut out = Vec::new();

        for (index, step) in steps.iter().enumerate() {
            // `~ ['lit']*` followed by a step that begins with `'lit'`: the run
            // of characters before the needle is what `take_until` finds, and
            // the needle has to be there for the sequence to carry on at all.
            if let Some(text) = reads_until(step) {
                let lit = Literal::string(text);

                if self.next_wants(steps.get(index + 1), text) {
                    out.push(quote!(#parserc::take_until(#lit).parse(#var)?;));

                    continue;
                }

                out.push(quote! {
                    {
                        let __at = #var.find(#lit).unwrap_or_else(|| #var.len());

                        #var.split_to(__at);
                    }
                });

                continue;
            }

            out.extend(self.stmts(step, var)?);
        }

        Ok(out)
    }

    /// Returns `true` when `step` begins with `text`, which is what makes a
    /// `take_until` for it safe.
    fn next_wants(&self, step: Option<&Step>, text: &str) -> bool {
        matches!(step, Some(Step::Lit(later)) if later.starts_with(text))
    }

    /// The statements of a repetition: one scan where its item is a character
    /// wide, a loop where it is longer.
    fn repeat(
        &self,
        min: usize,
        max: Option<usize>,
        item: &Step,
        var: &Ident,
    ) -> Result<Vec<TokenStream>, CodegenError> {
        let parserc = &self.paths.parserc;

        if let Some(class) = item.char_class(self.classes) {
            let cond = class.closure(self.paths);

            let take = match (min, max) {
                // `take_while` is the unbounded form, and needs no bounds.
                (0, None) => quote!(#parserc::take_while(#cond).parse(#var)?),
                _ => {
                    let bounds = bounds(min, max);

                    quote!(#parserc::take_while_with(#bounds, #cond).parse(#var)?)
                }
            };

            // `take_while_with` reports the text running out before the least
            // was read, so a scan of a repetition needs no guard of its own.
            return Ok(vec![quote!(#take;)]);
        }

        let closure = self.closure(item, var)?;
        let fail = self.fail(var);

        if min == 0 && max.is_none() {
            return Ok(vec![quote! {
                loop {
                    let __step = #closure;

                    if __step.ok().parse(#var)?.is_none() {
                        break;
                    }
                }
            }]);
        }

        let count = format_ident!("__count");
        let least = Literal::usize_unsuffixed(min);

        let stop = match max {
            None => quote!(),
            Some(max) => {
                let most = Literal::usize_unsuffixed(max);

                quote!(if #count >= #most { break; })
            }
        };

        Ok(vec![
            quote!(let mut #count = 0usize;),
            quote! {
                loop {
                    let __step = #closure;

                    if __step.ok().parse(#var)?.is_none() {
                        break;
                    }

                    #count += 1;

                    #stop
                }
            },
            quote!(if #count < #least { return Err(#fail); }),
        ])
    }

    /// The statements of `~ […]`, which asks whether the set fits here and then
    /// takes one character that is not among its members.
    fn complement(&self, item: &Step, var: &Ident) -> Result<TokenStream, CodegenError> {
        let parserc = &self.paths.parserc;
        let fail = self.fail(var);

        // `~ […]*` reads to where the set would fit: the same scan, with the
        // question asked the other way about.
        if let Step::Rep { min, max, item } = item
            && let Some(class) = item.char_class(self.classes)
        {
            let take = match (*min, *max) {
                (0, None) => {
                    let cond = class.closure(self.paths);

                    quote!(#parserc::take_till(#cond).parse(#var)?)
                }
                _ => {
                    let cond = class.negated().closure(self.paths);
                    let bounds = bounds(*min, *max);

                    quote!(#parserc::take_while_with(#bounds, #cond).parse(#var)?)
                }
            };

            return Ok(quote!(#take;));
        }

        // A complement of one character is the class of every character that is
        // not it, which needs no question asked twice.
        if let Some(class) = item.char_class(self.classes) {
            let cond = class.negated().closure(self.paths);

            return Ok(quote!(#parserc::next_if(#cond).parse(#var)?;));
        }

        // Anything else is asked about, then taken: one character, which is all
        // `~` ever consumes, whether or not what it asked about reaches further.
        let probe = self.probe(item, quote!(#var.clone()))?;

        Ok(quote! {
            {
                if #probe {
                    return Err(#fail);
                }

                #parserc::next_if(|_: char| true).parse(#var)?;
            }
        })
    }

    /// A closure that runs one step and returns nothing, which is what `ok` and
    /// `or` are handed.
    fn closure(&self, step: &Step, var: &Ident) -> Result<TokenStream, CodegenError> {
        let stmts = self.stmts(step, var)?;
        let parserc = &self.paths.parserc;

        Ok(quote! {
            |#var: &mut I| -> ::core::result::Result<(), <I as #parserc::Input>::Error> {
                #(#stmts)*

                ::core::result::Result::Ok(())
            }
        })
    }

    /// Asks whether one step fits where `at` points, without consuming it.
    fn probe(&self, step: &Step, at: TokenStream) -> Result<TokenStream, CodegenError> {
        let parserc = &self.paths.parserc;

        if let Some(class) = step.char_class(self.classes) {
            let cond = class.closure(self.paths);

            return Ok(quote!(#at.iter().next().is_some_and(#cond)));
        }

        if let Step::Lit(text) = step {
            if let Some(c) = one_char(text) {
                let c = Literal::character(c);

                return Ok(quote!(#at.iter().next() == Some(#c)));
            }

            let lit = Literal::string(text);

            return Ok(quote!(#at.starts_with(#lit).is_some()));
        }

        let probe = format_ident!("__probe");
        let stmts = self.stmts(step, &probe)?;

        Ok(quote! {
            {
                let mut #probe = #at;

                (|#probe: &mut I| -> ::core::result::Result<(), <I as #parserc::Input>::Error> {
                    #(#stmts)*

                    ::core::result::Result::Ok(())
                })
                .parse(&mut #probe)
                .is_ok()
            }
        })
    }

    /// The failure a step reports: recovable, and one character wide, so that
    /// the caller may rewind and look elsewhere.
    fn fail(&self, var: &Ident) -> TokenStream {
        let parserc = &self.paths.parserc;
        let name = Literal::string(self.report);

        quote! {
            #parserc::Kind::Token(
                #name,
                #parserc::ControlFlow::Recovable,
                #var.to_span_at(1),
            )
            .into()
        }
    }

    /// The call of the external `name`, which is the path the table of the
    /// [`Codegen`](super::Codegen) gives that name and nothing else: what a
    /// `<NAME>` means is decided by whoever runs the generator, so a name the
    /// run did not give is reported rather than looked up by a convention.
    fn external(&self, name: &str, span: &parserc::Span) -> Result<TokenStream, CodegenError> {
        self.externals
            .get(name)
            .cloned()
            .ok_or_else(|| CodegenError::unmapped(name, span.clone()))
    }

    /// The span of a step the generator wrote for a rule that gave the emitter
    /// nothing to point at.
    fn no_span(&self) -> parserc::Span {
        parserc::Span::None
    }
}

/// The bounds of a repetition, as the range of characters a scan takes.
fn bounds(min: usize, max: Option<usize>) -> TokenStream {
    let least = Literal::usize_unsuffixed(min);

    match max {
        None => quote!(#least..),
        Some(max) => {
            let most = Literal::usize_unsuffixed(max);

            quote!(#least..=#most)
        }
    }
}

/// The literal a step reads up to, when it is `~ ['lit']*`.
fn reads_until(step: &Step) -> Option<&str> {
    match step {
        Step::Not(item) => match &**item {
            Step::Rep {
                min: 0,
                max: None,
                item,
            } => match &**item {
                Step::Lit(text) if !text.is_empty() => Some(text),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

/// The character a literal of exactly one is, which is the difference between
/// asking for a character and asking for a word.
fn one_char(text: &str) -> Option<char> {
    let mut chars = text.chars();

    match (chars.next(), chars.next()) {
        (Some(one), None) => Some(one),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// The one mapping every pattern here that names a function needs: nothing
    /// in the generator is named on its own, so a `<NAME>` step is only
    /// writable when the test says what the name stands for. The path is the
    /// test's own; the generator never resolves one.
    fn externals() -> HashMap<String, TokenStream> {
        HashMap::from([(
            "WORD_CARRY_ON".to_owned(),
            quote!(::my_backend::word_continue),
        )])
    }

    fn emit(step: &Step) -> String {
        let paths = Paths::default();
        let given = externals();
        let classes = HashMap::new();
        let emitter = Emitter::named(&paths, &given, &classes, "TEST");

        emitter
            .stmts(step, &format_ident!("input"))
            .expect("the pattern should be writable")
            .into_iter()
            .map(|stmt| stmt.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn a_class_is_a_question_about_one_character() {
        let code = emit(&Step::Class(CharClass::of_range('a', 'z')));

        assert!(code.contains("next_if"), "{code}");
        assert!(code.contains("char"), "{code}");
    }

    #[test]
    fn a_repetition_of_a_class_is_scanned() {
        let star = Step::Rep {
            min: 0,
            max: None,
            item: Box::new(Step::Class(CharClass::of('a'))),
        };

        assert!(emit(&star).contains("take_while ("), "{:?}", emit(&star));

        let bounded = Step::Rep {
            min: 2,
            max: Some(4),
            item: Box::new(Step::Class(CharClass::of('a'))),
        };

        let code = emit(&bounded);

        assert!(code.contains("take_while_with"), "{code}");
        assert!(code.contains("2 ..= 4"), "{code}");
    }

    #[test]
    fn a_repetition_of_an_external_is_looped() {
        let star = Step::Rep {
            min: 0,
            max: None,
            item: Box::new(Step::External(
                "WORD_CARRY_ON".to_owned(),
                parserc::Span::None,
            )),
        };

        let code = emit(&star);

        assert!(code.contains("loop"), "{code}");
        assert!(code.contains("my_backend"), "{code}");
        assert!(code.contains("word_continue"), "{code}");
    }

    #[test]
    fn a_complement_of_a_class_is_the_class_of_the_rest() {
        let code = emit(&Step::Not(Box::new(Step::Class(CharClass::of('n')))));

        assert!(code.contains("next_if"), "{code}");
        assert!(code.contains('!'), "{code}");
    }

    #[test]
    fn a_run_up_to_a_literal_is_taken_when_the_next_step_asks_for_it() {
        let paths = Paths::default();
        let externals = HashMap::new();
        let classes = HashMap::new();
        let emitter = Emitter::named(&paths, &externals, &classes, "TEST");
        let var = format_ident!("input");

        let steps = vec![
            Step::Not(Box::new(Step::Rep {
                min: 0,
                max: None,
                item: Box::new(Step::Lit("*/".to_owned())),
            })),
            Step::Lit("*/".to_owned()),
        ];

        let code = emitter
            .seq(&steps, &var)
            .expect("a sequence")
            .into_iter()
            .map(|stmt| stmt.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(code.contains("take_until"), "{code}");

        // Without the needle after it the same run reads to the end of the text,
        // which `take_until` would refuse to do.
        let alone = vec![Step::Not(Box::new(Step::Rep {
            min: 0,
            max: None,
            item: Box::new(Step::Lit("*/".to_owned())),
        }))];

        let code = emitter
            .seq(&alone, &var)
            .expect("a sequence")
            .into_iter()
            .map(|stmt| stmt.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(!code.contains("take_until"), "{code}");
        assert!(code.contains("find"), "{code}");
    }

    #[test]
    fn a_single_character_rule_is_a_class_and_a_longer_one_is_not() {
        let patterns = vec![
            TokenPattern::new("A", Step::Class(CharClass::of('a'))),
            TokenPattern::new("B", Step::Ref("A".to_owned())),
            TokenPattern::new(
                "C",
                Step::Seq(vec![Step::Ref("A".to_owned()), Step::Lit("z".to_owned())]),
            ),
            TokenPattern::new("D", Step::Ref("D".to_owned())),
        ];

        let found = classes(&patterns);

        assert_eq!(found.get("A"), Some(&CharClass::of('a')));
        assert_eq!(found.get("B"), Some(&CharClass::of('a')));
        assert_eq!(found.get("C"), None);
        assert_eq!(found.get("D"), None, "a rule that names itself");
    }
}
