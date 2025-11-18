### Object Pool

This module provides a thread-safe, pre-allocating object pool.

## Overview

In low-latency systems, the global memory allocator (malloc/free) is a primary source of non-deterministic latency spikes (jitter). An ObjectPool solves this by pre-allocating a fixed number of objects on the heap at startup.

Instead of creating and destroying objects, your application "gets" an object from the pool to use it and "puts" it back when finished. This turns a slow, blocking malloc call into a fast, thread-safe Vec::pop from the pool's "free list".

## Design: The Pooled Guard

This pool is thread-safe via a Mutex wrapping the free list.

To ensure objects are always returned to the pool, the get() method returns a Pooled<T> guard. This smart pointer:

Implements Deref and DerefMut so you can treat it just like a &mut T.

Implements Drop. When the Pooled<T> guard goes out of scope, its drop implementation automatically returns the object to the pool.

This makes it impossible to "lose" or forget to return a pooled object, preventing leaks.

## Pool Exhaustion & Backpressure

If the pool is empty (all objects are currently in use), try_get() will immediately return None.

This is a critical, non-blocking behavior. It allows your application to handle backpressure (e.g., reject an incoming request, signal a "busy" state) instead of blocking the thread or (even worse) allocating a new object.