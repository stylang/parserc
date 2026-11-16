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
    codegen::{Codegen, CodegenError, CodegenKind, Paths},
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

/// The variants of the `enum` a generated file declares under `name`, split at
/// the commas that separate them and at nothing nested inside one of them.
///
/// The text has had its whitespace removed, so a comma separates two variants
/// only when no delimiter is open around it.
fn enum_variants(written: &str, name: &str) -> Vec<String> {
    let head = format!("pubenum{name}<I>{{");

    let start = written
        .find(&head)
        .unwrap_or_else(|| panic!("`{name}` is written as no enum"));

    let mut depth = 0usize;
    let mut current = String::new();
    let mut variants = Vec::new();

    for ch in written[start + head.len()..].chars() {
        match ch {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' if depth > 0 => depth -= 1,
            ',' if depth == 0 => variants.push(std::mem::take(&mut current)),
            '}' if depth == 0 => {
                variants.push(std::mem::take(&mut current));

                return variants;
            }
            other => current.push(other),
        }
    }

    panic!("`{name}` is never closed");
}

/// A generator told what the two names a spec of identifiers writes calls.
///
/// This is the caller's own table: the paths lead to a backend of the test's
/// naming, since nothing in unsyn ships or assumes an implementation of a
/// `<NAME>`. It is what a spec needs as soon as it writes an angle-bracketed
/// call, and nothing more: a name the spec never wrote is a name no table is
/// asked for, which is why the tests that spell no `<NAME>` use [`plain`]
/// instead.
fn with_a_backend() -> Codegen {
    Codegen::new().with_externals([
        ("WORD_START".to_owned(), quote!(::my_backend::word_start)),
        (
            "WORD_CARRY_ON".to_owned(),
            quote!(::my_backend::word_continue),
        ),
    ])
}

/// The same as [`plain`], for a spec that asks a name the generator was given.
fn given(content: &'static str) -> String {
    match generate(content, with_a_backend()) {
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

    // The two operands are two fields, and the trivia between them is a third,
    // since neither of them reaches it: `Word` is a run of characters, which
    // keeps nothing of its own but the text it matched.
    assert!(
        file.contains("pubstructPair<I>(pubWord<I>,pub::core::option::Option<WS<I>>,pubWord<I>);"),
        "{file}"
    );
    assert!(file.contains("input.parse::<Word<I>>()?"), "{file}");
}

#[test]
fn a_trivia_node_written_out_as_an_operand_is_the_separator_of_its_rule() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Word -> ['a'-'z']+;
         syntax Pair -> Word WS Word;
         syntax Lead -> WS Word;
         syntax Tail -> Word WS;",
    );

    let file = plain(content);

    // The mention of the trivia is the run between the two operands, so the
    // rule is shaped as `Word Word` is rather than holding a token of trivia
    // between two runs of it.
    assert!(
        file.contains("pubstructPair<I>(pubWord<I>,pub::core::option::Option<WS<I>>,pubWord<I>);"),
        "{file}"
    );

    // At either end of a sequence there is nothing for the mention to separate,
    // and the trivia it asks for belongs to the rule around it rather than to
    // this one: the mention still earns no field, and no gap either.
    assert!(file.contains("pubstructLead<I>(pubWord<I>);"), "{file}");
    assert!(file.contains("pubstructTail<I>(pubWord<I>);"), "{file}");

    // The whitespace is never held one character at a time, which is what a run
    // of the trivia type between two operands would have to be.
    assert_eq!(file.matches("Vec<WS<I>>").count(), 0, "{file}");
}

#[test]
fn a_rule_about_the_trivia_itself_keeps_the_operands_it_names() {
    let content = leaked(
        "whitespace WS -> [' '];
         syntax Blanks -> WS WS;",
    );

    let file = plain(content);

    // Nothing else is left for a mention to be folded into, so the two runs are
    // held as the two operands they are, and a gap still stands between them:
    // the trivia node keeps none of its own neighbours, so this rule holds a
    // run, whatever lies between, and another run.
    assert!(
        file.contains("pubstructBlanks<I>(pubWS<I>,pub::core::option::Option<WS<I>>,pubWS<I>);"),
        "{file}"
    );
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

    let file = given(content);

    assert!(file.contains("pubenumChoice<I>"), "{file}");
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
    // `Vec` of the backend alone would not take: each repetition carries the
    // gap before it.
    assert!(
        file.contains("Vec<(::core::option::Option<WS<I>>,Word<I>)>"),
        "{file}"
    );
    // The two brackets own the trivia on each side of themselves, so what lies
    // between them is held as one delimited run rather than as four fields.
    assert!(
        file.contains("Delimiter<BracketStart<I>,BracketEnd<I>,"),
        "{file}"
    );
}

