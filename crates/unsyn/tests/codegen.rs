//! The generator's own acceptance tests: a spec is analyzed, generated, and the
//! file that comes out is read back as Rust.
//!
//! Reading the output with [`syn`] says only that the tokens are a well formed
//! Rust file; that the types it declares *compile* and parse text is checked by
//! `tests/generated.rs`, which holds a copy of what the generator writes for a
//! small spec and runs it against real input.
//!
//! Assertions here are made against the generated text with every run of
//! whitespace removed, because `proc_macro2` spaces its tokens by its own rules
//! and what a generator means does not depend on them.

use std::{fs, path::PathBuf};

use parserc::syntax::Syntax;
use quote::quote;
use unsyn::{
    codegen::{Codegen, CodegenError, Paths},
    input::TokenStream,
    semantics::analyze,
    syntax::Crate,
};

/// The text of a spec, kept for as long as the modules built from it live.
fn leaked(text: &str) -> &'static str {
    Box::leak(text.to_owned().into_boxed_str())
}

/// Reads `crates/unsyn/bootstrap.uns`, kept alive for the modules built from it.
fn read_bootstrap() -> &'static str {
    let filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("bootstrap.uns")
        .canonicalize()
        .unwrap();

    Box::leak(fs::read_to_string(&filepath).unwrap().into_boxed_str())
}

/// The text of what `codegen` writes for a spec, with the spaces removed, or
/// the reports of what it could not write.
///
/// What comes out is read back as a Rust file before it is returned, since a
/// generator that wrote tokens no parser would take has failed however well it
/// read the spec.
fn generate(content: &'static str, codegen: Codegen) -> Result<String, Vec<CodegenError>> {
    let mut tokens = TokenStream::from(content);
    let file = Crate::parse(&mut tokens).expect("the spec should parse");

    let module = match analyze(&file) {
        Ok(module) => module,
        Err(errors) => panic!("the spec is not well-formed: {errors:?}"),
    };

    codegen.generate(&module).map(|tokens| {
        let text = tokens.to_string();

        syn::parse_file(&text).expect("the generated file is not valid Rust");

        text.split_whitespace().collect()
    })
}

/// The same for the settings a file is generated with by default.
fn plain(content: &'static str) -> String {
    match generate(content, Codegen::new()) {
        Ok(file) => file,
        Err(errors) => panic!("this spec should be writable: {errors:?}"),
    }
}

#[test]
fn a_rule_of_one_alternative_becomes_a_struct_of_its_operands() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Word -> ['a'-'z']+;
         syntax Pair -> Word Word;",
    );

    let file = plain(content);

    // The two operands are two fields, the second reached over a run of trivia.
    assert!(
        file.contains("pubstructPair<I>(pubWord<I>,pub::std::vec::Vec<WS<I>>,pubWord<I>);"),
        "{file}"
    );
    assert!(file.contains("input.parse::<Word<I>>()?"), "{file}");
}

#[test]
fn a_rule_of_several_alternatives_becomes_an_enum() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Word -> ['a'-'z']+;
         lexer One -> 'one';
         lexer Two -> 'two';
         syntax Choice -> One Word | Two | Word Word;",
    );

    let file = plain(content);

    assert!(file.contains("pubenumChoice<I>{"), "{file}");
    // Each alternative is named for what it opens with, which is what makes the
    // variants of a generated file say something the rule said.
    assert!(file.contains("ChoiceOne(One<I>,"), "{file}");
    assert!(file.contains("ChoiceTwo(Two<I>)"), "{file}");
    assert!(file.contains("ChoiceWord(Word<I>,"), "{file}");
    // An alternative that gives up is rewound, and the next one is tried.
    assert_eq!(file.matches(".ok().parse(input)?").count(), 3, "{file}");
}

#[test]
fn a_rule_that_names_itself_holds_itself_behind_a_pointer() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Word -> ['a'-'z']+;
         syntax Nested -> '[' Word* Nested? ']';",
    );

    let file = plain(content);

    // `Nested` sits inside itself, so the field of it is a pointer, and the
    // operand that may be absent is a pointer inside an `Option`.
    assert!(
        file.contains("Option<::std::boxed::Box<Nested<I>>>"),
        "{file}"
    );
    // A run of nodes is reached over the trivia between its elements, which a
    // `Vec` of the backend alone would not take.
    assert!(
        file.contains("Vec<(::std::vec::Vec<WS<I>>,Word<I>)>"),
        "{file}"
    );
}

#[test]
fn a_suffix_that_needs_a_check_of_its_own_gets_a_type_for_it() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Word -> ['a'-'z']+;
         lexer Key -> 'key';
         syntax Fixed -> Word{2..4};
         syntax Asked -> Word except Key;",
    );

    let file = plain(content);

    // The counted run is a type of its own, which checks its count after it.
    assert!(file.contains("pubstructFixed1<I>"), "{file}");
    assert!(file.contains("__items.len()<2"), "{file}");
    assert!(file.contains(">=4"), "{file}");
    // The assertion is a type of its own too, and it asks its question over a
    // copy of the stream, which is what keeps `except` from taking anything.
    assert!(file.contains("pubstructAsked1<I>"), "{file}");
    assert!(file.contains("letmut__probe=input.clone()"), "{file}");
    assert!(file.contains("__fits.is_some()"), "{file}");
}

