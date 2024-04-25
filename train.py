from typing import Dict, Any

import torch
from ConfigSpace import Configuration

from models import FeedFowardModel, RecurrentModel
from dataset import MarkovValueDataset


class EmptyLRScheduler:
    def step(self):
        pass


def initialize_model_and_optimizer(
    configuration: Configuration,
    num_optimization_steps: int,
    device: torch.device = torch.device("cpu"),
) -> Dict[str, Any]:
    if configuration["model.type"] == "feedforward":
        model_type = FeedFowardModel
    elif configuration["model.type"] == "recurrent":
        model_type = RecurrentModel
    else:
        raise ValueError(f"Unknown model type: {configuration['model.type']}")

    model = model_type(
        num_hidden_layers=configuration["model.num_hidden_layers"],
        hidden_size=configuration["model.hidden_size"],
        non_linearity=configuration["model.non_linearity"],
        dropout=configuration["model.dropout"],
    )
    model.to(device)

    optimizer = torch.optim.AdamW(
        model.parameters(),
        lr=configuration["optimizer.learning_rate"],
        betas=(configuration["optimizer.betas.1"], configuration["optimizer.betas.2"]),
        weight_decay=configuration["optimizer.weight_decay"],
    )

    if configuration["optimizer.learning_rate_schedule"] == "constant":
        lr_scheduler = EmptyLRScheduler()
    elif configuration["optimizer.learning_rate_schedule"] == "cosine_warm_restarts":
        lr_scheduler = torch.optim.lr_scheduler.CosineAnnealingWarmRestarts(
            optimizer, T_0=num_optimization_steps // 3, T_mult=1
        )
    elif configuration["optimizer.learning_rate_schedule"] == "one_cycle":
        lr_scheduler = torch.optim.lr_scheduler.OneCycleLR(
            optimizer,
            max_lr=configuration["optimizer.learning_rate"],
            total_steps=num_optimization_steps,
        )
    else:
        raise ValueError(
            f"Unknown learning rate schedule: {configuration['optimizer.learning_rate_schedule']}"
        )

    return {
        "model": model,
        "optimizer": optimizer,
        "lr_scheduler": lr_scheduler,
    }


def run_trial(
    configuration: Configuration,
    verbose: bool = False,
    seed: int = 0,
    budget: int = 100,
) -> float:
    if verbose:
        print("Loading data...")
    dataset = MarkovValueDataset.from_file("data")
    splits = dataset.split(num_splits=5, shuffle=True, seed=seed)
    split_results = []
    for reserved_split in range(5):
        if verbose:
            print(f"Running split {reserved_split}...")
        train_dataset = MarkovValueDataset.concat(
            [split for idx, split in enumerate(splits) if idx != reserved_split]
        )
        validation_dataset = splits[reserved_split]

        training_dataloader = torch.utils.data.DataLoader(
            train_dataset, batch_size=configuration["training.batch_size"], shuffle=True
        )
        validation_dataloader = torch.utils.data.DataLoader(
            validation_dataset,
            batch_size=configuration["training.batch_size"],
            shuffle=True,
        )
        num_optimization_steps = configuration["training.num_epochs"] * len(
            training_dataloader
        )

        if verbose:
            print("Initializing model and optimizer...")
        model_and_optimizer = initialize_model_and_optimizer(
            configuration, num_optimization_steps, device=torch.device("cuda")
        )
        model = model_and_optimizer["model"]
        optimizer = model_and_optimizer["optimizer"]
        lr_scheduler = model_and_optimizer["lr_scheduler"]

        best_validation_loss = float("inf")
        for epoch_num in range(min(configuration["training.num_epochs"], int(budget))):
            if verbose:
                # Have this print over itself
                print(
                    f"Epoch {epoch_num + 1}/{configuration['training.num_epochs']}",
                    end="\r",
                )

            model.train()
            total_training_loss = 0
            for batch in training_dataloader:
                inputs, target = batch["inputs"], batch["target"]
                inputs, target = inputs.cuda(), target.cuda()
                optimizer.zero_grad()
                output = model(inputs)
                loss = torch.nn.functional.binary_cross_entropy_with_logits(
                    output, target
                )
                loss.backward()
                optimizer.step()
                lr_scheduler.step()

                total_training_loss += loss.item()

            model.eval()
            total_validation_loss = 0
            with torch.no_grad():
                for batch in validation_dataloader:
                    inputs, target = batch["inputs"], batch["target"]
                    inputs, target = inputs.cuda(), target.cuda()
                    output = model(inputs)
                    loss = torch.nn.functional.binary_cross_entropy_with_logits(
                        output, target
                    )
                    total_validation_loss += loss.item()

            total_validation_loss /= len(validation_dataloader)
            if total_validation_loss < best_validation_loss:
                best_validation_loss = total_validation_loss

        split_results.append(best_validation_loss)

    if verbose:
        print(f"Best validation losses: {split_results}")
    return sum(split_results) / len(split_results)


import matplotlib.pyplot as plt
from smac.facade import AbstractFacade


def plot_trajectory(facades: list[AbstractFacade]) -> None:
    """Plots the trajectory (incumbents) of the optimization process."""
    plt.figure()
    plt.title("Trajectory")
    plt.xlabel("Wallclock time [s]")
    plt.ylabel(facades[0].scenario.objectives)
    plt.ylim(0, 1.0)

    for facade in facades:
        X, Y = [], []
        for item in facade.intensifier.trajectory:
            # Single-objective optimization
            assert len(item.config_ids) == 1
            assert len(item.costs) == 1

            y = item.costs[0]
            x = item.walltime

            X.append(x)
            Y.append(y)

        plt.plot(X, Y, label=facade.intensifier.__class__.__name__)
        plt.scatter(X, Y, marker="x")

    plt.legend()
    plt.show()


if __name__ == "__main__":
    # import time
    # from config import test_config

    # start_time = time.time()
    # print(run_trial(test_config, verbose=True))
    # print(f"Time elapsed: {time.time() - start_time}")

    scenario_name = "08a56a4f5fdf6eb8c14d03298841be49"

    from smac import MultiFidelityFacade, Scenario
    from config import markov_approximator_space

    scenario = Scenario(
        name=scenario_name,
        configspace=markov_approximator_space,
        deterministic=True,
        n_trials=500,
        min_budget=2,
        max_budget=100,
    )

    initial_design = MultiFidelityFacade.get_initial_design(
        scenario=scenario, n_configs=20
    )
    intensifier = MultiFidelityFacade.get_intensifier(scenario=scenario)

    smac = MultiFidelityFacade(
        scenario=scenario,
        target_function=run_trial,
        intensifier=intensifier,
        initial_design=initial_design,
    )

    best_configuration = smac.optimize()
    print(best_configuration)
    best_configuration_value = smac.validate(best_configuration)
    print(best_configuration_value)

    plot_trajectory([smac])
