# DTK v2 Full Rebuild

## Tier 1 — Rust Ingestion Pipeline [COMPLETED]

- [x] `bcore/ingest/ws_feed.rs` — Rust WS client + SO_TIMESTAMPING
- [x] `bcore/ingest/ring_buffer.rs` — SPSC lock-free ring (replaces ZeroMQ)
- [x] `bcore/oracle/fixed_oracle.rs` — Q32.32 deterministic oracle (replaces Python)
- [x] Update `main.rs` — wire full Rust pipeline with CPU pinning

## Tier 2 — Deterministic Replay Harness [COMPLETED]

- [x] `bcore/ledger/capture.rs` — binary hash-chained tick log
- [x] `execution/replay/` — standalone replay verifier binary

## Tier 3 — FPGA Formal Verification [COMPLETED]

- [x] Add SVA assertions to `boreal_dual_clamp.sv`
- [x] Add SVA assertions to `token_bucket.sv`
- [x] `hardware/formal/boreal_clamp.sby` — Symbiyosys proof script
- [x] Formal verification implementation complete.

## Tier 4 — Latency Instrumentation [COMPLETED]

- [x] `bcore/telemetry/histogram.rs` — HDR histogram (lock-free)
- [x] `bcore/telemetry/perf_counter.rs` — RDTSC cycle counter
- [x] Instrument `main.rs` — per-tick cycle recording

## Tier 5 — Security Hardening [COMPLETED]

- [x] `bcore/signing/hsm_signer.rs` — `OrderSigner` trait + backends
- [x] `bcore/net/mtls_config.rs` — rustls mTLS config + cert pinning
- [x] Security model implementation complete.

## Tier 6 — Profitability Microstructure [COMPLETED]

- [x] `oracles/microstructure/slippage_model.py`
- [x] `oracles/microstructure/fill_model.py`
- [x] `oracles/microstructure/fee_model.py`
- [x] `oracles/backtest/replay_backtest.py`
