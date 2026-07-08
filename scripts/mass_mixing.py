from typing import Iterator

import numpy as np

from utils import validate_matrix

def generate_simple_valid(row_constraints: list[int], col_constraints: list[int]) -> np.ndarray:
    if sum(row_constraints) != sum(col_constraints):
        raise ValueError("Row and column constraints must have equal total mass")
    
    output = np.zeros((len(row_constraints), len(col_constraints)))

    remaining_row_mass = np.array(row_constraints, dtype=np.int64)
    remaining_col_mass = np.array(col_constraints, dtype=np.int64)

    for _ in range(len(row_constraints) + len(col_constraints)):
        masked_row_constraints = np.ma.masked_array(
            remaining_row_mass,
            mask=remaining_row_mass == 0,
        )
        smallest_row_idx = int(np.argmin(masked_row_constraints))
        largest_row_idx = int(np.argmax(masked_row_constraints))
        smallest_row_value = int(remaining_row_mass[smallest_row_idx])

        masked_col_constraints = np.ma.masked_array(
            remaining_col_mass,
            mask=remaining_col_mass == 0,
        )
        smallest_col_idx = int(np.argmin(masked_col_constraints))
        largest_col_idx = int(np.argmax(masked_col_constraints))
        smallest_col_value = int(remaining_col_mass[smallest_col_idx])

        if smallest_row_value < smallest_col_value:
            output[smallest_row_idx, largest_col_idx] = smallest_row_value

            remaining_row_mass[smallest_row_idx] -= smallest_row_value
            remaining_col_mass[largest_col_idx] -= smallest_row_value
        else:
            output[largest_row_idx, smallest_col_idx] = smallest_col_value

            remaining_row_mass[largest_row_idx] -= smallest_col_value
            remaining_col_mass[smallest_col_idx] -= smallest_col_value
        
        if np.all(remaining_row_mass == 0) and np.all(remaining_col_mass == 0):
            break

    try:
        validate_matrix(output, row_constraints, col_constraints)
    except Exception as e:
        print(row_constraints, col_constraints)
        print(output)
        raise e
    return output

def mix_one(state: np.ndarray):
    num_rows, num_cols = state.shape

    # Select 2 distinct rows and 2 distinct columns
    r0, r1 = np.random.choice(num_rows, 2, replace=False)
    c0, c1 = np.random.choice(num_cols, 2, replace=False)

    # Current 2x2 block:
    #
    # [ a  b ]
    # [ c  d ]
    #
    # We apply:
    #
    # [ a+t  b-t ]
    # [ c-t  d+t ]
    #
    # This preserves both row and column sums.
    a = state[r0, c0]
    b = state[r0, c1]
    c = state[r1, c0]
    d = state[r1, c1]

    lower = -min(a, d)
    upper = min(b, c)
    
    if lower == upper == 0:
        return
    
    if lower == upper:
        t = lower
    else:
        t = np.random.randint(lower, upper + 1)

    state[r0, c0] += t
    state[r0, c1] -= t
    state[r1, c0] -= t
    state[r1, c1] += t


def generate_with_mixing(num_to_generate: int, row_constraints: list[int], col_constraints: list[int], initial_mixing_steps: int, mixing_per_sample: int) -> Iterator[np.ndarray]:
    initial_state = generate_simple_valid(row_constraints=row_constraints, col_constraints=col_constraints)

    for _ in range(initial_mixing_steps):
        mix_one(initial_state)

    for _ in range(num_to_generate - 1):
        yield initial_state

        for _ in range(mixing_per_sample):
            mix_one(initial_state)

    yield initial_state


if __name__ == "__main__":
    row_constraints = [3, 2, 2]
    col_constraints = [2, 2, 3]

    print(generate_simple_valid(row_constraints=row_constraints, col_constraints=col_constraints))

    for sample in generate_with_mixing(1, row_constraints=row_constraints, col_constraints=col_constraints, initial_mixing_steps=20, mixing_per_sample=0):
        print(sample)
