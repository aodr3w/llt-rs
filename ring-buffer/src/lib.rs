use crossbeam_utils::CachePadded;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A Single-Producer, Single-Consumer (SPSC) lock free ring buffer.
/// This queue is "wait-free" (bounded time) for both producer and consumer.
/// It does not block, but return `Err` or `None` if the queue is full or empty.
pub struct RingBuffer<T> {
    ///The buffer, allocated on the heap
    /// We use `UnsafeCell` for interior mutability (to write from `&self`).
    /// We use `MaybeUninit` to store uninitialized data and take ownership
    /// of `T's` when we `recv`
    buffer: Box<[UnsafeCell<MaybeUninit<T>>]>,

    /// The capacity of the buffer, Must be a power of 2 (why ?)
    cap: usize,

    /// The `head` counter.
    /// This is where the producer will write the *next* item.
    /// Only the producer modifies this.
    /// Padded to prevent false sharing with `tail`.
    head: CachePadded<AtomicUsize>,

    ///The "tail" counter
    /// This is where the consumer will read the *next* item from.
    /// Only the consumer modifies this
    /// Padded to prevent false sharing with `head`.
    tail: CachePadded<AtomicUsize>,
}

/// We can safely send the RingBuffer to other threads if T is Send
/// `Unsafe` is not `Sync` BUT WE *know* we are only accessing
/// the buffer safely from the *single* producer and *single* consumer.
/// The `head` and `tail` atomics prevent reading/writing the same slot.
unsafe impl<T: Send> Sync for RingBuffer<T> {}
unsafe impl<T: Send> Send for RingBuffer<T> {}

impl<T> RingBuffer<T> {
    /// Creates a new SPSC ring buffer with *at least* the given capacity
    /// The actual capacity will be rounded up to the next power of 2.
    pub fn new(capacity: usize) -> Self {
        // Round up to the next power of 2
        let cap = capacity.next_power_of_two();
        //Create a Vec and fill it with uninitialized data
        let mut buffer = Vec::with_capacity(cap);
        for _ in 0..cap {
            buffer.push(UnsafeCell::new(MaybeUninit::uninit()));
        }

        // Convert the Vec to a Box<[]>
        let buffer = buffer.into_boxed_slice();

        Self {
            buffer,
            cap,
            head: CachePadded::new(AtomicUsize::new(0)),
            tail: CachePadded::new(AtomicUsize::new(0)),
        }
    }

    /// Returns the capacity of the ring buffer.
    pub fn capacity(&self) -> usize {
        self.cap
    }

    ///Returns the number of items currently in the buffer.
    /// This is a snapshot and maybe out of date immediately.
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        head.wrapping_sub(tail)
    }
    ///Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// trues to send a item into a buffer
    ///
    /// Fails if the buffer is full, returning an `Err(item)`.
    /// This is the *Producer* method.
    pub fn send(&self, item: T) -> Result<(), T> {
        // Load the current head and tail.
        // `head` can be Relaxed because only *we* can change it.
        // `tail` must be `Acquire` to "see" the consumer's `Release` (or producer's release)
        // store, which signals that a slot has been freed.
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        //Check if the buffer is full
        // `wrapping_sub` handles counter wrap-around.
        if head.wrapping_sub(tail) == self.cap {
            return Err(item);
        }
        // Calculate the slot index using the power-of-2 bit-trick.
        // This is much faster than `head % self.cap`.
        let slot_idx = head & (self.cap - 1);

        // SAFETY:
        // 1. `&self` is_ok because  `UnsafeCell` provides interior mutability.
        // 2. The `is_full` check (head - tail == cap) guarantees that
        // this slot is "owned" by the producer. The consumer will
        // *never* read from this slot until we increment `head`.
        // 3. We are writing a `MaybeUninit::new(item)` which is valid.
        unsafe {
            let slot_ptr = self.buffer[slot_idx].get();
            (*slot_ptr).write(item);
        }
        // "Publish" the write.
        // We use `Release` to ensure that the data write (above)
        // is *not* reordered *after* this store. This makes the
        // data visible to the consumer's `Acquire` load.
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Ok(())
    }
    pub fn recv(&self) -> Option<T> {
        //Load the current head and tail.
        // `tail` can be Relaxed because only *we* change it.
        // `head` must be `Acquire` to "see" the producer's `Release`
        // store, which signals that data is available.
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        //Check if the buffer is empty
        if tail == head {
            return None;
        }

        let slot_idx = tail & (self.cap - 1);
        //Calculate the slot index.
        //SAFETY.
        //1. `&self` is ok because `UnsafeCell`.
        //2. The `is_empty` check (head == tail) guarantees that
        // this slot *contains data* put there by the producer.
        //3. The producer's `Release` store (on `head`) ensures that
        // the data write is visible *before*  we saw the new `head`.
        // 4 . `assume_init_read` takes ownership of the `T`, leaving
        // The `MaybeUninit` in a "uninitialized" state which is fine

        let item = unsafe {
            let slot_ptr = self.buffer[slot_idx].get();
            (*slot_ptr).assume_init_read()
        };
        // "Publish" that we have a freed up a slot.
        // We use `Release` to ensure that our "take" (the read)
        // is visible to the producer's `Acquire` load of `tail`.

        self.tail.store(tail.wrapping_add(1), Ordering::Release);

        Some(item)
    }
}

