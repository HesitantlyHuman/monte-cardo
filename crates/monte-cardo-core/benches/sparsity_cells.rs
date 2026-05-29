use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;

use monte_cardo_core::world::sparsity::{
    cell_positive_probabilities_log_lookup, cell_positive_probabilities_product,
};

const M: usize = 18;
const N: usize = 18;

const MASSES: [usize; 5] = [20, 50, 100, 200, 1_000];

fn balanced_margins<const M: usize, const N: usize>(mass: usize) -> ([usize; M], [usize; N]) {
    let mut rows = [0usize; M];
    let mut cols = [0usize; N];

    for i in 0..mass {
        rows[i % M] += 1;
        cols[i % N] += 1;
    }

    (rows, cols)
}

fn skewed_margins<const M: usize, const N: usize>(mass: usize) -> ([usize; M], [usize; N]) {
    let mut rows = [0usize; M];
    let mut cols = [0usize; N];

    for i in 0..mass {
        let row = (i * i + 3 * i) % M;
        let col = (i * i + 5 * i) % N;

        rows[row] += 1;
        cols[col] += 1;
    }

    (rows, cols)
}

fn sparse_margins<const M: usize, const N: usize>(mass: usize) -> ([usize; M], [usize; N]) {
    let mut rows = [0usize; M];
    let mut cols = [0usize; N];

    let active_rows = (M / 4).max(1);
    let active_cols = (N / 4).max(1);

    for i in 0..mass {
        rows[i % active_rows] += 1;
        cols[i % active_cols] += 1;
    }

    (rows, cols)
}

fn card_like_margins<const M: usize, const N: usize>(mass: usize) -> ([usize; M], [usize; N]) {
    let mut rows = [0usize; M];
    let mut cols = [0usize; N];

    // Rows: roughly balanced hand sizes.
    for i in 0..mass {
        rows[i % M] += 1;
    }

    // Columns: small bounded piles spread over card types in a pseudo-random order.
    let mut remaining = mass;

    for i in 0..N {
        if remaining == 0 {
            break;
        }

        let col = (i * 7 + 3) % N;

        let cap = match i % 6 {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            _ => 4,
        };

        let amount = cap.min(remaining);
        cols[col] += amount;
        remaining -= amount;
    }

    let mut i = 0;
    while remaining > 0 {
        let col = (i * 7 + 3) % N;
        cols[col] += 1;
        remaining -= 1;
        i += 1;
    }

    (rows, cols)
}

fn bench_cell_probability_algorithm<F>(
    c: &mut Criterion,
    group_name: &str,
    algorithm_name: &str,
    margin_name: &str,
    mass: usize,
    margins: fn(usize) -> ([usize; M], [usize; N]),
    algorithm: F,
) where
    F: Fn(&[usize; M], &[usize; N]) -> [[f64; N]; M] + Copy,
{
    let mut group = c.benchmark_group(group_name);

    group.throughput(Throughput::Elements((M * N) as u64));

    group.bench_with_input(
        BenchmarkId::new(
            algorithm_name,
            format!("{margin_name},M={M},N={N},mass={mass}"),
        ),
        &mass,
        |b, &mass| {
            b.iter_batched(
                || {
                    let (row_margins, col_margins) = margins(mass);
                    (row_margins, col_margins)
                },
                |(row_margins, col_margins)| {
                    let output = algorithm(black_box(&row_margins), black_box(&col_margins));

                    black_box(output);
                },
                criterion::BatchSize::SmallInput,
            );
        },
    );

    group.finish();
}

fn bench_case(
    c: &mut Criterion,
    margin_name: &str,
    mass: usize,
    margins: fn(usize) -> ([usize; M], [usize; N]),
) {
    bench_cell_probability_algorithm(
        c,
        "cell_positive_probabilities_18x18",
        "log_lookup",
        margin_name,
        mass,
        margins,
        cell_positive_probabilities_log_lookup::<M, N>,
    );

    bench_cell_probability_algorithm(
        c,
        "cell_positive_probabilities_18x18",
        "product",
        margin_name,
        mass,
        margins,
        cell_positive_probabilities_product::<M, N>,
    );
}

fn bench_margin_family(
    c: &mut Criterion,
    margin_name: &str,
    margins: fn(usize) -> ([usize; M], [usize; N]),
) {
    for mass in MASSES {
        bench_case(c, margin_name, mass, margins);
    }
}

fn bench_cell_probabilities(c: &mut Criterion) {
    bench_margin_family(c, "card_like", card_like_margins::<M, N>);
    bench_margin_family(c, "balanced", balanced_margins::<M, N>);
    bench_margin_family(c, "skewed", skewed_margins::<M, N>);
    bench_margin_family(c, "sparse", sparse_margins::<M, N>);
}

criterion_group!(benches, bench_cell_probabilities);
criterion_main!(benches);
