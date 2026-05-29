use rand::rngs::SmallRng;
use rand::RngExt;

pub fn card_dealing<const M: usize, const N: usize>(
    row_margins: [usize; M],
    column_margins: [usize; N],
    rng: &mut SmallRng,
) -> [[usize; N]; M] {
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
