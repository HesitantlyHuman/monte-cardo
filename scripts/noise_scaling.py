from typing import Iterator

import numpy as np

def scale_noise_naive(
    row_constraints: list[int], col_constraints: list[int]
) -> np.ndarray:
    state = np.random.random((len(row_constraints), len(col_constraints)))
    row_constraints, col_constraints = np.array(row_constraints), np.array(
        col_constraints
    )
    if not np.sum(row_constraints) == np.sum(col_constraints):
        raise ValueError("Constraints do not equal same mass!")

    MAX_ITERATIONS = 1000
    for iteration_count in range(MAX_ITERATIONS):
        row_sums = np.sum(state, axis=1)
        if np.any((row_sums == 0) & (row_constraints != 0)):
            raise RuntimeError("Unbound scaling detected!")
        row_factors = row_constraints / row_sums
        state = state * row_factors.reshape(-1, 1)

        col_sums = np.sum(state, axis=0)
        if np.any((col_sums == 0) & (col_constraints != 0)):
            raise RuntimeError("Unbound scaling detected!")
        col_factors = col_constraints / col_sums
        state = state * col_factors

        if iteration_count % 5 == 0:
            state = np.round(state)

        row_sums, col_sums = np.sum(state, axis=1), np.sum(state, axis=0)
        if np.all(row_sums == row_constraints) and np.all(col_sums == col_constraints):
            return iteration_count

    raise RuntimeError(f"Failed to converge after {MAX_ITERATIONS}!")

def submatrix_generations(row_constraints: list[int], col_constraints: list[int]) -> np.ndarray:
    noise = np.zeros((len(row_constraints) - 1, len(col_constraints) - 1))

    for row in range(len(row_constraints) - 1):
        row_max = row_constraints[row]
        for col in range(len(col_constraints) - 1):
            col_max = col_constraints[col]
            noise[row, col] = np.random.randint(0, min(row_max, col_max))

    # Now project down
    row_sums = np.sum(noise, axis=1)
    necessary_decrease = np.maximum(row_sums - row_constraints[:-1], 0)
    noise -= (np.ceil(necessary_decrease / (len(row_constraints) - 1))).reshape(-1, 1)

    col_sums = np.sum(noise, axis=0)
    necessary_decrease = np.maximum(col_sums - col_constraints[:-1], 0)
    noise -= np.ceil(necessary_decrease / (len(col_constraints) - 1))

    
    # Now solve for edges
    output =  np.zeros((len(row_constraints), len(col_constraints) ))
    output[:-1, :-1] = noise

    row_sums = np.sum(output, axis=1)
    row_residuals = row_constraints - row_sums
    output[:-1, -1] = row_residuals[:-1]

    col_sums = np.sum(output, axis=0)
    col_residuals = col_constraints - col_sums
    output[-1] = col_residuals

    # Final check
    if np.any(output < 0):
        raise RuntimeError("Failed to generate valid solution!")
    
    return output


num_trials = 10_000

row_constraints = [4, 1]
col_constraints = [3, 2]

failed = 0

print("Running Tiny...")
for _ in range(num_trials):
    try:
        submatrix_generations(row_constraints=row_constraints, col_constraints=col_constraints)
    except RuntimeError:
        failed +=1

print(f'Failure ratio: {failed / num_trials}')


row_constraints = [3, 4, 7]
col_constraints = [4, 5, 5]

failed = 0

print("Running Simple...")
for _ in range(num_trials):
    try:
        submatrix_generations(row_constraints=row_constraints, col_constraints=col_constraints)
    except RuntimeError:
        failed +=1

print(f'Failure ratio: {failed / num_trials}')

row_constraints = [3, 4, 7, 2, 4]
col_constraints = [4, 5, 6, 3, 2]

failed = 0

print("Running Bigger...")
for _ in range(num_trials):
    try:
        submatrix_generations(row_constraints=row_constraints, col_constraints=col_constraints)
    except RuntimeError:
        failed +=1

print(f'Failure ratio: {failed / num_trials}')

row_constraints = [3, 4, 7, 2, 5]
col_constraints = [4, 3, 4, 3, 2, 3, 2]

failed = 0

print("Running Largest...")
for _ in range(num_trials):
    try:
        submatrix_generations(row_constraints=row_constraints, col_constraints=col_constraints)
    except RuntimeError:
        failed +=1

print(f'Failure ratio: {failed / num_trials}')

