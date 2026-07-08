use std::sync::LazyLock;

use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{RngExt, SeedableRng};

use crate::world::simple::simple_valid_greedy;
use crate::world::sparsity::calculate_sparsity_factor;

const SUBMATRIX_CYCLE_SEED: u64 = 1234;

#[derive(Debug, Clone, Copy)]
struct Submatrix {
    r0: u8,
    r1: u8,
    c0: u8,
    c1: u8,
}

fn generate_submatrix_cycle<const M: usize, const N: usize>() -> Vec<Submatrix> {
    assert!(M <= u8::MAX as usize + 1);
    assert!(N <= u8::MAX as usize + 1);

    let num_row_pairs = M * (M - 1) / 2;
    let num_col_pairs = N * (N - 1) / 2;

    let mut cycle = Vec::with_capacity(num_row_pairs * num_col_pairs);

    for r0 in 0..(M - 1) {
        for r1 in (r0 + 1)..M {
            for c0 in 0..(N - 1) {
                for c1 in (c0 + 1)..N {
                    cycle.push(Submatrix {
                        r0: r0 as u8,
                        r1: r1 as u8,
                        c0: c0 as u8,
                        c1: c1 as u8,
                    });
                }
            }
        }
    }

    let mut shuffle_rng = SmallRng::seed_from_u64(SUBMATRIX_CYCLE_SEED);
    cycle.shuffle(&mut shuffle_rng);

    cycle
}

// TODO: import the card matrix size (actually shouldn't it be slightly less than that, since we have the current player removed?)
static SUBMATRIX_CYCLE: LazyLock<Box<[Submatrix]>> =
    LazyLock::new(|| generate_submatrix_cycle::<18, 18>().into_boxed_slice());

const INITIALIZATION_SWEEPS: usize = 5;
const MIXING_SWEEPS_PER_SAMPLE: usize = 1;
const STEPS_PER_SWEEP: usize = (18 - 1) * (18 - 1);

#[inline(always)]
fn mix_one_from_cycle<const M: usize, const N: usize>(
    matrix_to_mix: &mut [[usize; N]; M],
    submatrix_cycle: &[Submatrix],
    submatrix_selection_index: &mut usize,
    rng: &mut SmallRng,
) {
    let submatrix = submatrix_cycle[*submatrix_selection_index];

    *submatrix_selection_index += 1;
    if *submatrix_selection_index == submatrix_cycle.len() {
        *submatrix_selection_index = 0;
    }

    let (r0, r1, c0, c1) = (
        submatrix.r0 as usize,
        submatrix.r1 as usize,
        submatrix.c0 as usize,
        submatrix.c1 as usize,
    );

    let a = matrix_to_mix[r0][c0] as i32;
    let b = matrix_to_mix[r0][c1] as i32;
    let c = matrix_to_mix[r1][c0] as i32;
    let d = matrix_to_mix[r1][c1] as i32;

    let lower = -a.min(d);
    let upper = b.min(c);

    if lower == upper {
        return; // The only way that lower can equal upper is if they are both zero, because zero is always an option
    }

    let t = rng.random_range(lower..=upper);

    matrix_to_mix[r0][c0] = (a + t) as usize;
    matrix_to_mix[r0][c1] = (b - t) as usize;
    matrix_to_mix[r1][c0] = (c - t) as usize;
    matrix_to_mix[r1][c1] = (d + t) as usize;
}

