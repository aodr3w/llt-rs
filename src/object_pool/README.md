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




## Usage

```
use llt_rs::ObjectPool;

struct Order {
    id: u64,
    price: f64,
}

fn main() {
    // 1. Create a pool of 1024 pre-allocated orders
    // The closure is called 1024 times to fill the pool.
    let pool = ObjectPool::new(1024, || Order { id: 0, price: 0.0 });

    // 2. Get an object from the pool (returns a guard)
    if let Some(mut order_guard) = pool.try_get() {
        // 3. Use it like a normal mutable reference
        order_guard.id = 101;
        order_guard.price = 99.99;
        
        println!("Processing Order #{}", order_guard.id);
        
        // 4. When `order_guard` goes out of scope here, 
        // the Order is automatically reset (if you impl a reset) 
        // and returned to the pool.
    }
    
    // Pool is full again
    assert_eq!(pool.available(), 1024);
}

```