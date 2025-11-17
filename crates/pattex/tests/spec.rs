use parserc::syntax::Syntax;
use pattex::{input::TokenStream, pattern::Pattern};

#[test]
fn pattern_parse_spec() {
    let mut errors = 0;

    for line in include_str!("regex.spec").lines() {
        print!("spec `{}`", line);

        match Pattern::parse(&mut TokenStream::from(line)) {
            Ok(_) => {
                color_print::cprintln!(" ... <g>ok</>");
            }

            Err(err) => {
                color_print::cprintln!("<r>... failed.</>");
                println!("{}", err);

                errors += 1;
            }
        }
    }

    if errors != 0 {
        panic!("run spec {} failed", errors);
    }
}
