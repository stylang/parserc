//! The gate on which code may name the shape of a node.
//!
//! Three of the types the language is written in are named for the productions
//! that reference them — [`ExprWithoutSuffix`], [`ExprWithSuffix`], [`SetItem`] —
//! so a bootstrapped parser, being built from the file that says so rather than
//! from Rust that happens to, gives those nodes the same shapes only if the file
//! keeps saying the same thing. The names are therefore load-bearing in exactly
//! one place: the module that defines them, and the walk that reads a node down to
//! its tokens and its punctuation, which it can only do by asking which shape the
//! node has.
//!
//! Everywhere else — a check, a graph, a generator — a pass wants to know *which
//! shape* a node has, not what the hand-written enum that happened to hold it last
//! time called it. [`Operand`], [`Suffixed`] and [`SetMember`] answer that
//! question, and a pass written against them reads a generated tree exactly as it
//! reads this one. A pass that names a variant of a shape type instead does not
//! break when the grammar changes its shape; it breaks when the grammar keeps its
//! shape and changes its *name*, which is the failure no test of a running program
//! can catch and this gate exists to prevent.
//!
//! The scan runs over tokens rather than over a [`syn`] AST. The thing looked for
//! is a qualified path, and a qualified path is the same two tokens wherever it
//! appears: an AST walk would have to visit every expression and pattern position
//! there is, and would still miss the one inside `matches!`, where the pattern is
//! not an expression at all.
//!
//! [`ExprWithoutSuffix`]: unsyn::syntax::ExprWithoutSuffix
//! [`ExprWithSuffix`]: unsyn::syntax::ExprWithSuffix
//! [`SetItem`]: unsyn::syntax::SetItem
//! [`Operand`]: unsyn::syntax::Operand
//! [`Suffixed`]: unsyn::syntax::Suffixed
//! [`SetMember`]: unsyn::syntax::SetMember

use std::path::{Path, PathBuf};

use proc_macro2::{Spacing, TokenStream, TokenTree};
use walkdir::WalkDir;

/// The types whose variants only their own module and the walk may name.
const SHAPES: [&str; 3] = ["ExprWithoutSuffix", "ExprWithSuffix", "SetItem"];

/// The directories, named by the first component of a path under `src`, that earn
/// the exception: `syntax` is where the shapes are written, and `visit` is where
/// the walk reports the delimiters and punctuation a pass would not otherwise be
/// told about.
const EXCLUDED: [&str; 2] = ["syntax", "visit"];

/// Reads one file of this crate.
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Every Rust file under `src`, with its path both relative to `src` and whole.
fn sources() -> Vec<(PathBuf, PathBuf)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");

    WalkDir::new(&root)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.expect("src is walkable");

            entry.file_type().is_file().then(|| entry.into_path())
        })
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .map(|path| {
            let relative = path.strip_prefix(&root).expect("under src").to_path_buf();

            (relative, path)
        })
        .collect()
}

/// Whether a file, given by its path under `src`, is one of the two excused.
fn excused(relative: &Path) -> bool {
    relative
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .is_some_and(|first| EXCLUDED.contains(&first))
}

/// One qualified use of a shape type: which type, and which variant of it.
type Qualified = (String, String);

/// The qualified uses of [`SHAPES`] variants in a file's text.
///
/// Reports one for every `Shape :: Variant` the file has, wherever it has it —
/// including inside the brackets of a macro call, which is where a `matches!` over
/// the shape of a node would be written.
fn qualified(relative: &Path, text: &str) -> Vec<Qualified> {
    let tokens = text
        .parse::<TokenStream>()
        .unwrap_or_else(|e| panic!("{} is not tokenisable Rust: {e}", relative.display()));

    let mut found = Vec::new();

    walk(&mut tokens.into_iter(), &mut None, &mut |shape, variant| {
        found.push((shape.to_owned(), variant.to_owned()));
    });

    found
}

