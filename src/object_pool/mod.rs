#![doc = include_str!("README.md")]

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

/// The core Object Pool.
/// This struct holds the "free list" of pre-allocated objects.
struct PoolInner<T> {
    items: Mutex<Vec<T>>,
}

/// A thread-safe, pre-allocating object pool.
pub struct ObjectPool<T> {
    inner: Arc<PoolInner<T>>,
}

impl<T> Clone for ObjectPool<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// A smart-pointer "guard" that holds a pooled object.
///
/// When this guard is dropped, the object is automatically
/// returned to the pool.
pub struct Pooled<'a, T> {
    // We store the item *inside* an Option so we can `take()` it
    // in our `Drop` impl.
    item: Option<T>,
    pool: &'a ObjectPool<T>,
}

impl<T> ObjectPool<T> {
    /// Creates a new `ObjectPool` with a fixed capacity.
    ///
    /// The `init` closure is called `capacity` times to create
    /// the pre-allocated objects.
    pub fn new<F>(capacity: usize, mut init: F) -> Self
    where
        F: FnMut() -> T,
    {
        let mut items = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            items.push(init());
        }

        Self {
            inner: Arc::new(PoolInner {
                items: Mutex::new(items),
            }),
        }
    }

    /// Retrieves an object from the pool.
    ///
    /// If the pool is empty (all objects are in use), this
    /// returns `None`.
    pub fn try_get(&'_ self) -> Option<Pooled<'_, T>> {
        let item = self.inner.items.lock().unwrap().pop()?;

        Some(Pooled {
            item: Some(item),
            pool: self,
        })
    }

    /// Returns an object to the pool.
    ///
    /// Note: This is called automatically by the `Pooled` guard.
    /// You should rarely need to call this directly.
    fn put(&self, item: T) {
        self.inner.items.lock().unwrap().push(item);
    }

    /// Returns the number of objects *available* in the pool.
    pub fn available(&self) -> usize {
        self.inner.items.lock().unwrap().len()
    }
}

// --- Pooled Guard Implementations ---

impl<'a, T> Deref for Pooled<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // This is safe because `item` is guaranteed to be `Some`
        // until it is dropped.
        self.item.as_ref().unwrap()
    }
}

impl<'a, T> DerefMut for Pooled<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // This is safe because `item` is guaranteed to be `Some`
        // until it is dropped.
        self.item.as_mut().unwrap()
    }
}

impl<'a, T> Drop for Pooled<'a, T> {
    /// When the guard goes out of scope, return the item to the pool.
    fn drop(&mut self) {
        if let Some(item) = self.item.take() {
            self.pool.put(item);
        }
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    // A simple struct to test pooling.
    // Note: This implies T is NOT Clone, testing our fix.
    struct Order {
        id: u64,
        #[allow(dead_code)] // Suppress unused field warning
        price: f64,
    }

    // A function to create a "blank" order for the pool
    fn new_order() -> Order {
        Order { id: 0, price: 0.0 }
    }

    #[test]
    fn test_get_and_put() {
        let pool = ObjectPool::new(2, new_order);
        assert_eq!(pool.available(), 2);

        // Get an object
        let mut order1 = pool.try_get().unwrap();
        assert_eq!(pool.available(), 1);

        // Modify it
        order1.id = 100;

        // Get another object
        let order2 = pool.try_get().unwrap();
        assert_eq!(pool.available(), 0);

        // Pool is now empty
        assert!(pool.try_get().is_none());

        // Drop order1
        drop(order1);

        // order1 is returned to the pool
        assert_eq!(pool.available(), 1);

        // Get it again
        let mut order3 = pool.try_get().unwrap();
        assert_eq!(pool.available(), 0);

        // IMPORTANT: It should be the *same* object, but we must
        // reset it ourselves. Here, we check if it's "dirty".
        assert_eq!(order3.id, 100); // It's the old object!
        order3.id = 0; // Reset it

        // Drop the guards
        drop(order2);
        drop(order3);

        assert_eq!(pool.available(), 2);
    }

    #[test]
    fn test_multithreaded_pool() {
        let pool = ObjectPool::new(100, new_order);
        let num_threads = 10;
        let items_per_thread = 50;

        let mut handles = vec![];

        for _ in 0..num_threads {
            let pool_clone = pool.clone();
            handles.push(thread::spawn(move || {
                for i in 0..items_per_thread {
                    let mut item = pool_clone.try_get().unwrap();
                    item.id = i as u64;
                    // Item is automatically returned when `item` guard is dropped
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // All threads ran, but since they returned their items,
        // the pool should be full again.
        assert_eq!(pool.available(), 100);
    }
}
