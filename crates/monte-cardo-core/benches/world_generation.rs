use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{rngs::SmallRng, SeedableRng};
use std::hint::black_box;

use monte_cardo_core::world::{card_dealing, greedy_stars_and_bars, mass_mixing};

const M: usize = 18;
const N: usize = 18;

const MASSES: [usize; 4] = [20, 50, 100, 200];
const NUM_MATRICES: [usize; 4] = [20, 50, 100, 200];

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

        // Deterministic uneven cap between 0 and 4.
        // Some columns get zero, mimicking exhausted card types.
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

    // If mass exceeds the total first-pass capacity, keep distributing.
    let mut i = 0;
    while remaining > 0 {
        let col = (i * 7 + 3) % N;
        cols[col] += 1;
        remaining -= 1;
        i += 1;
    }

    (rows, cols)
}

fn bench_single_matrix_algorithm<F>(
    c: &mut Criterion,
    group_name: &str,
    algorithm_name: &str,
    margin_name: &str,
    mass: usize,
    num_matrices: usize,
    margins: fn(usize) -> ([usize; M], [usize; N]),
    algorithm: F,
) where
    F: Fn([usize; M], [usize; N], &mut SmallRng) -> [[usize; N]; M] + Copy,
{
    let mut group = c.benchmark_group(group_name);

    group.throughput(Throughput::Elements(num_matrices as u64));

    group.bench_with_input(
        BenchmarkId::new(
            algorithm_name,
            format!("{margin_name},mass={mass},num_matrices={num_matrices}"),
        ),
        &(mass, num_matrices),
        |b, &(mass, num_matrices)| {
            b.iter_batched(
                || {
                    let (row_margins, col_margins) = margins(mass);

                    // Recreate the RNG inside setup so the measured call starts
                    // from the same state for each sample.
                    let rng = SmallRng::seed_from_u64(12345);

                    (row_margins, col_margins, rng)
                },
                |(row_margins, col_margins, mut rng)| {
                    let mut outputs = Vec::with_capacity(num_matrices);

                    for _ in 0..num_matrices {
                        let output = algorithm(
                            black_box(row_margins),
                            black_box(col_margins),
                            black_box(&mut rng),
                        );

                        outputs.push(output);
                    }

                    black_box(outputs);
                },
                criterion::BatchSize::SmallInput,
            );
        },
    );

    group.finish();
}

fn bench_batch_algorithm<F>(
    c: &mut Criterion,
    group_name: &str,
    algorithm_name: &str,
    margin_name: &str,
    mass: usize,
    num_matrices: usize,
    margins: fn(usize) -> ([usize; M], [usize; N]),
    algorithm: F,
) where
    F: Fn(usize, [usize; M], [usize; N], &mut SmallRng) -> Vec<[[usize; N]; M]> + Copy,
{
    let mut group = c.benchmark_group(group_name);

    group.throughput(Throughput::Elements(num_matrices as u64));

    group.bench_with_input(
        BenchmarkId::new(
            algorithm_name,
            format!("{margin_name},mass={mass},num_matrices={num_matrices}"),
        ),
        &(mass, num_matrices),
        |b, &(mass, num_matrices)| {
            b.iter_batched(
                || {
                    let (row_margins, col_margins) = margins(mass);

                    // Recreate the RNG inside setup so the measured call starts
                    // from the same state for each sample.
                    let rng = SmallRng::seed_from_u64(12345);

                    (row_margins, col_margins, rng)
                },
                |(row_margins, col_margins, mut rng)| {
                    let outputs = algorithm(
                        black_box(num_matrices),
                        black_box(row_margins),
                        black_box(col_margins),
                        black_box(&mut rng),
                    );

                    black_box(outputs);
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
    num_matrices: usize,
    margins: fn(usize) -> ([usize; M], [usize; N]),
) {
    bench_single_matrix_algorithm(
        c,
        "world_generation_18x18_batch",
        "card_dealing",
        margin_name,
        mass,
        num_matrices,
        margins,
        card_dealing::<M, N>,
    );

    bench_single_matrix_algorithm(
        c,
        "world_generation_18x18_batch",
        "greedy_stars_and_bars",
        margin_name,
        mass,
        num_matrices,
        margins,
        greedy_stars_and_bars::<M, N>,
    );

    bench_batch_algorithm(
        c,
        "world_generation_18x18_batch",
        "mass_mixing",
        margin_name,
        mass,
        num_matrices,
        margins,
        mass_mixing::<M, N>,
    );
}

fn bench_margin_family(
    c: &mut Criterion,
    margin_name: &str,
    margins: fn(usize) -> ([usize; M], [usize; N]),
) {
    for mass in MASSES {
        for num_matrices in NUM_MATRICES {
            bench_case(c, margin_name, mass, num_matrices, margins);
        }
    }
}

fn bench_world_generation(c: &mut Criterion) {
    bench_margin_family(c, "card_like", card_like_margins::<M, N>);
    bench_margin_family(c, "balanced", balanced_margins::<M, N>);
    bench_margin_family(c, "skewed", skewed_margins::<M, N>);
    bench_margin_family(c, "sparse", sparse_margins::<M, N>);
}

criterion_group!(benches, bench_world_generation);
criterion_main!(benches);
