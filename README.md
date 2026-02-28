# POLY-TRADER: Deterministic Trading Kernel (DTK) v1

> A production-grade, hardware-enforced, cryptographically-auditable financial execution appliance built on Polymarket's CTF exchange.

This is not a simple trading bot. It is a **multi-layer deterministic execution stack** with hardware-enforced risk limits, a bounded decision VM, and a full cryptographic audit trail. Every design decision prioritizes: **no hidden state, deterministic replay, hard risk bounds before signing**.

---

## Architecture Overview

```
                ┌─────────────────────┐
  Binance WS ─► │   Oracle Layer       │  Python (ZeroMQ broadcast)
  / REST API    │ poly_oracle.py       │  Black-Scholes binary fair value
                │ apex_oracle.py       │  Mamba-2 SSM AI (H100 GPU)
                └────────┬────────────┘
                         │ ZeroMQ IPC
                ┌────────▼────────────┐
                │   Execution Layer   │  Rust (DTK v1)
                │ Sequencer (gap det) │◄─ Frame drop = Amnesia State = HALT
                │ Decision VM (Q32.32)│◄─ Hard 500-instruction budget/tick
                │ Ledger (SHA-256)    │◄─ Hash-chained audit trail
                └────────┬────────────┘
                         │ DMA / PCIe
                ┌────────▼────────────┐
                │   Hardware Layer    │  SystemVerilog (FPGA)
                │ boreal_dual_clamp   │◄─ SIL-3 dual-channel inline firewall
                │ token_bucket        │◄─ Rate limiter: max N orders/µs
                │ bsb_core            │◄─ Physics-based QUBO solver
                │ heab_gate           │◄─ Optical kill switch on overflow
                └─────────────────────┘
```

---

## Repository Structure

```
POLY-TRADER/
│
├── hardware/               # FPGA / SystemVerilog
│   ├── src/
│   │   ├── bsb_core.sv             # Ballistic Simulated Bifurcation solver
│   │   ├── heab_gate.sv            # Hardware Execution Advisory Board gate
│   │   ├── war_machine_v4_apex.sv  # Top-level FPGA wrapper
│   │   ├── boreal_dual_clamp.sv    # SIL-3 dual-channel inline risk firewall
│   │   └── token_bucket.sv         # Hardware token bucket rate limiter
│   └── sim/
│       ├── tb_bsb.sv               # BSB physics testbench
│       ├── tb_boreal_clamp.sv      # Risk clamp verification (PASS ✅)
│       └── tb_token_bucket.sv      # Rate limiter verification (PASS ✅)
│
├── oracles/                # Pricing / Signal Layer
│   ├── poly_oracle.py              # Binance WS → Black-Scholes → ZeroMQ
│   ├── apex_oracle.py              # AI oracle (Mamba-2 SSM, H100 GPU)
│   └── mc_simulation.py            # Monte Carlo EV reality check
│
├── execution/              # Low-latency Execution Layer
│   ├── dtk/                        # ★ Deterministic Trading Kernel v1 (Rust)
│   │   └── src/
│   │       ├── main.rs             # Hot loop: Ingest → VM → Clamp → Egress
│   │       └── bcore/
│   │           ├── features/fixed_point.rs  # Q32.32 deterministic arithmetic
│   │           ├── feed/tick.rs             # Canonical market tick struct
│   │           ├── memory.rs               # Pre-allocated zero-GC arena
│   │           ├── sequencer.rs            # TCP gap detection / Amnesia State
│   │           └── decision_vm/
│   │               └── interpreter.rs      # Bounded bytecode VM (500 instr budget)
│   ├── poly_sniper/                # Legacy Rust sniper (EIP-712 signing)
│   ├── boreal_audit/               # Cryptographic ledger audit CLI (Rust)
│   └── cpp_router.cpp              # Optional bare-metal UDP bypass
│
└── ops/                    # Deployment
    ├── setup_ubuntu.sh             # Deps, TCP stack tuning, build pipeline
    ├── setup_h100.sh               # CUDA 12.1 + Mamba-SSM for AI oracle
    ├── poly-oracle.service         # systemd: Python oracle daemon
    ├── poly-sniper.service         # systemd: Rust sniper daemon
    └── apex-oracle.service         # systemd: AI oracle daemon
```

---

## Core Design Principles (DTK Spec)

| Principle | Implementation |
|---|---|
| **No hidden state** | All market state in `VmStateArea` segments A–D |
| **Deterministic replay** | SHA-256 hash-chained ledger via `boreal_audit` |
| **Bounded execution** | VM halts at 500 instruction budget per tick |
| **Zero dynamic allocation** | Pre-allocated arena, no `Box/Vec` in hot path |
| **Hard risk before signing** | FPGA clamp drops packet before it hits the MAC |
| **Amnesia protection** | Sequencer halts VM on any TCP frame gap |
| **Dual-channel SIL-3** | Both channels must agree or `fault_flag` fires |

---

## Quick Start

### 1. Build the DTK (Rust)

```bash
# Ensure Rust is installed: https://rustup.rs
cd execution/dtk
cargo build --release
cargo test
```

Expected output: `6 passed; 0 failed`

### 2. Run the Hardware Simulations (Icarus Verilog)

```bash
# Install: brew install icarus-verilog
iverilog -g2012 -o hardware/sim/tb_token_bucket.out \
  hardware/sim/tb_token_bucket.sv hardware/src/token_bucket.sv
vvp hardware/sim/tb_token_bucket.out

iverilog -g2012 -o hardware/sim/tb_boreal_clamp.out \
  hardware/sim/tb_boreal_clamp.sv hardware/src/boreal_dual_clamp.sv
vvp hardware/sim/tb_boreal_clamp.out
```

Expected: `[PASS] Hardware caught the burst.` / `[PASS] Clamp caught notional breach.` / `[PASS] Clamp caught inventory breach.`

### 3. Run the Oracle (Python)

```bash
cp .env.example .env   # add your Alchemy WSS key + wallet private key

python3 -m venv venv && source venv/bin/activate
pip install websockets pyzmq scipy
python oracles/poly_oracle.py
```

### 4. Deploy to Production (Ubuntu/AWS)

```bash
chmod +x ops/setup_ubuntu.sh
sudo ./ops/setup_ubuntu.sh

# Copy and enable services
sudo cp ops/*.service /etc/systemd/system/
sudo systemctl enable poly-oracle poly-sniper
sudo systemctl start poly-oracle poly-sniper
```

---

## Hardware Risk Limits (Default)

| Limit | Value | Enforced By |
|---|---|---|
| Max notional per order | $10,000 | `boreal_dual_clamp.sv` |
| Max aggregate inventory | $50,000 | `boreal_dual_clamp.sv` |
| Max orders per microsecond | 5 | `token_bucket.sv` |
| Software HEAB inventory | $5,000 | `poly_sniper/src/main.rs` |

---

## Verification Results

| Component | Test | Status |
|---|---|---|
| Q32.32 Fixed-Point Arithmetic | Determinism + overflow wrapping | ✅ PASS |
| Sequencer | Perfect sequence + amnesia gap detection | ✅ PASS |
| Decision VM | Emission + budget halt enforcement | ✅ PASS |
| Token Bucket | Burst detection + refill | ✅ PASS |
| Boreal Dual Clamp | Notional breach + inventory ceiling | ✅ PASS |

---

## Security Notice

This system is for **research and educational purposes**. Cross-exchange arbitrage may be subject to derivatives trading regulations in your jurisdiction. Never fund a live wallet with funds you cannot afford to lose.
