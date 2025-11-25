# Non-Blocking Logger

A high-performance logging facility designed for low-latency applications.

## Overview

Standard logging (like println! or writing to files) involves Blocking I/O.
If your hot-path thread calls a logger that blocks waiting for a disk write or a mutex on stdout, you introduce massive, non-deterministic latency spikes.

This module solves this by offloading the I/O to a dedicated thread.

## Architecture

**The SPSC Channel**: We use the llt-rs::channel to pass log messages from the hot thread to the logger thread.

**Zero-Blocking Guarantee**: The logger uses try_send. If the logging buffer is full, the message is dropped (and a counter incremented) rather than blocking your application.

**Pinned Worker**: The background logging thread can be optionally pinned to a specific CPU core (using llt-rs::affinity)

## Usage

This example simulates a High-Frequency Trading (HFT) loop. Notice how the hot path logs complex events without ever blocking on I/O.


```
use llt_rs::logger::Logger;
use std::thread;
use std::time::Duration;

fn main() {
    // 1. Initialize logger (spawns background thread)
    // 4096 is the buffer capacity.
    let logger = Logger::new(4096);

    println!("System starting...");

    // 2. Simulate a Hot Path
    let log_handle = logger.clone();
    
    let handle = thread::spawn(move || {
        // Burst of high-speed events
        for i in 0..100 {
            // CRITICAL: This call takes nanoseconds.
            // It DOES NOT wait for stdout/disk.
            log_handle.log(format!("[MD] Tick: AAPL @ ${}.00", 150 + i));
            
            // Simulate work (nanoseconds)
            thread::sleep(Duration::from_nanos(100));
        }
        log_handle.log("[MD] Burst complete.");
    });

    // 3. Main thread continues...
    handle.join().unwrap();
    
    // 4. Check metrics
    let dropped = logger.get_dropped_count();
    if dropped > 0 {
        eprintln!("Warning: Dropped {} logs due to backpressure.", dropped);
    }
    
    // Flush time
    thread::sleep(Duration::from_millis(10));
}

```