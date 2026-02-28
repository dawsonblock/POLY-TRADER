use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub mod bcore {
    pub mod feed     { pub mod tick; }
    pub mod features { pub mod fixed_point; }
    pub mod memory;
    pub mod sequencer;
    pub mod decision_vm { pub mod interpreter; }
    pub mod ingest   { pub mod ws_feed; pub mod ring_buffer; }
    pub mod oracle   { pub mod fixed_oracle; }
    pub mod ledger   { pub mod capture; }
    pub mod telemetry { pub mod histogram; pub mod perf_counter; }
    pub mod net      { pub mod mtls_config; }
    pub mod signing  { pub mod hsm_signer; }
}

use bcore::features::fixed_point::Fixed;
use bcore::memory::VmStateArea;
use bcore::sequencer::Sequencer;
use bcore::decision_vm::interpreter::{execute, Instruction, OP_LOAD_STATE, OP_EMIT_ORDER};
use bcore::ingest::ring_buffer::make_ring;
use bcore::oracle::fixed_oracle::compute_signal;
use bcore::ledger::capture::{LedgerCapture, LedgerBlock};
use bcore::telemetry::histogram::LATENCY;
use bcore::telemetry::perf_counter::CycleGuard;
use bcore::signing::hsm_signer::{build_signer, OrderIntent};

/// Pin the calling thread to a specific OS CPU core.
/// Production: pair with `isolcpus` kernel param + `irqbalance off`.
fn pin_to_core(core_id: usize) {
    #[cfg(target_os = "linux")]
    unsafe {
        let mut cpuset: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_SET(core_id, &mut cpuset);
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &cpuset);
    }
    #[cfg(not(target_os = "linux"))]
    let _ = core_id; // macOS: pinning not available via standard API
}

#[tokio::main]
async fn main() {
    println!("[DTK v2] Booting Deterministic Trading Kernel...");

    // --- Shared state ---
    let ring = make_ring();
    let running = Arc::new(AtomicBool::new(true));

    // --- Signer (Tier 5) ---
    let signer = build_signer();
    println!("[DTK v2] Signer backend: {}", signer.backend_name());

    // --- Ledger (Tier 2) ---
    let mut ledger = LedgerCapture::new("dtk_ledger.bin")
        .expect("[DTK v2][FATAL] Cannot open ledger file.");
    println!("[DTK v2] Ledger: dtk_ledger.bin (hash-chained)");

    // --- Spawn ingest task on Tokio — separate async thread via spawn_blocking ---
    let ring_ingest = Arc::clone(&ring);
    let running_ingest = Arc::clone(&running);
    let ingest_handle = tokio::task::spawn_blocking(move || {
        // Pin ingest to Core 0 (Linux only)
        pin_to_core(0);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let symbol = std::env::var("MARKET_SYMBOL").unwrap_or_else(|_| "BTCUSDT".into());
            println!("[INGEST] Starting on {symbol} WS feed...");
            bcore::ingest::ws_feed::run_ingest(&symbol, ring_ingest, 0).await;
        });
        let _ = running_ingest;
    });

    // --- VM thread: pinned to Core 1 ---
    pin_to_core(1);

    let mut seq = Sequencer::new();
    let mut arena = VmStateArea::new();
    let mut nonce: u64 = 0;

    // Oracle configuration (from env or defaults)
    let strike    = Fixed::from_f64(std::env::var("STRIKE_PRICE").ok()
        .and_then(|s| s.parse().ok()).unwrap_or(110_000.0));
    let sigma     = Fixed::from_f64(0.80);  // 80% annualized vol
    let tau       = Fixed::from_f64(0.00274); // ~1 day to expiry
    let risk_free = Fixed::from_f64(0.05);

    // Minimal demo bytecode: if oracle EV in R0 > 0.55, emit buy
    let program: Vec<Instruction> = vec![
        Instruction { opcode: OP_LOAD_STATE, dst: 0, src_a: 0, src_b: 0, imm: 0 }, // R0 = price
        Instruction { opcode: OP_EMIT_ORDER, dst: 0, src_a: 0, src_b: 1, imm: 0 }, // BUY
    ];

    println!("[DTK v2] VM thread online. Waiting for ticks...");

    let mut tick_count:  u64 = 0;
    let report_every: u64 = 10_000;

    loop {
        // Pop next tick from lock-free ring (busy-wait = lowest latency)
        let tick = match ring.pop() {
            Some(t) => t,
            None    => continue, // Spin
        };

        let ingest_ts = tick.ts_mono_ns;

        // 1. SEQUENCE VALIDATION
        if !seq.validate_tick_sequence(tick.seq) {
            eprintln!("[DTK v2][FATAL] Amnesia State on seq={}. Halting.", tick.seq);
            break;
        }

        // 2. ORACLE (deterministic Q32.32)
        let signal = compute_signal(&tick, strike, sigma, tau, risk_free);
        arena.current_tick   = tick.clone();
        arena.vpin_toxicity  = signal.vpin_toxicity;

        // 3. DECISION VM (bounded 500 instructions)
        let mut cycles_used: u64 = 0;
        {
            let _guard = CycleGuard::start(&mut cycles_used);
            execute(&program, &mut arena);
        }

        // 4. TELEMETRY: record tick-to-intent latency
        let egress_ts = {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64
        };
        LATENCY.record(egress_ts.saturating_sub(ingest_ts));

        // 5. LEDGER CAPTURE (Tier 2)
        let block = LedgerBlock {
            seq:            tick.seq,
            ts_mono_ns:     ingest_ts,
            price:          tick.price,
            size:           tick.size,
            oracle_ev:      signal.fair_value,
            vm_intent_side: arena.order_intent_side,
            vm_intent_size: arena.order_intent_size,
            fpga_tx_enable: if arena.order_intent_side != 0 { 1 } else { 0 },
        };
        if let Err(e) = ledger.append(&block) {
            eprintln!("[DTK v2][WARN] Ledger append failed: {e}");
        }

        // 6. SIGN & EGRESS (Tier 5)
        if arena.order_intent_side != 0 {
            nonce += 1;
            let intent = OrderIntent {
                side:  arena.order_intent_side,
                price: arena.order_intent_price,
                size:  arena.order_intent_size,
                nonce,
            };
            match signer.sign(&intent) {
                Ok(sig) => {
                    // In production: submit sig to Polymarket CTF Exchange API
                    let _ = sig;
                }
                Err(e) => eprintln!("[DTK v2][WARN] Sign failed: {e}"),
            }
        }

        tick_count += 1;

        // 7. PERIODIC TELEMETRY REPORT
        if tick_count.is_multiple_of(report_every) {
            LATENCY.print_report();
            let _ = ledger.flush();
            println!("[DTK v2] Ticks processed: {tick_count}  Last VM cycles: {cycles_used}");
        }

        if !running.load(Ordering::Relaxed) { break; }
    }

    LATENCY.print_report();
    let _ = ledger.flush();
    println!("[DTK v2] Final ledger hash: {}", hex::encode(ledger.current_hash()));
    println!("[DTK v2] Execution cleanly terminated.");

    ingest_handle.abort();
}
