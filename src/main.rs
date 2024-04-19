use rand::Rng;

// Lets have the max number of players be 16. And the minimum is obviously 2.
const MAX_PLAYERS: usize = 16;
// Maximum different numbers the cards can have
const MAX_CARD_ORDINALITY: usize = 16;
// Maximum number of a single ordinality of card
const MAX_CARD_NUMBER: usize = 16;

// Default deck for "The Great Dalmuti"
const DEFAULT_DALMUTI_DECK: [u16; 16] = [
    2, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 0,
    0, // The first position is always wilds, or "jesters"
];
// Default deck for "Scum"
const DEFAULT_SCUM_DECK: [u16; 16] = [2, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 0, 0];

type PlayerNumber = usize;
type CardType = usize;
type HandSizes = [u16; MAX_PLAYERS];
type Hand = [u16; MAX_CARD_ORDINALITY];
type OpponentCards = [u16; MAX_CARD_ORDINALITY];
type Move = Option<(CardType, u16, u16)>; // Card ordinality, number of cards, number of wilds
type CardInPlay = Option<(CardType, u16)>;
type IncompleteInformationGameState = (PlayerNumber, Hand, OpponentCards, HandSizes, CardInPlay);
type FullInformationGameState = (Vec<Hand>, CardInPlay);

trait Heuristic {
    fn estimate_state_value(&self, game_state: IncompleteInformationGameState) -> f32;
}

// Takes the information we have currently about the game and uses a heuristic function to generate
// a best move. Searches using a markov tree to a given number of nodes before using the heuristic.
// Uses the markov_tree_state_value function to find the best move.
fn get_best_move(
    game_state: IncompleteInformationGameState,
    heuristic: &dyn Heuristic,
    num_search_nodes: usize,
) -> Move {
    panic!("Not implemented");
}

// Searches nodes and uses the heuristic to update a win value probability for the given
// board state. The value of an end state is the normalized position of that player in the
// rankings. 1st is 1.0, last is 0.0.
fn markov_tree_state_value_from_incomplete_information(
    game_state: IncompleteInformationGameState,
    heuristic: &dyn Heuristic,
    num_search_nodes: usize,
) -> f32 {
    panic!("Not implemented");
}

fn simple_markov_rollout(
    game_state: IncompleteInformationGameState,
    heuristic: &dyn Heuristic,
    num_rollouts: usize,
) -> f64 {
    let (player_number, player_hand, opponent_cards, hand_sizes, card_in_play) = game_state;

    for _ in 0..num_rollouts {
        let game_state = generate_random_full_information_game_state(game_state);
    }

    0.0
}

// Should prob check for wins here
fn update_game_state(
    game_state: FullInformationGameState,
    player_number: PlayerNumber,
    player_move: Move,
) -> FullInformationGameState {
}

struct BasicHeuristic {}

impl Heuristic for BasicHeuristic {
    fn estimate_state_value(&self, game_state: IncompleteInformationGameState) -> f32 {
        0.5
    }
}

fn get_available_moves(player_hand: Hand, card_in_play: CardInPlay) -> Vec<Move> {
    match card_in_play {
        Some((card_type, card_number)) => {
            println!(
                "Current card in play: {} of type {}",
                card_number, card_type
            );
            // We must play something from our hand with the same card number
            // and a lower card type
            let mut moves = Vec::new();
            let num_wilds = player_hand[0];
            for i in 1..card_type {
                if player_hand[i] + num_wilds >= card_number {
                    for num_non_wilds_played in (1..=card_number).rev() {
                        let num_wilds_played = card_number - num_non_wilds_played;
                        if num_wilds_played <= num_wilds {
                            moves.push(Some((i, num_non_wilds_played, num_wilds_played)));
                        } else {
                            break;
                        }
                    }
                }
            }
            moves.push(None); // We can always pass
            moves
        }
        None => {
            println!("No current card in play");
            let mut moves = Vec::new();
            let num_wilds = player_hand[0];
            for num_wilds_played in 1..num_wilds {
                moves.push(Some((0, 0, num_wilds_played)));
            }
            for i in 1..MAX_CARD_ORDINALITY {
                for num_non_wilds_played in 1..=player_hand[i] {
                    for num_wilds_played in 0..=num_wilds {
                        moves.push(Some((i, num_non_wilds_played, num_wilds_played)));
                    }
                }
            }
            moves.push(None); // We can always pass
            moves
        }
    }
}

fn generate_simulated_game_batch(
    heuristic: &dyn Heuristic,
    num_games: usize,
    num_players: usize,
    deck: [u16; MAX_CARD_ORDINALITY],
) -> Vec<(IncompleteInformationGameState, f32)> {
    vec![]
}

