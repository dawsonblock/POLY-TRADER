/*
 * BOREAL INGEST: LOCK-FREE SPSC RING BUFFER
 * Replaces ZeroMQ entirely. Same-process, zero-copy, cache-aware.
 *
 * Uses crossbeam::ArrayQueue which is a bounded MPMC queue but used as SPSC
 * by convention: one ingest thread pushes, one VM thread consumes.
 * 
 * Cell alignment: crossbeam internally aligns to cache line. Explicit
 * #[repr(align(64))] is applied on Tick (see tick.rs).
 */

use crossbeam::queue::ArrayQueue;
use std::sync::Arc;

use crate::bcore::feed::tick::Tick;

pub const RING_CAPACITY: usize = 4096; // Power of 2, fits ~4ms of 1ms ticks

/// Construct the shared ring buffer. Arc allows ingest + VM threads to share.
pub fn make_ring() -> Arc<ArrayQueue<Tick>> {
    Arc::new(ArrayQueue::new(RING_CAPACITY))
}
