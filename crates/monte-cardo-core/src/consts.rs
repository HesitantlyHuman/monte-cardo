// Lets have the max number of players be 16. And the minimum is obviously 2.
pub const MAX_PLAYERS: usize = 16;

// Maximum different ranks the cards can have
pub const MAX_CARD_ORDINALITY: usize = 18;

// Maximum number of cards of a same rank the deck can have
pub const MAX_CARD_NUMBER: usize = 18;

// Default deck for "The Great Dalmuti"
// The first position is always wilds, or "jesters"
pub const DEFAULT_DALMUTI_DECK: [usize; 18] =
    [2, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0, 0, 0, 0, 0];

// Default deck for "Scum"
pub const DEFAULT_SCUM_DECK: [usize; 18] = [2, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 0, 0, 0, 0];
