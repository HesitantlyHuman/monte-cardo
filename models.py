import torch

NUM_INPUTS = 738


def get_activation(non_linearity: str):
    if non_linearity == "relu":
        return torch.nn.ReLU()
    elif non_linearity == "tanh":
        return torch.nn.Tanh()
    elif non_linearity == "gelu":
        return torch.nn.GELU()
    else:
        raise ValueError(f"Unknown non-linearity: {non_linearity}")


class FeedFowardModel(torch.nn.Module):
    def __init__(
        self,
        num_hidden_layers: int,
        hidden_size: int,
        non_linearity: str,
        dropout: float,
    ):
        super(FeedFowardModel, self).__init__()
        self.model = torch.nn.Sequential()
        self.model.add_module("input", torch.nn.Linear(NUM_INPUTS, hidden_size))
        self.model.add_module("input_act", get_activation(non_linearity))
        for i in range(num_hidden_layers):
            self.model.add_module(f"drouput_{i}", torch.nn.Dropout(p=dropout))
            self.model.add_module(
                f"hidden_{i}", torch.nn.Linear(hidden_size, hidden_size)
            )
            self.model.add_module(f"hidden_{i}_act", get_activation(non_linearity))
        self.model.add_module("output", torch.nn.Linear(hidden_size, 1))

    def forward(self, x):
        return self.model(x)


class RecurrentModel(torch.nn.Module):
    def __init__(
        self,
        num_hidden_layers: int,
        hidden_size: int,
        non_linearity: str,
        dropout: float,
    ):
        super(RecurrentModel, self).__init__()
        self.num_hidden_layers = num_hidden_layers
        self.input = torch.nn.Linear(NUM_INPUTS, hidden_size)
        self.input_act = get_activation(non_linearity)
        self.recurrent = torch.nn.Linear(hidden_size, hidden_size)
        self.recurrent_act = get_activation(non_linearity)
        self.output = torch.nn.Linear(hidden_size, 1)
        self.dropout = torch.nn.Dropout(p=dropout)

    def forward(self, x):
        x = self.input(x)
        x = self.input_act(x)
        for _ in range(self.num_hidden_layers):
            x = self.dropout(x)
            x = self.recurrent(x)
            x = self.recurrent_act(x)
        return self.output(x)
