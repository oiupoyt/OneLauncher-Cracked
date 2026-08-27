// TODO Remove this once freya has some workaround for this
#![allow(float_literal_f32_fallback)]
// Instrumented oneclient_core async call chains exceed the default limit
#![recursion_limit = "256"]

mod assets;
mod components;
pub mod events;
pub mod hooks;
mod install;
mod launcher;
mod layout;
mod motion;
mod notifications;
pub mod platform;
pub mod recovery;
mod routes;
pub mod state;
pub mod theme;
mod transfer;
mod ui;
pub mod updater;
pub(crate) mod utils;
mod view;

pub mod constants;

pub use assets::AppAssets;
pub use components::ConfirmLinkOverlay;
pub use events::EventPump;
pub use hooks::*;
pub use routes::{Route, router};
pub use state::{AppChannel, AppState};
