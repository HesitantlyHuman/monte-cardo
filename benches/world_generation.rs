use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{rngs::SmallRng, SeedableRng};
use std::hint::black_box;

use monte_cardo::ai::world::{card_dealing, progressive_stars_and_bars};

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

    // Put more mass in early rows/cols, while preserving total mass.
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

fn bench_one<const M: usize, const N: usize, const T: usize, F>(
    c: &mut Criterion,
    group_name: &str,
    algorithm_name: &str,
    margin_name: &str,
    mass: usize,
    margins: fn(usize) -> ([usize; M], [usize; N]),
    algorithm: F,
) where
    F: Fn([usize; M], [usize; N], &mut SmallRng) -> [[usize; N]; M] + Copy,
{
    let mut group = c.benchmark_group(group_name);

    group.throughput(Throughput::Elements(mass as u64));

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

                    // Recreate the RNG inside setup so the measured call starts
                    // from the same state for each sample.
                    let rng = SmallRng::seed_from_u64(12345);

                    (row_margins, col_margins, rng)
                },
                |(row_margins, col_margins, mut rng)| {
                    let output = algorithm(
                        black_box(row_margins),
                        black_box(col_margins),
                        black_box(&mut rng),
                    );

                    black_box(output);
                },
                criterion::BatchSize::SmallInput,
            );
        },
    );

    group.finish();
}

fn bench_case<const M: usize, const N: usize, const T: usize>(
    c: &mut Criterion,
    margin_name: &str,
    mass: usize,
    margins: fn(usize) -> ([usize; M], [usize; N]),
) {
    bench_one::<M, N, T, _>(
        c,
        "world_generation",
        "card_dealing",
        margin_name,
        mass,
        margins,
        card_dealing::<M, N, T>,
    );

    bench_one::<M, N, T, _>(
        c,
        "world_generation",
        "progressive_stars_and_bars",
        margin_name,
        mass,
        margins,
        progressive_stars_and_bars::<M, N, T>,
    );
}

fn bench_world_generation(c: &mut Criterion) {
    // Small square
    bench_case::<4, 4, 16>(c, "balanced", 20, balanced_margins::<4, 4>);
    bench_case::<4, 4, 16>(c, "balanced", 100, balanced_margins::<4, 4>);
    bench_case::<4, 4, 16>(c, "balanced", 1_000, balanced_margins::<4, 4>);

    // Medium square
    bench_case::<8, 8, 64>(c, "balanced", 100, balanced_margins::<8, 8>);
    bench_case::<8, 8, 64>(c, "balanced", 1_000, balanced_margins::<8, 8>);
    bench_case::<8, 8, 64>(c, "balanced", 10_000, balanced_margins::<8, 8>);

    // Rectangular
    bench_case::<4, 16, 64>(c, "balanced", 100, balanced_margins::<4, 16>);
    bench_case::<4, 16, 64>(c, "balanced", 1_000, balanced_margins::<4, 16>);
    bench_case::<4, 16, 64>(c, "balanced", 10_000, balanced_margins::<4, 16>);

    bench_case::<16, 4, 64>(c, "balanced", 100, balanced_margins::<16, 4>);
    bench_case::<16, 4, 64>(c, "balanced", 1_000, balanced_margins::<16, 4>);
    bench_case::<16, 4, 64>(c, "balanced", 10_000, balanced_margins::<16, 4>);

    // Larger square
    bench_case::<16, 16, 256>(c, "balanced", 100, balanced_margins::<16, 16>);
    bench_case::<16, 16, 256>(c, "balanced", 1_000, balanced_margins::<16, 16>);
    bench_case::<16, 16, 256>(c, "balanced", 10_000, balanced_margins::<16, 16>);

    // Wider/taller cases
    bench_case::<8, 32, 256>(c, "balanced", 1_000, balanced_margins::<8, 32>);
    bench_case::<8, 32, 256>(c, "balanced", 10_000, balanced_margins::<8, 32>);

    bench_case::<32, 8, 256>(c, "balanced", 1_000, balanced_margins::<32, 8>);
    bench_case::<32, 8, 256>(c, "balanced", 10_000, balanced_margins::<32, 8>);

    // Margin-structure comparisons
    bench_case::<16, 16, 256>(c, "skewed", 100, skewed_margins::<16, 16>);
    bench_case::<16, 16, 256>(c, "skewed", 1_000, skewed_margins::<16, 16>);
    bench_case::<16, 16, 256>(c, "skewed", 10_000, skewed_margins::<16, 16>);

    bench_case::<16, 16, 256>(c, "sparse", 100, sparse_margins::<16, 16>);
    bench_case::<16, 16, 256>(c, "sparse", 1_000, sparse_margins::<16, 16>);
    bench_case::<16, 16, 256>(c, "sparse", 10_000, sparse_margins::<16, 16>);
}

criterion_group!(benches, bench_world_generation);
criterion_main!(benches);
