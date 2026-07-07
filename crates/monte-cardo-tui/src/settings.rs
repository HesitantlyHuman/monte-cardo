use monte_cardo_core::{consts, eval::SearchConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    PlayComputers,
    PlayLive,
}

impl GameMode {
    pub fn label(self) -> &'static str {
        match self {
            GameMode::PlayComputers => "Play Computers",
            GameMode::PlayLive => "Play Live",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SolverSettings {
    pub enabled: bool,
    pub exploration_factor: f32,
    pub temperature: f32,
    pub greediness: f32,
    pub full_tree_depth: usize,
    pub num_worlds: usize,
    pub puct_rollouts_per_leaf: usize,
    pub puct_rollout_bounds: (usize, usize),
    pub puct_mature_node_min_visits: usize,
    pub puct_node_capacity: usize,
    pub random_seed: u64,
}

impl SolverSettings {
    pub fn inference_default() -> Self {
        let config = SearchConfig::inference();

        Self {
            enabled: true,
            exploration_factor: config.exploration_factor,
            temperature: config.temperature,
            greediness: config.greediness,
            full_tree_depth: config.full_tree_depth,
            num_worlds: config.num_worlds,
            puct_rollouts_per_leaf: config.puct_rollouts_per_leaf,
            puct_rollout_bounds: config.puct_rollout_bounds,
            puct_mature_node_min_visits: config.puct_mature_node_min_visits,
            puct_node_capacity: config.puct_node_capacity,
            random_seed: 42,
        }
    }

    pub fn to_search_config(&self) -> SearchConfig {
        SearchConfig {
            exploration_factor: self.exploration_factor,
            temperature: self.temperature,
            greediness: self.greediness,
            full_tree_depth: self.full_tree_depth,
            num_worlds: self.num_worlds,
            puct_rollouts_per_leaf: self.puct_rollouts_per_leaf,
            puct_rollout_bounds: self.puct_rollout_bounds,
            puct_mature_node_min_visits: self.puct_mature_node_min_visits,
            puct_node_capacity: self.puct_node_capacity,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameSettings {
    pub mode: GameMode,
    pub deck: [usize; consts::MAX_CARD_ORDINALITY],
    pub inverted_ordering: bool,
    pub number_of_players: usize,
    pub player_names: Vec<String>,
    pub ai_suggestions_enabled: bool,
    pub solver: SolverSettings,
}

impl GameSettings {
    pub fn ensure_player_names(&mut self) {
        while self.player_names.len() < consts::MAX_PLAYERS {
            let next_index = self.player_names.len();
            self.player_names.push(format!("Player {}", next_index + 1));
        }
    }
}

impl Default for GameSettings {
    fn default() -> Self {
        let mut settings = Self {
            mode: GameMode::PlayComputers,
            deck: consts::DEFAULT_DALMUTI_DECK,
            inverted_ordering: false,
            number_of_players: 4,
            player_names: vec![
                "Tanner".to_string(),
                "Tiffany".to_string(),
                "Kieran".to_string(),
                "Dallin".to_string(),
            ],
            ai_suggestions_enabled: true,
            solver: SolverSettings::inference_default(),
        };

        settings.ensure_player_names();
        settings
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsFocus {
    Mode,
    Deck,
    Players,
    Rules,
    Start,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerPanelSelection {
    NumberOfPlayers,
    PlayerName(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    InvertedOrdering,
    AiSuggestionsEnabled,
    SolverEnabled,
    ExplorationFactor,
    Temperature,
    Greediness,
    FullTreeDepth,
    NumWorlds,
    PuctRolloutsPerLeaf,
    PuctRolloutLowerBound,
    PuctRolloutUpperBound,
    PuctMatureNodeMinVisits,
    PuctCacheSize,
    RandomSeed,
}

impl SettingsField {
    pub fn visible_rules(settings: &GameSettings) -> Vec<Self> {
        let mut fields = Vec::new();

        // fields.push(Self::InvertedOrdering);
        fields.push(Self::AiSuggestionsEnabled);
        fields.push(Self::SolverEnabled);

        if settings.solver.enabled {
            fields.push(Self::ExplorationFactor);
            fields.push(Self::Temperature);
            fields.push(Self::Greediness);
            fields.push(Self::FullTreeDepth);
            fields.push(Self::NumWorlds);
            fields.push(Self::PuctRolloutsPerLeaf);
            fields.push(Self::PuctRolloutLowerBound);
            fields.push(Self::PuctRolloutUpperBound);
            fields.push(Self::PuctMatureNodeMinVisits);
            fields.push(Self::PuctCacheSize);
            fields.push(Self::RandomSeed);
        }

        fields
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::InvertedOrdering => "Inverted Ordering",
            Self::AiSuggestionsEnabled => "AI Suggestions and Move Values",
            Self::SolverEnabled => "AI Solver",
            Self::ExplorationFactor => "Exploration Factor",
            Self::Temperature => "Temperature",
            Self::Greediness => "Greediness",
            Self::FullTreeDepth => "Full Tree Depth",
            Self::NumWorlds => "Number of Worlds to Consider",
            Self::PuctRolloutsPerLeaf => "PUCT Rollouts per Leaf",
            Self::PuctRolloutLowerBound => "PUCT Rollout Lower Bound",
            Self::PuctRolloutUpperBound => "PUCT Rollout Upper Bound",
            Self::PuctMatureNodeMinVisits => "PUCT Mature Node Min Visits",
            Self::PuctCacheSize => "PUCT Cache Size",
            Self::RandomSeed => "Random Seed",
        }
    }

    pub fn value(self, settings: &GameSettings) -> String {
        match self {
            Self::InvertedOrdering => enabled_disabled(settings.inverted_ordering).to_string(),
            Self::AiSuggestionsEnabled => {
                enabled_disabled(settings.ai_suggestions_enabled).to_string()
            }
            Self::SolverEnabled => enabled_disabled(settings.solver.enabled).to_string(),
            Self::ExplorationFactor => format!("{:.2}", settings.solver.exploration_factor),
            Self::Temperature => format!("{:.2}", settings.solver.temperature),
            Self::Greediness => format!("{:.2}", settings.solver.greediness),
            Self::FullTreeDepth => settings.solver.full_tree_depth.to_string(),
            Self::NumWorlds => settings.solver.num_worlds.to_string(),
            Self::PuctRolloutsPerLeaf => settings.solver.puct_rollouts_per_leaf.to_string(),
            Self::PuctRolloutLowerBound => settings.solver.puct_rollout_bounds.0.to_string(),
            Self::PuctRolloutUpperBound => settings.solver.puct_rollout_bounds.1.to_string(),
            Self::PuctMatureNodeMinVisits => {
                settings.solver.puct_mature_node_min_visits.to_string()
            }
            Self::PuctCacheSize => settings.solver.puct_node_capacity.to_string(),
            Self::RandomSeed => settings.solver.random_seed.to_string(),
        }
    }

    pub fn is_bool(self) -> bool {
        matches!(
            self,
            Self::InvertedOrdering | Self::AiSuggestionsEnabled | Self::SolverEnabled
        )
    }

    pub fn allows_decimal_text(self) -> bool {
        matches!(
            self,
            Self::ExplorationFactor | Self::Temperature | Self::Greediness
        )
    }

    pub fn adjust(self, settings: &mut GameSettings, delta: isize) {
        match self {
            Self::InvertedOrdering => {
                if delta != 0 {
                    settings.inverted_ordering = !settings.inverted_ordering;
                }
            }
            Self::AiSuggestionsEnabled => {
                if delta != 0 {
                    settings.ai_suggestions_enabled = !settings.ai_suggestions_enabled;
                }
            }
            Self::SolverEnabled => {
                if delta != 0 {
                    settings.solver.enabled = !settings.solver.enabled;
                }
            }
            Self::ExplorationFactor => {
                settings.solver.exploration_factor =
                    adjust_f32(settings.solver.exploration_factor, delta, 0.05, 0.0, 10.0);
            }
            Self::Temperature => {
                settings.solver.temperature =
                    adjust_f32(settings.solver.temperature, delta, 0.05, 0.01, 10.0);
            }
            Self::Greediness => {
                settings.solver.greediness =
                    adjust_f32(settings.solver.greediness, delta, 0.05, 0.05, 10.0);
            }
            Self::FullTreeDepth => {
                settings.solver.full_tree_depth =
                    adjust_usize(settings.solver.full_tree_depth, delta, 1, 0, 5);
            }
            Self::NumWorlds => {
                settings.solver.num_worlds =
                    adjust_usize(settings.solver.num_worlds, delta, 5, 1, 10_000);
            }
            Self::PuctRolloutsPerLeaf => {
                settings.solver.puct_rollouts_per_leaf =
                    adjust_usize(settings.solver.puct_rollouts_per_leaf, delta, 5, 1, 10_000);
            }
            Self::PuctRolloutLowerBound => {
                let upper = settings.solver.puct_rollout_bounds.1;
                settings.solver.puct_rollout_bounds.0 =
                    adjust_usize(settings.solver.puct_rollout_bounds.0, delta, 1, 0, upper);
            }
            Self::PuctRolloutUpperBound => {
                let lower = settings.solver.puct_rollout_bounds.0;
                settings.solver.puct_rollout_bounds.1 = adjust_usize(
                    settings.solver.puct_rollout_bounds.1,
                    delta,
                    1,
                    lower.max(1),
                    10_000,
                );
            }
            Self::PuctMatureNodeMinVisits => {
                settings.solver.puct_mature_node_min_visits = adjust_usize(
                    settings.solver.puct_mature_node_min_visits,
                    delta,
                    8,
                    1,
                    1_000_000,
                );
            }
            Self::PuctCacheSize => {
                settings.solver.puct_node_capacity = adjust_usize(
                    settings.solver.puct_node_capacity,
                    delta,
                    100_000,
                    1_000,
                    100_000_000,
                );
            }
            Self::RandomSeed => {
                if delta > 0 {
                    settings.solver.random_seed =
                        settings.solver.random_seed.saturating_add(delta as u64);
                } else if delta < 0 {
                    settings.solver.random_seed =
                        settings.solver.random_seed.saturating_sub((-delta) as u64);
                }
            }
        }
    }

    pub fn set_from_text(self, settings: &mut GameSettings, text: &str) {
        if text.is_empty() {
            return;
        }

        match self {
            Self::ExplorationFactor => {
                if let Ok(value) = text.parse::<f32>() {
                    settings.solver.exploration_factor = value.clamp(0.0, 10.0);
                }
            }
            Self::Temperature => {
                if let Ok(value) = text.parse::<f32>() {
                    settings.solver.temperature = value.clamp(0.01, 10.0);
                }
            }
            Self::Greediness => {
                if let Ok(value) = text.parse::<f32>() {
                    settings.solver.greediness = value.clamp(0.05, 10.0);
                }
            }
            Self::FullTreeDepth => {
                if let Ok(value) = text.parse::<usize>() {
                    settings.solver.full_tree_depth = value.min(5);
                }
            }
            Self::NumWorlds => {
                if let Ok(value) = text.parse::<usize>() {
                    settings.solver.num_worlds = value.clamp(1, 10_000);
                }
            }
            Self::PuctRolloutsPerLeaf => {
                if let Ok(value) = text.parse::<usize>() {
                    settings.solver.puct_rollouts_per_leaf = value.clamp(1, 10_000);
                }
            }
            Self::PuctRolloutLowerBound => {
                if let Ok(value) = text.parse::<usize>() {
                    settings.solver.puct_rollout_bounds.0 =
                        value.min(settings.solver.puct_rollout_bounds.1);
                }
            }
            Self::PuctRolloutUpperBound => {
                if let Ok(value) = text.parse::<usize>() {
                    settings.solver.puct_rollout_bounds.1 =
                        value.max(settings.solver.puct_rollout_bounds.0).min(10_000);
                }
            }
            Self::PuctMatureNodeMinVisits => {
                if let Ok(value) = text.parse::<usize>() {
                    settings.solver.puct_mature_node_min_visits = value.clamp(1, 1_000_000);
                }
            }
            Self::PuctCacheSize => {
                if let Ok(value) = text.parse::<usize>() {
                    settings.solver.puct_node_capacity = value.clamp(1_000, 100_000_000);
                }
            }
            Self::RandomSeed => {
                if let Ok(value) = text.parse::<u64>() {
                    settings.solver.random_seed = value;
                }
            }
            Self::InvertedOrdering | Self::AiSuggestionsEnabled | Self::SolverEnabled => {}
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingsFormState {
    pub focus: SettingsFocus,

    pub mode_cursor: GameMode,

    pub deck_rank: usize,
    pub deck_editing: bool,
    pub deck_edit_buffer: String,

    pub player_selection: PlayerPanelSelection,
    pub player_name_editing: bool,
    pub player_count_editing: bool,
    pub player_count_edit_buffer: String,

    pub rules_index: usize,
    pub rules_editing: bool,
    pub rules_edit_buffer: String,
}

impl SettingsFormState {
    pub fn new() -> Self {
        Self {
            focus: SettingsFocus::Mode,

            mode_cursor: GameMode::PlayComputers,

            deck_rank: 0,
            deck_editing: false,
            deck_edit_buffer: String::new(),

            player_selection: PlayerPanelSelection::NumberOfPlayers,
            player_name_editing: false,
            player_count_editing: false,
            player_count_edit_buffer: String::new(),

            rules_index: 0,
            rules_editing: false,
            rules_edit_buffer: String::new(),
        }
    }

    pub fn clear_editing(&mut self) {
        self.deck_editing = false;
        self.deck_edit_buffer.clear();

        self.player_name_editing = false;
        self.player_count_editing = false;
        self.player_count_edit_buffer.clear();

        self.rules_editing = false;
        self.rules_edit_buffer.clear();
    }

    pub fn clamp_to_settings(&mut self, settings: &GameSettings) {
        self.deck_rank = self.deck_rank.min(consts::MAX_CARD_ORDINALITY - 1);

        if let PlayerPanelSelection::PlayerName(index) = self.player_selection {
            if index >= settings.number_of_players {
                self.player_selection =
                    PlayerPanelSelection::PlayerName(settings.number_of_players.saturating_sub(1));
            }
        }

        let rules_len = SettingsField::visible_rules(settings).len();
        if rules_len == 0 {
            self.rules_index = 0;
        } else if self.rules_index >= rules_len {
            self.rules_index = rules_len - 1;
        }
    }

    pub fn selected_rule_field(&self, settings: &GameSettings) -> Option<SettingsField> {
        SettingsField::visible_rules(settings)
            .get(self.rules_index)
            .copied()
    }

    pub fn focus_mode(&mut self) {
        self.clear_editing();
        self.focus = SettingsFocus::Mode;
    }

    pub fn focus_deck_start(&mut self) {
        self.clear_editing();
        self.focus = SettingsFocus::Deck;
        self.deck_rank = 0;
    }

    pub fn focus_players(&mut self) {
        self.clear_editing();
        self.focus = SettingsFocus::Players;
    }

    pub fn focus_rules(&mut self) {
        self.clear_editing();
        self.focus = SettingsFocus::Rules;
    }

    pub fn focus_start(&mut self) {
        self.clear_editing();
        self.focus = SettingsFocus::Start;
    }

    pub fn move_mode_cursor(&mut self, delta: isize) {
        if delta != 0 {
            self.mode_cursor = match self.mode_cursor {
                GameMode::PlayComputers => GameMode::PlayLive,
                GameMode::PlayLive => GameMode::PlayComputers,
            };
        }
    }

    pub fn move_deck_rank(&mut self, delta: isize) {
        if delta > 0 {
            self.deck_rank = (self.deck_rank + 1).min(consts::MAX_CARD_ORDINALITY - 1);
        } else if delta < 0 {
            self.deck_rank = self.deck_rank.saturating_sub(1);
        }
    }

    pub fn start_deck_editing(&mut self) {
        self.deck_editing = true;
        self.deck_edit_buffer.clear();
    }

    pub fn finish_deck_editing(&mut self) {
        self.deck_editing = false;
        self.deck_edit_buffer.clear();
    }

    pub fn move_player_selection_up(&mut self) -> bool {
        match self.player_selection {
            PlayerPanelSelection::NumberOfPlayers => false,
            PlayerPanelSelection::PlayerName(0) => {
                self.player_selection = PlayerPanelSelection::NumberOfPlayers;
                true
            }
            PlayerPanelSelection::PlayerName(index) => {
                self.player_selection = PlayerPanelSelection::PlayerName(index - 1);
                true
            }
        }
    }

    pub fn move_player_selection_down(&mut self, settings: &GameSettings) -> bool {
        match self.player_selection {
            PlayerPanelSelection::NumberOfPlayers => {
                if settings.number_of_players > 0 {
                    self.player_selection = PlayerPanelSelection::PlayerName(0);
                    true
                } else {
                    false
                }
            }
            PlayerPanelSelection::PlayerName(index) => {
                if index + 1 < settings.number_of_players {
                    self.player_selection = PlayerPanelSelection::PlayerName(index + 1);
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn start_player_editing(&mut self) {
        match self.player_selection {
            PlayerPanelSelection::NumberOfPlayers => {
                self.player_count_editing = true;
                self.player_count_edit_buffer.clear();
            }
            PlayerPanelSelection::PlayerName(_) => {
                self.player_name_editing = true;
            }
        }
    }

    pub fn finish_player_editing(&mut self) {
        self.player_name_editing = false;
        self.player_count_editing = false;
        self.player_count_edit_buffer.clear();
    }

    pub fn move_rules_up(&mut self) -> bool {
        if self.rules_index > 0 {
            self.rules_index -= 1;
            true
        } else {
            false
        }
    }

    pub fn move_rules_down(&mut self, settings: &GameSettings) -> bool {
        let rules_len = SettingsField::visible_rules(settings).len();

        if self.rules_index + 1 < rules_len {
            self.rules_index += 1;
            true
        } else {
            false
        }
    }

    pub fn start_rule_editing(&mut self, settings: &mut GameSettings) {
        let Some(field) = self.selected_rule_field(settings) else {
            return;
        };

        if field.is_bool() {
            field.adjust(settings, 1);
            self.clamp_to_settings(settings);
            return;
        }

        self.rules_editing = true;
        self.rules_edit_buffer.clear();
    }

    pub fn finish_rule_editing(&mut self) {
        self.rules_editing = false;
        self.rules_edit_buffer.clear();
    }
}

pub fn adjust_deck_count(settings: &mut GameSettings, rank: usize, delta: isize) {
    settings.deck[rank] = adjust_usize(settings.deck[rank], delta, 1, 0, 99);
}

pub fn set_deck_count_from_text(settings: &mut GameSettings, rank: usize, text: &str) {
    if text.is_empty() {
        return;
    }

    if let Ok(value) = text.parse::<usize>() {
        settings.deck[rank] = value.min(99);
    }
}

pub fn adjust_number_of_players(settings: &mut GameSettings, delta: isize) {
    settings.number_of_players =
        adjust_usize(settings.number_of_players, delta, 1, 2, consts::MAX_PLAYERS);
    settings.ensure_player_names();
}

pub fn set_number_of_players_from_text(settings: &mut GameSettings, text: &str) {
    if text.is_empty() {
        return;
    }

    if let Ok(value) = text.parse::<usize>() {
        settings.number_of_players = value.clamp(2, consts::MAX_PLAYERS);
        settings.ensure_player_names();
    }
}

fn enabled_disabled(value: bool) -> &'static str {
    if value {
        "Enabled"
    } else {
        "Disabled"
    }
}

fn adjust_usize(value: usize, delta: isize, step: usize, min: usize, max: usize) -> usize {
    if delta > 0 {
        value
            .saturating_add(step.saturating_mul(delta as usize))
            .min(max)
    } else if delta < 0 {
        value
            .saturating_sub(step.saturating_mul((-delta) as usize))
            .max(min)
    } else {
        value
    }
}

fn adjust_f32(value: f32, delta: isize, step: f32, min: f32, max: f32) -> f32 {
    let value = value + delta as f32 * step;
    value.clamp(min, max)
}
