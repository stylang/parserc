//! Benchmarks for the AST building blocks behind the `syntax` feature.

use divan::{Bencher, black_box, counter::ItemsCount};
use parserc::{
    Parser,
    chars::TokenStream,
    syntax::{Char, Delimiter, Or, Punctuated, Syntax},
};

type Chars = TokenStream<'static>;

type A = Char<Chars, 'a'>;
type B = Char<Chars, 'b'>;
type Comma = Char<Chars, ','>;
type Run = Vec<A>;
type Parens = Delimiter<Char<Chars, '('>, Char<Chars, ')'>, Run>;

/// Leaks a string once per benchmark case so generated inputs can borrow it.
fn leak(text: impl Into<String>) -> &'static str {
    Box::leak(text.into().into_boxed_str())
}

mod nodes {
    use super::*;

    #[divan::bench]
    fn char_node(bencher: Bencher) {
        bencher
            .with_inputs(|| Chars::from("a"))
            .bench_values(|mut input| black_box(A::parse(&mut input)));
    }

    #[divan::bench]
    fn char_node_via_parser(bencher: Bencher) {
        bencher
            .with_inputs(|| Chars::from("a"))
            .bench_values(|mut input| black_box(A::into_parser().parse(&mut input)));
    }

    #[divan::bench]
    fn option_some(bencher: Bencher) {
        bencher
            .with_inputs(|| Chars::from("a"))
            .bench_values(|mut input| black_box(Option::<A>::parse(&mut input)));
    }

    #[divan::bench]
    fn option_none(bencher: Bencher) {
        bencher
            .with_inputs(|| Chars::from("b"))
            .bench_values(|mut input| black_box(Option::<A>::parse(&mut input)));
    }

    #[divan::bench]
    fn tuple_pair(bencher: Bencher) {
        type Pair = (A, B);

        bencher
            .with_inputs(|| Chars::from("ab"))
            .bench_values(|mut input| black_box(Pair::parse(&mut input)));
    }
}

mod composite {
    use super::*;

    #[divan::bench(args = [16, 1024, 65536])]
    fn vec_run(bencher: Bencher, count: usize) {
        let text = leak("a".repeat(count));

        bencher
            .counter(ItemsCount::from(count))
            .with_inputs(move || Chars::from(text))
            .bench_values(|mut input| black_box(Run::parse(&mut input)));
    }

    #[divan::bench(args = [16, 1024, 4096])]
    fn punctuated_pairs(bencher: Bencher, count: usize) {
        let text = leak("a,".repeat(count));

        bencher
            .counter(ItemsCount::from(count))
            .with_inputs(move || Chars::from(text))
            .bench_values(|mut input| black_box(Punctuated::<A, Comma>::parse(&mut input)));
    }

    #[divan::bench(args = [16, 1024, 65536])]
    fn delimiter_group(bencher: Bencher, body_len: usize) {
        let text = leak(format!("({})", "a".repeat(body_len)));

        bencher
            .counter(ItemsCount::from(body_len))
            .with_inputs(move || Chars::from(text))
            .bench_values(|mut input| black_box(Parens::parse(&mut input)));
    }

    #[divan::bench]
    fn or_first_hit(bencher: Bencher) {
        bencher
            .with_inputs(|| Chars::from("a"))
            .bench_values(|mut input| black_box(Or::<A, B>::parse(&mut input)));
    }

    #[divan::bench]
    fn or_second_fallback(bencher: Bencher) {
        bencher
            .with_inputs(|| Chars::from("b"))
            .bench_values(|mut input| black_box(Or::<A, B>::parse(&mut input)));
    }
}

fn main() {
    divan::main();
}
