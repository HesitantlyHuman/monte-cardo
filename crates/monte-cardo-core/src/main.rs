fn main() {
    let mut heuristic = monte_cardo_core::eval::SimpleHeuristic::default();
    let search_config = monte_cardo_core::eval::SearchConfig::inference();
    let mut search_context =
        monte_cardo_core::eval::SearchContext::with_seed(&mut heuristic, search_config, 42);

    let initial_game_state = monte_cardo_core::game::generate_random_initial_game_state(
        4,
        &monte_cardo_core::consts::DEFAULT_DALMUTI_DECK,
        &mut search_context.rng,
    );

    let incomplete_information_state =
        monte_cardo_core::game::create_incomplete_information_game_state(
            &initial_game_state,
            initial_game_state.current_player_number,
        );
    let moves = monte_cardo_core::eval::get_action_values(
        &incomplete_information_state,
        &mut search_context,
    )
    .unwrap();

    monte_cardo_core::debug::debug_display_player_action_values(
        &moves,
        initial_game_state.current_player_number,
    );
    search_context.stats.print_stats();
}