pub fn mass_mixing_cycle_based<const M: usize, const N: usize>(
    num_samples: usize,
    row_margins: [usize; M],
    column_margins: [usize; N],
    rng: &mut SmallRng,
) -> Vec<[[usize; N]; M]> {
    let mut state = simple_valid_greedy(row_margins, column_margins);

    let submatrix_cycle: &[Submatrix] = &SUBMATRIX_CYCLE;
    let mut submatrix_selection_index = rng.random_range(0..submatrix_cycle.len());

    let sparsity = calculate_sparsity_factor(&row_margins, &column_margins);
    let sweep_size = (STEPS_PER_SWEEP as f64 / sparsity).ceil() as usize;

    for _ in 0..INITIALIZATION_SWEEPS * sweep_size {
        mix_one_from_cycle(
            &mut state,
            submatrix_cycle,
            &mut submatrix_selection_index,
            rng,
        );
    }

    let mut samples = Vec::with_capacity(num_samples);
    samples.push(state);

    for _ in 0..(num_samples - 1) {
        for _ in 0..MIXING_SWEEPS_PER_SAMPLE * sweep_size {
            mix_one_from_cycle(
                &mut state,
                submatrix_cycle,
                &mut submatrix_selection_index,
                rng,
            );
        }
        samples.push(state);
    }

    return samples;
}

fn build_positive_support<const M: usize, const N: usize>(
    matrix: &[[usize; N]; M],
) -> (Vec<(usize, usize)>, [[usize; N]; M], [usize; M], [usize; N]) {
    let mut positive_entries = Vec::new();
    let mut positive_entry_locations = [[NOT_POSITIVE; N]; M];
    let mut positive_row_counts = [0usize; M];
    let mut positive_col_counts = [0usize; N];

    for row in 0..M {
        for col in 0..N {
            if matrix[row][col] > 0 {
                add_positive_entry(
                    &mut positive_entries,
                    &mut positive_entry_locations,
                    &mut positive_row_counts,
                    &mut positive_col_counts,
                    row,
                    col,
                );
            }
        }
    }

    (
        positive_entries,
        positive_entry_locations,
        positive_row_counts,
        positive_col_counts,
    )
}

#[inline(always)]
fn sample_submatrix<const M: usize, const N: usize>(
    positive_entries: &[(usize, usize)],
    positive_row_counts: &[usize],
    positive_col_counts: &[usize],
    rng: &mut SmallRng,
) -> Submatrix {
    for _ in 0..500 {
        let first_item = rng.random_range(0..positive_entries.len());
        let (r0, c0) = positive_entries[first_item];

        let allowed_remaining =
            positive_entries.len() - positive_row_counts[r0] - positive_col_counts[c0] + 1;

        if allowed_remaining == 0 {
            continue;
        }

        let second_item = rng.random_range(0..allowed_remaining);
        let mut counter = second_item;
        for &(r1, c1) in positive_entries {
            if r1 == r0 || c1 == c0 {
                continue;
            }

            if counter == 0 {
                // We found it!
                return Submatrix {
                    r0: r0 as u8,
                    r1: r1 as u8,
                    c0: c0 as u8,
                    c1: c1 as u8,
                };
            }

            counter -= 1;
        }
    }

    panic!("Failed to find a valid mix location after 500 iterations!");
}

const NOT_POSITIVE: usize = usize::MAX;

#[inline(always)]
fn add_positive_entry<const M: usize, const N: usize>(
    positive_entries: &mut Vec<(usize, usize)>,
    positive_entry_locations: &mut [[usize; N]; M],
    positive_row_counts: &mut [usize],
    positive_col_counts: &mut [usize],
    row: usize,
    col: usize,
) {
    debug_assert_eq!(positive_entry_locations[row][col], NOT_POSITIVE);

    let index = positive_entries.len();
    positive_entries.push((row, col));
    positive_entry_locations[row][col] = index;

    positive_row_counts[row] += 1;
    positive_col_counts[col] += 1;
}

