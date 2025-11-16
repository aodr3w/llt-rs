## Atomic Ring Buffer (SPSC)

A wait-free, fixed-size, Single-Producer Single-Consumer (SPSC) ring buffer.

--

## Overview
This primitive provides the lowest possible latency for passing messages between two threads. Unlike 
standard channels, it avoids all locking overhead in the "happy path" (when the buffer is neither empty nor full).

## Design & Internals

### 1. Wait-Free Progress

This structure uses Atomic Operations (AtomicUsize) exclusively. It guarantees that both the producer and consumer can make progress in a bounded number of steps, provided the buffer conditions (not full/not empty) are met.


### 2. Memory Ordering (Acquire / Release)

We use explicit memory ordering to synchronize the producer and consumer without global locks.

Producer: Uses Release ordering when updating the head to "publish" new data.

Consumer: Uses Acquire ordering when reading the head to ensure it sees the published data.

(and vice-versa for the tail pointer)


### 3. False Sharing Prevention

The head and tail counters are heavily contended. If they share a CPU cache line, the cores will fight over ownership of that line ("cache-line ping-pong"), destroying performance.
We use crossbeam_utils::CachePadded to force head and tail onto separate cache lines (typically 64 bytes apart).

### 4. Power-of-2 Optimization

We force the capacity to be the next power of 2. This allows us to use a fast bitwise-AND (head & mask) to calculate buffer indices, replacing the expensive modulo (%) instruction found in standard ring buffers.


## USAGE

```
use llt_rs::RingBuffer;
use std::sync::Arc;
use std::thread;
use std::hint;

fn main() {
    let capacity = 1024;
    let rb = Arc::new(RingBuffer::new(capacity));
    let producer = rb.clone();
    let consumer = rb.clone();

    // Producer Thread
    thread::spawn(move || {
        for i in 0..10_000 {
            // Busy-wait until space is available.
            // We use `hint::spin_loop()` to be CPU-friendly.
            while let Err(item) = producer.send(i) {
                hint::spin_loop();
            }
        }
    });

    // Consumer Thread
    for _ in 0..10_000 {
        // Busy-wait until data is available
        loop {
            match consumer.recv() {
                Some(val) => {
                    // Process the data...
                    break;
                }
                None => {
                    // Buffer is empty. Spin.
                    hint::spin_loop();
                }
            }
        }
    }
}

```
