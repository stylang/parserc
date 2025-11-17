use divan::bench;
use parserc::syntax::Syntax;
use pattex::{input::TokenStream, pattern::Pattern};
use regex::bytes::Regex;

fn main() {
    divan::main();
}

#[bench(sample_count = 10000)]
fn bench_regex() {
    divan::black_box_drop(
        Regex::new(r"^(http|https)://[a-zA-Z0-9\-\.]+\.[a-zA-Z]{2,3}(/\S*)?$").unwrap(),
    );
}

#[bench(sample_count = 10000)]
fn bench_pattex() {
    divan::black_box_drop(
        Pattern::parse(&mut TokenStream::from(
            r"^(http|https)://[a-zA-Z0-9\-\.]+\.[a-zA-Z]{2,3}(/\S*)?$",
        ))
        .unwrap(),
    );
}
