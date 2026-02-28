# DTK v2 Full Rebuild Plan

Close the structural fracture between deterministic lower layers and non-deterministic Python upper layers. Build a cohesive, end-to-end deterministic financial execution appliance.

## Proposed Changes

---

### Tier 1 — Rust Ingestion Pipeline (Close Structural Fracture)

Replace Python oracle + ZeroMQ with a fully deterministic Rust ingestion layer.

#### [NEW] execution/dtk/src/bcore/ingest/ws_feed.rs

- Async Rust WebSocket client (tokio-tungstenite) consuming raw Binance WS frames
- `SO_TIMESTAMPING` kernel-level NIC arrival timestamp at socket recv
- Zero-copy frame parsing into pre-allocated `Tick` structs
- No heap allocation after startup

#### [NEW] execution/dtk/src/bcore/ingest/ring_buffer.rs

- SPSC (single-producer single-consumer) lock-free ring buffer
- `crossbeam::ArrayQueue` or custom cache-line-aligned ring
- Replaces ZeroMQ entirely — same-process, same-host, zero IPC overhead
- `#[repr(align(64))]` on buffer cells to prevent false sharing

#### [NEW] execution/dtk/src/bcore/oracle/fixed_oracle.rs

- Fixed-point Black-Scholes binary approximation in Q32.32
- Precomputed cumulative normal lookup table (no `libm` floats)
- Deterministic EV calculation: no randomness, no GC, no Python
- Volatility bucket table loaded at startup from config file

#### [MODIFY] execution/dtk/src/main.rs

- Wire: `ws_feed` → `ring_buffer` → `fixed_oracle` → `decision_vm` → FPGA egress
- Ingest thread pinned to Core 0, VM thread pinned to Core 1 via `libc::sched_setaffinity`
- Validate: 0 heap allocations in hot loop via `#[global_allocator]` instrumented allocator

---

### Tier 2 — Deterministic Replay Harness

Build the infrastructure to capture, store, and re-execute every tick deterministically.

#### [NEW] execution/dtk/src/bcore/ledger/capture.rs

- Binary log writer: `[seq:u64][ts_ns:u64][raw_tick:Tick][oracle_ev:Fixed][vm_intent:Intent][fpga_decision:u8]`
- SHA-256 hash-chain: each block's hash is `SHA256(prev_hash || block_bytes)`
- Lockless append-only log via memory-mapped file (`memmap2` crate)

#### [NEW] execution/replay/src/main.rs

- Standalone Rust binary: `replay --log <path> --verify-hash`
- Re-executes the fixed oracle + VM on captured ticks
- Asserts: identical oracle EV, identical VM intent, identical ledger hash
- If any diverge: prints exact tick index and state diff — **hidden state detected**

#### [NEW] execution/replay/Cargo.toml

---

### Tier 3 — FPGA Formal Verification

Add SVA assertions and Symbiyosys proof targets to the hardware modules.

#### [NEW] hardware/formal/boreal_clamp.sby

- Symbiyosys proof script for `boreal_dual_clamp.sv`
- Cover + prove mode
- Properties: inventory never exceeds bound, channel mismatch always halts, token count ≥ 0

#### [MODIFY] hardware/src/boreal_dual_clamp.sv

- Add SVA assertion blocks:

  ```
  assert property (always pos_ram_A[i] <= max_position[i]);
  assert property (clamp_A !== clamp_B |-> fault_flag);
  ```

#### [MODIFY] hardware/src/token_bucket.sv

- Add SVA: `assert property (tokens >= 0 && tokens <= MAX_BUCKET_SIZE)`
- Add `$past()` reference for refill monotonicity proof

#### [NEW] hardware/formal/tb_formal_clamp.sv

- Assume/guarantee formal testbench with symbolic inputs

---

### Tier 4 — Latency Instrumentation

Add precise measurement infrastructure to validate all timing claims.

#### [NEW] execution/dtk/src/bcore/telemetry/histogram.rs

- Lock-free HDR histogram (High Dynamic Range) — fixed allocation
- Buckets: `<1µs`, `1–10µs`, `10–100µs`, `>100µs`
- Measured: tick-to-intent latency (NIC receive → VM emit)
- Dumps to stdout every N ticks and on SIGTERM

#### [NEW] execution/dtk/src/bcore/telemetry/perf_counter.rs

- RDTSC-based cycle counter for intra-VM instruction timing
- Per-opcode cycle attribution
- Worst-case cycle accounting per tick → true WCET bound

#### [MODIFY] execution/dtk/src/bcore/decision_vm/interpreter.rs

- Instrument: record RDTSC at entry/exit of `execute()`
- Record cycle count per tick into histogram

---

### Tier 5 — Security Hardening Blueprint

Key management and transport hardening. This tier is primarily architecture + stubs since HSM requires hardware.

#### [NEW] execution/dtk/src/bcore/signing/hsm_signer.rs

- Trait `OrderSigner` with `sign(&self, intent: &Intent) -> Result<Signature>`
- `SoftwareSigner` impl: current ECDSA from env (dev only)
- `HsmSigner` stub: interface for YubiHSM2 / AWS CloudHSM / Nitro Enclave
- Runtime selection via `SIGNER_BACKEND` env var

#### [NEW] execution/dtk/src/bcore/net/mtls_config.rs

- `rustls` TLS config builder for mTLS between oracle and DTK layers
- Certificate pinning: reject any cert not matching embedded fingerprint

#### [NEW] docs/security_model.md

- Documents: key rotation strategy, enclave binding plan, replay nonce tracking, host hardening checklist

---

### Tier 6 — Profitability Microstructure Rebuild

Replace naive Black-Scholes EV with realistic market microstructure modeling.

#### [NEW] oracles/microstructure/slippage_model.py

- L2 order book depth snapshot ingestion
- Impact function: `slippage(size, depth) = size / (depth_at_level * elasticity)`
- Spread widening regime detection (volatility-adjusted)

#### [NEW] oracles/microstructure/fill_model.py

- Queue position Monte Carlo: probability of fill given latency and queue depth
- Partial fill simulator: partial_fill_pct distribution per size tier

#### [NEW] oracles/microstructure/fee_model.py

- Maker/taker fee tier calculation
- Incentive rebate capture
- Net EV after all friction: `ev_net = ev_gross - slippage - fees - (1 - fill_prob) * opportunity_cost`

#### [NEW] oracles/backtest/replay_backtest.py

- Reads from DTK replay log (Tier 2)
- Re-evaluates strategy decisions with realistic microstructure friction applied
- Outputs: Sharpe, max drawdown, fill rate, EV per trade, P&L curve

---

## Verification Plan

### Tier 1

- `cargo test` — 0 heap alloc assertion test in hot loop
- `cargo bench` — ring buffer throughput > 1M ticks/sec

### Tier 2

- Run replay on 1000 captured ticks → assert hash match
- Introduce artificial state mutation → assert replay detects divergence

### Tier 3

- `sby -f hardware/formal/boreal_clamp.sby` → proof complete, no counterexample
- `iverilog` regression: existing testbenches still pass

### Tier 4

- Under simulated tick load: print histogram
- Assert P99 < 10µs on pinned core

### Tier 5

- Unit test: `SoftwareSigner` produces valid ECDSA sig
- Integration test: mTLS handshake completes between oracle and DTK stub

### Tier 6

- Backtest on 30 days of historical data
- Assert net EV per trade > 0 after all friction