/// We must implement Drop to clean up any `T` a left in the buffer.
impl<T> Drop for RingBuffer<T> {
    fn drop(&mut self) {
        //We are in `&MUT self`, so no other threads can be accessing
        // the buffer, We can use `Relaxed` ordering;
        let mut tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Relaxed);
        while tail != head {
            let slot_idx = tail & (self.cap - 1);
            //SAFETY:
            //1. We have `&mut self`, so no other thread is racing.
            //2. We are iterating from `tail` to `head` which are the
            // slots that contain initialized data.
            // 3 `drop_in_place` calls the destructtor for `T`
            unsafe {
                let slot_ptr = self.buffer[slot_idx].get();
                //Use `as_mut` to get `&mut MaybeUninit<T>`
                //and then `drop_in_place` on its contents.
                std::ptr::drop_in_place((*slot_ptr).as_mut_ptr());
            }
            tail = tail.wrapping_add(1);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_single_thread_send_recv() {
        let rb = RingBuffer::new(4);
        assert_eq!(rb.capacity(), 4); // 4 is a power of 2

        rb.send("hello").unwrap();
        rb.send("world").unwrap();

        assert_eq!(rb.len(), 2);

        assert_eq!(rb.recv(), Some("hello"));
        assert_eq!(rb.recv(), Some("world"));
        assert_eq!(rb.recv(), None);
        assert_eq!(rb.len(), 0);
    }

    #[test]
    fn test_full_and_empty() {
        let rb = RingBuffer::new(2);
        assert_eq!(rb.capacity(), 2);

        rb.send(1).unwrap();
        rb.send(2).unwrap();

        // Buffer is full
        assert_eq!(rb.send(3), Err(3));
        assert_eq!(rb.len(), 2);

        // Receive one
        assert_eq!(rb.recv(), Some(1));
        assert_eq!(rb.len(), 1);

        // Now we can send again
        rb.send(3).unwrap();
        assert_eq!(rb.len(), 2);

        assert_eq!(rb.recv(), Some(2));
        assert_eq!(rb.recv(), Some(3));
        assert_eq!(rb.recv(), None);
        assert_eq!(rb.len(), 0);
    }

    #[test]
    fn test_multi_thread_spsc() {
        // Use Arc to share the RingBuffer between threads
        let rb = Arc::new(RingBuffer::new(1024));
        let num_items = 1_000_000;

        let producer_rb = rb.clone();
        let producer_thread = thread::spawn(move || {
            for i in 0..num_items {
                // Spin-wait if the buffer is full
                // FIX: Use `_item` to mark the variable as intentionally unused.
                while let Err(_item) = producer_rb.send(i) {
                    // In a real app, you might `std::hint::spin_loop()`
                    // or `thread::yield_now()`
                    thread::yield_now();
                }
            }
        });

        let consumer_rb = rb.clone();
        let consumer_thread = thread::spawn(move || {
            let mut received_count = 0;
            let mut next_expected = 0;
            while received_count < num_items {
                // Spin-wait if the buffer is empty
                match consumer_rb.recv() {
                    Some(item) => {
                        assert_eq!(item, next_expected);
                        next_expected += 1;
                        received_count += 1;
                    }
                    None => {
                        thread::yield_now();
                    }
                }
            }
        });

        producer_thread.join().unwrap();
        consumer_thread.join().unwrap();
    }

    #[test]
    fn test_drop_cleanup() {
        // A simple type to track drops
        static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);
        // FIX: Add `#[derive(Debug)]` so that `unwrap()` on `Result` can compile.
        #[derive(Debug)]
        struct Dropper;
        impl Drop for Dropper {
            fn drop(&mut self) {
                DROP_COUNT.fetch_add(1, Ordering::SeqCst);
            }
        }

        DROP_COUNT.store(0, Ordering::SeqCst);

        // Create a new scope
        {
            let rb = RingBuffer::new(8);
            rb.send(Dropper).unwrap();
            rb.send(Dropper).unwrap();
            rb.send(Dropper).unwrap();

            // Receive one
            // FIX: Introduce a new scope to force `_d` to be
            // dropped immediately after it's received.
            {
                let _d = rb.recv().unwrap();
            } // _d (and the Dropper it holds) is dropped right here.

            // at this point, 1 should be dropped
            assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);

            // two items are still in the buffer
            // when `rb` goes out of scope, it should be dropped
        }

        // The Drop implementation of RingBuffer should have
        // dropped the remaining 2 items.
        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 3);
    }
}
