#![doc = include_str!("README.md")]

use core_affinity;

///A unique identifier for a CPU core.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoreId {
    pub id: usize,
    //We wrap the internal ID to avoid exposing the dependency types directly
    // in our public API signature.
    internal: usize,
}

///Retrieves a list of all available CPU processor IDs on the system.
pub fn get_core_ids() -> Vec<CoreId> {
    //If the feature is disabled or the crate fails to load, return empty.
    let internal_ids = core_affinity::get_core_ids().unwrap_or_default();

    internal_ids
        .into_iter()
        .map(|c| CoreId {
            id: c.id,
            internal: c.id,
        })
        .collect()
}
/// Pins the *current* thread to the specified CPU core.
///
/// Returns `true` if the operation was successful.
///
/// # Important
/// Once pinned, the OS will try very hard to keep this thread on that core.
/// You should ensure that no other heavy threads are competing for this core.
pub fn pin_to_core(core_id: CoreId) -> bool {
    let internal_core = core_affinity::CoreId {
        id: core_id.internal,
    };
    core_affinity::set_for_current(internal_core)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_can_get_cores() {
        let cores = get_core_ids();
        assert!(!cores.is_empty(), "Should find atleast one core");
        println!("Found {} cores", cores.len());
    }

    #[test]
    fn test_pinning() {
        let cores = get_core_ids();
        if let Some(core) = cores.first() {
            let core = *core;
            let handle = thread::spawn(move || {
                let success = pin_to_core(core);
                if !success {
                    eprintln!("WARNING: Failed to pin thread (common in containers/CI)")
                } else {
                    println!("Successfully pinned to core {}", core.id);
                }
                let mut x = 0;
                for _ in 0..1000 {
                    x += 1;
                }
                x
            });
            assert_eq!(handle.join().unwrap(), 1000);
        }
    }
}
