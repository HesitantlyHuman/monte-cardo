use ai::monte_carlo::Heuristic;

mod ai;
mod consts;
mod ui;

fn main() {
    let basic_heuristic = ai::monte_carlo::BasicHeuristic {};
    let random_heuristic = ai::monte_carlo::RandomHeuristic {};
    let heuristics: Vec<&dyn Heuristic> = vec![&basic_heuristic, &random_heuristic];

    let results = ai::tourney::run_ai_game(&heuristics);

    for (heuristic, score) in results.iter().enumerate() {
        println!("Heuristic {:?}: {}", heuristics[heuristic], score);
    }
}
