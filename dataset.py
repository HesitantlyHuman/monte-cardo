from typing import List, Dict, Tuple, Union

import os
import torch
import numpy as np

from consts import MAX_PLAYERS

SINGLE_EXAMPLE_BYTES = 746
FLOAT_BYTES = 8


class MarkovValueDataset(torch.utils.data.Dataset):
    def __init__(self, data: List[Dict[str, np.ndarray]]):
        self.data = data

    def __len__(self):
        return len(self.data)

    def __getitem__(self, idx):
        return self.data[idx]

    def load_data(self, folder: str):
        # First, find all the files in the folder
        for file_name in os.listdir(folder):
            if file_name.endswith(".bin"):
                file_path = os.path.join(folder, file_name)
                try:
                    self.data.extend(self.try_load_batch(file_path))
                except ValueError as e:
                    print(f"Failed to load batch from {file_path}: {e}")

    def try_load_batch(self, file_path) -> List[Dict[str, np.ndarray]]:
        # Load a batch of data from a file
        batch_bytes = np.fromfile(file_path, dtype=np.uint8)
        num_bytes = len(batch_bytes)
        # If the number of bytes is not a multiple of the size of a single example, then we can't load it
        if num_bytes % SINGLE_EXAMPLE_BYTES != 0:
            raise ValueError(
                f"File {file_path} has an invalid number of bytes: {num_bytes}"
            )
        num_examples = num_bytes // SINGLE_EXAMPLE_BYTES
        # Reshape the data into a 2D array
        batch_data = batch_bytes.reshape((num_examples, SINGLE_EXAMPLE_BYTES))
        target_bytes = batch_data[:, -FLOAT_BYTES:]

        # Format the data
        targets = [np.frombuffer(b, dtype=np.float64) for b in target_bytes]
        for i in range(num_examples):
            if np.isnan(targets[i]).any():
                raise ValueError(f"Found NaN in target {i} of {file_path}")
            if np.isinf(targets[i]).any():
                print(target_bytes[i])
                raise ValueError(f"Found inf in target {i} of {file_path}")
        targets = np.array(targets)
        inputs = batch_data[:, :-FLOAT_BYTES]
        # Change the input dtype to float64
        inputs = inputs.astype(np.float64)
        remaining_card_numbers = np.sum(inputs[:, -MAX_PLAYERS:], axis=1)
        # Normalize the last 18 to be the proportion of the remaining card numbers
        inputs[:, -MAX_PLAYERS:] = (
            inputs[:, -MAX_PLAYERS:] / remaining_card_numbers[:, None]
        )
        # Normalize the data to be between -1 and 1, except for the last 18 columns
        inputs[:, :-MAX_PLAYERS] = (inputs[:, :-MAX_PLAYERS] * 2) - 1
        # Convert data to f32
        inputs = inputs.astype(np.float32)
        targets = targets.astype(np.float32)

        return [
            {"inputs": inputs[i], "target": targets[i]} for i in range(num_examples)
        ]

    def split(
        self,
        fractions: Union[float, Tuple[float]] = None,
        num_splits: int = None,
        shuffle: bool = True,
        seed: int = None,
    ) -> Tuple["MarkovValueDataset", "MarkovValueDataset"]:
        if fractions is not None and num_splits is not None:
            raise ValueError("Only one of fractions or num_splits should be provided")
        if fractions is None and num_splits is None:
            raise ValueError("Either fractions or num_splits should be provided")
        if fractions is not None:
            if isinstance(fractions, float):
                fractions = (fractions, 1 - fractions)
            if sum(fractions) < 1:
                fractions = fractions + (1 - sum(fractions),)
            elif sum(fractions) > 1:
                raise ValueError("Sum of fractions must be less than or equal to 1")
        if num_splits is not None:
            fractions = tuple(1 / num_splits for _ in range(num_splits))

        if shuffle:
            if seed is not None:
                np.random.seed(seed)
            np.random.shuffle(self.data)

        split_size = [int(f * len(self)) for f in fractions]
        # Make sure the sum of the split sizes is equal to the length of the dataset
        split_size[-1] += len(self) - sum(split_size)

        # Split the data
        split_data = []
        start_idx = 0
        for size in split_size:
            split_data.append(self.data[start_idx : start_idx + size])
            start_idx += size

        return tuple(MarkovValueDataset(d) for d in split_data)

    @classmethod
    def concat(cls, datasets: List["MarkovValueDataset"]) -> "MarkovValueDataset":
        data = []
        for dataset in datasets:
            data.extend(dataset.data)
        return cls(data)

    @classmethod
    def from_file(cls, folder: str) -> "MarkovValueDataset":
        dataset = cls([])
        dataset.load_data(folder)
        return dataset
