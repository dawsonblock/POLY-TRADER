/*
 * BOREAL TELEMETRY: LOCK-FREE HDR LATENCY HISTOGRAM
 * Records tick-to-intent latency (NIC receive → VM emit).
 * No allocation after startup. Never blocks the hot loop.
 */

use std::sync::atomic::{AtomicU64, Ordering};

const BUCKET_COUNT: usize = 6;

/// Bucket boundaries in nanoseconds
const BOUNDARIES_NS: [u64; BUCKET_COUNT] = [
    1_000,        // < 1µs
    10_000,       // 1–10µs
    100_000,      // 10–100µs
    1_000_000,    // 100µs–1ms
    10_000_000,   // 1ms–10ms
    u64::MAX,     // > 10ms (anomaly)
];

pub struct LatencyHistogram {
    counts:   [AtomicU64; BUCKET_COUNT],
    total:    AtomicU64,
    sum_ns:   AtomicU64,
    max_ns:   AtomicU64,
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyHistogram {
    pub const fn new() -> Self {
        Self {
            counts: [
                AtomicU64::new(0), AtomicU64::new(0),
                AtomicU64::new(0), AtomicU64::new(0),
                AtomicU64::new(0), AtomicU64::new(0),
            ],
            total:  AtomicU64::new(0),
            sum_ns: AtomicU64::new(0),
            max_ns: AtomicU64::new(0),
        }
    }

    /// Record a single latency sample. Called on every tick — must be fast.
    #[inline(always)]
    pub fn record(&self, latency_ns: u64) {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.sum_ns.fetch_add(latency_ns, Ordering::Relaxed);

        // Update max (best-effort, not perfectly atomic — acceptable for telemetry)
        let current_max = self.max_ns.load(Ordering::Relaxed);
        if latency_ns > current_max {
            self.max_ns.store(latency_ns, Ordering::Relaxed);
        }

        // Bucket assignment
        for (i, &boundary) in BOUNDARIES_NS.iter().enumerate() {
            if latency_ns < boundary {
                self.counts[i].fetch_add(1, Ordering::Relaxed);
                return;
            }
        }
    }

    /// Print a human-readable summary. Call periodically (not in hot loop).
    pub fn print_report(&self) {
        let total = self.total.load(Ordering::Relaxed);
        if total == 0 {
            println!("[TELEMETRY] No samples yet.");
            return;
        }
        let sum = self.sum_ns.load(Ordering::Relaxed);
        let max = self.max_ns.load(Ordering::Relaxed);
        let mean = sum / total;

        println!("[TELEMETRY] Tick-to-Intent Latency ({total} samples)");
        println!("  Mean:  {}ns  Max: {}ns", mean, max);
        println!("  < 1µs  : {}", self.counts[0].load(Ordering::Relaxed));
        println!("  1–10µs : {}", self.counts[1].load(Ordering::Relaxed));
        println!("  10–100µs: {}", self.counts[2].load(Ordering::Relaxed));
        println!("  100µs–1ms: {}", self.counts[3].load(Ordering::Relaxed));
        println!("  1ms–10ms : {}", self.counts[4].load(Ordering::Relaxed));
        println!("  > 10ms [!]: {}", self.counts[5].load(Ordering::Relaxed));
    }
}

/// Global static histogram — accessible from any thread, zero allocation.
pub static LATENCY: LatencyHistogram = LatencyHistogram::new();
