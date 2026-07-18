mod app;
mod cards;
mod game;
mod hand;
mod input;
mod live_widgets;
mod main_menu_widgets;
mod players;
mod rank_count_editor;
mod settings;
mod settings_widgets;
mod table;
mod view_model;

#[cfg(not(target_arch = "wasm32"))]
pub mod solver_worker;

pub use app::App;
pub use input::AppKey;
