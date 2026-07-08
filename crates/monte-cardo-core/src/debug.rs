use crate::game;

pub fn debug_display_player_action_values(
    action_values: &[(game::Move, f32)],
    player: game::PlayerID,
) {
    println!("Action values for player {:?}:", player);
    println!("--------------------------------");

    for (rank, (player_move, value)) in action_values.iter().enumerate() {
        println!("{:>3}. {:?}: {:.6}", rank + 1, player_move, value,);
    }

    println!("--------------------------------");
    println!("{} valid actions", action_values.len());
}
