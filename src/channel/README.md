## SPSC Channel (Hybrid Wait Strategy)

This module provides a high-level, blocking, single-producer single-consumer (SPSC) channel.

### Overview

This Channel is a wrapper around the core RingBuffer primitive. 

It adds a crucial feature: `CPU-efficient blocking when the channel is empty or full.`

The raw RingBuffer is blazing fast but requires a busy-wait (a spin_loop) that burns 100% CPU when idle. This Channel implements a hybrid wait strategy to get the best of both worlds.

### Design: The Hybrid Wait

The `Sender::send` and `Receiver::recv` methods operate in two stages:

### Fast Path (User Space):

The channel first attempts a lock-free operation on the internal RingBuffer.

If the operation succeeds (data is sent or received), it returns immediately.

This path has nanosecond latency and is the primary path for a healthy, high-throughput system.

### Slow Path (Kernel Space):

If the fast path fails (buffer is full or empty), the thread must wait.

Instead of spinning, it acquires a Mutex and calls `Condvar::wait()`.

This "parks" the thread (puts it to sleep), consuming 0% CPU while it waits.

When the other thread sends/receives, it calls `Condvar::notify_one()` to wake up the sleeping thread.

This design provides the raw speed of a lock-free queue when work is active, but the efficiency of an OS-level lock when the system is idle.

### Disconnection

If the Sender is dropped, `recv()` will drain any remaining items from the buffer and then return None, signaling that the channel is closed.