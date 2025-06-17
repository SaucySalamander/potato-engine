use std::time::{Duration, Instant};

use log::info;

#[derive(Debug)]
pub struct FPSCounter {
    last_instant: Instant,
    frame_count: u32,
}

impl Default for FPSCounter {
    fn default() -> Self {
        Self {
            last_instant: Instant::now(),
            frame_count: 0,
        }
    }
}

impl FPSCounter {
    pub fn tick(&mut self) {
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_instant);

        if elapsed >= Duration::from_secs(1) {
            let fps = self.frame_count as f64 / elapsed.as_secs_f64();

            info!("FPS: {:.2}", fps);

            self.frame_count = 0;
            self.last_instant = now;
        }
    }
}

pub trait Registry<K, V> {
    fn insert(&mut self, key: K, value: V);
    fn get(&self, key: &K) -> Option<&V>;
    fn get_mut(&mut self, key: &K) -> Option<&mut V>;
}