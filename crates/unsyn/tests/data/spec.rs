//  The spec both `tests/generated.rs` and `tests/dump.rs` are written against.
//
//  One copy of it is all the fixture can be checked against, since the drift
//  test compares what the generator writes for these rules with what
//  `tests/data/mini.rs` holds.

const SPEC: &str = r#"whitespace WS ->
      ' '
    | U+0009
    | U+000A
    ;

lexer LETTER -> ['a'-'z', 'A'-'Z'];
lexer DIGIT -> ['0'-'9'];
lexer WORD -> LETTER (LETTER | DIGIT | '_')*;
lexer LET -> 'let';
lexer FN -> 'fn';
lexer KEYWORDS -> LET | FN;
lexer IDENT -> WORD except KEYWORDS;
lexer OPEN -> '(';
lexer CLOSE -> ')';
lexer COMMA -> ',';
lexer SEMI -> ';';
lexer ARROW -> '->';
lexer SHARP -> '#';
lexer LEFT -> '[';
lexer RIGHT -> ']';
lexer TILDE -> '~';
lexer LETTER_OR_ANY -> (LETTER | _)+;
lexer QUOTE -> '\'';
lexer BSLASH -> '\\';

syntax ITEM -> KEYWORDS IDENT ARROW EXPR SEMI;

syntax EXPR -> PRIMARY (BINOP PRIMARY)*;

syntax PRIMARY -> IDENT | LITERAL | GROUP | NEGATED;

syntax GROUP -> OPEN EXPR CLOSE;

syntax NEGATED -> TILDE PRIMARY;

syntax BINOP -> ARROW | '+' | '-';

syntax LITERAL -> NUMBER | TEXT;

syntax NUMBER -> DIGIT (DIGIT | '_')*;

syntax TEXT -> QUOTE (~[BSLASH, QUOTE] | BSLASH ['n', 't'])* QUOTE;

syntax PARAMS -> OPEN (IDENT (COMMA IDENT)*)? CLOSE;

syntax LIST -> OPEN IDENT* CLOSE;

syntax TUPLE -> OPEN IDENT (COMMA IDENT)+ CLOSE;

syntax SHAPE -> SHARP DIGIT{4};

lexer MATCHER -> <WORD_START>;
lexer SLASH -> '/';
lexer STAR -> '*';
lexer UNDER -> '_';

syntax ANYONE -> (MATCHER | _) (MATCHER | _)*;
syntax Joined -> DIGIT concat UNDER DIGIT;
syntax Guarded -> IDENT followed '-';
syntax Commented -> SLASH STAR (~[STAR] | UNDER)* STAR SLASH;
syntax Either -> (IDENT | DIGIT) LEFT IDENT;
syntax Dotted -> IDENT{2, 3};

"#;

//  The header `tests/data/mini.rs` carries and the generator does not write.
//
//  It is shared by the two tests that read and write that fixture, since the
//  one says what the other keeps. It is line comments rather than a document
//  comment because a comment is the one part of a generated file a kept copy
//  may add without the copy ceasing to be what was generated.
const HEADER: &str = "\
// The Rust the generator wrote for the spec in `data/spec.rs`.
//
// This file is machine-made: `cargo test -p unsyn --test dump -- --nocapture`
// prints what `Codegen` produces for that spec, and the result is kept here so
// that the emitted types are compiled and run rather than only read back as
// tokens. `tests/generated.rs` compares the two whole, and says so when they
// fall apart.
";
