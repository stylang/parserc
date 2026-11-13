//! The four stages of `unsyn`, run over two specs and reported as they go.
//!
//! A spec is read once per stage of the crate: its text becomes a stream of
//! tokens, the tokens the CST of the DSL, the CST the analyzed module of the
//! nodes it names, and the module the Rust of the tree those nodes describe.
//!
//! ```text
//! cargo run -p unsyn --example pipeline
//! cargo run -p unsyn --example pipeline -- target/unsyn_bootstrap.rs
//! ```
//!
//! Both forms report the same four stages of the two specs. The second also
//! writes what `bootstrap.uns` generates, which is the spec worth generating,
//! being the grammar of `unsyn` itself: what comes out is a fragment of items,
//! for a crate that depends on `parserc` and `unsyn` to `include!` or to declare
//! as a module. [`Codegen`] is where the paths a generated file reaches those
//! crates by, the visibility of its types and their derives are chosen.
//!
//! A generated file is handed to `rustfmt` when that is on the `PATH`, since the
//! spacing a generator's tokens come out with says nothing about the file it
//! means; a report shows the types it declares rather than the whole of it, and
//! without `rustfmt` it shows the tokens themselves, cut short.
//!
//! Two things a report like this one cannot say, and two tests can: that the
//! generated types *run* against real text is what `tests/generated.rs` says, and
//! that the analyzer's decisions are the ones the language means is what the
//! tests of the `semantics` module says. The small spec carried here is
//! deliberately written with two redundancies in it, so that the lint channel has
//! something to report: lints note a spec, they never refuse it.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use parserc::{ParseError, Span, syntax::Syntax};
use unsyn::{
    codegen::{Codegen, CodegenError},
    errors::Lint,
    input::TokenStream,
    semantics::{NodeKind, analyze},
    syntax::{Crate, Item},
};

/// The edition a generated file is formatted as, which is the crate's own.
const EDITION: &str = "2024";

/// The characters of a generated file an unformatted report quotes, which is
/// about what a formatted one shows of a type.
const EXCERPT: usize = 4 * 24;

/// A small language, written in the DSL, whose every generated type a report has
/// room to show.
///
/// `OP` lists `':'` twice and `Entry` names the trivia its separator takes
/// anyway, which are the two redundancies the analyzer notes.
const DEMO: &str = r#"//! A tiny configuration language.

whitespace WS -> U+0009 | U+000A | U+0020;

lexer LETTER -> ['a'-'z', 'A'-'Z'];
lexer DIGIT -> ['0'-'9'];
lexer KEY -> LETTER ( LETTER | DIGIT | '_' )*;
lexer NAME -> <XID_Start> <XID_Continue>*;
lexer OP -> ['=', ':', '+', ':'];
lexer SEP -> ',' | ';';
lexer NOTE -> '#' ~[U+000A]*;

syntax Value -> KEY | Number | List | Group;
syntax Number -> DIGIT{1, 9};
syntax List -> '[' Value ( SEP Value )* ']';
syntax Group -> ( '{' Binding* '}' ) except ':';
syntax Binding -> NAME OP Value;
syntax Entry -> Binding WS Binding;
syntax Config -> Entry*;
"#;

fn main() {
    let written = std::env::args().nth(1).map(PathBuf::from);

    let mut refused = 0;

    refused += run(
        "a tiny configuration language, written in this example",
        DEMO,
        None,
    );

    refused += run(
        "the grammar of `unsyn`, written in `unsyn`: bootstrap.uns",
        read("bootstrap.uns"),
        written.as_deref(),
    );

    if refused > 0 {
        eprintln!("{refused} of the two specs did not reach the end of the pipeline");
        std::process::exit(1);
    }
}

