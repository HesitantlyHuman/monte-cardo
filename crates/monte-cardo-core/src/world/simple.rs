pub fn simple_valid_smart<const M: usize, const N: usize>(
    row_margins: [usize; M],
    column_margins: [usize; N],
) -> [[usize; N]; M] {
    let mut output = [[0; N]; M];

    let mut remaining_row_mass = row_margins;
    let mut remaining_col_mass = column_margins;

    let mut active_rows = [0usize; M];
    let mut active_cols = [0usize; N];

    let mut num_active_rows = 0usize;
    let mut num_active_cols = 0usize;

    for row in 0..M {
        if row_margins[row] > 0 {
            active_rows[num_active_rows] = row;
            num_active_rows += 1;
        }
    }

    for col in 0..N {
        if column_margins[col] > 0 {
            active_cols[num_active_cols] = col;
            num_active_cols += 1;
        }
    }

    while num_active_rows > 0 && num_active_cols > 0 {
        let mut smallest_row_location = 0usize;
        let mut smallest_row_index = active_rows[0];
        let mut smallest_row = remaining_row_mass[smallest_row_index];

        for row_location in 1..num_active_rows {
            let row_index = active_rows[row_location];
            let row_mass = remaining_row_mass[row_index];

            if row_mass < smallest_row {
                smallest_row_location = row_location;
                smallest_row_index = row_index;
                smallest_row = row_mass;
            }
        }

        let mut smallest_col_location = 0usize;
        let mut smallest_col_index = active_cols[0];
        let mut smallest_col = remaining_col_mass[smallest_col_index];

        for col_location in 1..num_active_cols {
            let col_index = active_cols[col_location];
            let col_mass = remaining_col_mass[col_index];

            if col_mass < smallest_col {
                smallest_col_location = col_location;
                smallest_col_index = col_index;
                smallest_col = col_mass;
            }
        }

        let assigned_mass = smallest_row.min(smallest_col);

        output[smallest_row_index][smallest_col_index] = assigned_mass;

        remaining_row_mass[smallest_row_index] -= assigned_mass;
        remaining_col_mass[smallest_col_index] -= assigned_mass;

        if remaining_row_mass[smallest_row_index] == 0 {
            num_active_rows -= 1;
            active_rows[smallest_row_location] = active_rows[num_active_rows];
        }

        if remaining_col_mass[smallest_col_index] == 0 {
            num_active_cols -= 1;
            active_cols[smallest_col_location] = active_cols[num_active_cols];
        }
    }

    output
}

pub fn simple_valid_greedy<const M: usize, const N: usize>(
    row_margins: [usize; M],
    column_margins: [usize; N],
) -> [[usize; N]; M] {
    let mut output = [[0; N]; M];

    let mut remaining_row_mass = row_margins;
    let mut remaining_col_mass = column_margins;

    let mut row_index = 0;
    let mut col_index = 0;

    while row_index < M && col_index < N {
        let assigned_mass = remaining_row_mass[row_index].min(remaining_col_mass[col_index]);
        output[row_index][col_index] = assigned_mass;
        remaining_row_mass[row_index] -= assigned_mass;
        remaining_col_mass[col_index] -= assigned_mass;

        if remaining_row_mass[row_index] == 0 {
            row_index += 1;
        }

        if remaining_col_mass[col_index] == 0 {
            col_index += 1;
        }
    }

    return output;
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn simple_valid_smart_generates_valid_solutions() {
        let row_margins = [3, 4, 7, 2, 5, 6, 1, 6, 8, 3, 2, 5, 7, 4, 6, 5, 3, 8];
        let column_margins = [4, 3, 6, 5, 4, 7, 2, 5, 6, 3, 8, 4, 2, 7, 5, 3, 6, 5];

        assert_eq!(
            row_margins.iter().sum::<usize>(),
            column_margins.iter().sum::<usize>(),
        );

        let sample = simple_valid_smart::<18, 18>(row_margins, column_margins);
        println!("{:?}", sample);
        assert_valid_solution(&sample, row_margins, column_margins);
    }

    #[test]
    fn simple_valid_greedy_generates_valid_solutions() {
        let row_margins = [3, 4, 7, 2, 5, 6, 1, 6, 8, 3, 2, 5, 7, 4, 6, 5, 3, 8];
        let column_margins = [4, 3, 6, 5, 4, 7, 2, 5, 6, 3, 8, 4, 2, 7, 5, 3, 6, 5];

        assert_eq!(
            row_margins.iter().sum::<usize>(),
            column_margins.iter().sum::<usize>(),
        );

        let sample = simple_valid_greedy::<18, 18>(row_margins, column_margins);
        println!("{:?}", sample);
        assert_valid_solution(&sample, row_margins, column_margins);
    }
}
