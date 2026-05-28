import numpy as np

def validate_matrix(matrix: np.ndarray, row_margins: list[int], col_margins: list[int]) -> None:
    if matrix.shape != (len(row_margins), len(col_margins)):
        raise ValueError(
            f"Matrix had shape {matrix.shape}, expected {(len(row_margins), len(col_margins))}"
        )

    if np.any(matrix < 0):
        raise ValueError(f"Matrix contains negative entries:\n{matrix}")

    row_sums = matrix.sum(axis=1)
    col_sums = matrix.sum(axis=0)

    if not np.array_equal(row_sums, np.array(row_margins)):
        raise ValueError(f"Invalid row sums: {row_sums} != {row_margins}\n{matrix}")

    if not np.array_equal(col_sums, np.array(col_margins)):
        raise ValueError(f"Invalid col sums: {col_sums} != {col_margins}\n{matrix}")