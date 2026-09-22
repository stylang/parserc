//! Benchmarks for the tokenizer combinators and the `Parser` adapters.

use divan::{
    Bencher, black_box,
    counter::{BytesCount, ItemsCount},
};
use parserc::{
    Input, Parser, bytes::TokenStream, keyword, next, next_if, take_till, take_until, take_while,
    take_while_with,
};

type Bytes = TokenStream<'static>;

/// Leaks a string once per benchmark case so generated inputs can borrow it.
fn leak(text: impl Into<String>) -> &'static str {
    Box::leak(text.into().into_boxed_str())
}

mod combinators {
    use super::*;

    #[divan::bench]
    fn next_hit(bencher: Bencher) {
        let text = leak("a");

        bencher
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| black_box(next::<Bytes>(b'a').parse(&mut input)));
    }

    #[divan::bench]
    fn next_miss(bencher: Bencher) {
        let text = leak("b");

        bencher
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| black_box(next::<Bytes>(b'a').parse(&mut input)));
    }

    #[divan::bench]
    fn next_at_end(bencher: Bencher) {
        let text = leak("");

        bencher
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| black_box(next::<Bytes>(b'a').parse(&mut input)));
    }

    #[divan::bench]
    fn next_if_hit(bencher: Bencher) {
        let text = leak("a");

        bencher
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| {
                black_box(next_if::<Bytes, _>(|c| c == b'a').parse(&mut input))
            });
    }

    #[divan::bench]
    fn next_if_miss(bencher: Bencher) {
        let text = leak("b");

        bencher
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| {
                black_box(next_if::<Bytes, _>(|c| c == b'a').parse(&mut input))
            });
    }

    #[divan::bench(args = [4, 16, 64])]
    fn keyword_hit(bencher: Bencher, kw_len: usize) {
        let kw = leak("k".repeat(kw_len));
        let text = leak(format!("{kw}rest"));

        bencher
            .counter(BytesCount::from(kw_len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| black_box(keyword::<_, Bytes>(kw).parse(&mut input)));
    }

    #[divan::bench(args = [4, 16, 64])]
    fn keyword_miss(bencher: Bencher, kw_len: usize) {
        let kw = leak("k".repeat(kw_len));
        let text = leak(format!("{}rest", "z".repeat(kw_len)));

        bencher
            .counter(BytesCount::from(kw_len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| black_box(keyword::<_, Bytes>(kw).parse(&mut input)));
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn take_until_at_end(bencher: Bencher, len: usize) {
        let text = leak(format!("{}|", "a".repeat(len)));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| black_box(take_until::<Bytes, _>("|").parse(&mut input)));
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn take_until_absent(bencher: Bencher, len: usize) {
        let text = leak("a".repeat(len));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| black_box(take_until::<Bytes, _>("|").parse(&mut input)));
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn take_while_full_run(bencher: Bencher, len: usize) {
        let text = leak("a".repeat(len));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| {
                black_box(take_while::<Bytes, _>(|c| c == b'a').parse(&mut input))
            });
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn take_while_zero_length(bencher: Bencher, len: usize) {
        let text = leak("b".repeat(len));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| {
                black_box(take_while::<Bytes, _>(|c| c == b'a').parse(&mut input))
            });
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn take_while_with_at_most(bencher: Bencher, len: usize) {
        let text = leak("a".repeat(len * 2));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| {
                black_box(take_while_with::<Bytes, _, _>(..=len, |c| c == b'a').parse(&mut input))
            });
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn take_while_with_under_min(bencher: Bencher, len: usize) {
        let text = leak("a".repeat(len));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| {
                black_box(
                    take_while_with::<Bytes, _, _>(len + 1.., |c| c == b'a').parse(&mut input),
                )
            });
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn take_till_break_at_end(bencher: Bencher, len: usize) {
        let text = leak(format!("{}b", "a".repeat(len)));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| {
                black_box(take_till::<Bytes, _>(|c| c == b'b').parse(&mut input))
            });
    }
}

/// Overhead of the `Parser` adapters on an identical one-byte parse.
mod adapters {
    use super::*;

    #[divan::bench]
    fn bare(bencher: Bencher) {
        bencher
            .with_inputs(|| Bytes::from("a"))
            .bench_values(|mut input| black_box(next::<Bytes>(b'a').parse(&mut input)));
    }

    #[divan::bench]
    fn map(bencher: Bencher) {
        bencher
            .with_inputs(|| Bytes::from("a"))
            .bench_values(|mut input| {
                black_box(
                    next::<Bytes>(b'a')
                        .map(|taken| taken.len())
                        .parse(&mut input),
                )
            });
    }

    #[divan::bench]
    fn ok_hit(bencher: Bencher) {
        bencher
            .with_inputs(|| Bytes::from("a"))
            .bench_values(|mut input| black_box(next::<Bytes>(b'a').ok().parse(&mut input)));
    }

    #[divan::bench]
    fn ok_miss(bencher: Bencher) {
        bencher
            .with_inputs(|| Bytes::from("b"))
            .bench_values(|mut input| black_box(next::<Bytes>(b'a').ok().parse(&mut input)));
    }

    #[divan::bench]
    fn fatal(bencher: Bencher) {
        bencher
            .with_inputs(|| Bytes::from("a"))
            .bench_values(|mut input| black_box(next::<Bytes>(b'a').fatal().parse(&mut input)));
    }

    #[divan::bench]
    fn boxed(bencher: Bencher) {
        bencher
            .with_inputs(|| Bytes::from("a"))
            .bench_values(|mut input| black_box(next::<Bytes>(b'a').boxed().parse(&mut input)));
    }

    #[divan::bench]
    fn or_left_hit(bencher: Bencher) {
        bencher
            .with_inputs(|| Bytes::from("a"))
            .bench_values(|mut input| {
                black_box(
                    next::<Bytes>(b'a')
                        .or(next::<Bytes>(b'b'))
                        .parse(&mut input),
                )
            });
    }

    #[divan::bench]
    fn or_right_fallback(bencher: Bencher) {
        bencher
            .with_inputs(|| Bytes::from("b"))
            .bench_values(|mut input| {
                black_box(
                    next::<Bytes>(b'a')
                        .or(next::<Bytes>(b'b'))
                        .parse(&mut input),
                )
            });
    }
}

/// Realistic loops composing tokenizer combinators.
mod tokenize {
    use super::*;

    /// Splits `count` space-separated words with [`take_while`].
    #[divan::bench(args = [8, 128, 2048])]
    fn words(bencher: Bencher, count: usize) {
        let text = leak(vec!["word"; count].join(" "));

        bencher
            .counter(ItemsCount::from(count))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| {
                let mut seen = 0usize;

                while !input.is_empty() {
                    let _ = black_box(
                        take_while::<Bytes, _>(|c| c.is_ascii_alphabetic()).parse(&mut input),
                    );
                    let _ = black_box(take_while::<Bytes, _>(|c| c == b' ').parse(&mut input));
                    seen += 1;
                }

                black_box(seen)
            });
    }

    /// Parses `count` comma-separated fields with [`take_while`] and [`next`].
    #[divan::bench(args = [8, 128, 2048])]
    fn csv_fields(bencher: Bencher, count: usize) {
        let text = leak("field,".repeat(count));

        bencher
            .counter(ItemsCount::from(count))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| {
                let mut seen = 0usize;

                while !input.is_empty() {
                    let _ = black_box(
                        take_while::<Bytes, _>(|c| c.is_ascii_alphabetic()).parse(&mut input),
                    );
                    let _ = black_box(next::<Bytes>(b',').parse(&mut input));
                    seen += 1;
                }

                black_box(seen)
            });
    }
}

fn main() {
    divan::main();
}
