//! Benchmarks for the `TokenStream` input primitives behind the `input` feature.

use divan::{Bencher, black_box, counter::BytesCount};
use parserc::{Find, Input, StartWith, bytes, chars};

type Bytes = bytes::TokenStream<'static>;
type Chars = chars::TokenStream<'static>;

/// Leaks a string once per benchmark case so generated inputs can borrow it.
fn leak(text: impl Into<String>) -> &'static str {
    Box::leak(text.into().into_boxed_str())
}

mod bytes_stream {
    use super::*;

    #[divan::bench(args = [64, 1024, 16384])]
    fn split_to_half(bencher: Bencher, len: usize) {
        let text = leak("ab".repeat(len / 2));

        bencher
            .counter(BytesCount::from(len / 2))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|mut input| black_box(input.split_to(len / 2)));
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn iter_all(bencher: Bencher, len: usize) {
        let text = leak("ab".repeat(len / 2));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|input| black_box(input.iter().map(u64::from).sum::<u64>()));
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn iter_indices_all(bencher: Bencher, len: usize) {
        let text = leak("ab".repeat(len / 2));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|input| {
                black_box(
                    input
                        .iter_indices()
                        .map(|(i, b)| i + b as usize)
                        .sum::<usize>(),
                )
            });
    }

    #[divan::bench(args = [4, 16, 64])]
    fn starts_with_hit(bencher: Bencher, len: usize) {
        let needle = leak("k".repeat(len));
        let text = leak(format!("{needle}rest"));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|input| black_box(StartWith::<&str>::starts_with(&input, needle)));
    }

    #[divan::bench(args = [4, 16, 64])]
    fn starts_with_miss(bencher: Bencher, len: usize) {
        let needle = leak("k".repeat(len));
        let text = leak(format!("{}rest", "z".repeat(len)));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|input| black_box(StartWith::<&str>::starts_with(&input, needle)));
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn find_at_end(bencher: Bencher, len: usize) {
        let text = leak(format!("{}z", "a".repeat(len)));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|input| black_box(Find::<&str>::find(&input, "z")));
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn find_absent(bencher: Bencher, len: usize) {
        let text = leak("a".repeat(len));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Bytes::from(text))
            .bench_values(|input| black_box(Find::<&str>::find(&input, "z")));
    }
}

mod chars_stream {
    use super::*;

    #[divan::bench(args = [64, 1024, 16384])]
    fn split_to_half(bencher: Bencher, len: usize) {
        let text = leak("ab".repeat(len / 2));

        bencher
            .counter(BytesCount::from(len / 2))
            .with_inputs(move || Chars::from(text))
            .bench_values(|mut input| black_box(input.split_to(len / 2)));
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn iter_ascii(bencher: Bencher, len: usize) {
        let text = leak("ab".repeat(len / 2));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Chars::from(text))
            .bench_values(|input| black_box(input.iter().map(|c| c as u64).sum::<u64>()));
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn iter_multibyte(bencher: Bencher, len: usize) {
        // `é` encodes to two bytes per char.
        let text = leak("é".repeat(len));

        bencher
            .counter(BytesCount::from(len * 2))
            .with_inputs(move || Chars::from(text))
            .bench_values(|input| black_box(input.iter().map(|c| c as u64).sum::<u64>()));
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn iter_indices_multibyte(bencher: Bencher, len: usize) {
        let text = leak("é".repeat(len));

        bencher
            .counter(BytesCount::from(len * 2))
            .with_inputs(move || Chars::from(text))
            .bench_values(|input| {
                black_box(
                    input
                        .iter_indices()
                        .map(|(i, c)| i + c as usize)
                        .sum::<usize>(),
                )
            });
    }

    #[divan::bench]
    fn starts_with_hit(bencher: Bencher) {
        let text = leak("keyword and the rest");

        bencher
            .with_inputs(move || Chars::from(text))
            .bench_values(|input| black_box(StartWith::<&str>::starts_with(&input, "keyword")));
    }

    #[divan::bench(args = [64, 1024, 16384])]
    fn find_absent(bencher: Bencher, len: usize) {
        let text = leak("a".repeat(len));

        bencher
            .counter(BytesCount::from(len))
            .with_inputs(move || Chars::from(text))
            .bench_values(|input| black_box(Find::<&str>::find(&input, "z")));
    }
}

fn main() {
    divan::main();
}
