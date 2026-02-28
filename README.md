
# APEX WAR MACHINE v4.0 & POLYMARKET SNIPER PIPELINE

This repository contains the complete execution stack for the **Polymarket Sniper**: a sub-millisecond crypto-binary arbitrage platform bridging Binance Futures websockets to Polymarket's CTF matching engine.

## 📁 Repository Structure

* **`hardware/`**: High-performance system-on-chip components.
  * `src/bsb_core.sv` & `src/heab_gate.sv`: SystemVerilog modules defining the ballistic Simulated Bifurcation (bSB) physics unrolling and Hardware Execution Advisory Board (HEAB) gate clamping limits.
  * `src/war_machine_v4_apex.sv`: The top-level FPGA wrapper uniting the physical components.
  * `sim/tb_bsb.sv`: Simulation constraints and logical checks.
* **`oracles/`**: Application level pricing mathematics.
  * `apex_oracle.py`: A conceptual PyTorch script for an AI Oracle utilizing Mamba-2 SSM architecture.
  * `poly_oracle.py`: The production-ready Python Binance price listener. Calculates the True Probability via the Black-Scholes binary option math and broadcasts decisions via ZeroMQ.
* **`execution/`**: The ultra-low-latency Rust and C++ layer.
  * `poly_sniper/`: The Rust Execution Engine. Listens to the ZeroMQ socket in `< 50us`. Tracks inventory to satisfy software-HEAB limitations. Immediately generates an offline **EIP-712 Typed Data Signature** for the Polymarket CTF Exchange.
  * `cpp_router.cpp`: (Optional) Direct UDP bypass injector for bare-metal trading bypassing python loops.
* **`ops/`**: AWS deployment scripts.
  * `setup_ubuntu.sh`: provisions dependencies, TCP networking stack optimizations for latency, and builds binaries for the sniper pipeline.
  * `setup_h100.sh`: provisions PyTorch, CUDA 12.1, and Mamba-SSM structural dependencies for the APEX AI Oracle on hardware accelerator instances.
  * `poly-oracle.service` / `poly-sniper.service` / `apex-oracle.service`: systemd daemons for the background agents.

## 🚀 Quick Start (Simulation)

If you'd like to test the Polymarket execution flow locally:

1. **Configure environment**

    ```bash
    cp .env.example .env
    # Edit .env with your Alchemy WSS API key and a burn wallet private key
    ```

2. **Start the Rust Engine (The Executor)**
    Open a terminal and run the execution loop:

    ```bash
    cd poly_sniper
    cargo run
    ```

3. **Start the Python Oracle (The Brain)**
    In a separate terminal, trigger the Binance connection:

    ```bash
    python3 -m venv venv
    source venv/bin/activate
    pip install websockets pyzmq scipy
    python poly_oracle.py
    ```

You will see the Oracle listening to `$BTC` in real-time, computing the `poly_fair_value` against the $110,000 threshold, and immediately dispatching signals to the Rust sniper which will conditionally sign an `EIP-712` packet.