#[inline(always)]
fn remove_positive_entry<const M: usize, const N: usize>(
    positive_entries: &mut Vec<(usize, usize)>,
    positive_entry_locations: &mut [[usize; N]; M],
    positive_row_counts: &mut [usize],
    positive_col_counts: &mut [usize],
    row: usize,
    col: usize,
) {
    let index = positive_entry_locations[row][col];
    debug_assert_ne!(index, NOT_POSITIVE);

    positive_row_counts[row] -= 1;
    positive_col_counts[col] -= 1;

    let last_index = positive_entries.len() - 1;
    let last_entry = positive_entries[last_index];

    positive_entries.swap_remove(index);
    positive_entry_locations[row][col] = NOT_POSITIVE;

    // If we removed something other than the last element, update the moved
    // entry's stored location.
    if index != last_index {
        let (moved_row, moved_col) = last_entry;
        positive_entry_locations[moved_row][moved_col] = index;
    }
}

#[inline(always)]
fn update_positive_entry<const M: usize, const N: usize>(
    matrix_to_mix: &[[usize; N]; M],
    positive_entries: &mut Vec<(usize, usize)>,
    positive_entry_locations: &mut [[usize; N]; M],
    positive_row_counts: &mut [usize],
    positive_col_counts: &mut [usize],
    row: usize,
    col: usize,
) {
    let is_positive = matrix_to_mix[row][col] > 0;
    let was_positive = positive_entry_locations[row][col] != NOT_POSITIVE;

    match (was_positive, is_positive) {
        (false, true) => add_positive_entry(
            positive_entries,
            positive_entry_locations,
            positive_row_counts,
            positive_col_counts,
            row,
            col,
        ),
        (true, false) => remove_positive_entry(
            positive_entries,
            positive_entry_locations,
            positive_row_counts,
            positive_col_counts,
            row,
            col,
        ),
        _ => {}
    }
}

#[inline(always)]
fn mix_one_sampled<const M: usize, const N: usize>(
    matrix_to_mix: &mut [[usize; N]; M],
    positive_entries: &mut Vec<(usize, usize)>,
    positive_entry_locations: &mut [[usize; N]; M],
    positive_row_counts: &mut [usize],
    positive_col_counts: &mut [usize],
    rng: &mut SmallRng,
) {
    let submatrix = sample_submatrix::<M, N>(
        positive_entries,
        positive_row_counts,
        positive_col_counts,
        rng,
    );

    let (r0, r1, c0, c1) = (
        submatrix.r0 as usize,
        submatrix.r1 as usize,
        submatrix.c0 as usize,
        submatrix.c1 as usize,
    );

    let a = matrix_to_mix[r0][c0] as i32;
    let b = matrix_to_mix[r0][c1] as i32;
    let c = matrix_to_mix[r1][c0] as i32;
    let d = matrix_to_mix[r1][c1] as i32;

    let lower = -a.min(d);
    let upper = b.min(c);

    let t = rng.random_range(lower..=upper);

    if t == 0 {
        return;
    }

    matrix_to_mix[r0][c0] = (a + t) as usize;
    matrix_to_mix[r0][c1] = (b - t) as usize;
    matrix_to_mix[r1][c0] = (c - t) as usize;
    matrix_to_mix[r1][c1] = (d + t) as usize;

    update_positive_entry(
        matrix_to_mix,
        positive_entries,
        positive_entry_locations,
        positive_row_counts,
        positive_col_counts,
        r0,
        c0,
    );
    update_positive_entry(
        matrix_to_mix,
        positive_entries,
        positive_entry_locations,
        positive_row_counts,
        positive_col_counts,
        r0,
        c1,
    );
    update_positive_entry(
        matrix_to_mix,
        positive_entries,
        positive_entry_locations,
        positive_row_counts,
        positive_col_counts,
        r1,
        c0,
    );
    update_positive_entry(
        matrix_to_mix,
        positive_entries,
        positive_entry_locations,
        positive_row_counts,
        positive_col_counts,
        r1,
        c1,
    );
}

