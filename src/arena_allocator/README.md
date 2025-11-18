# Arena Allocator (Bump Allocator)

This module provides a high-performance, batch-scoped memory allocator.

## Overview

While an ObjectPool is designed for recycling long-lived objects individually, an Arena is designed for short-lived, batch-oriented allocations.

It works by pre-allocating a large contiguous block of memory (a byte array). Allocating an object is as simple as advancing a pointer ("bumping" the offset). This is orders of magnitude faster than malloc and faster than an ObjectPool because there is no overhead for tracking free slots.

## The Trade-off: "Reset" vs "Free"

You cannot "free" an individual object in an Arena. Instead, you reset the entire Arena at once.

This makes it perfect for processing "units of work," such as a single incoming order match cycle:

arena.reset() (Clear the scratchpad)

arena.alloc(TradeEvent { ... })

arena.alloc(BookUpdate { ... })

(Process/Send events)

(End of cycle - Arena is ready to be reset for the next order)

## ⚠️ Important Warning: No Drop

To achieve maximum speed, this Arena does NOT call Drop on the objects allocated inside it when it is reset.

Do not use this for objects that manage resources (like Box, Vec, String, or File). If you do, those resources will leak.
Only use this for "Plain Old Data" (POD) types: structs containing u64, f64, bool, arrays, etc.



## Usage

This example demonstrates the intended "Batch Processing" workflow where the arena is reset at the start of every logic cycle.


```
use llt_rs::arena_allocator::Arena;

// A "Plain Old Data" struct (no Drop impl, no heap pointers)
#[derive(Debug, Clone, Copy)]
struct TradeEvent {
    id: u64,
    price: f64,
    quantity: u32,
}

fn main() {
    // 1. Pre-allocate 1MB of memory on the heap
    // This is the only "slow" allocation in the program.
    let mut arena = Arena::new(1024 * 1024);

    // Simulate processing incoming market data packets in a loop
    for packet_id in 0..5 {
        // 2. RESET the arena at the start of each batch.
        // This invalidates all previous pointers and sets offset to 0.
        // It is effectively free (one integer assignment).
        arena.reset();

        println!("Processing Batch #{}", packet_id);

        // 3. Allocate scratch objects for this batch
        // These allocations are just pointer bumps (nanoseconds).
        let trade_1 = arena.alloc(TradeEvent { id: 1, price: 100.0, quantity: 50 });
        let trade_2 = arena.alloc(TradeEvent { id: 2, price: 100.5, quantity: 10 });

        // Use the objects like normal mutable references
        trade_1.price = 101.0; 
        
        println!("  Generated Trade: {:?}", trade_1);
        println!("  Generated Trade: {:?}", trade_2);
    }
    // No memory is leaked, and no malloc/free was called inside the loop.
}

```