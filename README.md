# llt-rs
low latency tools - rust

---

### Low-Latency Toolkit Primitives

This toolkit provides core primitives designed for building high-performance, low-latency applications.

#### 1. Lock-Free and Wait-Free Data Structures
* **Wait-Free MPMC Queue**: A queue that guarantees non-blocking progress for message passing between multiple producers and consumers, eliminating the latency spikes caused by locks.
* **Atomic Ring Buffer**: A highly efficient, fixed-size queue for single-producer, single-consumer scenarios, ideal for fast event pipelines.

---

#### 2. High-Performance Channels
* **Bounded MPMC Channel**: A channel with a fixed capacity to manage backpressure and prevent unbounded memory growth.
* **SPSC Channel**: A hyper-optimized channel for one-to-one communication, offering the lowest possible overhead.

---

#### 3. Thread Management
* **Steal-able Task Scheduler**: A scheduler that dynamically balances workloads by allowing idle threads to "steal" tasks from busy threads.
* **CPU Affinity-Aware Thread Pool**: A thread pool that can pin threads to specific CPU cores, reducing cache misses and context-switching overhead.

---

#### 4. Memory Management
* **Object Pool**: A system for pre-allocating and recycling objects to avoid the latency spikes associated with dynamic memory allocation.
* **Arena Allocator**: An allocator that manages memory in a large, pre-allocated block, freeing all objects at once for efficient batch processing.

---

#### 5. Utilities & Diagnostics
* **High-Resolution Clock**: A precise, low-overhead clock for accurate latency measurement and profiling.
* **Latency Profiler**: Tools to measure and visualize latency distribution, helping identify and eliminate performance outliers.
* **Non-Blocking Logger**: A logger that writes messages without blocking the main execution thread, enabling production debugging without performance impact.