#[test]
fn a_suffix_that_needs_a_check_of_its_own_gets_a_type_for_it() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Word -> ['a'-'z']+;
         lexer Key -> 'key';
         syntax Fixed -> Word{2, 4};
         syntax Asked -> Word except Key;",
    );

    let file = given(content);

    // The counted run is a type of its own, which checks its count after it.
    assert!(file.contains("pubstructFixed1<I>"), "{file}");
    assert!(file.contains("__items.len()<2"), "{file}");
    assert!(file.contains(">=4"), "{file}");
    // The assertion is a type of its own too. Its question is asked of the text
    // the operand read, which is cut out for the removed language to be run
    // over: `except` takes nothing itself, and refuses only what the removal
    // exhausts.
    assert!(file.contains("pubstructAsked1<I>"), "{file}");
    assert!(
        file.contains("__ahead.split_to(input.start()-__from.start())"),
        "{file}"
    );
    assert!(file.contains("__text.is_empty()"), "{file}");
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

/// A quantity operator is stored as the one group the production writes it as,
/// but the variant naming a rule is still named for the operator it picked.
#[test]
fn the_four_quantity_operators_are_one_alternative_but_four_names() {
    let content = leaked(
        "lexer A -> ['a'];
         syntax S -> A* | A? | A+ | A{2};",
    );

    let file = plain(content);

    // Folding `('*' | '?' | '+' | Repeat)` into a single alternative of the rule
    // must not fold the names with it: the generator reads the operator back out
    // of the group, so four alternatives of a rule are still four distinguishable
    // variants rather than four copies of one `Grouped`.
    assert!(file.contains("SStar("), "{file}");
    assert!(file.contains("SQuestion("), "{file}");
    assert!(file.contains("SPlus("), "{file}");
    assert!(file.contains("SRepeat("), "{file}");
    assert!(!file.contains("SGrouped"), "{file}");
}

#[test]
fn a_group_that_needs_trivia_between_its_operands_gets_a_type() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Word -> ['a'-'z']+;
         syntax Outer -> '(' (Word Word) ')';",
    );

    let file = plain(content);

    // A gap is a field like any other, so the group is carried by a tuple of
    // its own after all, and the whole of the rule is one delimited run: the
    // two brackets own the trivia on either side of themselves, which is what
    // lets the group sit between them as one field of the rule.
    assert!(
        file.contains(
            "Delimiter<ParenStart<I>,ParenEnd<I>,(Word<I>,::core::option::Option<WS<I>>,Word<I>)>"
        ),
        "{file}"
    );
    assert!(!file.contains("structOuter1"), "{file}");
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
fn a_literal_means_only_the_characters_it_spells() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Letter -> ['a'-'z'];
         syntax Let -> 'let' Letter 'x';",
    );

    let file = plain(content);

    // `'let'` is three characters and a rule over them, which is all the spec
    // said of it. Whether the word ends before a longer one is a fact about the
    // language being described, and a spec that wants a boundary there has to
    // write one: the generator asks nothing after the literal.
    assert!(file.contains("pubstructLet<I>"), "{file}");
    assert!(file.contains("::parserc::keyword(\"let\")"), "{file}");
    assert!(file.contains("pubLet_2<I>"), "{file}");

    // The bound of `GeneratedInput` is the only thing a generated file may take
    // from unsyn. Any other path into unsyn would be an answer the generator
    // supplied about the language the spec describes.
    for reached in file.split("::unsyn::runtime::").skip(1) {
        assert!(
            reached.starts_with("GeneratedInput"),
            "the generator borrowed an answer of its own from unsyn: {file}"
        );
    }
}