/// Walks a token stream, holding the name an `::` was seen just after.
///
/// `after` is `Some` only in the stretch of tokens between `Foo` and the `::` that
/// qualifies it, so a name followed by anything else — a bracket, a comma, a
/// single colon of a struct field — starts the next path clean.
fn walk(
    tokens: &mut dyn Iterator<Item = TokenTree>,
    after: &mut Option<String>,
    on: &mut dyn FnMut(&str, &str),
) {
    while let Some(tree) = tokens.next() {
        match tree {
            TokenTree::Ident(ident) => {
                let text = ident.to_string();

                // Only an uppercase name after `::` can be a variant: the rule the
                // language follows is Rust's, that a variant is written in camel
                // case and an associated function is not, which keeps a call like
                // `SetItem::parse` off this list.
                if let Some(shape) = after.take()
                    && SHAPES.contains(&shape.as_str())
                    && text.starts_with(|c: char| c.is_ascii_uppercase())
                {
                    on(&shape, &text);
                }

                *after = text
                    .starts_with(|c: char| c.is_ascii_uppercase())
                    .then_some(text);
            }
            // A `::` is the only thing that may stand between a type and one of
            // its variants, so it is the only thing that leaves `after` standing.
            // A single `:` — a struct field, a closure parameter — ends it.
            TokenTree::Punct(punct)
                if punct.as_char() == ':' && punct.spacing() == Spacing::Joint =>
            {
                // The second colon is taken here rather than by the next turn of
                // the loop, and a joint colon with nothing to join to is not Rust
                // that parses, so nothing is lost by dropping the token.
                if !matches!(tokens.next(), Some(TokenTree::Punct(second)) if second.as_char() == ':')
                {
                    *after = None;
                }
            }
            TokenTree::Group(group) => {
                *after = None;

                walk(&mut group.stream().into_iter(), &mut None, on);
            }
            _ => *after = None,
        }
    }
}

#[test]
fn no_pass_outside_the_shapes_names_a_variant_of_one() {
    let mut violations = Vec::new();
    let mut scanned = 0;

    for (relative, path) in sources() {
        if excused(&relative) {
            continue;
        }

        scanned += 1;

        violations.extend(qualified(&relative, &read(&path)));
    }

    assert!(
        scanned >= 10,
        "only {scanned} files were scanned, which is not the crate"
    );

    assert!(
        violations.is_empty(),
        "a pass named a variant of a shape type it should read through a view \
         (`Operand`, `Suffixed` or `SetMember`) instead:\n{}",
        violations
            .iter()
            .map(|(shape, variant)| format!("{shape}::{variant}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_scan_is_not_blind() {
    let text = "\
fn f(e: &ExprWithSuffix<I>) -> usize {
    match e {
        ExprWithSuffix::Concat { .. } => 1,
        // A `matches!` is where a shape test hides, and its pattern is not an
        // expression, which is why this is a token scan.
        _ => matches!(e, SetItem::Range { .. }) as usize,
    }
}

fn g(t: &ExprWithoutSuffix<I>) {
    let _ = ExprWithoutSuffix::Paren(t);
    // Naming the type is legal: as a parameter, as a qualified call of an
    // associated function, and inside a path that belongs to something else.
    let _: Option<ExprWithSuffix<I>> = None;
    let _ = SetItem::parse;
    let _ = <ExprWithoutSuffix<I> as Default>::default();
}
";

    let named = qualified(Path::new("inline.rs"), text);

    assert_eq!(
        named,
        [
            ("ExprWithSuffix".to_owned(), "Concat".to_owned()),
            ("SetItem".to_owned(), "Range".to_owned()),
            ("ExprWithoutSuffix".to_owned(), "Paren".to_owned()),
        ],
        "the scan did not read that text"
    );
}

#[test]
fn the_two_excused_directories_do_indeed_name_the_shapes() {
    // Were either of them to stop qualifying a shape, it should come off the list
    // of exceptions rather than go on excusing whatever is written in it next.
    for dir in EXCLUDED {
        let named = sources()
            .into_iter()
            .filter(|(relative, _)| excused(relative))
            .filter(|(relative, _)| relative.starts_with(dir))
            .flat_map(|(relative, path)| qualified(&relative, &read(&path)))
            .count();

        assert!(
            named > 0,
            "{dir} qualifies no shape type, so it has no business being excused"
        );
    }
}
