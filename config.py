from ConfigSpace import ConfigurationSpace, Float

markov_approximator_space = ConfigurationSpace(
    {
        "optimizer.learning_rate": Float(
            "optimizer.learning_rate", bounds=(1e-8, 1e-1), log=True
        ),
        "optimizer.betas.1": Float("optimizer.betas.1", bounds=(0.8, 0.999), log=True),
        "optimizer.betas.2": Float(
            "optimizer.betas.2", bounds=(0.99, 0.9999), log=True
        ),
        "optimizer.weight_decay": Float(
            "optimizer.weight_decay", bounds=(5e-3, 5e-1), log=True
        ),
        "optimizer.learning_rate_schedule": [
            "constant",
            "cosine_warm_restarts",
            "one_cycle",
        ],
        "model.num_hidden_layers": (1, 8),
        "model.hidden_size": (16, 1024),
        "model.non_linearity": ["relu", "tanh", "gelu"],
        "model.type": ["feedforward", "recurrent"],
        "model.dropout": (0.0, 0.5),
        "training.batch_size": (4, 256),
        "training.num_epochs": (2, 100),
    }
)

test_config = {
    "optimizer.learning_rate": 1e-3,
    "optimizer.betas.1": 0.9,
    "optimizer.betas.2": 0.99,
    "optimizer.weight_decay": 1e-2,
    "optimizer.learning_rate_schedule": "one_cycle",
    "model.num_hidden_layers": 2,
    "model.hidden_size": 256,
    "model.non_linearity": "relu",
    "model.type": "feedforward",
    "model.dropout": 0.0,
    "training.batch_size": 8,
    "training.num_epochs": 10,
}
