# llt-rs
low latency tools - rust

---

### Low-Latency Toolkit Primitives

This toolkit provides core primitives designed for building high-performance, low-latency applications, specifically targeting Single-Producer Single-Consumer (SPSC) architectures like Limit Order Books.

#### 1. Lock-Free and Wait-Free Data Structures
[x]  **Atomic Ring Buffer (SPSC)**: A raw, wait-free, fixed-size ring buffer for single-producer, single-consumer scenarios. Optimized with cache-line padding and Acquire/Release semantics. (v0.1.0)

#### 2. High-Performance Channels
[x] **SPSC Channel**: A hybrid channel wrapper around the Atomic Ring Buffer. Combines nanosecond-scale lock-free latency with the CPU efficiency of Condvar blocking during idle periods. (v0.2.0)

---

#### 3. Thread Management
[x] **CPU Affinity-Aware Thread Pool**: Utilities to enumerate cores and pin threads to specific CPU cores. Critical for isolating the "hot path" (Matching Engine) from OS scheduler jitter

---

#### 4. Memory Management
[x] **Object Pool**: A thread-safe system for recycling fixed-size objects (e.g., Orders) to avoid the non-deterministic latency of the global allocator. (v0.3.0)

[x] **Arena Allocator**: A batch-reset bump allocator for short-lived events (e.g., Market Data updates). Allows zero-cost allocation/deallocation cycles per tick. (v0.4.0)

---

#### 5. Utilities & Diagnostics

[x] **Non-Blocking Logger**: A high-performance logging facility that offloads I/O to a pinned background thread via an SPSC channel, ensuring the critical path never blocks on disk or console. (v0.6.0)
