use std::process;

use log::{error, info};
use winit::event_loop::EventLoop;

use crate::engine::Engine;

mod engine;
mod engine_loop;
mod utils;
mod ecs;

fn main() {
    env_logger::init();

    info!("initializing event loop");
    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(err) => panic!("failed to start the event loop, {}", err),
    };

    info!("running Engine");
    let _ = event_loop.run_app(&mut Engine::default()).unwrap_or_else(|err| {
        error!("failed to run EngineState. {:?}", err);
        process::exit(1);
    });
}
