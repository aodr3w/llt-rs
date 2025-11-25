# Non-Blocking Logger

A high-performance logging facility designed for low-latency applications.

## Overview

Standard logging (like println! or log crate implementations writing to files) involves Blocking I/O.
If your hot-path thread (e.g., a Real-time Audio Processor, Game Loop, or Event Handler) calls a logger that blocks waiting for a disk write or a mutex on stdout, you introduce massive, non-deterministic latency spikes.

This module solves this by offloading the I/O to a dedicated thread.


## Architecture

**The SPSC Channel**: We use llt-rs::channel to pass log messages from the hot thread to the logger thread.

**Zero-Blocking Guarantee**: The logger uses try_send. If the logging buffer is full, the message is dropped (and a counter incremented) rather than blocking the main execution thread. In low-latency systems, it is better to lose a log line than to violate timing constraints or stall the critical path.

**Pinned Worker**: The background logging thread can be optionally pinned to a specific CPU core (using llt-rs::affinity) to keep it away from your critical processing cores.

## Setup

```
[dependencies]
llt-rs = { version = "0.6.0", features = ["logger"] }

```
## Usage

This example simulates a generic High-Frequency Event Loop. Notice how the hot path logs complex events without ever blocking on I/O.


```
use llt_rs::logger::Logger;
use std::thread;
use std::time::Duration;

fn main() {
    // 1. Initialize the logger with a large buffer (power of 2).
    // This spawns the background writer thread immediately.
    let logger = Logger::new(4096);

    println!("System starting...");

    // 2. Simulate a Hot Path (e.g., Sensor Data Ingest or Audio Callback)
    // We clone the logger to pass it to the new thread.
    let log_handle = logger.clone();
    
    let handle = thread::spawn(move || {
        // Simulating a burst of high-frequency events
        for i in 0..100 {
            // CRITICAL: This call takes nanoseconds.
            // It creates the string and pushes pointers.
            // It does NOT wait for stdout/disk.
            log_handle.log(format!("[SENSOR] Event ID: {} | Value: {:.4}", i, i as f64 * 1.5));
            
            // Simulate work (processing the event)
            // In reality, this sleep would be absent or microsecond-scale.
            thread::sleep(Duration::from_micros(10));
        }
        log_handle.log("[SYSTEM] Batch processing complete.");
    });

    // 3. The main thread can do other work...
    thread::sleep(Duration::from_millis(50));
    println!("Main thread working...");

    handle.join().unwrap();
    
    // 4. Check metrics before exit
    let dropped = logger.get_dropped_count();
    if dropped > 0 {
        eprintln!("Warning: Logger dropped {} messages due to backpressure.", dropped);
    }
    
    // Give the background thread a tiny slice of time to flush the remaining buffer
    thread::sleep(Duration::from_millis(10));
}

```