//! The committed front end is what the generator writes, and nothing else.
//!
//! `src/generated` holds code, not a record of a build: the crate is compiled from
//! it, and `src/bin/unsyn-gen.rs` is the run that makes it. That is only honest
//! while the two agree, and a file nobody regenerates drifts from the generator and
//! from the spec silently — the compile passes, and the parser a reader thinks they
//! are looking at is not the parser the crate ships.
//!
//! So the run is asked for here, into a directory of its own, and each file is
//! compared with what is committed. The comparison is over tokens and not over
//! text, because `cargo fmt` is expected to have laid the committed files out: a
//! line break, an indent and the trailing comma a broken list ends with say nothing
//! about what the generator wrote.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::{Item, ItemUse, UseTree};

/// The directory the generated files are committed to.
const GENERATED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/generated");

#[test]
fn the_committed_files_are_the_ones_the_generator_writes() {
    let out = fresh();

    let mut written = names(&out);
    let mut committed = names(Path::new(GENERATED));
    written.sort();
    committed.sort();

    assert_eq!(
        committed, written,
        "`src/generated` holds a file the generator does not write, or is missing one"
    );

    for file in written {
        let kept = fs::read_to_string(format!("{GENERATED}/{file}")).expect("a committed file");
        let made = fs::read_to_string(out.join(&file)).expect("a freshly written file");

        assert_eq!(
            tokens(&kept),
            tokens(&made),
            "`src/generated/{file}` is not what the generator writes; run `cargo run -p unsyn \
             --bin unsyn-gen && cargo fmt`. The two first part company here: {}",
            where_they_part(&tokens(&kept), &tokens(&made)),
        );
    }
}

#[test]
fn every_committed_file_says_it_is_written() {
    for file in names(Path::new(GENERATED)) {
        let text = fs::read_to_string(format!("{GENERATED}/{file}")).expect("a committed file");

        assert!(
            text.starts_with("//!") && text.contains("unsyn-gen"),
            "`src/generated/{file}` does not begin by saying where it comes from"
        );
    }
}

/// The code the generator writes now, in a directory nothing else reads.
///
/// The binary is run rather than its logic copied here: the point is that the
/// committed files are what *that* run makes, and a second statement of the same
/// configuration would be a second thing to keep true.
fn fresh() -> PathBuf {
    let out = env::temp_dir().join(format!("unsyn-generated-{}", std::process::id()));

    let _ = fs::remove_dir_all(&out);

    let status = Command::new(env!("CARGO_BIN_EXE_unsyn-gen"))
        .arg(&out)
        .status()
        .expect("the generator runs");

    assert!(status.success(), "the generator did not run");
    assert!(out.is_dir(), "the generator wrote no directory");

    out
}

