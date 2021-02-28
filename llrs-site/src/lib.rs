#![deny(unreachable_pub)]
#![recursion_limit = "512"]

mod agents;
mod app;
mod components;
mod pages;
mod route;

use std::str::FromStr;
use log::Level;
use wasm_bindgen::prelude::*;
use wasm_logger::Config;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// This is the entry point for the web app
#[wasm_bindgen]
pub fn run_app() -> Result<(), JsValue> {
    let config = Config::new(option_env!("RUST_LOG").map_or(Level::Info, |level_str| {
        Level::from_str(level_str).unwrap_or(Level::Info)
    }));
    wasm_logger::init(config);
    yew::start_app::<app::App>();
    Ok(())
}
