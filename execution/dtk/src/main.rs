use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// NOTE: Module is named 'bcore' — NOT 'core' — to avoid colliding with
// Rust's built-in 'core' crate which is always in scope.
pub mod bcore {
    pub mod feed { pub mod tick; }
    pub mod features { pub mod fixed_point; }
    pub mod memory;
    pub mod sequencer;
    pub mod decision_vm { pub mod interpreter; }
}

use bcore::features::fixed_point::Fixed;
use bcore::feed::tick::Tick;
use bcore::memory::VmStateArea;
use bcore::sequencer::Sequencer;
use bcore::decision_vm::interpreter::{execute, Instruction, OP_LOAD_STATE, OP_EMIT_ORDER};

fn main() {
    println!("[DTK v1] Booting Deterministic Trading Kernel...");

    let mut seq = Sequencer::new();
    let mut arena = VmStateArea::new();

    // Minimal demo bytecode: Load price into R0, emit a buy order
    let program = vec![
        Instruction { opcode: OP_LOAD_STATE, dst: 0, src_a: 0, src_b: 0, imm: 0 }, // R0 = price
        Instruction { opcode: OP_EMIT_ORDER, dst: 0, src_a: 0, src_b: 1, imm: 0 }, // Emit Buy (side 1)
    ];

    println!("[DTK v1] Sequence synchronizer: ONLINE");
    println!("[DTK v1] VMArena instantiated (Segment A-D strict partitioning).");
    println!("[DTK v1] Commencing zero-allocation hot loop...");

    let running = Arc::new(AtomicBool::new(true));

    let mut dummy_seq_counter = 100u64;
    while running.load(Ordering::Relaxed) {
        // 1. INGEST: validate frame sequence (gap = Amnesia State)
        if !seq.validate_tick_sequence(dummy_seq_counter) {
            println!("[DTK v1][FATAL] Amnesia State. Network gap detected. Halting.");
            break;
        }

        // 2. FEATURE EXTRACTION (deterministic, no allocation)
        arena.current_tick = Tick {
            seq: dummy_seq_counter,
            source_id: 1,
            ts_mono_ns: 0,
            raw_hash: [0; 32],
            price: Fixed::from_f64(0.65),
            size: Fixed::from_f64(100.0),
        };
        arena.vpin_toxicity = Fixed::from_f64(0.12);

        // 3. DECISION VM (hard budget: MAX_BUDGET instructions)
        execute(&program, &mut arena);

        // 4. CLAMP & EGRESS — signals FPGA via DMA in production
        if arena.order_intent_side != 0 {
            println!(
                "[DTK v1] Tick {}: Intent side={} size={:.4}",
                dummy_seq_counter,
                arena.order_intent_side,
                arena.order_intent_size.to_f64()
            );
        }

        dummy_seq_counter += 1;
        if dummy_seq_counter > 105 {
            break;
        }
    }

    println!("[DTK v1] Execution cleanly terminated.");
}
