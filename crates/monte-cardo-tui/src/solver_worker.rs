use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use monte_cardo_core::{
    eval::{self, NaiveHeuristic, SearchConfig, SearchContext},
    game,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverPurpose {
    Suggestion,
    AiMove,
}

pub enum SolverRequest {
    EvaluateActions {
        request_id: u64,
        incomplete_information_state: game::IncompleteInformationGameState,
        config: SearchConfig,
        seed: u64,
    },
    Shutdown,
}

pub enum SolverResponse {
    ActionValues {
        request_id: u64,
        values: Result<Vec<(game::Move, f32)>, String>,
    },
}

pub struct SolverClient {
    request_tx: Sender<SolverRequest>,
    response_rx: Receiver<SolverResponse>,
    next_request_id: u64,
    worker: Option<JoinHandle<()>>,
}

impl SolverClient {
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<SolverRequest>();
        let (response_tx, response_rx) = mpsc::channel::<SolverResponse>();

        let worker = thread::spawn(move || {
            let mut heuristic = NaiveHeuristic::new();

            while let Ok(request) = request_rx.recv() {
                match request {
                    SolverRequest::EvaluateActions {
                        request_id,
                        incomplete_information_state,
                        config,
                        seed,
                    } => {
                        let mut search_context =
                            SearchContext::with_seed(&mut heuristic, config, seed);

                        let values = eval::get_action_values(
                            &incomplete_information_state,
                            &mut search_context,
                        )
                        .map_err(|err| format!("{:?}", err));

                        let _ =
                            response_tx.send(SolverResponse::ActionValues { request_id, values });
                    }
                    SolverRequest::Shutdown => break,
                }
            }
        });

        Self {
            request_tx,
            response_rx,
            next_request_id: 0,
            worker: Some(worker),
        }
    }

    pub fn request_action_values(
        &mut self,
        incomplete_information_state: game::IncompleteInformationGameState,
        config: SearchConfig,
        seed: u64,
    ) -> u64 {
        let request_id = self.next_request_id;
        self.next_request_id += 1;

        let _ = self.request_tx.send(SolverRequest::EvaluateActions {
            request_id,
            incomplete_information_state,
            config,
            seed,
        });

        request_id
    }

    pub fn drain_responses(&mut self) -> Vec<SolverResponse> {
        let mut responses = Vec::new();

        while let Ok(response) = self.response_rx.try_recv() {
            responses.push(response);
        }

        responses
    }
}

impl Drop for SolverClient {
    fn drop(&mut self) {
        let _ = self.request_tx.send(SolverRequest::Shutdown);

        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
