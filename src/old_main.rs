mod consts;
mod data_generation;
mod game;
mod monte_carlo;
mod network;

fn main() {
    let mut current_game_state =
        game::generate_random_initial_game_state(4, consts::DEFAULT_DALMUTI_DECK);

    // Play a simple game
    // Display the available moves for the current player, and allow the user to select a move
    // using an integer entered in the console.
    // Then, update the game state using the selected move and display the new game state.
    // Repeat until the game is over.
    while current_game_state
        .player_is_out
        .iter()
        .filter(|&&x| !x)
        .count()
        > 1
    {
        // Print out all the player's hands
        for (i, player_hand) in current_game_state.player_hands.iter().enumerate() {
            if current_game_state.player_is_out[i] {
                continue;
            }
            println!("Player {}'s hand: {:?}", i, player_hand);
        }
        println!("Top set: {:?}", current_game_state.trick.top_set);
        println!();

        let available_moves = game::get_available_moves(
            current_game_state.player_hands[current_game_state.current_player_number],
            current_game_state.trick.top_set,
        );

        println!(
            "Player {}'s available moves:",
            current_game_state.current_player_number
        );
        for (i, available_move) in available_moves.iter().enumerate() {
            match available_move {
                game::Move::Play(play) => {
                    println!(
                        "{}. Play {} normal and {} wild of rank {}",
                        i, play.num_non_wilds, play.num_wilds, play.rank
                    );
                }
                game::Move::Pass => {
                    println!("{}. Pass", i);
                }
            }
        }

        println!("{}. Quit", available_moves.len());

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        let input: usize = input.trim().parse().expect("Please type a number!");

        if input == available_moves.len() {
            break;
        }

        println!("---------------------------------");

        let player_move = &available_moves[input];
        game::update_full_information_game_state(&mut current_game_state, player_move);
    }

    // data_generation::generate_data(
    //     std::path::Path::new("data").to_path_buf(),
    //     512 * 35,
    //     512,
    //     true,
    // );
}
