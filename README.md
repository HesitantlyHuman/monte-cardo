# ladder-shedding
Simple MCTS based AI for the card game ladder shedding. The heuristic is currently very simple and not very good, but the MCTS is functional and can be used to play the game.

A model will be trained against the MCTS to improve the heuristic.

## TODO
- Add a progress bar for the data generation process
- Verify that the data generation process can start and stop correctly
- Parallelize the data generation process?
- Update the data generation to do random decks and numbers of players
- Validate the rust generated data and write a loader for it
- Write the model and try training on small subset of data to validate and debug
- Add checkpointing and hyperparameter search to model training?
- Write a console application to play against the model and/or use it for outside play
- Test the current lame heuristic with the playing, to validate that it does make reasonable decisions
- setup.py
- auto validation to select best heuristic