/// The three first stages of one spec and the Rust the fourth writes, reported
/// line by line. Returns `1` for a spec that was refused on the way.
fn run(label: &str, source: &'static str, write_to: Option<&Path>) -> u8 {
    println!("\n{}", "-".repeat(78));
    println!("{label}");
    println!(
        "  {} lines, {} bytes of DSL",
        source.lines().count(),
        source.len()
    );

    // The text becomes tokens, and the tokens the CST of the language.
    let mut tokens = TokenStream::from(source);

    let file = match Crate::parse(&mut tokens) {
        Ok(file) => file,
        Err(error) => {
            println!("  parse       FAILED at {}", at(source, &error.to_span()));
            println!("              {error}");
            return 1;
        }
    };

    println!(
        "  parse       {} items, {} of them definitions",
        file.items.len(),
        file.items
            .iter()
            .filter(|item| matches!(item, Item::Stmt(_)))
            .count()
    );

    // The CST becomes the module of nodes it names: what every reference
    // denotes, what every value is, and whether any rule can start with itself.
    let module = match analyze(&file) {
        Ok(module) => module,
        Err(errors) => {
            println!("  analyze     FAILED, {} reports", errors.len());
            for error in errors.iter() {
                // The message of an `UnsynError` ends with the region it was
                // found in as offsets; the line above it says the same place
                // where a reader counts lines and columns.
                println!(
                    "              {} covering `{}`",
                    at(source, &error.to_span()),
                    covering(source, &error.to_span()),
                );
                println!("                {error}");
            }

            return 1;
        }
    };

    println!("  analyze     {} nodes", module.symbols().len());

    for kind in [NodeKind::Whitespace, NodeKind::Lexer, NodeKind::Syntax] {
        let names: Vec<&str> = module
            .symbols()
            .iter()
            .filter(|def| def.kind() == kind)
            .map(|def| def.name())
            .collect();

        if !names.is_empty() {
            println!("  {}", fold(&format!("{:>11} ", kind.keyword()), &names));
        }
    }

    // `<NAME>` asks a matcher the crate that includes the file must provide.
    println!(
        "              {} matchers called: {}",
        module.matchers().len(),
        if module.matchers().is_empty() {
            "none".to_owned()
        } else {
            module.matchers().join(", ")
        }
    );

    if module.lints().is_empty() {
        println!("              nothing said twice");
    } else {
        println!(
            "              {} lints, which note the spec rather than refuse it:",
            module.lints().len()
        );

        for lint in module.lints() {
            println!("{}", linted(source, lint));
        }
    }

    // The module becomes the Rust of the tree it describes.
    let raw = match Codegen::new().generate_str(&module) {
        Ok(raw) => raw,
        Err(errors) => {
            println!("  codegen     FAILED, {} reports", errors.len());
            for error in errors.iter() {
                let CodegenError { kind, span } = error;
                println!("              {kind} at {}", at(source, span));
            }

            return 1;
        }
    };

    let formatted = prettify(&raw);
    let rust = formatted.as_deref().unwrap_or(&raw);

    println!(
        "  codegen     {} types over {} bytes of Rust{}",
        types(rust),
        raw.len(),
        if formatted.is_some() {
            String::new()
        } else {
            " (not formatted: `rustfmt` is not on the PATH)".to_owned()
        }
    );

    if let Some(path) = write_to {
        match fs::write(path, rust) {
            Ok(()) => println!("  written     {}", path.display()),
            Err(error) => println!("  written     FAILED {path:?}: {error}"),
        }
    }

    let (declared, left) = declarations(rust);

    println!();

    if declared.is_empty() {
        println!("  {}", unformatted(rust));
    } else {
        println!(
            "  the {} types it declares{}:",
            types(rust),
            if left == 0 {
                String::new()
            } else {
                format!(", {} of them here", types(rust) - left)
            }
        );

        for line in &declared {
            println!("    {line}");
        }
    }

    0
}

/// Reads a spec for as long as the module built from it will borrow it.
fn read(filename: &str) -> &'static str {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(filename);
    let text = fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {path:?}: {error}"));

    // A `Module` borrows the source it was analyzed from, which outlives the
    // stages of this example that were built from it.
    Box::leak(text.into_boxed_str())
}

/// One lint: what was said twice, and where.
fn linted(source: &str, lint: &Lint) -> String {
    format!(
        "              {} at {}\n                covering `{}`",
        lint.kind,
        at(source, &lint.span),
        covering(source, &lint.span),
    )
}

/// Lays the names of one kind out over as few lines as `WIDTH` allows, each led
/// by the keyword that declared them.
fn fold(lead: &str, names: &[&str]) -> String {
    const WIDTH: usize = 78;

    let mut lines: Vec<String> = vec![];
    let mut line = lead.to_owned();

    for name in names {
        if line.len() + name.len() + 1 > WIDTH {
            lines.push(line.trim_end().to_owned());
            line = " ".repeat(lead.len());
        }

        line.push_str(name);
        line.push(' ');
    }

    lines.push(line.trim_end().to_owned());

    lines.join("\n")
}

