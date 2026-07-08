use std::sync::LazyLock;

const MAX_MASS: usize = 1_000;

static LOG_FACTORIALS: LazyLock<[f64; MAX_MASS + 1]> = LazyLock::new(|| {
    let mut out = [0.0; MAX_MASS + 1];

    let mut n = 1;
    while n <= MAX_MASS {
        out[n] = out[n - 1] + (n as f64).ln();
        n += 1;
    }

    out
});

pub fn cell_positive_probabilities_log_lookup<const M: usize, const N: usize>(
    row_margins: &[usize; M],
    col_margins: &[usize; N],
) -> [[f64; N]; M] {
    let total_mass: usize = row_margins.iter().sum();

    debug_assert_eq!(total_mass, col_margins.iter().sum::<usize>());

    let mut output = [[0.0; N]; M];

    if total_mass == 0 {
        return output;
    }

    let log_factorials: &[f64; MAX_MASS + 1] = &LOG_FACTORIALS;

    let log_total_factorial = log_factorials[total_mass];

    for row in 0..M {
        let row_mass = row_margins[row];

        if row_mass == 0 {
            continue;
        }

        for col in 0..N {
            let col_mass = col_margins[col];

            if col_mass == 0 {
                continue;
            }

            // If row + col > total, then the row must contain at least one
            // item from this column/type.
            if row_mass + col_mass > total_mass {
                output[row][col] = 1.0;
                continue;
            }

            let log_p_zero = log_factorials[total_mass - col_mass]
                + log_factorials[total_mass - row_mass]
                - log_total_factorial
                - log_factorials[total_mass - col_mass - row_mass];

            // p_positive = 1 - exp(log_p_zero)
            //
            // log_p_zero is <= 0. Using exp_m1 improves accuracy when
            // p_positive is small.
            let p_positive = -log_p_zero.exp_m1();

            output[row][col] = p_positive.clamp(0.0, 1.0);
        }
    }

    output
}

#[inline(always)]
fn hypergeom_cell_positive_probability_product(
    row_mass: usize,
    col_mass: usize,
    total_mass: usize,
) -> f64 {
    if row_mass == 0 || col_mass == 0 {
        return 0.0;
    }

    let draws = row_mass.min(col_mass);
    let excluded = row_mass.max(col_mass);

    // If the two subsets are too large to be disjoint, overlap is guaranteed.
    if draws + excluded > total_mass {
        return 1.0;
    }

    let mut p_zero = 1.0f64;

    for k in 0..draws {
        p_zero *= (total_mass - excluded - k) as f64 / (total_mass - k) as f64;
    }

    (1.0 - p_zero).clamp(0.0, 1.0)
}

pub fn cell_positive_probabilities_product<const M: usize, const N: usize>(
    row_margins: &[usize; M],
    col_margins: &[usize; N],
) -> [[f64; N]; M] {
    let total_mass: usize = row_margins.iter().sum();

    debug_assert_eq!(total_mass, col_margins.iter().sum::<usize>());

    let mut output = [[0.0; N]; M];

    if total_mass == 0 {
        return output;
    }

    for row in 0..M {
        let row_mass = row_margins[row];

        if row_mass == 0 {
            continue;
        }

        for col in 0..N {
            let col_mass = col_margins[col];

            if col_mass == 0 {
                continue;
            }

            output[row][col] =
                hypergeom_cell_positive_probability_product(row_mass, col_mass, total_mass);
        }
    }

    output
}

