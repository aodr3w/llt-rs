#![doc = include_str!("README.md")]

use std::cell::UnsafeCell;
use std::mem;
use std::ptr;

/// A fast, linear bump allocator.
///
/// See [README.md](README.md) for details and safety warnings.
pub struct Arena {
    /// The backing memory.
    buffer: Box<[u8]>,
    /// The current offset into the buffer (the "bump pointer").
    offset: UnsafeCell<usize>,
}

impl Arena {
    /// Creates a new Arena with the specified capacity in bytes.
    pub fn new(capacity_bytes: usize) -> Self {
        // Create a zeroed buffer of the given size
        let buffer = vec![0u8; capacity_bytes].into_boxed_slice();

        Self {
            buffer,
            offset: UnsafeCell::new(0),
        }
    }

    /// Allocates a value in the arena and returns a mutable reference to it.
    ///
    /// # Panics
    /// Panics if the arena runs out of space.
    ///
    /// # Safety check
    /// This function does NOT run `Drop` on the object when the arena is reset.
    /// Only use this for types that do not implement `Drop` (like integers,
    /// fixed-size arrays, or simple POD structs).
    #[allow(clippy::mut_from_ref)]
    pub fn alloc<T>(&self, value: T) -> &mut T {
        let size = mem::size_of::<T>();
        let align = mem::align_of::<T>();

        // We need to modify the offset, but we want to allow shared (&self) access
        // so we can allocate multiple things "simultaneously" (conceptually).
        // In a single-threaded LOB context, this effectively allows
        // multiple references to the arena to exist.
        // SAFETY: This is only safe if the Arena is not shared across threads.
        // We haven't marked it Sync, so we are good.
        let current_offset = unsafe { *self.offset.get() };

        // Calculate padding needed to satisfy alignment requirements
        let padding = (align - (current_offset % align)) % align;
        let start = current_offset + padding;
        let end = start + size;

        if end > self.buffer.len() {
            panic!(
                "Arena OOM: Capacity {} bytes, requested {} bytes",
                self.buffer.len(),
                end
            );
        }

        unsafe {
            // 1. Get the pointer to the destination
            let ptr = self.buffer.as_ptr().add(start) as *mut T;

            // 2. Write the value
            ptr::write(ptr, value);

            // 3. Bump the pointer
            *self.offset.get() = end;

            // 4. Return the mutable reference
            &mut *ptr
        }
    }

    /// Resets the arena, effectively freeing all objects at once.
    ///
    /// Note: Destructors (`Drop`) for allocated objects are NOT called.
    pub fn reset(&mut self) {
        // We require &mut self here to ensure no one else is holding
        // a reference to an allocated object.
        unsafe {
            *self.offset.get() = 0;
        }
    }

    /// Returns the number of bytes currently used.
    pub fn used_bytes(&self) -> usize {
        unsafe { *self.offset.get() }
    }

    /// Returns the total capacity in bytes.
    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct TradeEvent {
        id: u64,
        price: f64,
        qty: u32,
    }

    #[test]
    fn test_alloc_and_reset() {
        // Small arena: 1KB
        let mut arena = Arena::new(1024);

        let event1 = arena.alloc(TradeEvent {
            id: 1,
            price: 100.0,
            qty: 10,
        });
        assert_eq!(event1.id, 1);

        // Modify the allocated data
        event1.price = 101.5;

        let event2 = arena.alloc(TradeEvent {
            id: 2,
            price: 200.0,
            qty: 20,
        });

        assert_eq!(event1.price, 101.5); // Still accessible
        assert_eq!(event2.id, 2);

        // Reset the arena
        arena.reset();

        // Pointers event1 and event2 are now invalid (and checked by borrow checker!)
        // Allocating again should start from 0
        assert_eq!(arena.used_bytes(), 0);

        let event3 = arena.alloc(123u64);
        assert_eq!(*event3, 123);
    }

    #[test]
    #[should_panic(expected = "Arena OOM")]
    fn test_oom() {
        let arena = Arena::new(16);
        arena.alloc(0u64); // 8 bytes
        arena.alloc(0u64); // 8 bytes (Total 16)
        arena.alloc(0u8); // 1 byte -> Panic!
    }
}
