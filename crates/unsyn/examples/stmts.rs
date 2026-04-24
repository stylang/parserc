//! This example demonstrates how to parse an unsyn source file into statements

use std::{fs, path::PathBuf};

use clap::Parser;
use parserc::{
    sourceinput::AsStr,
    syntax::{Syntax, SyntaxExt},
};
use unsyn::{
    input::Chars,
    syntax::{File, Item, ModuleDeclaration, OuterDoc, Stmt, UseDeclaration},
    token::punct::Semi,
};

#[derive(Parser, Debug)]
struct Args {
    /// The parsing `unsyn` source file.
    input: PathBuf,
    /// The Output directory for the generated files.
    output: PathBuf,
}

fn main() {
    // parse input arguments
    let args = Args::parse();

    // read source file
    let buf = fs::read_to_string(&args.input).unwrap();

    // construct parser input.
    let mut input = Chars::new(&buf);

    // parse file.
    let file = File::parse(&mut input).unwrap();

    let spec_root_dir = args.output;

    let spec_outer_doc_dir = spec_root_dir.join("outer_doc");

    fs::create_dir_all(&spec_outer_doc_dir).unwrap();

    let spec_use_dir = spec_root_dir.join("use");

    fs::create_dir_all(&spec_use_dir).unwrap();

    let spec_mod_dir = spec_root_dir.join("module");

    fs::create_dir_all(&spec_mod_dir).unwrap();

    let spec_stmt_dir = spec_root_dir.join("stmt");

    fs::create_dir_all(&spec_stmt_dir).unwrap();

    // Construct a new input for code extraction.
    let input = Chars::new(&buf);

    let source = args.input.with_extension("");

    let filename = source.file_name().unwrap().to_str().unwrap();

    println!("{}", filename);

    for item in file.items {
        match item {
            Item::OuterDoc(outer_doc) => {
                let span = outer_doc.to_span();

                let filepath = spec_outer_doc_dir.join(format!(
                    "{}({},{}).syn",
                    filename,
                    span.start().unwrap(),
                    span.end().unwrap()
                ));

                fs::write(&filepath, input.as_str_with(span).unwrap()).unwrap();

                let outer_doc =
                    OuterDoc::parse(&mut Chars::new(input.as_str_with(span).unwrap())).unwrap();

                fs::write(
                    filepath.with_extension("json"),
                    serde_json::to_string_pretty(&outer_doc).unwrap(),
                )
                .unwrap();
            }
            Item::S(_) => {}
            Item::Use(use_declaration, semi) => {
                let span = use_declaration.to_span() + semi.to_span();

                let filepath = spec_use_dir.join(format!(
                    "{}({},{}).syn",
                    filename,
                    span.start().unwrap(),
                    span.end().unwrap()
                ));

                fs::write(&filepath, input.as_str_with(span).unwrap()).unwrap();

                let item = Chars::new(input.as_str_with(span).unwrap())
                    .parse::<(UseDeclaration<_>, Semi<_>)>()
                    .unwrap();

                fs::write(
                    filepath.with_extension("json"),
                    serde_json::to_string_pretty(&item).unwrap(),
                )
                .unwrap();
            }
            Item::Mod(module_declaration, semi) => {
                let span = module_declaration.to_span() + semi.to_span();

                let filepath = spec_mod_dir.join(format!(
                    "{}({},{}).syn",
                    filename,
                    span.start().unwrap(),
                    span.end().unwrap()
                ));

                fs::write(&filepath, input.as_str_with(span).unwrap()).unwrap();

                let item = Chars::new(input.as_str_with(span).unwrap())
                    .parse::<(ModuleDeclaration<_>, Semi<_>)>()
                    .unwrap();

                fs::write(
                    filepath.with_extension("json"),
                    serde_json::to_string_pretty(&item).unwrap(),
                )
                .unwrap();
            }
            Item::Stmt(stmt) => {
                let span = stmt.to_span();

                let filepath = spec_stmt_dir.join(format!(
                    "{}({},{}).syn",
                    filename,
                    span.start().unwrap(),
                    span.end().unwrap()
                ));

                fs::write(&filepath, input.as_str_with(span).unwrap()).unwrap();

                let item = Chars::new(input.as_str_with(span).unwrap())
                    .parse::<Stmt<_>>()
                    .unwrap();

                fs::write(
                    filepath.with_extension("json"),
                    serde_json::to_string_pretty(&item).unwrap(),
                )
                .unwrap();
            }
        }
    }
}