pub fn mass_mixing_sample_based<const M: usize, const N: usize>(
    num_samples: usize,
    row_margins: [usize; M],
    column_margins: [usize; N],
    rng: &mut SmallRng,
) -> Vec<[[usize; N]; M]> {
    let mut state = simple_valid_greedy(row_margins, column_margins);

    let (
        mut positive_entries,
        mut positive_entry_locations,
        mut positive_row_counts,
        mut positive_col_counts,
    ) = build_positive_support(&state);

    for _ in 0..INITIALIZATION_SWEEPS * STEPS_PER_SWEEP {
        mix_one_sampled(
            &mut state,
            &mut positive_entries,
            &mut positive_entry_locations,
            &mut positive_row_counts,
            &mut positive_col_counts,
            rng,
        );
    }

    let mut samples = Vec::with_capacity(num_samples);
    samples.push(state);

    for _ in 0..(num_samples - 1) {
        for _ in 0..MIXING_SWEEPS_PER_SAMPLE * STEPS_PER_SWEEP {
            mix_one_sampled(
                &mut state,
                &mut positive_entries,
                &mut positive_entry_locations,
                &mut positive_row_counts,
                &mut positive_col_counts,
                rng,
            );
        }
        samples.push(state);
    }

    return samples;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::SmallRng, SeedableRng};

    #[test]
    fn submatrix_cycle_generation_functions() {
        let cycle = generate_submatrix_cycle::<10, 10>();

        let num_row_pairs = 10 * (10 - 1) / 2;
        let num_col_pairs = 10 * (10 - 1) / 2;

        assert!(num_row_pairs * num_col_pairs == cycle.len());
    }

    fn assert_valid_solution<const M: usize, const N: usize>(
        matrix: &[[usize; N]; M],
        expected_row_margins: [usize; M],
        expected_column_margins: [usize; N],
    ) {
        for row_idx in 0..M {
            let row_sum: usize = matrix[row_idx].iter().sum();

            assert_eq!(
                row_sum, expected_row_margins[row_idx],
                "row {row_idx} had wrong sum; matrix = {matrix:?}",
            );
        }

        for col_idx in 0..N {
            let col_sum: usize = matrix.iter().map(|row| row[col_idx]).sum();

            assert_eq!(
                col_sum, expected_column_margins[col_idx],
                "column {col_idx} had wrong sum; matrix = {matrix:?}",
            );
        }
    }

    #[test]
    fn mass_mixing_generates_valid_solutions() {
        let row_margins = [3, 4, 7, 2, 5, 6, 1, 6, 8, 3, 2, 5, 7, 4, 6, 5, 3, 8];
        let column_margins = [4, 3, 6, 5, 4, 7, 2, 5, 6, 3, 8, 4, 2, 7, 5, 3, 6, 5];

        assert_eq!(
            row_margins.iter().sum::<usize>(),
            column_margins.iter().sum::<usize>(),
        );

        let mut rng = SmallRng::seed_from_u64(12345);

        let samples = mass_mixing_cycle_based::<18, 18>(100, row_margins, column_margins, &mut rng);

        assert_eq!(samples.len(), 100);

        for sample in &samples {
            assert_valid_solution(sample, row_margins, column_margins);
        }
    }

    fn assert_positive_support_consistent<const M: usize, const N: usize>(
        matrix: &[[usize; N]; M],
        positive_entries: &[(usize, usize)],
        positive_entry_locations: &[[usize; N]; M],
        positive_row_counts: &[usize],
        positive_col_counts: &[usize],
    ) {
        let mut expected_entries = Vec::new();
        let mut expected_row_counts = [0usize; M];
        let mut expected_col_counts = [0usize; N];

        for row in 0..M {
            for col in 0..N {
                let location = positive_entry_locations[row][col];

                if matrix[row][col] > 0 {
                    expected_entries.push((row, col));
                    expected_row_counts[row] += 1;
                    expected_col_counts[col] += 1;

                    assert_ne!(
                        location, NOT_POSITIVE,
                        "positive cell ({row}, {col}) was not listed in locations",
                    );

                    assert!(
                        location < positive_entries.len(),
                        "positive cell ({row}, {col}) had out-of-bounds location {location}",
                    );

                    assert_eq!(
                        positive_entries[location],
                        (row, col),
                        "location table pointed to the wrong positive entry",
                    );
                } else {
                    assert_eq!(
                        location, NOT_POSITIVE,
                        "zero cell ({row}, {col}) was marked positive",
                    );
                }
            }
        }

        assert_eq!(
            positive_entries.len(),
            expected_entries.len(),
            "positive_entries had wrong length",
        );

        assert_eq!(
            positive_row_counts, &expected_row_counts,
            "positive_row_counts were inconsistent",
        );

        assert_eq!(
            positive_col_counts, &expected_col_counts,
            "positive_col_counts were inconsistent",
        );

        // Make sure every positive_entries item is real and points back to itself.
        for (index, &(row, col)) in positive_entries.iter().enumerate() {
            assert!(
                row < M && col < N,
                "positive entry {index} had invalid coordinate ({row}, {col})",
            );

            assert!(
                matrix[row][col] > 0,
                "positive entry {index} pointed to zero cell ({row}, {col})",
            );

            assert_eq!(
                positive_entry_locations[row][col], index,
                "positive entry {index} did not round-trip through location table",
            );
        }
    }

    #[test]
    fn build_positive_support_matches_matrix() {
        let matrix = [[0, 2, 0, 1], [3, 0, 0, 0], [0, 0, 4, 0]];

        let (positive_entries, positive_entry_locations, positive_row_counts, positive_col_counts) =
            build_positive_support(&matrix);

        assert_positive_support_consistent(
            &matrix,
            &positive_entries,
            &positive_entry_locations,
            &positive_row_counts,
            &positive_col_counts,
        );

        assert_eq!(positive_entries.len(), 4);
        assert_eq!(positive_row_counts, [2, 1, 1]);
        assert_eq!(positive_col_counts, [1, 1, 1, 1]);
    }

    #[test]
    fn update_positive_entry_adds_new_positive_cell() {
        let mut matrix = [[0, 2, 0], [1, 0, 0], [0, 0, 3]];

        let (
            mut positive_entries,
            mut positive_entry_locations,
            mut positive_row_counts,
            mut positive_col_counts,
        ) = build_positive_support(&matrix);

        matrix[0][2] = 5;

        update_positive_entry(
            &matrix,
            &mut positive_entries,
            &mut positive_entry_locations,
            &mut positive_row_counts,
            &mut positive_col_counts,
            0,
            2,
        );

        assert_positive_support_consistent(
            &matrix,
            &positive_entries,
            &positive_entry_locations,
            &positive_row_counts,
            &positive_col_counts,
        );

        assert_eq!(positive_entries.len(), 4);
        assert_ne!(positive_entry_locations[0][2], NOT_POSITIVE);
    }

    #[test]
    fn update_positive_entry_removes_zeroed_cell() {
        let mut matrix = [[0, 2, 0], [1, 0, 0], [0, 0, 3]];

        let (
            mut positive_entries,
            mut positive_entry_locations,
            mut positive_row_counts,
            mut positive_col_counts,
        ) = build_positive_support(&matrix);

        matrix[0][1] = 0;

        update_positive_entry(
            &matrix,
            &mut positive_entries,
            &mut positive_entry_locations,
            &mut positive_row_counts,
            &mut positive_col_counts,
            0,
            1,
        );

        assert_positive_support_consistent(
            &matrix,
            &positive_entries,
            &positive_entry_locations,
            &positive_row_counts,
            &positive_col_counts,
        );

        assert_eq!(positive_entries.len(), 2);
        assert_eq!(positive_entry_locations[0][1], NOT_POSITIVE);
    }

    #[test]
    fn update_positive_entry_handles_swap_remove_location_update() {
        let mut matrix = [[1, 2, 0], [3, 0, 0], [0, 0, 4]];

        let (
            mut positive_entries,
            mut positive_entry_locations,
            mut positive_row_counts,
            mut positive_col_counts,
        ) = build_positive_support(&matrix);

        // Remove a cell that is likely not the last positive entry.
        // The test should pass regardless of the actual internal order.
        matrix[0][0] = 0;

        update_positive_entry(
            &matrix,
            &mut positive_entries,
            &mut positive_entry_locations,
            &mut positive_row_counts,
            &mut positive_col_counts,
            0,
            0,
        );

        assert_positive_support_consistent(
            &matrix,
            &positive_entries,
            &positive_entry_locations,
            &positive_row_counts,
            &positive_col_counts,
        );
    }

    #[test]
    fn sample_submatrix_returns_compatible_positive_diagonal() {
        let matrix = [[1, 0, 2], [0, 3, 0], [4, 0, 5]];

        let (positive_entries, _positive_entry_locations, positive_row_counts, positive_col_counts) =
            build_positive_support(&matrix);

        let mut rng = SmallRng::seed_from_u64(12345);

        for _ in 0..100 {
            let submatrix = sample_submatrix::<3, 3>(
                &positive_entries,
                &positive_row_counts,
                &positive_col_counts,
                &mut rng,
            );

            let r0 = submatrix.r0 as usize;
            let r1 = submatrix.r1 as usize;
            let c0 = submatrix.c0 as usize;
            let c1 = submatrix.c1 as usize;

            assert_ne!(r0, r1);
            assert_ne!(c0, c1);

            assert!(
                matrix[r0][c0] > 0 && matrix[r1][c1] > 0,
                "sampled submatrix did not use a positive diagonal: {submatrix:?}",
            );
        }
    }

    #[test]
    fn mix_one_sampled_preserves_margins_and_support_consistency() {
        let row_margins = [3, 4, 5, 2];
        let col_margins = [4, 3, 2, 5];

        let mut matrix = [[1, 0, 1, 1], [0, 2, 0, 2], [2, 1, 1, 1], [1, 0, 0, 1]];

        assert_valid_solution(&matrix, row_margins, col_margins);

        let (
            mut positive_entries,
            mut positive_entry_locations,
            mut positive_row_counts,
            mut positive_col_counts,
        ) = build_positive_support(&matrix);

        let mut rng = SmallRng::seed_from_u64(67890);

        for _ in 0..500 {
            mix_one_sampled(
                &mut matrix,
                &mut positive_entries,
                &mut positive_entry_locations,
                &mut positive_row_counts,
                &mut positive_col_counts,
                &mut rng,
            );

            assert_valid_solution(&matrix, row_margins, col_margins);

            assert_positive_support_consistent(
                &matrix,
                &positive_entries,
                &positive_entry_locations,
                &positive_row_counts,
                &positive_col_counts,
            );
        }
    }

    #[test]
    fn update_positive_entry_is_noop_when_status_does_not_change() {
        let matrix = [[0, 2, 0], [1, 0, 0], [0, 0, 3]];

        let (
            mut positive_entries,
            mut positive_entry_locations,
            mut positive_row_counts,
            mut positive_col_counts,
        ) = build_positive_support(&matrix);

        let original_entries = positive_entries.clone();
        let original_locations = positive_entry_locations;
        let original_row_counts = positive_row_counts.clone();
        let original_col_counts = positive_col_counts.clone();

        // Still zero.
        update_positive_entry(
            &matrix,
            &mut positive_entries,
            &mut positive_entry_locations,
            &mut positive_row_counts,
            &mut positive_col_counts,
            0,
            0,
        );

        // Still positive.
        update_positive_entry(
            &matrix,
            &mut positive_entries,
            &mut positive_entry_locations,
            &mut positive_row_counts,
            &mut positive_col_counts,
            0,
            1,
        );

        assert_eq!(positive_entries, original_entries);
        assert_eq!(positive_entry_locations, original_locations);
        assert_eq!(positive_row_counts, original_row_counts);
        assert_eq!(positive_col_counts, original_col_counts);
    }
}
