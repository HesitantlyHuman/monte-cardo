fn main() {
    #[cfg(target_arch = "wasm32")]
    monte_cardo_tui::solver_worker::run_web_worker();
}
