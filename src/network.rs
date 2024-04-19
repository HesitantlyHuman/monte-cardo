// There are a total of 16 * 16 + 16 = 272 inputs to the network
const NETWORK_INPUTS_SIZE: usize = MAX_CARD_ORDINALITY * MAX_CARD_NUMBER + MAX_PLAYERS;

// There are two components to the network inputs:
// 1. Encoding of the state of the cards. Cards which you have in your hand are encoded as 127, cards
// which have been played are encoded as 0, cards which still remain in the deck are encoded as -127
// The first dimension is the ordinality of the card, and the second dimension is each card themselves.
// Since order does not matter, cards in the hand are placed first, then cards which remain in the deck, and finally
// the cards which are not in play for that trick.
// 2. Proportion of the deck each player has, normalized between -127 and 127
type NetworkInputs = [i8; NETWORK_INPUTS_SIZE];

const NETWORK_HIDDEN_LAYER_SIZE: usize = 32;

struct NetworkParameters(
    [i8; NETWORK_INPUTS_SIZE // Input biases
        + NETWORK_INPUTS_SIZE * NETWORK_HIDDEN_LAYER_SIZE // Input layer weights
        + NETWORK_HIDDEN_LAYER_SIZE // Input layer biases
        + NETWORK_HIDDEN_LAYER_SIZE // Output layer weights
        + 1], // Output layer bias
);

type NetworkGradients = [f32; NETWORK_INPUTS_SIZE
    + NETWORK_INPUTS_SIZE * NETWORK_HIDDEN_LAYER_SIZE
    + NETWORK_HIDDEN_LAYER_SIZE
    + NETWORK_HIDDEN_LAYER_SIZE
    + 1];

type NetworkGradientMoments = [f32; NETWORK_INPUTS_SIZE
    + NETWORK_INPUTS_SIZE * NETWORK_HIDDEN_LAYER_SIZE
    + NETWORK_HIDDEN_LAYER_SIZE
    + NETWORK_HIDDEN_LAYER_SIZE
    + 1];

impl NetworkParameters {
    fn he_initialization() -> Result<Self, rand_distr::NormalError> {
        let output_layer_bias = 32;
        let input_biases = [1; NETWORK_INPUTS_SIZE];
        let input_layer_biases = [1; NETWORK_HIDDEN_LAYER_SIZE];

        let mut rng = rand::thread_rng();

        // Now randomly initialize the weights with N(0, 2/n) where n is the number of inputs to the node
        let input_layer_normal = Normal::new(0.0, (2.0 / NETWORK_INPUTS_SIZE as f64) * 255.0)?;
        let input_layer_float_weights: Vec<f64> = input_layer_normal
            .sample_iter(&mut rng)
            .take(NETWORK_INPUTS_SIZE * NETWORK_HIDDEN_LAYER_SIZE)
            .collect();
        let input_layer_quantized_weights = input_layer_float_weights
            .iter()
            .map(|x| (x * 127.0) as i8)
            .collect::<Vec<i8>>();
        let mut input_layer_weights = [0; NETWORK_INPUTS_SIZE * NETWORK_HIDDEN_LAYER_SIZE];
        for i in 0..NETWORK_INPUTS_SIZE * NETWORK_HIDDEN_LAYER_SIZE {
            input_layer_weights[i] = input_layer_quantized_weights[i];
        }

        let output_layer_normal =
            Normal::new(0.0, (2.0 / NETWORK_HIDDEN_LAYER_SIZE as f64) * 255.0)?;
        let output_layer_float_weights: Vec<f64> = output_layer_normal
            .sample_iter(&mut rng)
            .take(NETWORK_HIDDEN_LAYER_SIZE)
            .collect();
        let output_layer_quantized_weights = output_layer_float_weights
            .iter()
            .map(|x| (x * 127.0) as i8)
            .collect::<Vec<i8>>();
        let mut output_layer_weights = [0; NETWORK_HIDDEN_LAYER_SIZE];
        for i in 0..NETWORK_HIDDEN_LAYER_SIZE {
            output_layer_weights[i] = output_layer_quantized_weights[i];
        }

        let mut parameters = [0; NETWORK_INPUTS_SIZE
            + NETWORK_INPUTS_SIZE * NETWORK_HIDDEN_LAYER_SIZE
            + NETWORK_HIDDEN_LAYER_SIZE
            + NETWORK_HIDDEN_LAYER_SIZE
            + 1];
        let mut index = 0;
        parameters[index..index + NETWORK_INPUTS_SIZE].copy_from_slice(&input_biases);
        index += NETWORK_INPUTS_SIZE;
        parameters[index..index + NETWORK_INPUTS_SIZE * NETWORK_HIDDEN_LAYER_SIZE]
            .copy_from_slice(&input_layer_weights);
        index += NETWORK_INPUTS_SIZE * NETWORK_HIDDEN_LAYER_SIZE;
        parameters[index..index + NETWORK_HIDDEN_LAYER_SIZE].copy_from_slice(&input_layer_biases);
        index += NETWORK_HIDDEN_LAYER_SIZE;
        parameters[index..index + NETWORK_HIDDEN_LAYER_SIZE].copy_from_slice(&output_layer_weights);
        index += NETWORK_HIDDEN_LAYER_SIZE;
        parameters[index] = output_layer_bias;

        Ok(NetworkParameters(parameters))
    }

