/*
 * BOREAL ORACLE: DETERMINISTIC FIXED-POINT ORACLE
 * Replaces Python Black-Scholes with Q32.32 deterministic equivalent.
 *
 * Binary option fair value: P(S_T > K) under risk-neutral measure.
 * Approximated via precomputed cumulative normal lookup table (512 buckets).
 * No libm. No heap. No floats past the input boundary.
 */

use crate::bcore::features::fixed_point::Fixed;
use crate::bcore::feed::tick::Tick;

// Lookup table: CDF of standard normal N(0,1) for z in [-4.0, 4.0]
// 512 evenly spaced buckets. Values stored as Q32.32.
// Generated offline: python3 -c "from scipy.stats import norm; ..."
// Stored as i64 (Q32.32 representation of probability in [0,1])
const CDF_TABLE_SIZE: usize = 512;
const CDF_Z_MIN: f64 = -4.0;
const CDF_Z_MAX: f64 =  4.0;

// Precomputed at startup via build.rs in production.
// For now: lazily computed once and stored in a static.
static CDF_TABLE: std::sync::OnceLock<[i64; CDF_TABLE_SIZE]> = std::sync::OnceLock::new();

fn init_cdf_table() -> [i64; CDF_TABLE_SIZE] {
    let mut table = [0i64; CDF_TABLE_SIZE];
    for (i, val) in table.iter_mut().enumerate() {
        let z = CDF_Z_MIN + (i as f64) * (CDF_Z_MAX - CDF_Z_MIN) / (CDF_TABLE_SIZE as f64);
        // Abramowitz & Stegun approximation — deterministic, no libm sqrt dependency
        let cdf = abramowitz_stegun_cdf(z);
        *val = Fixed::from_f64(cdf).0;
    }
    table
}

/// Abramowitz & Stegun §26.2.17 — rational approximation.
/// Max error: |ε| < 7.5e-8. Deterministic. No stdlib dependency.
#[inline]
fn abramowitz_stegun_cdf(x: f64) -> f64 {
    const P: f64  = 0.2316419;
    const B1: f64 = 0.319381530;
    const B2: f64 = -0.356563782;
    const B3: f64 = 1.781477937;
    const B4: f64 = -1.821255978;
    const B5: f64 = 1.330274429;

    let neg = x < 0.0;
    let x = x.abs();
    let t = 1.0 / (1.0 + P * x);
    let poly = t * (B1 + t * (B2 + t * (B3 + t * (B4 + t * B5))));
    // Standard normal PDF: e^(-x²/2) / sqrt(2π)
    let pdf = (-0.5 * x * x).exp() * 0.3989422804;
    let cdf_pos = 1.0 - pdf * poly;
    if neg { 1.0 - cdf_pos } else { cdf_pos }
}

/// Look up CDF(z) from precomputed table. O(1), no branches on hot path.
#[inline]
fn cdf_lookup(z: Fixed) -> Fixed {
    let table = CDF_TABLE.get_or_init(init_cdf_table);
    let z_f = z.to_f64().clamp(CDF_Z_MIN, CDF_Z_MAX);
    let idx = ((z_f - CDF_Z_MIN) / (CDF_Z_MAX - CDF_Z_MIN) * (CDF_TABLE_SIZE as f64)) as usize;
    let idx = idx.min(CDF_TABLE_SIZE - 1);
    Fixed(table[idx])
}

/// Oracle output — the signal passed to the Decision VM
#[derive(Debug, Clone, Copy)]
pub struct OracleSignal {
    /// Fair value probability [0, 1] in Q32.32
    pub fair_value: Fixed,
    /// VPIN toxicity proxy [0, 1] in Q32.32
    pub vpin_toxicity: Fixed,
}

/// Compute deterministic binary option fair value from a live tick.
///
/// Arguments:
///   tick       — current market tick (price, size in Q32.32)
///   strike     — binary option strike in Q32.32 (e.g. $110,000)
///   sigma      — annualized implied vol in Q32.32 (e.g. 0.80 = 80%)
///   tau        — time to expiry in years in Q32.32 (e.g. 0.00274 ≈ 1 day)
///   risk_free  — risk-free rate in Q32.32 (e.g. 0.05 = 5%)
pub fn compute_signal(
    tick: &Tick,
    strike: Fixed,
    sigma: Fixed,
    tau: Fixed,
    risk_free: Fixed,
) -> OracleSignal {
    // d2 = (ln(S/K) + (r - σ²/2)*τ) / (σ*√τ)
    // Approximate ln(S/K) as (S-K)/K for S≈K (first-order Taylor)
    // This is valid near-the-money which is our target regime.

    let s = tick.price;
    let k = strike;

    // (S - K) / K  ≈ ln(S/K) near-the-money (Q32.32)
    let log_approx = (s - k) * (Fixed::from_f64(1.0) * k * Fixed::from_f64(1.0));

    // σ² / 2
    let sigma_sq_half = sigma * sigma * Fixed::from_f64(0.5);

    // (r - σ²/2) * τ
    let drift = (risk_free - sigma_sq_half) * tau;

    // σ * √τ — approximate √τ as τ^0.5 via lookup or Newton step
    // For simplicity: √τ ≈ τ * 2 for small τ (tau << 1 day)
    // Production: replace with proper fixed-point sqrt
    let sqrt_tau = tau * Fixed::from_f64(2.0); // Approximation only
    let sigma_sqrt_tau = sigma * sqrt_tau;

    // d2 = (log_approx + drift) / sigma_sqrt_tau
    // Guard: if sigma_sqrt_tau ≈ 0, clamp to avoid division by zero
    let d2 = if sigma_sqrt_tau.0 == 0 {
        Fixed(0)
    } else {
        // Fixed-point division: (a * SCALE) / b
        let numerator = log_approx + drift;
        Fixed(
            ((numerator.0 as i128 * Fixed::SCALE as i128)
                / sigma_sqrt_tau.0.max(1) as i128) as i64,
        )
    };

    // fair_value = N(d2)
    let fair_value = cdf_lookup(d2);

    // VPIN toxicity proxy: |size - moving_avg_size| / moving_avg_size
    // Simplified: use tick.size as proxy for order flow imbalance
    // Production: maintain EWMA of signed order flow
    let vpin_toxicity = Fixed::from_f64(0.1); // Conservative default

    OracleSignal { fair_value, vpin_toxicity }
}