pub fn calculate_sparsity_factor<const M: usize, const N: usize>(
    row_margins: &[usize; M],
    col_margins: &[usize; N],
) -> f64 {
    let positive_probabilities = cell_positive_probabilities_log_lookup(row_margins, col_margins);

    let mut probability_movable_mass = 0.0;
    let mut num_subrects: usize = 0;
    for r0 in 0..(M - 1) {
        for r1 in (r0 + 1)..M {
            for c0 in 0..(N - 1) {
                for c1 in (c0 + 1)..N {
                    let (a, b, c, d) = (
                        positive_probabilities[r0][c0],
                        positive_probabilities[r0][c1],
                        positive_probabilities[r1][c0],
                        positive_probabilities[r1][c1],
                    );
                    let diagonal_one = a * d;
                    let diagonal_two = b * c;
                    probability_movable_mass +=
                        diagonal_one + diagonal_two - diagonal_one * diagonal_two;
                    num_subrects += 1;
                }
            }
        }
    }

    return probability_movable_mass / num_subrects as f64;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f64, b: f64, tolerance: f64) {
        assert!(
            (a - b).abs() <= tolerance,
            "values were not close: {a} vs {b}, diff={}",
            (a - b).abs()
        );
    }

    fn assert_matrix_close<const M: usize, const N: usize>(
        left: &[[f64; N]; M],
        right: &[[f64; N]; M],
        tolerance: f64,
    ) {
        for row in 0..M {
            for col in 0..N {
                assert!(
                    (left[row][col] - right[row][col]).abs() <= tolerance,
                    "matrices differed at ({row}, {col}): {} vs {}, diff={}",
                    left[row][col],
                    right[row][col],
                    (left[row][col] - right[row][col]).abs(),
                );
            }
        }
    }

    #[test]
    fn product_and_log_lookup_match_small_balanced_case() {
        let row_margins = [2, 2, 2];
        let col_margins = [1, 2, 3];

        assert_eq!(
            row_margins.iter().sum::<usize>(),
            col_margins.iter().sum::<usize>(),
        );

        let product = cell_positive_probabilities_product::<3, 3>(&row_margins, &col_margins);
        let log_lookup = cell_positive_probabilities_log_lookup::<3, 3>(&row_margins, &col_margins);

        assert_matrix_close(&product, &log_lookup, 1e-12);
    }

    #[test]
    fn product_and_log_lookup_match_sparse_case() {
        let row_margins = [5, 0, 1, 4];
        let col_margins = [0, 3, 2, 5];

        assert_eq!(
            row_margins.iter().sum::<usize>(),
            col_margins.iter().sum::<usize>(),
        );

        let product = cell_positive_probabilities_product::<4, 4>(&row_margins, &col_margins);
        let log_lookup = cell_positive_probabilities_log_lookup::<4, 4>(&row_margins, &col_margins);

        assert_matrix_close(&product, &log_lookup, 1e-12);
    }

    #[test]
    fn zero_row_or_zero_column_has_zero_positive_probability() {
        let row_margins = [0, 3, 2];
        let col_margins = [0, 1, 4];

        let probabilities = cell_positive_probabilities_product::<3, 3>(&row_margins, &col_margins);

        for col in 0..3 {
            assert_close(probabilities[0][col], 0.0, 0.0);
        }

        for row in 0..3 {
            assert_close(probabilities[row][0], 0.0, 0.0);
        }
    }

    #[test]
    fn forced_overlap_has_probability_one() {
        // Total mass is 10.
        // A row of mass 8 and a column of mass 5 must overlap, because
        // 8 + 5 > 10.
        let row_margins = [8, 2];
        let col_margins = [5, 5];

        let probabilities = cell_positive_probabilities_product::<2, 2>(&row_margins, &col_margins);

        assert_close(probabilities[0][0], 1.0, 0.0);
        assert_close(probabilities[0][1], 1.0, 0.0);
    }

    #[test]
    fn known_hypergeometric_value_matches_manual_calculation() {
        // T = 10, row = 3, col = 4.
        //
        // P(X = 0) = C(6, 3) / C(10, 3) = 20 / 120 = 1/6.
        // P(X > 0) = 5/6.
        let row_margins = [3, 7];
        let col_margins = [4, 6];

        let probabilities = cell_positive_probabilities_product::<2, 2>(&row_margins, &col_margins);

        assert_close(probabilities[0][0], 5.0 / 6.0, 1e-12);
    }

    #[test]
    fn probabilities_are_symmetric_under_transpose_margins() {
        let row_margins = [1, 3, 4];
        let col_margins = [2, 6];

        assert_eq!(
            row_margins.iter().sum::<usize>(),
            col_margins.iter().sum::<usize>(),
        );

        let probabilities = cell_positive_probabilities_product::<3, 2>(&row_margins, &col_margins);

        let transposed_probabilities =
            cell_positive_probabilities_product::<2, 3>(&col_margins, &row_margins);

        for row in 0..3 {
            for col in 0..2 {
                assert_close(
                    probabilities[row][col],
                    transposed_probabilities[col][row],
                    1e-12,
                );
            }
        }
    }

    #[test]
    fn probabilities_are_between_zero_and_one() {
        let row_margins = [3, 4, 7, 2, 5];
        let col_margins = [4, 3, 6, 5, 3];

        assert_eq!(
            row_margins.iter().sum::<usize>(),
            col_margins.iter().sum::<usize>(),
        );

        let probabilities = cell_positive_probabilities_product::<5, 5>(&row_margins, &col_margins);

        for row in 0..5 {
            for col in 0..5 {
                assert!(
                    (0.0..=1.0).contains(&probabilities[row][col]),
                    "probability at ({row}, {col}) was {}",
                    probabilities[row][col],
                );
            }
        }
    }

    #[test]
    fn sparsity_factor_returns_valid_values() {
        let row_margins = [3, 4, 7, 2, 5];
        let col_margins = [4, 3, 6, 5, 3];

        assert_eq!(
            row_margins.iter().sum::<usize>(),
            col_margins.iter().sum::<usize>(),
        );

        let sparsity_factor = calculate_sparsity_factor(&row_margins, &col_margins);

        assert!(sparsity_factor >= 0.0 && sparsity_factor <= 1.0);
    }
}
