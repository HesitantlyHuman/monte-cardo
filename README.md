# ladder-shedding
Simple MCTS based AI for the card game ladder shedding. The heuristic is currently very simple and not very good, but the MCTS is functional and can be used to play the game.

A model will be trained against the MCTS to improve the heuristic.

## TODO
[ ] Add UCT to monte carlo system
[x] Find why we are loading an infinite value for some targets
[x] Add a progress bar for the data generation process
[ ] Verify that the data generation process can start and stop correctly
[ ] Parallelize the data generation process?
[x] Update the data generation to do random decks and numbers of players
[x] Validate the rust generated data and write a loader for it
[ ] Write the model and try training on small subset of data to validate and debug
[ ] Add checkpointing and hyperparameter search to model training?
[ ] Write a console application to play against the model and/or use it for outside play
[ ] Test the current lame heuristic with the playing, to validate that it does make reasonable decisions
[ ] setup.py
[ ] auto validation to select best heuristic
[ ] Get rust to handle the python environment and create a command line interface which does everything

## Random installed things
conda install pytorch torchvision torchaudio pytorch-cuda=12.1 -c pytorch -c nvidia
conda install gxx_linux-64 gcc_linux-64
ConfigSpace
python 3.11 (sadge)
pip install swig
pip install smac
pip install matplotlib

time to train one k-fold with cpu: 180.47

### Model
current best configuration:
Configuration(values={
  'model.dropout': 0.3103349040085379,
  'model.hidden_size': 644,
  'model.non_linearity': 'gelu',
  'model.num_hidden_layers': 1,
  'model.type': 'feedforward',
  'optimizer.betas.1': 0.8852826026277925,
  'optimizer.betas.2': 0.990006239463997,
  'optimizer.learning_rate': 0.000587031691530561,
  'optimizer.learning_rate_schedule': 'cosine_warm_restarts',
  'optimizer.weight_decay': 0.007470556320920037,
  'training.batch_size': 149,
  'training.num_epochs': 97,
})

value: 0.37807561578574


## UI
What the UI needs:
- Menu View
	- Title
	- Buttons to select which mode you need (playing against AIs, or playing against humans)
	- Button for rules screen
- Rules screen
  - Rules
  - Button to go back to menu
- Setup View
	- Select number of each type of card
	- Select number of players
- Game View
	- Player hand
	- Way to select your play
	- Opponents and their hands
	- Current set / table
	- Instructions for controls
	- Indication of whose turn it is
	- A thinking indicator when the AI is making its turn
	- A suggested move when it is your turn
	
╭╭╭╭╭─╮
│││││░│
╰╰╰╰╰─╯

❯

╭──╭──╭──╭──╭──╭──╭──╭──╭──╭─────╮
│4 │5 │5 │7 │8 │10│10│11│12│12   │
│  │  │  │  │  │  │  │  │  │  •  │
│  │  │  │  │  │  │  │  │  │   12│
╰──╰──╰──╰──╰──╰──╰──╰──╰──╰─────╯

                ╭──╭─────╮
╭──╭──╭──╭──╭───│10│10  ╭──╭──╭─────╮
│4 │5 │5 │7 │8  │  │  • │11│12│12   │
│  │  │  │  │   │  │   1│  │  │  •  │
│  │  │  │  │   ╰──╰────│  │  │   12│
╰──╰──╰──╰──╰─────╯     ╰──╰──╰─────╯

╭──╭──╭──╭──╭──╭──╭──╭──╭──╭─────╮
│4 │5 │5 │7 │8 │10│10│11│12│12   │
│  │  │  │  │  │  │  │  │  │     │
│  │  │  │  │  │  │  │  │  │   12│
╰──╰──╰──╰──╰──╰──╰──╰──╰──╰─────╯

                ╭──╭─────╮
╭──╭──╭──╭──╭───│10│10  ╭──╭──╭─────╮
│4 │5 │5 │7 │8  │  │    │11│12│12   │
│  │  │  │  │   │  │   1│  │  │     │
│  │  │  │  │   ╰──╰────│  │  │   12│
╰──╰──╰──╰──╰─────╯     ╰──╰──╰─────╯