#[test]
fn a_group_of_operands_is_a_field_and_not_a_type_where_nothing_divides_them() {
    let content = leaked(
        "lexer Digit -> ['0'-'9'];
         lexer Dot -> '.';
         syntax Number -> Digit (Digit | Dot);",
    );

    let file = plain(content);

    // The group of two alternatives cannot be a field of the rule that names
    // it, so it is given a type of its own, named for the rule it was written
    // in; with no `whitespace` rule there is nothing to keep between the two
    // operands of the rule, so they are two fields of it.
    // A rule of two alternatives is a type of its own and not a field of the
    // rule that names it, so the rule is two fields and the group is an enum.
    assert_eq!(file.matches("struct").count(), 3, "{file}");
    assert!(
        file.contains("pubstructNumber<I>(pubDigit<I>,pubNumber1<I>);"),
        "{file}"
    );
    // The group itself is the sum its two alternatives make.
    assert!(file.contains("pubenumNumber1<I>{"), "{file}");
    // The group itself is the sum its two alternatives make, and each is named
    // for the group it belongs to.
    assert!(file.contains("Number1Digit(Digit<I>)"), "{file}");
    assert!(file.contains("Number1Dot(Dot<I>)"), "{file}");
}

#[test]
fn a_group_that_needs_trivia_between_its_operands_gets_a_type() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Word -> ['a'-'z']+;
         syntax Outer -> '(' (Word Word) ')';",
    );

    let file = plain(content);

    // The operands of the group are reached over trivia, which a tuple cannot
    // carry, so the group is given a type of its own, named for its rule.
    assert!(file.contains("pubstructOuter1<I>"), "{file}");
    // It is not recursive, so it is held plainly.
    assert!(
        file.contains("pub::core::option::Option<Outer1<I>>") || file.contains("pubOuter1<I>"),
        "{file}"
    );
}

#[test]
fn a_set_written_in_a_rule_of_nodes_is_a_token_of_its_own() {
    let content = leaked(
        "whitespace WS -> [' '];
         syntax Marked -> '[' ['a', 'b'] ']';",
    );

    let file = plain(content);

    // The set is a scanner over characters, and it is written out as a type of
    // the file because a rule of nodes names it rather than repeating it. It
    // is one character wide, so the whole set is one question asked of the
    // character here, which is what keeps a member of a set cheap.
    assert!(file.contains("pubstructMarked1<I>"), "{file}");
    assert!(
        file.contains("next_if(|c:char|(::core::matches!(c,'a')||::core::matches!(c,'b')))"),
        "{file}"
    );
}

#[test]
fn a_keyword_does_not_match_the_front_of_a_longer_word() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Letter -> ['a'-'z'];
         syntax Let -> 'let' Letter 'x';",
    );

    let file = plain(content);

    // The literal `'let'` is a token that asks where it ends: the character
    // after it may not be one that would carry the word on. The question is
    // asked of the matcher this crate gives, over a copy of the stream.
    assert!(file.contains("pubstructLet<I>"), "{file}");
    assert!(
        file.contains("matcher::XID_Continue"),
        "the keyword ends nowhere: {file}"
    );
    assert!(file.contains("is_ok()"), "{file}");
    // The keyword is a token of its own, and the rule names it rather than the
    // characters it is made of.
    assert!(file.contains("pubLet_2<I>"), "{file}");
}

#[test]
fn a_call_of_a_matcher_the_backend_provides_is_a_call_of_it() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Name -> <Casing> ['a'-'z']*;",
    );

    let file = plain(content);

    // The matcher is reached through the path the generator was given, so a
    // project may keep its own implementations beside its generated file.
    assert!(file.contains("unsyn::codegen::matcher::Casing"), "{file}");

    // The same spec generated for a project that keeps the crates elsewhere.
    let moved = generate(
        content,
        Codegen::new().with_paths(Paths {
            unsyn: quote!(::my_unsyn),
            parserc: quote!(::my_parserc),
            matcher: quote!(::my_parserc::matchers),
        }),
    )
    .expect("a moved path should still be writable");

    assert!(moved.contains("::my_parserc::matchers::Casing"), "{moved}");
    assert!(
        moved.contains("my_unsyn::codegen::GeneratedInput"),
        "{moved}"
    );
}

#[test]
fn bootstrap_uns_is_generated_whole() {
    // The spec of `unsyn` written in `unsyn` is the input that exercises every
    // construct at once: sets, ranges, complements, `concat`, counted runs,
    // `except` and `followed`, the comment rules that reach over text in both
    // directions, and the recursion between expressions.
    let content = read_bootstrap();
    let mut tokens = TokenStream::from(content);
    let file = Crate::parse(&mut tokens).expect("the spec should parse");
    let module = analyze(&file).expect("the spec should be well-formed");

    let generated = match Codegen::new().generate(&module) {
        Ok(generated) => generated,
        Err(errors) => {
            let what = errors
                .iter()
                .map(|error| format!("{error}"))
                .collect::<Vec<_>>()
                .join("\n");

            panic!("bootstrap.uns is not generated yet:\n{what}");
        }
    };

    let text = generated.to_string();

    syn::parse_file(&text).expect("the generated bootstrap is not valid Rust");

    let written = text.split_whitespace().collect::<String>();

    // Every rule of the spec is a type of the generated file, under the name
    // the spec gave it.
    for def in module.symbols().iter() {
        assert!(
            written.contains(&format!("struct{}", def.name()))
                || written.contains(&format!("enum{}", def.name())),
            "`{}` was left out of the file",
            def.name()
        );
    }

    // The two matchers the spec calls are called, and not written out.
    assert!(written.contains("matcher::XID_Start"), "{written}");
    assert!(written.contains("matcher::XID_Continue"), "{written}");

    // The trivia of the language is its own type, and the separators name it.
    assert!(written.contains("structWHITESPACE"), "{written}");
}
