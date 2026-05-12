use crate::consts;

use rand::Rng;

pub type PlayerNumber = usize;
type CardRank = usize;
pub type Hand = [u8; consts::MAX_CARD_ORDINALITY];

#[derive(Debug, Clone, Copy)]
pub struct Play {
    pub rank: CardRank,
    pub num_non_wilds: u8,
    pub num_wilds: u8,
}

impl Play {
    pub fn new(rank: CardRank, num_wilds: u8, num_non_wilds: u8) -> Play {
        Play {
            rank,
            num_non_wilds,
            num_wilds,
        }
    }
}

// TODO: Create move to id and id to move functions. They need to be fast
#[derive(Debug, Clone, Copy)]
pub enum Move {
    Play(Play),
    Pass,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct TopSet {
    pub player: PlayerNumber,
    pub rank: CardRank,
    pub number: u8,
}

impl TopSet {
    pub fn new(player: PlayerNumber, rank: CardRank, number: u8) -> TopSet {
        TopSet {
            player,
            rank,
            number,
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Trick {
    pub top_set: Option<TopSet>,
    pub has_passed: [bool; consts::MAX_PLAYERS],
}

impl Trick {
    fn new() -> Trick {
        Trick {
            top_set: None,
            has_passed: [false; consts::MAX_PLAYERS],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IncompleteInformationGameState {
    pub current_player_number: PlayerNumber,
    pub perspective_player_number: PlayerNumber,
    pub player_hand: Hand,
    pub opponent_cards: [u8; consts::MAX_CARD_ORDINALITY],
    pub player_is_out: [bool; consts::MAX_PLAYERS],
    pub hand_sizes: [u16; consts::MAX_PLAYERS],
    pub trick: Trick,
}

impl IncompleteInformationGameState {
    fn new(
        current_player_number: PlayerNumber,
        perspective_player_number: PlayerNumber,
        player_hand: Hand,
        opponent_cards: [u8; consts::MAX_CARD_ORDINALITY],
        player_is_out: [bool; consts::MAX_PLAYERS],
        hand_sizes: [u16; consts::MAX_PLAYERS],
        trick: Trick,
    ) -> IncompleteInformationGameState {
        IncompleteInformationGameState {
            current_player_number,
            perspective_player_number,
            player_hand,
            opponent_cards,
            player_is_out,
            hand_sizes,
            trick,
        }
    }
}

// TODO: Maybe add a tracker for the number of players so that we don't have to iterate as much
#[derive(Debug, Clone, Copy)]
pub struct FullInformationGameState {
    pub current_player_number: PlayerNumber,
    pub player_hands: [Hand; consts::MAX_PLAYERS],
    pub player_is_out: [bool; consts::MAX_PLAYERS],
    pub trick: Trick,
}

impl FullInformationGameState {
    fn new(
        current_player_number: PlayerNumber,
        player_hands: [Hand; consts::MAX_PLAYERS],
        player_is_out: [bool; consts::MAX_PLAYERS],
        trick: Trick,
    ) -> FullInformationGameState {
        FullInformationGameState {
            current_player_number,
            player_hands,
            player_is_out,
            trick,
        }
    }
}

pub fn get_next_active_player(
    player_is_out: &[bool; consts::MAX_PLAYERS],
    current_player_number: PlayerNumber,
) -> Result<PlayerNumber, &'static str> {
    for i in 1..consts::MAX_PLAYERS {
        let next_player_number = (current_player_number + i) % consts::MAX_PLAYERS;
        if !player_is_out[next_player_number] {
            return Ok(next_player_number);
        }
    }
    Err("No active players left")
}

pub fn update_full_information_game_state(
    game_state: &mut FullInformationGameState,
    player_move: &Move,
) {
    match player_move {
        Move::Play(play) => {
            // Update the player's hand
            game_state.player_hands[game_state.current_player_number][0] -= play.num_wilds;
            game_state.player_hands[game_state.current_player_number][play.rank] -=
                play.num_non_wilds;

            // Update the top set
            game_state.trick.top_set = Some(TopSet {
                player: game_state.current_player_number,
                rank: play.rank,
                number: (play.num_non_wilds + play.num_wilds),
            });

            // Check if the player is out
            if game_state.player_hands[game_state.current_player_number]
                .iter()
                .sum::<u8>()
                == 0
            {
                game_state.player_is_out[game_state.current_player_number] = true;
            }

            // Reset the has_passed array
            game_state.trick.has_passed = [true; consts::MAX_PLAYERS];
            for (player_number, is_out) in game_state.player_is_out.iter().enumerate() {
                if !*is_out {
                    game_state.trick.has_passed[player_number] = false;
                }
            }

            // Update the player number
            for i in 1..consts::MAX_PLAYERS {
                let next_player_number =
                    (game_state.current_player_number + i) % consts::MAX_PLAYERS;
                if !game_state.player_is_out[next_player_number] {
                    game_state.current_player_number = next_player_number;
                    break;
                }
            }
        }
        Move::Pass => {
            // Update the has_passed array
            game_state.trick.has_passed[game_state.current_player_number] = true;

            // Check if all players have passed (except the top set player)
            let mut all_players_passed = true;
            for (player, has_passed) in game_state.trick.has_passed.iter().enumerate() {
                if player == game_state.trick.top_set.unwrap().player {
                    continue;
                }
                if !has_passed {
                    all_players_passed = false;
                    break;
                }
            }

            if all_players_passed {
                // Start a new trick

                // Reset the has_passed array
                game_state.trick.has_passed = [true; consts::MAX_PLAYERS];
                for (player_number, is_out) in game_state.player_is_out.iter().enumerate() {
                    if !*is_out {
                        game_state.trick.has_passed[player_number] = false;
                    }
                }

                let trick_winner = game_state
                    .trick
                    .top_set
                    .as_mut()
                    .expect("All players have passed on an empty top set!")
                    .player; // TODO: fix this

                // Reset the top set
                game_state.trick.top_set = None;

                // Update the player number
                if game_state.player_is_out[trick_winner] {
                    // Player still in after trick winner starts the next trick
                    for i in 1..consts::MAX_PLAYERS {
                        let next_player_number = (trick_winner + i) % consts::MAX_PLAYERS;
                        if !game_state.player_is_out[next_player_number] {
                            game_state.current_player_number = next_player_number;
                            break;
                        }
                    }
                } else {
                    // Trick winner starts the next trick
                    game_state.current_player_number = trick_winner;
                }
            } else {
                // Update the player number
                game_state.current_player_number = get_next_active_player(
                    &game_state.player_is_out,
                    game_state.current_player_number,
                )
                .unwrap();
            }
        }
    }
}

pub fn update_incomplete_information_game_state(
    game_state: &mut IncompleteInformationGameState,
    player_move: &Move,
) {
    match player_move {
        Move::Play(play) => {
            if game_state.current_player_number != game_state.perspective_player_number {
                // Update the opponent's hand
                game_state.opponent_cards[0] -= play.num_wilds;
                game_state.opponent_cards[play.rank] -= play.num_non_wilds;
            } else {
                // Update the player's hand
                game_state.player_hand[0] -= play.num_wilds;
                game_state.player_hand[play.rank] -= play.num_non_wilds;
            }

            // Update hand sizes
            game_state.hand_sizes[game_state.current_player_number] -=
                (play.num_non_wilds + play.num_wilds) as u16;

            // Update the top set
            game_state.trick.top_set = Some(TopSet {
                player: game_state.current_player_number,
                rank: play.rank,
                number: (play.num_non_wilds + play.num_wilds),
            });

            // Check if the player is out
            if game_state.hand_sizes[game_state.current_player_number] == 0 {
                game_state.player_is_out[game_state.current_player_number] = true;
            }

            // Reset the has_passed array
            game_state.trick.has_passed = [true; consts::MAX_PLAYERS];
            for (player_number, is_out) in game_state.player_is_out.iter().enumerate() {
                if !*is_out {
                    game_state.trick.has_passed[player_number] = false;
                }
            }

            // Update the player number
            for i in 1..consts::MAX_PLAYERS {
                let next_player_number =
                    (game_state.current_player_number + i) % consts::MAX_PLAYERS;
                if !game_state.player_is_out[next_player_number] {
                    game_state.current_player_number = next_player_number;
                    break;
                }
            }
        }
        Move::Pass => {
            // Update the has_passed array
            game_state.trick.has_passed[game_state.current_player_number] = true;

            // Check if all players have passed (except the top set player)
            let mut all_players_passed = true;
            for (player, has_passed) in game_state.trick.has_passed.iter().enumerate() {
                if player == game_state.trick.top_set.unwrap().player {
                    continue;
                }
                if !has_passed {
                    all_players_passed = false;
                    break;
                }
            }

            if all_players_passed {
                // Start a new trick

                // Reset the has_passed array
                game_state.trick.has_passed = [true; consts::MAX_PLAYERS];
                for (player_number, is_out) in game_state.player_is_out.iter().enumerate() {
                    if !*is_out {
                        game_state.trick.has_passed[player_number] = false;
                    }
                }

                let trick_winner = game_state
                    .trick
                    .top_set
                    .as_mut()
                    .expect("All players have passed on an empty top set!")
                    .player; // TODO: fix this

                // Reset the card in play
                game_state.trick.top_set = None;

                // Update the player number
                if game_state.player_is_out[trick_winner] {
                    // Player still in after trick winner starts the next trick
                    for i in 1..consts::MAX_PLAYERS {
                        let next_player_number = (trick_winner + i) % consts::MAX_PLAYERS;
                        if !game_state.player_is_out[next_player_number] {
                            game_state.current_player_number = next_player_number;
                            break;
                        }
                    }
                } else {
                    // Trick winner starts the next trick
                    game_state.current_player_number = trick_winner;
                }
            } else {
                // Update the player number
                game_state.current_player_number = get_next_active_player(
                    &game_state.player_is_out,
                    game_state.current_player_number,
                )
                .unwrap();
            }
        }
    }
}

pub fn get_available_moves(hand: Hand, top_set: Option<TopSet>) -> Vec<Move> {
    match top_set {
        Some(top_set) => {
            // We must play something from our hand with the same card number
            // and a lower card type
            let mut moves = Vec::new();
            let num_wilds = hand[0];
            for i in 1..top_set.rank {
                if hand[i] + num_wilds >= top_set.number {
                    let max_non_wilds_playable = top_set.number.min(hand[i]);
                    for num_non_wilds_played in (1..=max_non_wilds_playable).rev() {
                        let num_wilds_needed = top_set.number - num_non_wilds_played;
                        if num_wilds_needed <= num_wilds {
                            moves.push(Move::Play(Play {
                                rank: i,
                                num_non_wilds: num_non_wilds_played,
                                num_wilds: num_wilds_needed,
                            }));
                        } else {
                            break;
                        }
                    }
                }
            }
            moves.push(Move::Pass);
            moves
        }
        None => {
            let mut moves = Vec::new();
            let num_wilds = hand[0];
            for num_wilds_played in 1..=num_wilds {
                moves.push(Move::Play(Play {
                    rank: 0,
                    num_non_wilds: 0,
                    num_wilds: num_wilds_played,
                }));
            }
            for i in 1..consts::MAX_CARD_ORDINALITY {
                for num_non_wilds_played in 1..=hand[i] {
                    for num_wilds_played in 0..=num_wilds {
                        moves.push(Move::Play(Play {
                            rank: i,
                            num_non_wilds: num_non_wilds_played,
                            num_wilds: num_wilds_played,
                        }));
                    }
                }
            }
            moves
        }
    }
}

pub fn generate_random_initial_game_state(
    num_players: usize,
    deck: [u16; consts::MAX_CARD_ORDINALITY],
) -> FullInformationGameState {
    // Split the deck into equal parts for each player
    let mut rng = rand::thread_rng();

    let mut shuffled = Vec::new();
    for i in 0..consts::MAX_CARD_ORDINALITY {
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
        hands.push([0; consts::MAX_CARD_ORDINALITY]);
    }

    for i in 0..shuffled.len() {
        let player_number = i % num_players;
        let card = shuffled[i];
        hands[player_number][card] += 1;
    }

    let mut has_passed = [true; consts::MAX_PLAYERS];
    // Reset the has_passed array
    for (player_number, _) in hands.iter().enumerate() {
        has_passed[player_number] = false;
    }

    let mut trick = Trick::new();
    trick.has_passed = has_passed;

    let mut array_hands = [([0; consts::MAX_CARD_ORDINALITY]); consts::MAX_PLAYERS];
    for (i, hand) in hands.iter().enumerate() {
        array_hands[i] = *hand;
    }

    let mut player_is_out = [true; consts::MAX_PLAYERS];
    for (i, hand) in hands.iter().enumerate() {
        player_is_out[i] = hand.iter().sum::<u8>() == 0;
    }

    FullInformationGameState::new(0, array_hands, player_is_out, trick)
}

// For now, we will leave this to be naive sampling. Just need to set it up to receive the rng.
pub fn generate_random_full_information_game_state_from_incomplete_information_game_state(
    incomplete_information_game_state: &IncompleteInformationGameState,
) -> FullInformationGameState {
    let mut rng = rand::thread_rng();

    // TODO: Remove this duplication (also found in generate_random_initial_game_state)
    let mut shuffled = Vec::new();
    for i in 0..consts::MAX_CARD_ORDINALITY {
        let num_cards = incomplete_information_game_state.opponent_cards[i];
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

    let mut player_hands = [([0; consts::MAX_CARD_ORDINALITY]); consts::MAX_PLAYERS];
    player_hands[incomplete_information_game_state.perspective_player_number] =
        incomplete_information_game_state.player_hand;
    for (hand_size_player_number, hand_size) in incomplete_information_game_state
        .hand_sizes
        .iter()
        .enumerate()
    {
        if *hand_size == 0 {
            continue;
        }
        if hand_size_player_number == incomplete_information_game_state.perspective_player_number {
            continue;
        }
        let mut random_hand = [0; consts::MAX_CARD_ORDINALITY];
        for _ in 0..*hand_size {
            let card = shuffled
                .pop()
                .expect("Hand sizes and opponent cards do not match, not enough cards to shuffle");
            random_hand[card] += 1;
        }
        player_hands[hand_size_player_number] = random_hand;
    }

    FullInformationGameState::new(
        incomplete_information_game_state.current_player_number,
        player_hands,
        incomplete_information_game_state.player_is_out,
        incomplete_information_game_state.trick,
    )
}

pub fn create_incomplete_information_game_state(
    full_information_game_state: FullInformationGameState,
    perspective_player_number: PlayerNumber,
) -> IncompleteInformationGameState {
    let mut opponent_cards = [0; consts::MAX_CARD_ORDINALITY];
    let mut hand_sizes = [0; consts::MAX_PLAYERS];

    for i in 0..full_information_game_state.player_hands.len() {
        let hand = &full_information_game_state.player_hands[i];
        for j in 0..consts::MAX_CARD_ORDINALITY {
            hand_sizes[i] += hand[j] as u16;
            if i != perspective_player_number {
                opponent_cards[j] += hand[j]
            }
        }
    }

    IncompleteInformationGameState::new(
        full_information_game_state.current_player_number,
        perspective_player_number,
        full_information_game_state.player_hands[perspective_player_number],
        opponent_cards,
        full_information_game_state.player_is_out,
        hand_sizes,
        full_information_game_state.trick,
    )
}