| J  | 1  | 2  | 3  | 4  | 5  | 6  | 7  | 8  | 9  | 10 | 11 | 12 |
| 2  | 1  | 0  |    | 3  | 1  |    | 1  | 3  | 3  |    | 5  | 1  |


    ╭─────╮                                   ╭──╭──╭──╭─────╮
╭───│J   ╭──╭──╭──╭──╭──╭──╭──╭──╭──╭──╭──╭───│11│11│11│11  ╭─────╮
│J  │    │1 │4 │4 │4 │5 │7 │8 │8 │8 │9 │9 │9  │  │  │  │    │12   │
│   │    │  │  │  │  │  │  │  │  │  │  │  │   │  │  │  │   1│     │
│   ╰────│  │  │  │  │  │  │  │  │  │  │  │   ╰──╰──╰──╰────│   12│
╰─────╯  ╰──╰──╰──╰──╰──╰──╰──╰──╰──╰──╰──╰─────╯           ╰─────╯

Game View
┌ladder-shedding──────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Ctr+C : Quit to console              │ Ctr+Q : Quit to ladder-shedding menu                                             │
┝━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┥
│    ╭╭╭╭╭╭╭╭╭╭╭╭╭╭╭╭╭╭─╮         │                                                                                       │
│ ❯ Tanner│││││││││││││▒│      18 │                                   - Your Turn -                                       │
│    ╰╰╰╰╰╰╰╰╰╰╰╰╰╰╰╰╰╰─╯         │                                                                                       │
│    ╭╭╭╭╭╭╭╭╭╭─╮                 │                                                                                       │
│   Tiffany││││▒│              10 │                                                                                       │
│    ╰╰╰╰╰╰╰╰╰╰─╯                 │                                                                                       │
│    ╭╭╭╭╭╭╭╭╭╭╭╭─╮               │                                                                                       │
│   Kieran*││││││▒│            12 │                 Kieran is leading the trick with a set of 5 twelves                   │
│    ╰╰╰╰╰╰╰╰╰╰╰╰─╯               │                                                                                       │
│    ╭╭╭╭─╮                       │                              ╭───╭───╭───╭───╭─────╮                                  │
│   Dallin│                    4  │                              │12 │12 │12 │12 │12   │                                  │
│    ╰╰╰╰─╯                       │                              │   │   │   │   │     │                                  │
│                                 │                              │   │   │   │   │   12│                                  │
│   Jeff                       0  │                              ╰───╰───╰───╰───╰─────╯                                  │
│                                 │                                                                                       │
│                                 │                                                                                       │
│                                 │                                                                                       │
│                                 │                                                                                       │
│                                 │                                                                                       │
│                                 │                                                                                       │
│                                 ├───────────────────────────────────────────────────────────────────────────────────────┤
│                                 │   Suggested Move : Disabled               │   Currently Selected : 5 elevens          │
│                                 ├───────────────────────────────────────────────────────────────────────────────────────┤
│                                 │                                                                                       │
│                                 │              ╭─────╮                                   ╭──╭──╭──╭─────╮               │
│                                 │          ╭───│J   ╭──╭──╭──╭──╭──╭──╭──╭──╭──╭──╭──╭───│11│11│11│11  ╭─────╮          │
│                                 │          │J  │    │1 │4 │4 │4 │5 │7 │8 │8 │8 │9 │9 │9  │  │  │  │    │12   │          │
│                                 │          │   │    │  │  │  │  │  │  │  │  │  │  │  │   │  │  │  │   1│     │          │
│                                 │          │   ╰────│  │  │  │  │  │  │  │  │  │  │  │   ╰──╰──╰──╰────│   12│          │
│                                 │          ╰─────╯  ╰──╰──╰──╰──╰──╰──╰──╰──╰──╰──╰──╰─────╯           ╰─────╯          │
│                                 │                                                                                       │
│                                 ├───────────────────────────────────────────────────────────────────────────────────────┤
│                                 │    → : Next Move    │    ← : Prev Move    │     Tab : Pass      │   Enter : Confirm   │
└─────────────────────────────────┴───────────────────────────────────────────────────────────────────────────────────────┘