/// The types a generated file declares, which is the part of it a report can
/// show: each `pub struct` and `pub enum` with the fields or variants that make
/// it up.
///
/// Returns the lines to print and the number of types that did not fit.
fn declarations(rust: &str) -> (Vec<String>, usize) {
    /// The lines of Rust one report holds.
    const MOST: usize = 36;

    let lines: Vec<&str> = rust.lines().collect();
    let mut shown: Vec<String> = vec![];
    let mut left = 0;
    let mut here = 0;

    while here < lines.len() {
        if !lines[here].starts_with("pub struct ") && !lines[here].starts_with("pub enum ") {
            here += 1;
            continue;
        }

        // A declaration runs to the line that closes it, which `rustfmt` puts
        // back in the first column, whether it closes an enum or a tuple struct
        // wrapped over several lines; its members are the indented lines there.
        let start = here;
        here += 1;

        while here < lines.len() && lines[here].starts_with(' ') {
            here += 1;
        }

        if matches!(lines.get(here), Some(&"}") | Some(&")")) {
            here += 1;
        }

        let block = &lines[start..here];

        if shown.len() + block.len() > MOST {
            left += 1;
        } else {
            shown.extend(block.iter().map(|line| (*line).to_owned()));
        }
    }

    (shown, left)
}

/// How many types a generated file declares.
fn types(rust: &str) -> usize {
    rust.matches("pub struct ").count() + rust.matches("pub enum ").count()
}

/// What to show of a file `rustfmt` did not put into lines: its tokens, cut
/// short, since a wall of them says nothing to a reader.
fn unformatted(raw: &str) -> String {
    let text = raw.chars().take(EXCERPT).collect::<String>();

    if text.chars().count() < raw.chars().count() {
        format!("{text} …")
    } else {
        text
    }
}

/// The line and column a span starts at, both counted from one.
fn at(source: &str, span: &Span) -> String {
    let Some((start, _)) = region(span) else {
        return "an unknown place".to_owned();
    };

    let before = &source[..start.min(source.len())];
    let tail = &before[before.rfind('\n').map_or(0, |seen| seen + 1)..];

    format!(
        "{}:{}",
        before.matches('\n').count() + 1,
        tail.chars().count() + 1
    )
}

/// The text a span covers, folded onto one line and cut short, which is what
/// lets a report quote the construct it is talking about.
fn covering(source: &str, span: &Span) -> String {
    let Some((start, end)) = region(span) else {
        return "nothing".to_owned();
    };

    let start = start.min(source.len());
    let text = &source[start..end.clamp(start, source.len())];
    let one = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = one.chars().take(48).collect();

    if chars.len() < one.chars().count() {
        format!("{}…", chars.into_iter().collect::<String>())
    } else {
        one
    }
}

/// The region of the source a span covers.
///
/// A `Span` of the input this crate parses over is a region of byte offsets, so
/// it indexes straight into the text. `Span::None` covers nothing and
/// `Span::RangeFull` everything, neither of which is worth quoting.
fn region(span: &Span) -> Option<(usize, usize)> {
    match span {
        Span::Range(range) => Some((range.start, range.end)),
        Span::RangeFrom(range) => Some((range.start, range.start)),
        Span::RangeTo(range) => Some((range.end, range.end)),
        Span::RangeFull | Span::None => None,
    }
}

/// The generated tokens as `rustfmt` would write them, or `None` when it is not
/// around to be asked, or does not accept what was written.
fn prettify(raw: &str) -> Option<String> {
    let mut rustfmt = Command::new("rustfmt")
        .args(["--edition", EDITION, "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    {
        let mut pipe = rustfmt.stdin.take()?;
        pipe.write_all(raw.as_bytes()).ok()?;
    }
    // Closing the pipe here is what tells `rustfmt` that the file it reads is
    // over; the run would otherwise wait for the rest of it forever.

    let written = rustfmt.wait_with_output().ok()?;

    if written.status.success() {
        String::from_utf8(written.stdout).ok()
    } else {
        None
    }
}
