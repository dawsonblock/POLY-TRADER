# POLY-TRADER: Deterministic Trading Kernel (DTK) v2

> A production-grade, hardware-enforced, cryptographically-auditable financial execution appliance built on Polymarket's CTF exchange.

This is **DTK v2**—a fully deterministic execution stack that eliminates non-deterministic structural fractures (Python/ZeroMQ) in favor of a unified Rust pipeline. It features hardware-enforced risk limits, a bounded decision VM, and a binary hash-chained audit trail.

---

## Architecture Overview (v2)

```
                ┌─────────────────────┐
  Binance WS ──►│   Unified Ingest     │  Rust (Core 0 / SO_TIMESTAMPING)
                │   ws_feed.rs         │  Q32.32 Fixed-Point sealed boundary
                └────────┬────────────┘
                         │ SPSC Lock-free Ring (crossbeam)
                ┌────────▼────────────┐
                │   Execution Layer   │  Rust (DTK v2 / Core 1)
                │ Sequencer (gap det) │◄─ Frame drop = Amnesia State = HALT
                │ Fixed Oracle (BS)   │◄─ Deterministic A&S CDF Lookup
                │ Decision VM (Q32.32)│◄─ Hard 500-instruction budget/tick
                │ Ledger (SHA-256)    │◄─ Binary hash-chained audit trail
                └────────┬────────────┘
                         │ 
                ┌────────▼────────────┐
                │   Security & Risk   │  HSM / FPGA
                │ hsm_signer.rs       │◄─ Secure intent signing (Dev: Software)
                │ boreal_dual_clamp   │◄─ SIL-3 dual-channel inline firewall
                │ token_bucket        │◄─ Rate limiter (SVA verified)
                └─────────────────────┘
```

---

## Repository Structure

```
POLY-TRADER/
│
├── hardware/               # FPGA / SystemVerilog (SVA Verified)
│   ├── src/
│   │   ├── boreal_dual_clamp.sv    # SIL-3 dual-channel inline risk firewall
│   │   └── token_bucket.sv         # Hardware token bucket rate limiter
│   └── formal/
│       └── boreal_clamp.sby        # Symbiyosys formal proof script
│
├── oracles/                # Pricing / Microstructure Models
│   ├── microstructure/             # Realistic friction models (Slippage/Fees)
│   ├── backtest/
│   │   └── replay_backtest.py      # DTK v2 replay verifier harness
│   └── apex_oracle.py              # AI oracle (Mamba-2 SSM, H100 GPU)
│
├── execution/              # Low-latency Execution Layer
│   ├── dtk/                        # ★ Deterministic Trading Kernel v2 (Rust)
│   │   └── src/
│   │       ├── main.rs             # Unified Pipeline: Ingest → Oracle → VM
│   │       └── bcore/
│   │           ├── ingest/         # Rust WS client & SPSC ring buffer
│   │           ├── oracle/         # Deterministic Fixed-Point pricing
│   │           ├── ledger/         # Binary hash-chained capture
│   │           ├── signing/        # HSM OrderSigner abstraction
│   │           └── decision_vm/    # Bounded bytecode VM
│   └── replay/                     # Standalone replay verifier CLI (Rust)
│
└── ops/                    # Deployment
    ├── setup_ubuntu.sh             # Unified v2 setup (No legacy ZeroMQ)
    ├── setup_h100.sh               # CUDA 12.1 + Mamba-SSM for AI oracle
    └── dtk.service                 # systemd: DTK v2 production daemon
```

---

## v2 Design Principles

| Principle | Implementation |
|---|---|
| **Structural Integrity** | Floats are prohibited; all data converted to Q32.32 at the ingest edge. |
| **Zero Jitter** | No ZeroMQ/IPC between components; lock-free SPSC rings on isolated cores. |
| **Deterministic Replay** | Every tick is SHA-256 chained. Replay verifier asserts bit-parity. |
| **Bounded execution** | VM halts at 500 instruction budget per tick. |
| **Zero dynamic allocation** | Pre-allocated arena, no `Box/Vec` in hot path. |
| **Verified Safety** | Hardware risk clamps verified via formal Symbiyosys proofs. |
| **Secure Egress** | HSM-backed signing and TLS 1.3 mTLS with cert pinning. |

---

## Quick Start

### 1. Build & Test DTK v2

```bash
cd execution/dtk
cargo build --release
cargo test
```

### 2. Verify Deterministic Replay

```bash
# Process market data to generate a ledger
./target/release/dtk --symbol BTCUSDT --capture execution.log

# Run the verifier to assert hash chain integrity
cd ../replay
cargo run -- --log ../dtk/execution.log
```

### 3. Deploy to Production (Ubuntu/AWS)

```bash
chmod +x ops/setup_ubuntu.sh
sudo ./ops/setup_ubuntu.sh

# Start the unified kernel
sudo systemctl start dtk
```

---

## Security Notice

This system is for **research and educational purposes**. All production deployments should use the `HSM` signer backend. Never fund a live wallet with funds you cannot afford to lose.