/// The files of a directory, without their directories.
fn names(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .unwrap_or_else(|_| panic!("{} is a directory", dir.display()))
        .map(|entry| {
            entry
                .expect("an entry of a directory")
                .path()
                .file_name()
                .expect("a file of the generated directory")
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

/// Where two laid-flat files first disagree, with what stands around it.
///
/// A whole generated file flattened to one line is no answer to read, and the
/// difference is nearly always of one token.
fn where_they_part(kept: &str, made: &str) -> String {
    let mut apart = 0;

    for (a, b) in kept.chars().zip(made.chars()) {
        if a != b {
            break;
        }

        apart += a.len_utf8();
    }

    let around = |text: &str| {
        let here = apart.min(text.len());
        let before = text[..here].chars().rev().take(80).collect::<Vec<_>>();
        let after = &text[here..];
        let ends = after
            .char_indices()
            .nth(160)
            .map(|(at, _)| at)
            .unwrap_or(after.len());

        format!(
            "…{}⟨{}⟩{}",
            before.into_iter().rev().collect::<String>(),
            &after[..ends],
            &after[ends..],
        )
    };

    format!("\ncommitted: {}\nwritten:   {}", around(kept), around(made))
}

/// What a file is written with, past its layout.
///
/// Two texts the same code was laid out differently in come to one answer here,
/// which is what lets `cargo fmt` be run over the generated files and this test to
/// still mean "the generator wrote this". Three layout choices are given up
/// explicitly: the formatter sorts a list of `use` statements and the names inside
/// one, neither of which says anything about what the file reaches, and it ends a
/// broken list with a trailing comma, which says nothing about the items in it.
fn tokens(text: &str) -> String {
    let file = syn::parse_file(text).expect("a file of the generated Rust");
    let mut out = String::new();

    for attr in &file.attrs {
        word(&mut out, &flat(&attr.to_token_stream()));
    }

    of_items(&file.items, &mut out);

    out
}

/// The items of a module in the order they are declared, with its imports sorted
/// apart and gathered first.
///
/// This runs at every depth, because the formatter reorders an import wherever it
/// stands: a `use` at the head of a generated file and one inside the module of
/// walks it opens are the same question of layout.
fn of_items(list: &[Item], out: &mut String) {
    let (uses, rest): (Vec<&Item>, Vec<&Item>) =
        list.iter().partition(|item| matches!(item, Item::Use(_)));

    let mut uses = uses
        .into_iter()
        .filter_map(|item| match item {
            Item::Use(branch) => Some(import(branch)),
            _ => None,
        })
        .collect::<Vec<_>>();
    uses.sort();

    for name in uses {
        word(out, &name);
    }

    for item in rest {
        match item {
            Item::Mod(branch) => {
                for attr in &branch.attrs {
                    word(out, &flat(&attr.to_token_stream()));
                }

                word(out, &flat(&branch.vis.to_token_stream()));
                word(out, &format!("mod {}", branch.ident));

                if let Some((_, inner)) = &branch.content {
                    of_items(inner, out);
                }
            }
            other => word(out, &flat(&other.to_token_stream())),
        }
    }
}

/// One more piece of an answer laid flat.
fn word(out: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }

    if !out.is_empty() {
        out.push(' ');
    }

    out.push_str(text);
}

/// One import, as the sorted names it reaches.
///
/// The formatter puts the names of a `{ … }` in order and the order the generator
/// wrote them in says nothing about what the file needed: an import is a set, and
/// this reads it as one.
fn import(item: &ItemUse) -> String {
    let mut names = Vec::new();

    gather(&item.tree, &mut Vec::new(), &mut names);
    names.sort();

    format!("use {}", names.join(", "))
}

/// Every name one branch of an import reaches, with the path that leads to it.
fn gather(tree: &UseTree, prefix: &mut Vec<String>, names: &mut Vec<String>) {
    match tree {
        UseTree::Path(branch) => {
            prefix.push(branch.ident.to_string());
            gather(&branch.tree, prefix, names);
            prefix.pop();
        }
        UseTree::Name(leaf) => names.push(reach(prefix, &leaf.ident.to_string())),
        UseTree::Rename(leaf) => {
            names.push(reach(prefix, &format!("{} as {}", leaf.ident, leaf.rename)));
        }
        UseTree::Glob(_) => names.push(reach(prefix, "*")),
        UseTree::Group(branch) => {
            for inner in &branch.items {
                gather(inner, prefix, names);
            }
        }
    }
}

/// A name under the path that leads to it.
fn reach(prefix: &[String], last: &str) -> String {
    let mut path = prefix.join("::");

    if !path.is_empty() && !last.is_empty() {
        path.push_str("::");
    }

    path.push_str(last);
    path
}

/// One token stream, laid flat.
fn flat(tokens: &TokenStream) -> String {
    let mut out = String::new();

    write(&mut out, tokens);

    out
}

/// One token stream, laid flat.
fn write(out: &mut String, tokens: &TokenStream) {
    let mut trees = tokens.clone().into_iter().collect::<Vec<_>>();

    // A list broken across lines takes a trailing comma from the formatter, which
    // is layout and not code: the same items with and without it are one list.
    while matches!(trees.last(), Some(TokenTree::Punct(p)) if p.as_char() == ',') {
        trees.pop();
    }

    for tree in trees {
        if !out.is_empty() {
            out.push(' ');
        }

        match tree {
            TokenTree::Group(group) => {
                let (open, close) = match group.delimiter() {
                    proc_macro2::Delimiter::Parenthesis => ("(", ")"),
                    proc_macro2::Delimiter::Brace => ("{", "}"),
                    proc_macro2::Delimiter::Bracket => ("[", "]"),
                    proc_macro2::Delimiter::None => ("", ""),
                };

                out.push_str(open);
                write(out, &group.stream());
                out.push_str(close);
            }
            other => out.push_str(&other.to_string()),
        }
    }
}
