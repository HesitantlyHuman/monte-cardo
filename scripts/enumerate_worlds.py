from typing import Iterator

import math
import operator

import numpy as np
from tqdm.auto import tqdm


def stars_and_bars(mass: int, slots: int) -> list[np.ndarray]:
    """Enumerate weak compositions of `mass` into `slots`.

    Returns a list of 1D NumPy arrays. Each array has length `slots`.

    The returned arrays are row views into one contiguous backing array, which is
    much faster and more memory-efficient than allocating each row separately.
    """
    mass = operator.index(mass)
    slots = operator.index(slots)

    if mass < 0:
        raise ValueError("mass must be non-negative")
    if slots <= 0:
        raise ValueError("slots must be positive")

    dtype = np.int64

    if slots == 1:
        return [np.array([mass], dtype=dtype)]

    n = math.comb(mass + slots - 1, slots - 1)
    out = np.empty((n, slots), dtype=dtype)

    if mass == 0:
        out.fill(0)
        return list(out)

    if slots == 2:
        out[:, 0] = np.arange(mass + 1, dtype=dtype)
        out[:, 1] = mass - out[:, 0]
        return list(out)

    # counts[r][k] = number of weak compositions of mass r into k slots.
    # counts[r][k] = C(r + k - 1, k - 1)
    counts = [[0] * (slots + 1) for _ in range(mass + 1)]

    for r in range(mass + 1):
        counts[r][1] = 1

    for k in range(2, slots + 1):
        counts[0][k] = 1
        for r in range(1, mass + 1):
            counts[r][k] = counts[r][k - 1] + counts[r - 1][k]

    def fill(col: int, remaining: int, start: int, rows: int) -> None:
        if col == slots - 1:
            out[start : start + rows, col] = remaining
            return

        tail_slots = slots - col - 1
        pos = start

        for x in range(remaining + 1):
            block_size = counts[remaining - x][tail_slots]

            out[pos : pos + block_size, col] = x
            fill(col + 1, remaining - x, pos, block_size)

            pos += block_size

    fill(0, mass, 0, n)
    return list(out)


def _enumerate_from_state(
    state: np.ndarray,
    row_constraints: np.ndarray,
    column_constraints: np.ndarray,
    rows_complete: np.ndarray,
    cols_complete: np.ndarray,
) -> Iterator[np.ndarray]:
    # state[row_idx, col_idx]
    #
    # Row sums sum across columns.
    # Column sums sum across rows.
    row_sums = np.sum(state, axis=1)
    col_sums = np.sum(state, axis=0)

    remaining_row_mass = row_constraints - row_sums
    remaining_col_mass = column_constraints - col_sums

    if (np.all(rows_complete) and np.all(cols_complete)) or (
        np.all(remaining_row_mass == 0) and np.all(remaining_col_mass == 0)
    ):
        if not (np.all(remaining_row_mass == 0) and np.all(remaining_col_mass == 0)):
            raise RuntimeError("Final state is not a solution!")
        yield state
        return

    if np.any(remaining_row_mass < 0) or np.any(remaining_col_mass < 0):
        raise RuntimeError("Encountered fatal state!")

    masked_row_constraints = np.ma.masked_array(
        remaining_row_mass,
        mask=rows_complete,
    )
    row_candidate_idx = int(np.argmin(masked_row_constraints))
    row_candidate_value = int(remaining_row_mass[row_candidate_idx])
    rows_all_done = np.all(rows_complete)

    masked_col_constraints = np.ma.masked_array(
        remaining_col_mass,
        mask=cols_complete,
    )
    col_candidate_idx = int(np.argmin(masked_col_constraints))
    col_candidate_value = int(remaining_col_mass[col_candidate_idx])
    cols_all_done = np.all(cols_complete)

    if cols_all_done or (
        row_candidate_value < col_candidate_value and not rows_all_done
    ):
        # Fill one row.
        row_idx = row_candidate_idx

        # Open slots in this row are incomplete columns.
        # These are column indices, not row indices.
        open_col_idxs = np.flatnonzero(~cols_complete)

        allocations = stars_and_bars(row_candidate_value, len(open_col_idxs))

        for allocation in allocations:
            next_state = state.copy()
            next_rows_complete = rows_complete.copy()

            # This indexing may be non-contiguous in columns, so use explicit
            # integer-array indexing rather than slicing.
            next_state[row_idx, open_col_idxs] = allocation

            next_rows_complete[row_idx] = True

            yield from _enumerate_from_state(
                next_state,
                row_constraints,
                column_constraints,
                next_rows_complete,
                cols_complete,
            )

    else:
        # Fill one column.
        col_idx = col_candidate_idx

        # Open slots in this column are incomplete rows.
        # These are row indices, not column indices.
        open_row_idxs = np.flatnonzero(~rows_complete)

        allocations = stars_and_bars(col_candidate_value, len(open_row_idxs))

        for allocation in allocations:
            next_state = state.copy()
            next_cols_complete = cols_complete.copy()

            # This indexing may be non-contiguous in rows, so use explicit
            # integer-array indexing rather than slicing.
            next_state[open_row_idxs, col_idx] = allocation

            next_cols_complete[col_idx] = True

            yield from _enumerate_from_state(
                next_state,
                row_constraints,
                column_constraints,
                rows_complete,
                next_cols_complete,
            )


def enumerate_states(
    row_constraints: list[int], col_constraints: list[int]
) -> Iterator[np.ndarray]:
    initial_state = np.zeros((len(row_constraints), len(col_constraints)))
    row_constraints_array, col_constraints_array = np.array(row_constraints), np.array(
        col_constraints
    )
    if not np.sum(row_constraints) == np.sum(col_constraints):
        raise ValueError("Constraints do not equal same mass!")

    rows_complete, cols_complete = np.zeros_like(
        row_constraints, dtype=bool
    ), np.zeros_like(col_constraints, dtype=bool)

    yield from _enumerate_from_state(
        initial_state,
        row_constraints_array,
        col_constraints_array,
        rows_complete,
        cols_complete,
    )


if __name__ == "__main__":
    row_constraints = [3, 4, 1, 3, 5]
    col_constraints = [2, 5, 5, 2, 2]

    num_states = 0
    for state in enumerate_states(
        row_constraints=row_constraints, col_constraints=col_constraints
    ):
        num_states += 1

    print(f'Found {num_states} total states.')
