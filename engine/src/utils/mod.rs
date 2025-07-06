use std::{
    any::TypeId,
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex, atomic::AtomicBool},
    thread::{JoinHandle, spawn},
    time::{Duration, Instant},
};

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RegisterKey {
    pub type_id: TypeId,
    pub label: &'static str,
}

impl RegisterKey {
    pub fn from_label<T: 'static>(label: &'static str) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            label,
        }
    }
}

#[derive(Debug)]
pub struct Registry<T> {
    keys: Vec<RegisterKey>,
    registry: Vec<T>,
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            registry: Vec::new(),
        }
    }
}

impl<T: Send + Sync> Registry<T> {
    pub fn register_key(&mut self, key: RegisterKey, value: T) {
        if self.keys.contains(&key) {
            return;
        }
        self.keys.push(key);
        self.registry.push(value);
    }

    #[inline(always)]
    pub fn get(&self, key: &RegisterKey) -> Option<&T> {
        self.keys
            .iter()
            .position(|k| k == key)
            .map(|index| &self.registry[index])
    }

    #[inline(always)]
    pub fn get_mut(&mut self, key: &RegisterKey) -> Option<&mut T> {
        self.keys
            .iter()
            .position(|k| k == key)
            .map(|index| &mut self.registry[index])
    }

    pub fn keys(&self) -> impl Iterator<Item = &RegisterKey> {
        self.keys.iter()
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.registry.iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.registry.iter_mut()
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<JoinHandle<()>>,
    job_queue: Arc<(Mutex<VecDeque<Job>>, Condvar)>,
    is_running: Arc<AtomicBool>,
}

impl ThreadPool {
    pub fn new(num_threads: usize) -> Self {
        let job_queue = Arc::new((Mutex::new(VecDeque::<Job>::new()), Condvar::new()));
        let is_running = Arc::new(AtomicBool::new(true));
        let mut workers = Vec::new();

        for _ in 0..num_threads {
            let queue = Arc::clone(&job_queue);
            let running = Arc::clone(&is_running);

            let handle = spawn(move || {
                while running.load(std::sync::atomic::Ordering::Acquire) {
                    let job = {
                        let (lock, cvar) = &*queue;
                        let mut queue = lock.lock().unwrap();

                        while queue.is_empty() {
                            queue = cvar.wait(queue).unwrap();

                            if !running.load(std::sync::atomic::Ordering::Acquire) {
                                return;
                            }
                        }
                        queue.pop_front()
                    };

                    if let Some(job) = job {
                        job();
                    }
                }
            });
            workers.push(handle);
        }

        Self {
            workers,
            job_queue,
            is_running,
        }
    }

    pub fn submit<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let (lock, cvar) = &*self.job_queue;
        let mut queue = lock.lock().unwrap();
        queue.push_back(Box::new(job));
        cvar.notify_one();
    }

    pub fn shutdown(self) {
        self.is_running
            .store(false, std::sync::atomic::Ordering::Release);
        let (lock, cvar) = &*self.job_queue;
        cvar.notify_all();

        for handle in self.workers {
            let _ = handle.join();
        }
    }
}