#[test]
fn a_spec_that_writes_no_call_needs_no_table() {
    // The other half of the same rule, said of the whole file rather than of
    // one literal: a spec with no `<NAME>` in it asks nothing of a caller, and
    // a name no table was given is the only way that promise is broken.
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Letter -> ['a'-'z'];
         syntax Let -> 'let' Letter 'x';",
    );

    plain(content);

    // A name a spec did write is still nobody's business but the caller's.
    let asked = leaked(
        "whitespace WS -> [' '];
         lexer Letter -> <WORD_START> ['a'-'z']*;",
    );

    let Err(errs) = generate(asked, Codegen::new()) else {
        panic!("a name no table was given should be refused");
    };

    assert!(
        matches!(&errs[0].kind, CodegenKind::Unmapped(name) if name == "WORD_START"),
        "{errs:?}"
    );
}

#[test]
fn an_external_is_called_exactly_where_the_table_said() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Name -> <Casing> ['a'-'z']*;",
    );

    // A `<NAME>` is only ever what whoever ran the generator said it was, so
    // the file calls that one function and reaches it by the path the table
    // spelled, not by a name the generator invented for it.
    let file = generate(
        content,
        Codegen::new().with_externals([("Casing".to_owned(), quote!(::my_grammar::casing))]),
    )
    .expect("an external given by a table should be writable");

    assert!(file.contains("::my_grammar::casing"), "{file}");

    // The same spec for a project that keeps the two crates elsewhere. The
    // paths the file reaches the crates by and the paths the table gives are
    // separate decisions, so moving one does not move the other.
    let moved = generate(
        content,
        Codegen::new()
            .with_paths(Paths {
                unsyn: quote!(::my_unsyn),
                parserc: quote!(::my_parserc),
            })
            .with_externals([("Casing".to_owned(), quote!(::my_grammar::casing))]),
    )
    .expect("a moved path should still be writable");

    assert!(moved.contains("::my_grammar::casing"), "{moved}");
    assert!(
        moved.contains("my_unsyn::runtime::GeneratedInput"),
        "{moved}"
    );
}

#[test]
fn every_rule_that_calls_one_name_shares_the_type_that_calls_it() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Word -> ['a'-'z']+;
         syntax Head -> Word <Casing>;
         syntax Tail -> <Casing> Word;",
    );

    let file = generate(
        content,
        Codegen::new().with_externals([("Casing".to_owned(), quote!(::my_grammar::casing))]),
    )
    .expect("an external given by a table should still be writable");

    // One name is one pattern however many rules ask for it, since the
    // function behind the name was fixed before any of them was read: the two
    // rules share one type instead of the call being written out twice.
    assert!(file.contains("pubstructExternalCasing<I>"), "{file}");
    assert!(!file.contains("ExternalCasing_2"), "{file}");
}

#[test]
fn a_name_no_table_gives_is_reported_and_not_written() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Word -> ['a'-'z']+;
         syntax Head -> <WORD_START> <WORD_CARRY_ON>*;
         syntax Tail -> <WORD_CARRY_ON>+ Word;",
    );

    // A generator starts with an empty table and keeps one: nothing decides on
    // a caller's behalf what one of its names means. A file that
    // called a function nobody had settled on would compile in a project that
    // never meant it to, so the name is reported instead.
    let refused =
        generate(content, Codegen::new()).expect_err("a name no table gives should be reported");

    let what = refused
        .iter()
        .map(|error| error.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        what.contains("nothing is mapped to `<WORD_START>`"),
        "{what}"
    );
    assert!(
        what.contains("nothing is mapped to `<WORD_CARRY_ON>`"),
        "{what}"
    );
    // One report per name, not one per call: the rules that ask for the same
    // name share the one type that calls it, and that type is written once.
    assert_eq!(refused.len(), 2, "{what}");

    // Each report says where the name was written: the span of the `<NAME>` is
    // kept in the pattern that runs it, so a refusal points at the source
    // rather than at nothing.
    assert!(!what.contains(", at None"), "{what}");

    // The same spec with both names given is writable, and each is called where
    // the table said.
    let settled = generate(content, with_a_backend())
        .expect("a spec whose names are all given should be writable");

    assert!(settled.contains("::my_backend::word_start"), "{settled}");
    assert!(settled.contains("::my_backend::word_continue"), "{settled}");

    // A name given by a table is no less one pattern than a name the rules of
    // the file describe themselves, so the sharing of one type between the two
    // rules that ask for it does not depend on the table.
    assert!(
        settled.contains("pubstructExternalWORD_CARRY_ON<I>"),
        "{settled}"
    );
    assert!(!settled.contains("ExternalWORD_CARRY_ON_2"), "{settled}");
}

