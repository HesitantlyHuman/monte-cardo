use crate::{
    eval::{
        ActionMask, ActionPriorHeuristic, ActionProbabilities, RankCompressed, SearchConfig,
        SearchContext, SearchStats,
    },
    ml::network::NetworkInputs,
};

struct Batch<const N: usize> {
    inputs: [NetworkInputs; N],
    targets: [RankCompressed<ActionProbabilities>; N],
    valid_output_mask: [RankCompressed<ActionMask>; N],
}

struct Batcher<'a, H: ActionPriorHeuristic, const N: usize> {
    search_context: SearchContext<'a, H>,
}

// When we generate these batch outputs, we will be reusing the PUCT cache, for obvious reasons
// Would it be better to do a bunch of them all at once for a given game, and then shuffle things in?
impl<'a, H: ActionPriorHeuristic, const N: usize> Batcher<'a, H, N> {
    fn new(
        number_of_concurrent_games: usize,
        player_number_bounds: (usize, usize),
        deck_card_count_bounds: (usize, usize),
        heuristic: Box<impl ActionPriorHeuristic>,
        search_config: SearchConfig,
    ) -> Self {
        todo!();
    }

    fn update_config(&mut self, search_config: SearchConfig) {}

    fn next(&mut self) -> (Batch<N>, SearchStats) {
        self.search_context.reset_stats();
        todo!()
    }
}
