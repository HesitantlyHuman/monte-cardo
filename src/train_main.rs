use ai::monte_carlo::Heuristic;

mod ai;
mod consts;
mod ui;

fn main() {
    let basic_heuristic = ai::monte_carlo::BasicHeuristic {};
    let random_heuristic = ai::monte_carlo::RandomHeuristic {};
    let heuristics: Vec<&dyn Heuristic> = vec![&basic_heuristic, &random_heuristic];

    let results = ai::tourney::run_tourney(&heuristics, 100, true);
    println!("{:?}", results);
}
