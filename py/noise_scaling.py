from typing import Iterator

import numpy as np

# TODO: Thoughts
# The full fixed-margin matrix is uniquely determined by the (N-1)x(M-1) submatrix.
# Maybe there is a way to sample a random integer matrix such that the entries are less than certain values. Is that easier than the full problem.

# Doesn't always generate a valid matrix
# For example:
# | ? | ? || 1
# | 1 | ? || 4
# |---|---||---
#   3   2
#
# We can see that the sub-matrix [[1]] cannot generate a valid solution. That implies that the bottom right entry is 3, which exceeds what is allowed for that column
#
# A valid solution:
# | 1 | 0 || 1
# | 2 | 2 || 4
# |---|---||---
#   3   2
# TODO: maybe write out this thought process in the blog

# def scale_noise(
#     row_constraints: list[int], col_constraints: list[int]
# ) -> np.ndarray:
#     state = np.random.random((len(row_constraints), len(col_constraints)))
#     row_constraints, col_constraints = np.array(row_constraints), np.array(
#         col_constraints
#     )
#     if not np.sum(row_constraints) == np.sum(col_constraints):
#         raise ValueError("Constraints do not equal same mass!")

#     for _ in range(2):
#         row_sums = np.sum(state, axis=1)
#         row_factors = row_constraints / row_sums
#         state = state * row_factors.reshape(-1, 1)

#         col_sums = np.sum(state, axis=0)
#         col_factors = col_constraints / col_sums
#         state = state * col_factors

#         state = np.round(state)

#     row_sums = np.sum(state, axis=1)
#     col_sums = np.sum(state, axis=0)

#     state[-1] = 0
#     state[:, -1] = 0

#     return state


def scale_noise(row_constraints: list[int], col_constraints: list[int]) -> np.ndarray:
    noise = np.zeros((len(row_constraints) - 1, len(col_constraints) - 1))

    for row in range(len(row_constraints) - 1):
        row_max = row_constraints[row]
        for col in range(len(col_constraints) - 1):
            col_max = col_constraints[col]
            noise[row, col] = np.random.randint(0, min(row_max, col_max))

    # Now project down
    row_sums = np.sum(noise, axis=1)
    necessary_decrease = np.maximum(row_sums - row_constraints[:-1], 0)
    print(noise)
    print(necessary_decrease)
    noise -= (np.ceil(necessary_decrease / (len(row_constraints) - 1))).reshape(-1, 1)
    print(noise)
    print(necessary_decrease)


row_constraints = [3, 4, 7]
col_constraints = [4, 5, 5]

scale_noise(row_constraints=row_constraints, col_constraints=col_constraints)
