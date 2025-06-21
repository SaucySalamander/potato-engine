use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex, atomic::AtomicBool},
    thread::{JoinHandle, spawn},
};

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct FrameIndex {
    current: usize,
    count: usize,
}

impl FrameIndex {
    pub fn new(count: usize) -> Self {
        Self { current: 0, count }
    }

    pub fn index(&self) -> usize {
        self.current
    }

    pub fn advance(&mut self) {
        self.current = (self.current + 1) % self.count;
    }
}

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