#[test]
fn a_moved_path_leaves_a_given_external_where_the_table_put_it() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Name -> <WORD_START> <WORD_CARRY_ON>*;",
    );

    // The table is the caller's own, so moving the crates the file reaches by
    // does not rebuild the names it settled around the new paths.
    let file = generate(
        content,
        Codegen::new()
            .with_externals([
                ("WORD_START".to_owned(), quote!(::my_grammar::word_start)),
                (
                    "WORD_CARRY_ON".to_owned(),
                    quote!(::my_grammar::word_continue),
                ),
            ])
            .with_paths(Paths {
                unsyn: quote!(::my_unsyn),
                parserc: quote!(::my_parserc),
            }),
    )
    .expect("a table of its own should still be writable");

    assert!(file.contains("::my_grammar::word_start"), "{file}");
    assert!(file.contains("::my_grammar::word_continue"), "{file}");
    // Moving the crates the file reaches by leaves the table where the caller
    // put it: the two decisions are separate, and once both are made the file
    // names no crate of unsyn's at all.
    assert!(!file.contains("::unsyn::"), "{file}");
}

#[test]
fn a_name_rust_spends_on_its_own_grammar_is_callable_once_a_table_gives_it() {
    let content = leaked(
        "whitespace WS -> [' '];
         lexer Word -> ['a'-'z']+;
         syntax Typed -> Word <type>;",
    );

    // A name is never written out as an identifier of its own, so `<type>` is
    // as callable as any other once the table says where it lives: the path is
    // copied as the table spells it, and the name itself never reaches the
    // file. An unmapped `<type>` would be reported as any other name would.
    let settled = generate(
        content,
        Codegen::new().with_externals([("type".to_owned(), quote!(::my_grammar::ty))]),
    )
    .expect("a settled name should be writable");

    assert!(settled.contains("::my_grammar::ty"), "{settled}");
    assert!(!settled.contains("<type>"), "{settled}");
}

