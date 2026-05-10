# ladder-shedding
Simple MCTS based AI for the card game ladder shedding. The heuristic is currently very simple and not very good, but the MCTS is functional and can be used to play the game.

A model will be trained against the MCTS to improve the heuristic.

## TODO
- [ ] Add UCT to monte carlo system
- [x] Find why we are loading an infinite value for some targets
- [x] Add a progress bar for the data generation process
- [ ] Verify that the data generation process can start and stop correctly
- [ ] Parallelize the data generation process?
- [x] Update the data generation to do random decks and numbers of players
- [x] Validate the rust generated data and write a loader for it
- [ ] Write the model and try training on small subset of data to validate and debug
- [ ] Add checkpointing and hyperparameter search to model training?
- [x] Write a console application to play against the model
- [ ] Update the console application to allow for giving suggestions for playing an outside game
- [ ] Test the current lame heuristic with the playing, to validate that it does make reasonable decisions
- [ ] setup.py
- [ ] auto validation to select best heuristic
- [ ] Get rust to handle the python environment and create a command line interface which does everything
- [ ] Change all references of ordinality to rank for consistent terminology

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

# Random Thoughts
- Train a model to generate hypotheticals for the MCTS to explore
  - Would the value of the current state be the minimum or the mean of the hypotheticals?
    - If we are talking about Nash equilibria, then the value of the current state would be the minimum of the hypotheticals, assuming that the outcome of the hypotheticals is not worse than optimal play. You would almost want to do multiple rollouts for each hypothetical to get a better idea of the value of the hypothetical, and then take the minimum of the hypotheticals.
- I like the ideas in the DeepNash paper, where they add an additional condition that the model should remain similar to its past iterations.


# New notes / A better approach?
- We should be using min-max at the top of the tree, and use MCTS to improve our heuristics. MCTS does a poor job of simulating opponents who are using optimal strategy, which does not seem to do well in this type of game. (I wonder if you could formalize this notion of where MCTS would excel and where it would not)
- The other issue is that we are treating the hypothetical states as equally likely, when they are not. However, if we operate under the worst case scenario assumption, then this is less of a problem. That is because by considering situations which are bad for us with higher weight, we are implicitly assuming perfect play on the part of our opponents. This may not be optimal against weak opponents, and is more pessimistic that it needs to be, however.
- **See photo in phone for diagram of new approach, mixing min-max and MCTS**

**New Plan**

https://icml.cc/media/icml-2019/Slides/4443.pdf

https://poker.cs.ualberta.ca/publications/NIPS12.pdf

Use PUCT to drive the search

$$
PUCT(s, a) = Q(s, a) + cP(s, a)\frac{\sqrt{N(s)}}{1 + N(s, a)}
$$

Where $s$ is the current state, and $a$ is the action under consideration.

$P(s, a)$ is a model which predicts $N(s, a)/N(s)$, i.e. the fraction of time that this action is chosen.

However, the actual value of a given action is more complicated for us...

$$
V_p(k_p, a_n) = \sum_{u \in U} \mathbb{P}(u | k_p) \cdot V_p(f(u, a_n), \argmax_{a_{n+1} \in A} V_{p+1}(f(u, a_n), a_{n+1}))
$$

Nevermind, once we assume a particular unknown state, we don't go back. The only reason to consider a certain player's known state is to predict their behavior.

$$
V_p(s, a) = 1 - \frac{S(p)}{N}
$$

$$
V_p(s, a) = V_p(f(s, a), \argmax_{a_n \in A} E_{p+1}(k_{p+1}(f(s, a)), a_n))
$$

$$
E_p(k_p, a) = \sum_{s \in X} \mathbb{P}(s | k_p) V_p(s, a)
$$

Where $f(u, a_n)$ is the function which gets a new unknown state by applying action $a_n$ to unknown state $u$.

However, it may make it easier to calculate if we use probabilities for the other player's moves. The other benefits are that with >2 players, there is inherent instability to minmax. One of the other players playing non-optimally may cause you to loose value. Thus, a probabilistic model would improve the quality of play.

(Probability of player $p$ making move $a_n$ given current state $u$)
$$
P(u, a_n, p)
$$


> **Aside**: Does swapping the summation and the max maintain the monotonicity of the function $V$?
>
> $$
  V(k, a_0) = \max(0.1, 0.2, 0.4, 0.6) + \max(0.1, 0.3, 0.2) = 0.6 + 0.3 = 0.9
> $$
> $$
  V(k, a_1) = \max(0.1, 0.2, 0.3, 0.3) + \max(0.1, 0.4, 0.2, 0.2, 0.1, 0.3, 0.2) = 0.3 + 0.4 = 0.7
> $$
> $$
  V'(k, a_0) = \max(0.1 + 0.2 + 0.4 + 0.6, 0.1 + 0.3 + 0.2) = \max(1.3, 0.6) = 1.3
> $$
> $$
  V'(k, a_1) = \max(0.1 + 0.2 + 0.3 + 0.3, 0.1 + 0.4 + 0.2 + 0.2 + 0.1 + 0.3 + 0.2) = \max(0.9, 1.5) = 1.5
> $$
> No, it clearly does not, so we cannot move it around like that.

---

# New Design

value_to_probabilities(action_value_matrix, temperature) -> action_p_matrix:
	first, get the portion of the action value matrix that is specific to the player in question
	exp(action_value_matrix[i] / temperature) / sum i exp(action_value_matrix[i] / temperature)

generate_training_example(incomplete_information_state) -> incomplete_information_state, action_value_matrix:
	incomplete_information_state, value_to_probabilities(full_tree_evaluation(incomplete_information_state))

full_tree_evaluation(incomplete_information_state) -> action_value_matrix:
	- Generate some W possible worlds, and assign probability scores to each of them (start with uniform, but output values to train a model)
	- Generate all actions available to the player (the same for every possible world)
	- For each pair of possible world and player action, generate the incomplete information set of the next player, and depending on depth, call full_tree_evaluation or puct_evaluation
	- The value of an action is the expectation over the worlds

PUCT_counts = {}

puct_evaluation(incomplete_information_state) -> action_value_matrix:
	- Generate or take in some W possible worlds, and assign prob scores to each
	- Take turns rolling these worlds out, first applying a move to the world, then generating the next player's incomplete info, calculating the PUCT scores, choosing the max action, and then repeating until the game ends. We use the learned heuristic here, to guide the search, and we should probably limit the max size of the lookup.
	- When we get to the end of the game, we use the player positions to set a value to each, between 0 and 1. Those are added to the action value for that action.
	
	
All the returned action value matrices are 2d (action by resulting player value), but the training examples are only made for the "acting" player, because that is all we need the heuristic for.