    fn save_to_file(&self, mut file: &File) -> Result<(), std::io::Error> {
        let i8_slice = self.0.as_slice();
        let u8_slice = unsafe { &*(i8_slice as *const [i8] as *const [u8]) };
        file.write_all(&u8_slice)?;
        Ok(())
    }

    fn load_from_file(file: &mut File) -> Result<Self, std::io::Error> {
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let mut i8_buffer = [0; NETWORK_INPUTS_SIZE
            + NETWORK_INPUTS_SIZE * NETWORK_HIDDEN_LAYER_SIZE
            + NETWORK_HIDDEN_LAYER_SIZE
            + NETWORK_HIDDEN_LAYER_SIZE
            + 1];
        let u8_buffer = unsafe { &*(buffer.as_slice() as *const [u8] as *const [i8]) };
        i8_buffer.copy_from_slice(u8_buffer);
        Ok(NetworkParameters(i8_buffer))
    }

    fn inference(&self, inputs: &NetworkInputs) -> f32 {
        let mut inputs_after_biases = [0; NETWORK_INPUTS_SIZE];
        for i in 0..NETWORK_INPUTS_SIZE {
            let after_bias = inputs[i] as i32 + self.0[i] as i32;
            let clamped_after_bias: i8 = match after_bias {
                x if x > 127 => 127,
                x if x < -127 => -127,
                _ => after_bias as i8,
            };
            inputs_after_biases[i] = clamped_after_bias;
        }

        // First layer matrix multiplication
        let mut hidden_layer_outputs = [0; NETWORK_HIDDEN_LAYER_SIZE];
        for i in 0..NETWORK_HIDDEN_LAYER_SIZE {
            let mut sum: i32 = 0;
            for j in 0..NETWORK_INPUTS_SIZE {
                sum += inputs_after_biases[j] as i32
                    * self.0[NETWORK_INPUTS_SIZE + i * NETWORK_INPUTS_SIZE + j] as i32;
            }
            sum = sum / 127;
            sum = sum + self.0[NETWORK_INPUTS_SIZE * (1 + NETWORK_HIDDEN_LAYER_SIZE) + i] as i32;

            let clamped_sum: i8 = match sum {
                x if x > 127 * 127 => 127,
                x if x < -127 * 127 => -127,
                _ => (sum / 127) as i8,
            };
            hidden_layer_outputs[i] = clamped_sum;
        }
        println!("{:?}", hidden_layer_outputs);

        // First layer activation (ReLU)
        for i in 0..NETWORK_HIDDEN_LAYER_SIZE {
            if hidden_layer_outputs[i] < 0 {
                hidden_layer_outputs[i] = 0;
            }
        }
        println!("{:?}", hidden_layer_outputs);

        let mut output = 0;
        for i in 0..NETWORK_HIDDEN_LAYER_SIZE {
            output += hidden_layer_outputs[i] as i32
                * self.0[NETWORK_INPUTS_SIZE * (1 + NETWORK_HIDDEN_LAYER_SIZE) + i] as i32;
        }
        println!("{:?}", output);
        output = output / 127;
        output = output
            + self.0
                [NETWORK_INPUTS_SIZE * (1 + NETWORK_HIDDEN_LAYER_SIZE) + NETWORK_HIDDEN_LAYER_SIZE]
                as i32;

        let clamped_output: i8 = match output {
            x if x > 127 * 127 => 127,
            x if x < -127 * 127 => -127,
            _ => (output / 127) as i8,
        };

        println!("{}", clamped_output);

        // Now sigmoid activation
        let denominator = (8 + clamped_output.abs()) as f32;
        let output = 0.5 * (clamped_output as f32 / denominator) + 0.5;
        output
    }
}

