/*
 * BOREAL TELEMETRY: RDTSC CYCLE COUNTER
 * Measures exact CPU cycles consumed per VM execution tick.
 * On x86_64 / aarch64 — architecture-specific.
 */

/// Read CPU timestamp counter.
/// x86_64: RDTSC instruction (non-serializing — suitable for relative comparison).
/// aarch64 (Apple Silicon / ARM): CNTVCT_EL0 system register.
#[inline(always)]
pub fn rdtsc() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_rdtsc()
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        let val: u64;
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) val);
        val
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        // Fallback: std monotonic clock (less precise)
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }
}

/// Scoped cycle counter. Records entry TSC, computes delta on drop.
/// Usage:
///   let _guard = CycleGuard::start(&mut cycles_out);
///   // ... VM execution ...
///   // cycles_out populated on drop
pub struct CycleGuard<'a> {
    start:  u64,
    output: &'a mut u64,
}

impl<'a> CycleGuard<'a> {
    #[inline(always)]
    pub fn start(output: &'a mut u64) -> Self {
        Self { start: rdtsc(), output }
    }
}

impl<'a> Drop for CycleGuard<'a> {
    #[inline(always)]
    fn drop(&mut self) {
        *self.output = rdtsc().wrapping_sub(self.start);
    }
}
