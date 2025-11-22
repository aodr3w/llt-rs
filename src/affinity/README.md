# CPU Affinity (Thread Pinning)

Tools to bind (pin) threads to specific CPU cores.

## Overview

In low-latency systems, Context Switching and Cache Misses are major performance killers.

**Cache Thrashing**: If the OS moves your thread from Core 1 to Core 2, you lose the contents of the L1/L2 cache. Re-populating the cache takes hundreds of nanoseconds.

**OS Jitter**: The scheduler moving threads around incurs overhead (~1-3 microseconds).

Thread Pinning tells the OS Scheduler: "Do not move this thread. Keep it on Core X forever."


## Platform Support

While this module compiles on all major platforms (Linux, macOS, Windows) to facilitate cross-platform development, Thread Pinning is effectively a Linux-only feature.

**Linux**: ✅ Full Support (Confirmed). Pinning works as expected via sched_setaffinity.

**macOS**: ❌ No Support. Apple's XNU kernel does not allow user-space thread binding. pin_to_core will always return false.

**Windows**: ⚠️ Experimental/Unverified. The underlying calls exist, but this crate has currently only verified behavior on Linux.

**Recommendation**: Develop on your preferred OS, but rely on pin_to_core returning bool to handle environments where pinning is unavailable gracefully


## Usage

This example demonstrates a standard "Pipelined" architecture where the Gateway (Network I/O) and
the Matching Engine (Logic) are pinned to separate cores to maximize cache locality and throughput.

```
use llt_rs::affinity;
use std::thread;
// CRITICAL IMPORT: Required for the synchronization logic below
use std::sync::{Arc, Barrier};

fn main() {
    // 1. Get list of available core IDs
    let core_ids = affinity::get_core_ids();
    
    // We need at least 2 cores for this demo
    if core_ids.len() < 2 {
        eprintln!("Need at least 2 cores to demonstrate isolation.");
        return;
    }

    // Barrier to synchronize start. We wait for 3 parties:
    // 1. Gateway Thread
    // 2. Engine Thread
    // 3. Main Thread
    let barrier = Arc::new(Barrier::new(3));
    
    // --- THREAD 1: GATEWAY (Network I/O) ---
    // Pin to the FIRST core (Core 0)
    let gateway_core = core_ids[0];
    let b1 = barrier.clone();
    
    thread::spawn(move || {
        // 2. Pin the thread immediately
        if !affinity::pin_to_core(gateway_core) {
            eprintln!("Failed to pin Gateway thread");
        }
        
        println!("[Gateway] Pinned to Core ID: {}", gateway_core.id);
        
        // Wait for the system to be fully initialized
        b1.wait(); 
        
        // ... Run network event loop ...
        // loop { read_socket(); write_ring_buffer(); }
    });

    // --- THREAD 2: MATCHING ENGINE (Hot Logic) ---
    // Pin to the SECOND core (Core 1) to ensure L1 cache isolation
    let engine_core = core_ids[1];
    let b2 = barrier.clone();

    thread::spawn(move || {
        // 2. Pin the thread immediately
        if !affinity::pin_to_core(engine_core) {
            eprintln!("Failed to pin Engine thread");
        }
        
        println!("[Engine]  Pinned to Core ID: {}", engine_core.id);
        
        // Wait for the system to be fully initialized
        b2.wait();

        // ... Run matching logic loop ...
        // loop { read_ring_buffer(); match_orders(); }
    });

    // Main thread waits for everyone to be pinned and ready
    barrier.wait();
    println!("System started with thread isolation.");
}

```


## Best Practices for Limit Order Books (LOB)

For a single-threaded matching engine, the ideal setup is usually:

**Gateway Thread**: Pinned to Core 1 (Network I/O).

**Matching Engine**:** Pinned to Core 2 (Isolated). This core should ideally be isolated from the OS scheduler entirely using isolcpus boot parameters.

**Logger/Persister**: Pinned to Core 3.