from functools import cache
import random
from typing import Iterator


def bounded_compositions(total: int, bounds: tuple[int, ...]) -> Iterator[tuple[int, ...]]:
    """
    Generate all vectors x such that:

        sum(x) == total
        0 <= x[i] <= bounds[i]
    """
    if len(bounds) == 0:
        if total == 0:
            yield ()
        return

    first_bound = min(bounds[0], total)

    for x0 in range(first_bound + 1):
        for rest in bounded_compositions(total - x0, bounds[1:]):
            yield (x0,) + rest


def sample_fixed_margin_matrix(rows: list[int], cols: list[int]) -> list[list[int]]:
    if sum(rows) != sum(cols):
        raise ValueError("Row and column margins must have the same total mass.")

    rows = tuple(rows)
    cols = tuple(cols)

    m = len(rows)

    @cache
    def count_completions(row_idx: int, remaining_cols: tuple[int, ...]) -> int:
        if row_idx == m:
            return int(all(c == 0 for c in remaining_cols))

        row_total = rows[row_idx]

        if sum(remaining_cols) != sum(rows[row_idx:]):
            return 0

        total = 0

        for row in bounded_compositions(row_total, remaining_cols):
            next_cols = tuple(c - x for c, x in zip(remaining_cols, row))
            total += count_completions(row_idx + 1, next_cols)

        return total

    if count_completions(0, cols) == 0:
        raise ValueError("No valid matrix exists for these margins.")

    matrix = []
    remaining_cols = cols

    for row_idx in range(m):
        row_total = rows[row_idx]

        candidates = []
        weights = []

        for row in bounded_compositions(row_total, remaining_cols):
            next_cols = tuple(c - x for c, x in zip(remaining_cols, row))
            weight = count_completions(row_idx + 1, next_cols)

            if weight > 0:
                candidates.append(row)
                weights.append(weight)

        chosen = random.choices(candidates, weights=weights, k=1)[0]

        matrix.append(list(chosen))
        remaining_cols = tuple(c - x for c, x in zip(remaining_cols, chosen))

    return matrix

# Up to 15 players, with 18 ordinality slots
player_constraints = [5, 4, 7, 6, 5, 3, 4, 0, 0, 0, 0, 0, 0, 0, 0]
card_counts = [2, 4, 2, 2, 4, 2, 4, 1, 1, 3, 4, 2, 3, 0, 0, 0, 0, 0]

# Somone played 2 12s
# Then someone else played 2 10s
# Then someone played 2 7s
# Then someone played 2 4s
# Then someone played 1 11s
# Then someone played 1 8s
# Then someone played 1 7s
# Then someone played 2 2s

print(sample_fixed_margin_matrix(rows=player_constraints, cols=card_counts))