// How do we give the network a sense of the number of cards each opponent has?
// Is it just a proportion of the remaining deck?
fn prepare_game_state_for_heuristic_network(
    game_state: IncompleteInformationGameState,
    player_index: usize,
) -> NetworkInputs {
    // First, prepare the card inputs
    // We need to remap the cards so that any missing ordinalities are excluded, except for the
    // jesters, which have special abilities.
    let (hand, in_play, hand_sizes) = game_state;

    let mut card_inputs = [0; MAX_CARD_ORDINALITY * MAX_CARD_NUMBER];
    let mut total_cards_in_play = 0;

    // First, the jesters
    let mut offset = 0;
    for _ in 0..hand[0] {
        card_inputs[offset] = 127;
        offset += 1;
        total_cards_in_play += 1;
    }
    for _ in 0..in_play[0] {
        card_inputs[offset] = -127;
        offset += 1;
        total_cards_in_play += 1;
    }

    // Then, the rest of the cards
    let mut current_ordinality = 1;
    for i in 1..MAX_CARD_ORDINALITY {
        let mut is_empty = true;
        let mut offset = 0;
        for _ in 0..hand[i] {
            card_inputs[current_ordinality * MAX_CARD_NUMBER + offset] = 127;
            offset += 1;
            total_cards_in_play += 1;
            is_empty = false;
        }
        for _ in 0..in_play[i] {
            card_inputs[current_ordinality * MAX_CARD_NUMBER + offset] = -127;
            offset += 1;
            total_cards_in_play += 1;
            is_empty = false;
        }
        if !is_empty {
            current_ordinality += 1;
        }
    }

    // Now, prepare the player card proportions
    // We want to order them starting with the current player
    let mut player_proportions = [0.0; MAX_PLAYERS];
    let mut current_proportion_player_index = 0;
    for i in player_index..MAX_PLAYERS {
        player_proportions[current_proportion_player_index] =
            hand_sizes[i] as f32 / total_cards_in_play as f32;
        current_proportion_player_index += 1;
    }
    for i in 0..player_index {
        player_proportions[current_proportion_player_index] =
            hand_sizes[i] as f32 / total_cards_in_play as f32;
        current_proportion_player_index += 1;
    }
    let mut quantized_player_proportions = [0; MAX_PLAYERS];
    for i in 0..MAX_PLAYERS {
        quantized_player_proportions[i] = ((player_proportions[i] * 255.0) - 127.0) as i8;
    }

    // Now, write the flattened inputs
    let mut inputs = [0; NETWORK_INPUTS_SIZE];
    for i in 0..MAX_CARD_ORDINALITY * MAX_CARD_NUMBER {
        inputs[i] = card_inputs[i];
    }
    for i in 0..MAX_PLAYERS {
        inputs[MAX_CARD_ORDINALITY * MAX_CARD_NUMBER + i] = quantized_player_proportions[i];
    }
    inputs
}
