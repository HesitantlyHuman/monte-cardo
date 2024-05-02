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
[x] Write a console application to play against the model
[ ] Update the console application to allow for giving suggestions for playing an outside game
[ ] Test the current lame heuristic with the playing, to validate that it does make reasonable decisions
[ ] setup.py
[ ] auto validation to select best heuristic
[ ] Get rust to handle the python environment and create a command line interface which does everything
[ ] Change all references of ordinality to rank for consistent terminology

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
(Maybe a crown symbol for the current leader?)
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
  - Reverse the card display order
- Game View
	- Player hand
	- Way to select your play
	- Opponents and their hands
	- Current set / table
	- Instructions for controls
	- Indication of whose turn it is
	- A thinking indicator when the AI is making its turn
	- A suggested move when it is your turn