/// A generator told what the two names `bootstrap.uns` writes call.
///
/// That spec asks for the two classes of [`unicode_ident`] by their Unicode
/// names, and a name is only what the caller running the generator says it is.
/// This test is that caller, so it is where the names are placed — on a module
/// of the caller's own, which is all unsyn ever sees of them.
fn with_the_bootstrap_backend() -> Codegen {
    Codegen::new().with_externals([
        ("XID_Start".to_owned(), quote!(::my_backend::xid_start)),
        (
            "XID_Continue".to_owned(),
            quote!(::my_backend::xid_continue),
        ),
    ])
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

    // The spec names two externals, and a name is only what the caller's table
    // says, so the generator is handed both under the paths the caller keeps
    // them at.
    let generated = match with_the_bootstrap_backend().generate(&module) {
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

    // The two externals the spec names are called, and not written out.
    assert!(written.contains("my_backend::xid_start"), "{written}");
    assert!(written.contains("my_backend::xid_continue"), "{written}");

    // The trivia of the language is its own type, and the gaps name it.
    assert!(written.contains("structS<I>"), "{written}");
}

// The two folds the generator knows about are the whole reason a rule of the
// spec stays as small as the rule is written, so a run over the real spec is
// what pins them: `a b*` becomes one list of the backend's own `Punctuated`,
// and a run bounded by two terminals that own their trivia becomes one
// `Delimiter` of the two ends and the body between them.
#[test]
fn a_bootstrap_spec_is_read_as_the_shapes_it_writes() {
    let content = read_bootstrap();
    let mut tokens = TokenStream::from(content);
    let file = Crate::parse(&mut tokens).unwrap();
    let module = analyze(&file).unwrap();

    let text = with_the_bootstrap_backend()
        .generate(&module)
        .unwrap()
        .to_string();
    let written = text.split_whitespace().collect::<String>();

    assert!(written.contains("Punctuated<"), "{written}");
    assert!(written.contains("Delimiter<"), "{written}");

    // `Stmt -> LeadingKeywords Ident '->' Expr ';'`
    //
    // The name is what the statement is written with, the arrow and the
    // semicolon close a delimiter around the expression they hold, and the
    // only gap left is the one before the arrow, because the identifier does
    // not own trivia.
    assert!(
        written.contains(
            "structStmt<I>(pubLeadingKeywords<I>,pub::core::option::Option<S<I>>,\
             pubIdent<I>,pub::parserc::syntax::Delimiter<ArrowRight<I>,Semi<I>,Expr<I>>);"
        ),
        "`Stmt` is not the four fields the rule has: {written}"
    );

    // What one alternative of a rule folds into must be what the production
    // actually says, not the widest reach of the terminals written in it.
    //
    // `bootstrap.uns:123` is `'~' '[' ( SetItem ',' )* SetItem? ']'`, and the
    // hand-written node for it is `expr.rs:354`'s
    // `Tilde(Tilde<I>, Bracket<I, Punctuated<SetItem<I>, Comma<I>>>)`: the `~`
    // stands outside the brackets, which close a run of their own. Folding
    // instead by leftmost reach would pair the `~` with the `]`.
    //
    // The three other alternatives are here for the same reason, each against
    // the hand-written shape named beside it: `:124` is `Call(Angle<I, Ident<I>>)`,
    // `:125` is `Paren(Paren<I, Box<Expr<I>>>)`, `:126` is
    // `Set(Bracket<I, Punctuated<SetItem<I>, Comma<I>>>)`.
    for variant in [
        "ExprWithoutSuffixTilde(Tilde<I>,\
         ::parserc::syntax::Delimiter<BracketStart<I>,BracketEnd<I>,\
         ::parserc::syntax::Punctuated<SetItem<I>,Comma<I>>>)",
        "ExprWithoutSuffixLt(::parserc::syntax::Delimiter<Lt<I>,Gt<I>,Ident<I>>)",
        "ExprWithoutSuffixParenStart(::parserc::syntax::Delimiter<ParenStart<I>,ParenEnd<I>,\
         ::std::boxed::Box<Expr<I>>>)",
        "ExprWithoutSuffixBracketStart(::parserc::syntax::Delimiter<BracketStart<I>,\
         BracketEnd<I>,::parserc::syntax::Punctuated<SetItem<I>,Comma<I>>>)",
    ] {
        let folded = variant.split_whitespace().collect::<String>();

        assert!(
            written.contains(&folded),
            "`{variant}` was not folded into the shape the hand-written node has"
        );
    }

    // The group `('*' | '?' | '+' | Repeat)` of `bootstrap.uns:134-139` stays
    // the one alternative the production writes it as: `ExprWithSuffix` has one
    // variant per alternative of the rule — four — and the four operators inside
    // the group did not sink into the enum as variants of their own. This is the
    // shape the hand-written `ExprWithSuffix` now has too: a node for the left
    // side of a production, none for a group on its right side.
    let suffix_variants = enum_variants(&written, "ExprWithSuffix");

    assert_eq!(
        suffix_variants.len(),
        4,
        "`ExprWithSuffix` is {:?}",
        suffix_variants
    );

    for variant in &suffix_variants {
        for op in ["Question", "Plus", "Repeat"] {
            assert!(
                !variant.starts_with(&format!("ExprWithSuffix{op}(")),
                "`{op}` is a variant of `ExprWithSuffix`, so the group was flattened \
                 into the node of its left side"
            );
        }
    }

    // `'[' ( SetItem ',' )* SetItem? ']'` — the separator and the items are
    // one list, and the brackets close a delimiter around what is inside.
    assert!(
        written.contains("Punctuated<SetItem<I>,Comma<I>>"),
        "`a b*` was not folded into one list: {written}"
    );

    // `bootstrap.uns:145-153` is `'{' ( ',' LitDec | LitDec ','
    // LitDec? | LitDec ) '}'`, so the node named `Repeat` is the whole
    // braced run and not what sits between the braces. The hand-written side now
    // has that same boundary: `syntax/expr.rs` declares `Repeat<I>` as
    // `Brace<I, RepeatGroup<I>>`, and `lexical/delimiter.rs`'s
    // `the_span_of_a_group_reaches_its_closing_token` parses `"{2, 3}"` as a
    // `Repeat<_>` whose span reaches the `}`. The name the generated body carries
    // (`Repeat1`) is the right-side group P5 still owes as an inline `Or<…>`.
    assert!(
        written.contains(
            "structRepeat<I>(pub::parserc::syntax::Delimiter<BraceStart<I>,BraceEnd<I>,\
             Repeat1<I>>);"
        ),
        "`Repeat` is not a single delimiter over its body: {written}"
    );
}
