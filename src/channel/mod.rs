#![doc = include_str!("README.md")]

use crate::ring_buffer::RingBuffer;
use std::sync::{Arc, Condvar, Mutex};
/// The shared state between the Sender and Receiver.
struct Shared<T> {
    buffer: RingBuffer<T>,
    signal: Condvar,
    // The Mutex is required by Condvar. We use a () as a "dummy"
    // payload because the data itself is protected by the RingBuffer's atomics.
    lock: Mutex<()>,
}

/// The sending half of the SPSC channel.
pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

/// The receiving half of the SPSC channel.
pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
}

/// Creates a new SPSC channel with the given capacity.
///
/// Capacity will be rounded up to the next power of 2.
pub fn channel<T: Send>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared {
        buffer: RingBuffer::new(capacity),
        signal: Condvar::new(),
        lock: Mutex::new(()),
    });

    (
        Sender {
            shared: shared.clone(),
        },
        Receiver { shared },
    )
}

// --- Sender Implementation ---

impl<T> Sender<T> {
    /// Attempts to send an item immediately without blocking.
    ///
    /// If the channel is full, this returns `Err(item)`.
    pub fn try_send(&self, item: T) -> Result<(), T> {
        match self.shared.buffer.send(item) {
            Ok(_) => {
                // Wake up the receiver, in case it's sleeping.
                self.shared.signal.notify_one();
                Ok(())
            }
            Err(item) => Err(item),
        }
    }

    /// Sends an item, blocking the current thread if the channel is full.
    pub fn send(&self, mut item: T) {
        // 1. Fast Path: Try a lock-free send.
        match self.shared.buffer.send(item) {
            Ok(_) => {
                // Success! Notify the receiver and return.
                self.shared.signal.notify_one();
                return;
            }
            Err(returned_item) => {
                // Buffer is full, save the item and prepare to block.
                item = returned_item;
            }
        }

        // 2. Slow Path: The buffer is full. We must wait.
        let mut guard = self.shared.lock.lock().unwrap();
        loop {
            // Try again inside the lock (in case another thread
            // woke us up but we were too slow).
            match self.shared.buffer.send(item) {
                Ok(_) => {
                    self.shared.signal.notify_one();
                    return;
                }
                Err(returned_item) => {
                    item = returned_item;
                    // Still full. Go to sleep.
                    // `wait` atomically releases the lock and blocks.
                    // When it wakes up, it re-acquires the lock.
                    guard = self.shared.signal.wait(guard).unwrap();
                }
            }
        }
    }
}

// --- Receiver Implementation ---

impl<T> Receiver<T> {
    /// Attempts to receive an item immediately without blocking.
    ///
    /// If the channel is empty, this returns `None`.
    pub fn try_recv(&self) -> Option<T> {
        match self.shared.buffer.recv() {
            Some(item) => {
                // Notify the producer that space has opened up.
                self.shared.signal.notify_one();
                Some(item)
            }
            None => None,
        }
    }

    /// Receives an item, blocking the current thread if the channel is empty.
    ///
    /// Returns `None` if the `Sender` has been dropped.
    pub fn recv(&self) -> Option<T> {
        // 1. Fast Path: Try a lock-free receive.
        if let Some(item) = self.shared.buffer.recv() {
            self.shared.signal.notify_one();
            return Some(item);
        }

        // 2. Slow Path: The buffer is empty. We must wait.
        let mut guard = self.shared.lock.lock().unwrap();
        loop {
            match self.shared.buffer.recv() {
                Some(item) => {
                    self.shared.signal.notify_one();
                    return Some(item);
                }
                None => {
                    // Check for disconnection. If we are the *only*
                    // Arc owner left, the Sender must be gone.
                    if Arc::strong_count(&self.shared) == 1 {
                        return None;
                    }
                    // Still empty. Wait for a signal.
                    guard = self.shared.signal.wait(guard).unwrap();
                }
            }
        }
    }

    // You could also add `recv_timeout` here as a further exercise!
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        // When the sender drops, we must wake up any
        // sleeping receiver so it can check for disconnection.
        self.shared.signal.notify_one();
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread, time::Duration};

    #[test]
    fn test_blocking_send_recv() {
        let (tx, rx) = channel(1); // Capacity of 1

        // Send one item, should be fine.
        tx.send("hello");

        // Spawn a producer that will block
        let tx_clone = tx.shared.clone(); // Use Arc for test
        let _producer = thread::spawn(move || {
            let sender = Sender { shared: tx_clone };
            sender.send("world");
            // This thread is now blocked
        });

        // Wait a moment
        thread::sleep(Duration::from_millis(50));

        // Now, receive an item, which should unblock the producer
        assert_eq!(rx.recv(), Some("hello"));
        assert_eq!(rx.recv(), Some("world"));
    }

    #[test]
    fn test_blocking_recv() {
        let (tx, rx) = channel(4);

        // Spawn a producer that sends after a delay
        let producer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            tx.send(42);
        });

        // This `recv` call should block for ~100ms
        let start = std::time::Instant::now();
        let item = rx.recv();
        let duration = start.elapsed();

        assert_eq!(item, Some(42));
        assert!(duration.as_millis() >= 90, "Did not block");

        producer.join().unwrap();
    }

    #[test]
    fn test_disconnection() {
        let (tx, rx) = channel(4);
        tx.send(1);
        tx.send(2);

        // Drop the sender
        drop(tx);

        // Receiver should drain the buffer
        assert_eq!(rx.recv(), Some(1));
        assert_eq!(rx.recv(), Some(2));

        // Now that the buffer is empty and sender is gone,
        // recv() should return None.
        assert_eq!(rx.recv(), None);
    }
}
