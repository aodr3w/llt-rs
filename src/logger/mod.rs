#![doc = include_str!("README.md")]

use crate::affinity;
use crate::channel::{Sender, channel};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex}; // Added Mutex
use std::thread;

/// A handle to the non-blocking logger
/// This struct is cheap to clone and can be passed around the application
#[derive(Clone)]
pub struct Logger {
    // FIX: Wrap Sender in Arc<Mutex<>> to safely allow multiple producers (MPSC behavior)
    // on top of the underlying SPSC channel.
    sender: Arc<Mutex<Sender<String>>>,
    dropped_count: Arc<AtomicU64>,
}

impl Logger {
    /// Creates a new Logger and spawns a background worker thread.
    ///
    /// # Arguments
    /// * `capacity` - The size of the ring buffer (messages). Must be power of 2.
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = channel(capacity);
        let dropped = Arc::new(AtomicU64::new(0));

        // FIX: Removed unused variable `dropped_clone`

        // Spawn the dedicated logging thread
        thread::spawn(move || {
            // BEST EFFORT: Try to pin to the very last core
            // This is usually an efficient E-core or a core far from Core 0/1.
            // We ignore the result (using `let _`) so this doesn't crash on macOS.
            if let Some(last_core) = affinity::get_core_ids().last() {
                let _ = affinity::pin_to_core(*last_core);
            }

            while let Some(msg) = rx.recv() {
                println!("[LOG] {}", msg);
            }
        });

        Self {
            // Wrap the raw SPSC sender in a Mutex + Arc for thread-safe sharing
            sender: Arc::new(Mutex::new(tx)),
            dropped_count: dropped,
        }
    }

    /// Logs a message
    ///
    /// This method is **Wait-Free** (mostly). It acquires a lightweight lock to ensure
    /// MPSC safety, then pushes to the queue.
    /// If the logging buffer is full, the message is silently dropped
    /// and the internal `dropped_count` is incremented.
    pub fn log(&self, msg: impl Into<String>) {
        // FIX: Acquire the lock to safely access the SPSC sender
        if let Ok(guard) = self.sender.lock() {
            // We use `try_send` to ensure we NEVER block on the queue itself.
            if guard.try_send(msg.into()).is_err() {
                // Drop the message to preserve latency
                // Increment counter so we know we are losing data
                self.dropped_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Returns the number of messages dropped due to a full buffer.
    pub fn get_dropped_count(&self) -> u64 {
        self.dropped_count.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_logger_basic() {
        let logger = Logger::new(16);

        logger.log("Hello World");
        logger.log("This is a test");

        // Give the background thread a moment to print
        thread::sleep(Duration::from_millis(50));

        assert_eq!(logger.get_dropped_count(), 0);
    }

    #[test]
    fn test_dropped_logs_under_load() {
        // Create a tiny buffer
        let logger = Logger::new(2);

        // Flood it faster than the consumer (println) can drain it.
        // String allocation + channel push is faster than console I/O.
        for i in 0..20 {
            logger.log(format!("Flood {}", i));
        }

        // We expect some drops because we sent 20 items into a buffer of 2
        // instantaneously.
        let dropped = logger.get_dropped_count();
        println!("Dropped {} messages (Expected > 0)", dropped);
        assert!(dropped > 0);
    }
}
