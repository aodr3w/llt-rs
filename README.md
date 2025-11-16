llt-rs (Low-Latency Toolkit)

llt-rs is a collection of high-performance, lock-free primitives designed for low-latency applications in Rust.

This crate focuses on wait-free data structures that minimize CPU cache contention and eliminate the latency spikes associated with standard locking mechanisms.

Features

1. Atomic Ring Buffer (SPSC)

A wait-free, fixed-size, Single-Producer Single-Consumer (SPSC) ring buffer.

Wait-Free: Guarantees progress for both producer and consumer in a bounded number of steps.

Lock-Free: Uses AtomicUsize with explicit Acquire/Release memory ordering. Zero Mutex or spin_loop overhead in the hot path.

Cache-Friendly: Uses CachePadded (via crossbeam-utils) to prevent false sharing between the head and tail counters, ensuring independent CPU cache lines for producer and consumer.

Performance: Optimized for power-of-2 capacities to use bitwise-AND indexing instead of slow modulo operations.

Usage

```
use llt_rs::RingBuffer;
use std::thread;
use std::sync::Arc;

fn main() {
    let rb = Arc::new(RingBuffer::new(1024));
    let rb_consumer = rb.clone();

    thread::spawn(move || {
        for i in 0..100 {
            // Busy-wait loop (fastest possible latency)
            while let Err(_) = rb.send(i) {}
        }
    });

    for i in 0..100 {
        while let None = rb_consumer.recv() {}
        // Process item...
    }
}

```

Roadmap

v0.1.0 (Current): SPSC Atomic Ring Buffer.

v0.2.0 (Planned): Blocking SPSC Channel (Hybrid Wait Strategy).

Future: MPMC Queues, Object Pools, and Affinity-Aware Thread Pools.

License

MIT