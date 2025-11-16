llt-rs Project Roadmap

This document outlines the development trajectory for the llt-rs toolkit.

1. Lock-Free and Wait-Free Data Structures

Core primitives for data exchange without locking overhead.

[x] Atomic Ring Buffer (SPSC): A highly efficient, fixed-size queue for single-producer, single-consumer scenarios. (v0.1.0)

[ ] Wait-Free MPMC Queue: A queue that guarantees non-blocking progress for message passing between multiple producers and consumers.

2. High-Performance Channels

Ergonomic wrappers around the core primitives.

[ ] SPSC Channel: A hybrid channel (Spin + Condvar) for one-to-one communication. (Target: v0.2.0)

[ ] Bounded MPMC Channel: A channel with fixed capacity to manage backpressure and prevent unbounded memory growth.

3. Thread Management

Tools to control thread execution and placement.

[ ] CPU Affinity-Aware Thread Pool: A thread pool that can pin threads to specific CPU cores to reduce cache misses.

[ ] Steal-able Task Scheduler: A dynamic workload balancer (work-stealing) for efficient task distribution.

4. Memory Management

Strategies to avoid the non-deterministic latency of global allocators.

[ ] Object Pool: A system for pre-allocating and recycling objects (avoiding malloc/free in hot paths).

[ ] Arena Allocator: An allocator that manages memory in large blocks for efficient batch processing and cleanup.

5. Utilities & Diagnostics

Tools to measure and verify low-latency performance.

[ ] High-Resolution Clock: Low-overhead clock for precise profiling.

[ ] Non-Blocking Logger: A logging system that offloads I/O to a separate thread to prevent blocking the critical path.

[ ] Latency Profiler: Tools (HDR Histogram) to visualize latency distribution and identify outliers.