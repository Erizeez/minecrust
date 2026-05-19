use std::thread;
use crossbeam_channel::{unbounded, Sender};

type Job = Box<dyn FnOnce() + Send + 'static>;

/// A generalized background task pool for the engine
pub struct TaskPool {
    sender: Sender<Job>,
    workers: Vec<thread::JoinHandle<()>>,
}

impl TaskPool {
    /// Creates a new TaskPool with the specified number of worker threads.
    pub fn new(num_threads: usize) -> Self {
        let (sender, receiver) = unbounded::<Job>();
        
        let mut workers = Vec::with_capacity(num_threads);
        for i in 0..num_threads {
            let rx = receiver.clone();
            let handle = thread::Builder::new()
                .name(format!("EngineTaskWorker-{}", i))
                .spawn(move || {
                    while let Ok(job) = rx.recv() {
                        job();
                    }
                })
                .expect("Failed to spawn TaskWorker thread");
            workers.push(handle);
        }
        
        Self { sender, workers }
    }
    
    /// Spawns a new task into the pool.
    pub fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // If the channel is disconnected (which shouldn't happen unless dropped), we just ignore
        let _ = self.sender.send(Box::new(f));
    }
}

impl Default for TaskPool {
    fn default() -> Self {
        // Default to a reasonable number of background threads for a game (e.g., 4)
        Self::new(4)
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        // Drop the sender first so that all receivers know no more jobs are coming
        // However, we just rely on crossbeam_channel's drop semantics.
        // Actually, since sender is owned, when TaskPool drops, sender drops.
        // The threads will exit their recv() loop.
    }
}
