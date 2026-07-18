use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub enum SolverResponse {
    ActionValues {
        request_id: u64,
        values: Result<Vec<(game::Move, f32)>, String>,
    },
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::SolverClient;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        sync::mpsc::{self, Receiver, Sender},
        thread::{self, JoinHandle},
    };

    use super::*;

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
                #[allow(unused_assignments)]
                let mut naive_heuristic = NaiveHeuristic::new();
                #[allow(unused_assignments)]
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

                            let values = match cached_context
                                .as_mut()
                                .expect("context should exist")
                            {
                                CachedSearchContext::Naive { context, .. } => {
                                    eval::get_action_values(&incomplete_information_state, context)
                                }

                                CachedSearchContext::Simple { context, .. } => {
                                    eval::get_action_values(&incomplete_information_state, context)
                                }
                            }
                            .map_err(|err| format!("{:?}", err));

                            let _ = response_tx
                                .send(SolverResponse::ActionValues { request_id, values });
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
}

#[cfg(target_arch = "wasm32")]
pub use web::{run_web_worker, SolverClient};

#[cfg(target_arch = "wasm32")]
mod web {
    use std::{
        cell::{Cell, RefCell},
        collections::VecDeque,
        rc::Rc,
    };

    use futures_util::StreamExt;
    use wasm_bindgen::{closure::Closure, JsCast};
    use wasm_bindgen_futures::spawn_local;
    use web_sys::{DedicatedWorkerGlobalScope, ErrorEvent, MessageEvent, Worker};

    use super::*;

    pub struct SolverClient {
        worker: Worker,
        responses: Rc<RefCell<VecDeque<SolverResponse>>>,
        pending_request_id: Rc<Cell<Option<u64>>>,
        next_request_id: u64,

        // These closures must remain alive while the worker is alive.
        _onmessage: Closure<dyn FnMut(MessageEvent)>,
        _onerror: Closure<dyn FnMut(ErrorEvent)>,
    }

    impl SolverClient {
        pub fn new() -> Self {
            let worker = Worker::new("./solver_worker_loader.js")
                .expect("failed to create solver Web Worker");

            let responses = Rc::new(RefCell::new(VecDeque::<SolverResponse>::new()));

            let pending_request_id = Rc::new(Cell::new(None::<u64>));

            let onmessage = {
                let responses = Rc::clone(&responses);
                let pending_request_id = Rc::clone(&pending_request_id);

                Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                    let response = serde_wasm_bindgen::from_value::<SolverResponse>(event.data());

                    match response {
                        Ok(response) => {
                            pending_request_id.set(None);
                            responses.borrow_mut().push_back(response);
                        }

                        Err(error) => {
                            if let Some(request_id) = pending_request_id.take() {
                                responses
                                    .borrow_mut()
                                    .push_back(SolverResponse::ActionValues {
                                        request_id,
                                        values: Err(format!(
                                            "Could not decode solver response: \
                                                 {error}"
                                        )),
                                    });
                            }
                        }
                    }
                })
            };

            worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

            let onerror = {
                let responses = Rc::clone(&responses);
                let pending_request_id = Rc::clone(&pending_request_id);

                Closure::<dyn FnMut(ErrorEvent)>::new(move |event: ErrorEvent| {
                    let Some(request_id) = pending_request_id.take() else {
                        return;
                    };

                    responses
                        .borrow_mut()
                        .push_back(SolverResponse::ActionValues {
                            request_id,
                            values: Err(format!("Solver worker failed: {}", event.message(),)),
                        });
                })
            };

            worker.set_onerror(Some(onerror.as_ref().unchecked_ref()));

            Self {
                worker,
                responses,
                pending_request_id,
                next_request_id: 0,
                _onmessage: onmessage,
                _onerror: onerror,
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

            let request = SolverRequest::EvaluateActions {
                request_id,
                incomplete_information_state,
                config,
                heuristic_type,
                seed,
            };

            let send_result = serde_wasm_bindgen::to_value(&request)
                .map_err(|error| format!("Could not encode solver request: {error}"))
                .and_then(|value| {
                    self.worker
                        .post_message(&value)
                        .map_err(|error| format!("Could not send solver request: {error:?}"))
                });

            match send_result {
                Ok(()) => {
                    self.pending_request_id.set(Some(request_id));
                }

                Err(error) => {
                    self.responses
                        .borrow_mut()
                        .push_back(SolverResponse::ActionValues {
                            request_id,
                            values: Err(error),
                        });
                }
            }

            request_id
        }

        pub fn drain_responses(&mut self) -> Vec<SolverResponse> {
            self.responses.borrow_mut().drain(..).collect()
        }
    }

    impl Drop for SolverClient {
        fn drop(&mut self) {
            self.worker.terminate();
        }
    }

    pub fn run_web_worker() {
        console_error_panic_hook::set_once();

        let scope: DedicatedWorkerGlobalScope = js_sys::global().unchecked_into();

        let (request_tx, mut request_rx) = futures_channel::mpsc::unbounded::<SolverRequest>();

        let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let Ok(request) = serde_wasm_bindgen::from_value::<SolverRequest>(event.data()) else {
                return;
            };

            let _ = request_tx.unbounded_send(request);
        });

        scope.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

        // The worker lives for the page lifetime.
        onmessage.forget();

        let response_scope = scope.clone();

        spawn_local(async move {
            #[allow(unused_assignments)]
            let mut naive_heuristic = NaiveHeuristic::new();

            #[allow(unused_assignments)]
            let mut simple_heuristic = SimpleHeuristic::default();

            let mut cached_context: Option<CachedSearchContext<'_>> = None;

            while let Some(request) = request_rx.next().await {
                let response = match request {
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

                        let values = match cached_context
                            .as_mut()
                            .expect("solver context should exist")
                        {
                            CachedSearchContext::Naive { context, .. } => {
                                eval::get_action_values(&incomplete_information_state, context)
                            }

                            CachedSearchContext::Simple { context, .. } => {
                                eval::get_action_values(&incomplete_information_state, context)
                            }
                        }
                        .map_err(|error| format!("{error:?}"));

                        Some(SolverResponse::ActionValues { request_id, values })
                    }

                    SolverRequest::Shutdown => None,
                };

                let Some(response) = response else {
                    break;
                };

                let Ok(value) = serde_wasm_bindgen::to_value(&response) else {
                    continue;
                };

                let _ = response_scope.post_message(&value);
            }
        });
    }
}
