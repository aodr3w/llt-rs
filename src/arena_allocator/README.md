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