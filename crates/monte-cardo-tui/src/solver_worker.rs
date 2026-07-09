use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use monte_cardo_core::{
    eval::{self, NaiveHeuristic, SearchConfig, SearchContext, SimpleHeuristic},
    game,
};

use crate::settings::SolverHeuristic;

#[derive(Debug, Clone, PartialEq)]
struct CachedSearchContextKey {
    heuristic_type: SolverHeuristic,
    config: SearchConfig,
    seed: u64,
}

enum CachedSearchContext<'a> {
    Naive {
        key: CachedSearchContextKey,
        context: SearchContext<'a, NaiveHeuristic>,
    },
    Simple {
        key: CachedSearchContextKey,
        context: SearchContext<'a, SimpleHeuristic>,
    },
}

impl<'a> CachedSearchContext<'a> {
    fn key(&self) -> &CachedSearchContextKey {
        match self {
            CachedSearchContext::Naive { key, .. } | CachedSearchContext::Simple { key, .. } => key,
        }
    }
}

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
        heuristic_type: SolverHeuristic,
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
            let mut naive_heuristic = NaiveHeuristic::new();
            let mut simple_heuristic = SimpleHeuristic::default();

            let mut cached_context: Option<CachedSearchContext<'_>> = None;

            while let Ok(request) = request_rx.recv() {
                match request {
                    SolverRequest::EvaluateActions {
                        request_id,
                        incomplete_information_state,
                        config,
                        heuristic_type,
                        seed,
                    } => {
                        let requested_key = CachedSearchContextKey {
                            heuristic_type,
                            config: config.clone(),
                            seed,
                        };

                        let should_rebuild_context = cached_context
                            .as_ref()
                            .map_or(true, |cached| cached.key() != &requested_key);

                        if should_rebuild_context {
                            // Drop the old context first so it releases its mutable borrow
                            // of the old heuristic before we rebuild.
                            drop(cached_context.take());

                            cached_context = Some(match heuristic_type {
                                SolverHeuristic::Naive => {
                                    naive_heuristic = NaiveHeuristic::new();

                                    let context = SearchContext::with_seed(
                                        &mut naive_heuristic,
                                        config,
                                        seed,
                                    );

                                    CachedSearchContext::Naive {
                                        key: requested_key,
                                        context,
                                    }
                                }

                                SolverHeuristic::Simple => {
                                    simple_heuristic = SimpleHeuristic::default();

                                    let context = SearchContext::with_seed(
                                        &mut simple_heuristic,
                                        config,
                                        seed,
                                    );

                                    CachedSearchContext::Simple {
                                        key: requested_key,
                                        context,
                                    }
                                }
                            });
                        }

                        let values = match cached_context.as_mut().expect("context should exist") {
                            CachedSearchContext::Naive { context, .. } => {
                                eval::get_action_values(&incomplete_information_state, context)
                            }

                            CachedSearchContext::Simple { context, .. } => {
                                eval::get_action_values(&incomplete_information_state, context)
                            }
                        }
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
        heuristic_type: SolverHeuristic,
        seed: u64,
    ) -> u64 {
        let request_id = self.next_request_id;
        self.next_request_id += 1;

        let _ = self.request_tx.send(SolverRequest::EvaluateActions {
            request_id,
            incomplete_information_state,
            config,
            heuristic_type,
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