fn generate_random_initial_game_state(
    num_players: usize,
    deck: [u16; MAX_CARD_ORDINALITY],
) -> FullInformationGameState {
    // Split the deck into equal parts for each player
    let mut rng = rand::thread_rng();

    let mut shuffled = Vec::new();
    for i in 0..MAX_CARD_ORDINALITY {
        let num_cards = deck[i];
        for _ in 0..num_cards {
            match shuffled.len() == 0 {
                false => {
                    let random_index = rng.gen_range(0..shuffled.len());
                    if random_index != shuffled.len() {
                        shuffled.push(shuffled[random_index]);
                        shuffled[random_index] = i;
                    } else {
                        shuffled.push(i);
                    }
                }
                true => {
                    shuffled.push(i);
                }
            };
        }
    }

    let mut hands = Vec::new();
    for _ in 0..num_players {
        hands.push([0; MAX_CARD_ORDINALITY]);
    }

    for i in 0..shuffled.len() {
        let player_number = i % num_players;
        let card = shuffled[i];
        hands[player_number][card] += 1;
    }

    (hands, None)
}

fn generate_random_full_information_game_state(
    incomplete_information_game_state: IncompleteInformationGameState,
) -> FullInformationGameState {
    let (player_number, player_hand, opponent_cards, hand_sizes, card_in_play) =
        incomplete_information_game_state;
    let mut rng = rand::thread_rng();

    // TODO: Remove this duplication (also found in generate_random_initial_game_state)
    let mut shuffled = Vec::new();
    for i in 0..MAX_CARD_ORDINALITY {
        let num_cards = opponent_cards[i];
        for _ in 0..num_cards {
            match shuffled.len() == 0 {
                false => {
                    let random_index = rng.gen_range(0..shuffled.len());
                    if random_index != shuffled.len() {
                        shuffled.push(shuffled[random_index]);
                        shuffled[random_index] = i;
                    } else {
                        shuffled.push(i);
                    }
                }
                true => {
                    shuffled.push(i);
                }
            };
        }
    }

    let mut player_hands = Vec::new();
    for (hand_size_player_number, hand_size) in hand_sizes.iter().enumerate() {
        if *hand_size == 0 {
            continue;
        }
        if hand_size_player_number == player_number {
            player_hands.push(player_hand);
            continue;
        }
        let mut random_hand = [0; MAX_CARD_ORDINALITY];
        for _ in 0..*hand_size {
            let card = shuffled
                .pop()
                .expect("Hand sizes and opponent cards do not match, not enough cards to shuffle");
            random_hand[card] += 1;
        }
        player_hands.push(random_hand);
    }

    (player_hands, card_in_play)
}

fn create_incomplete_information_game_state(
    full_information_game_state: FullInformationGameState,
    player_number: PlayerNumber,
) -> IncompleteInformationGameState {
    let (hands, card_in_play) = full_information_game_state;
    let mut opponent_cards = [0; MAX_CARD_ORDINALITY];
    let mut hand_sizes = [0; MAX_PLAYERS];

    for i in 0..hands.len() {
        let hand = &hands[i];
        for j in 0..MAX_CARD_ORDINALITY {
            hand_sizes[i] += hand[j];
            if i != player_number {
                opponent_cards[j] += hand[j];
            }
        }
    }

    (
        player_number,
        hands[player_number],
        opponent_cards,
        hand_sizes,
        card_in_play,
    )
}

fn main() {
    let mut size_of_deck = 0;
    for card_type_number in &DEFAULT_DALMUTI_DECK {
        size_of_deck += card_type_number;
    }
    println!("Size of dalmuti deck {}", size_of_deck);
    let game_state = generate_random_initial_game_state(4, DEFAULT_DALMUTI_DECK);
    let mut totals = [0; MAX_CARD_ORDINALITY];
    for hand in &game_state.0 {
        println!("{:?}", hand);
        for (card_i, card_n) in hand.iter().enumerate() {
            totals[card_i] += card_n;
        }
    }
    println!("{:?}", totals);
    let incomplete_game_state = create_incomplete_information_game_state(game_state, 0);
    let predicted_game_state = generate_random_full_information_game_state(incomplete_game_state);
    let mut totals = [0; MAX_CARD_ORDINALITY];
    for hand in &predicted_game_state.0 {
        println!("{:?}", hand);
        for (card_i, card_n) in hand.iter().enumerate() {
            totals[card_i] += card_n;
        }
    }
    println!("{:?}", totals);
}

// General Plan:
// MCTS Random Rollout. When we consider the rollouts, we first randomly generate the missing state
// for the other players, then perform a full rollout using that information. Like normal, use our heuristic
// to guide the rollout patterns.
// Concerns: The generated state may be less homogenious than I would guess most games to be. Players with fewer cards will
// likely have kept their better cards if possible, skewing the hand distributions. This effect will become more pronounced
// with large disparities in the hand sizes, or skill levels of the players. This could lead to convergence on non-optimal play.
//
// Necessary functions:
// 1. Generating a random complete information state from an incomplete information state.
// 2. Performing a heuristic guided rollout of that state
