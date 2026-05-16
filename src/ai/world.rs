use core::num;

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

pub fn card_dealing<const M: usize, const N: usize, const T: usize>(
    row_margins: [usize; M],
    column_margins: [usize; N],
    rng: &mut SmallRng,
) -> [[usize; N]; M] {
    debug_assert!(T == M * N);
    let mass_total: usize = row_margins.iter().sum();
    debug_assert!(column_margins.iter().sum::<usize>() == mass_total);

    let mut output = [[0; N]; M];

    let mut remaining_row_mass = row_margins;
    let mut remaining_col_mass = column_margins;

    let mut active_rows = Vec::with_capacity(M);
    let mut active_cols = Vec::with_capacity(N);

    active_rows.extend((0..M).filter(|&row| remaining_row_mass[row] > 0));
    active_cols.extend((0..N).filter(|&col| remaining_col_mass[col] > 0));

    for _ in 0..mass_total {
        let num_cols = active_cols.len();
        let index = rng.random_range(0..active_rows.len() * num_cols);

        let row_idx = index / num_cols;
        let col_idx = index % num_cols;

        let row = active_rows[row_idx];
        let col = active_cols[col_idx];

        output[row][col] += 1;

        remaining_row_mass[row] -= 1;
        remaining_col_mass[col] -= 1;

        if remaining_row_mass[row] == 0 {
            active_rows.swap_remove(row_idx);
        }

        if remaining_col_mass[col] == 0 {
            active_cols.swap_remove(col_idx);
        }
    }

    debug_assert!(output.iter().flatten().sum::<usize>() == mass_total);

    return output;
}

pub fn stars_and_bars<const N: usize>(
    mass: usize,
    slots: usize,
    output: &mut [usize; N],
    rng: &mut SmallRng,
) {
    debug_assert!(slots <= N);
    debug_assert!(slots > 0);

    if slots == 1 {
        output[0] = mass;
    }

    let num_bars = slots - 1;
    let mut bar_positions = [0usize; N];

    for i in 0..num_bars {
        bar_positions[i] = rng.random_range(0..=mass);
    }

    bar_positions[..num_bars].sort_unstable();

    let mut previous_bar = 0;

    for idx in 0..num_bars {
        let bar = bar_positions[idx];
        output[idx] = bar - previous_bar;
        previous_bar = bar;
    }

    output[num_bars] = mass - previous_bar;

    debug_assert_eq!(output[..slots].iter().sum::<usize>(), mass);
}

pub fn progressive_stars_and_bars<const M: usize, const N: usize, const T: usize>(
    row_margins: [usize; M],
    column_margins: [usize; N],
    rng: &mut SmallRng,
) -> [[usize; N]; M] {
    let mut output = [[0; N]; M];

    let mut remaining_row_mass = row_margins;
    let mut remaining_col_mass = column_margins;

    let mut active_rows = Vec::with_capacity(M);
    let mut active_cols = Vec::with_capacity(N);

    let mut row_allocations = [0; N];
    let mut col_allocations = [0; M];

    active_rows.extend((0..M).filter(|&row| row_margins[row] > 0));
    active_cols.extend((0..N).filter(|&col| column_margins[col] > 0));

    while !active_rows.is_empty() && !active_cols.is_empty() {
        // First, find our candidates
        let mut best_row_index = 0;
        let mut best_row_index_location = 0;
        let mut best_row_value = usize::MAX;
        for (row_location, row_index) in active_rows.iter().enumerate() {
            if remaining_row_mass[*row_index] < best_row_value {
                best_row_index = *row_index;
                best_row_value = remaining_row_mass[*row_index];
                best_row_index_location = row_location;
            }
        }

        let mut best_col_index = 0;
        let mut best_col_index_location = 0;
        let mut best_col_value = usize::MAX;
        for (col_location, col_index) in active_cols.iter().enumerate() {
            if remaining_col_mass[*col_index] < best_col_value {
                best_col_index = *col_index;
                best_col_value = remaining_col_mass[*col_index];
                best_col_index_location = col_location;
            }
        }

        if best_row_value < best_col_value {
            stars_and_bars(best_row_value, active_cols.len(), &mut row_allocations, rng);
            for (col, allocation) in active_cols.iter().zip(row_allocations) {
                output[best_row_index][*col] = allocation;
                remaining_col_mass[*col] -= allocation;
            }
            active_rows.swap_remove(best_row_index_location);
        } else {
            stars_and_bars(best_col_value, active_rows.len(), &mut col_allocations, rng);
            for (row, allocation) in active_rows.iter().zip(col_allocations) {
                output[*row][best_col_index] = allocation;
                remaining_row_mass[*row] -= allocation;
            }
            active_cols.swap_remove(best_col_index_location);
        }
    }

    return output;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::SmallRng, SeedableRng};

    #[test]
    fn stars_and_bars_sums_to_mass() {
        let mut rng = SmallRng::seed_from_u64(123);

        let mut allocation = [0; 8];
        stars_and_bars::<8>(100, 5, &mut allocation, &mut rng);

        assert_eq!(allocation[..5].iter().sum::<usize>(), 100);
    }

    #[test]
    fn stars_and_bars_ignores_unused_slots() {
        let mut rng = SmallRng::seed_from_u64(123);

        let mut allocation = [0; 8];
        stars_and_bars::<8>(100, 5, &mut allocation, &mut rng);

        assert_eq!(allocation[5..].iter().sum::<usize>(), 0);
    }

    #[test]
    fn progressive_stars_and_bars_sums_to_margins() {
        let mut rng = SmallRng::seed_from_u64(123);

        let row_margins = [3, 4, 7, 2];
        let column_margins = [4, 6, 6];

        let output = progressive_stars_and_bars::<4, 3, 12>(row_margins, column_margins, &mut rng);

        let c_rows: Vec<usize> = output.iter().map(|row| row.iter().sum::<usize>()).collect();

        let c_cols: Vec<usize> = (0..3)
            .map(|col| output.iter().map(|row| row[col]).sum::<usize>())
            .collect();

        assert_eq!(c_rows, row_margins);
        assert_eq!(c_cols, column_margins);
    }
}
