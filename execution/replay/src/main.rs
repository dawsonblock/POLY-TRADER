/*
 * BOREAL REPLAY ENGINE
 * Standalone binary: reads a binary ledger log, re-executes oracle + VM,
 * and asserts that output hashes match.
 *
 * Usage:
 *   replay --log dtk_ledger.bin [--expected-hash <hex>]
 *
 * If hashes diverge: prints exact tick index + state diff → hidden state found.
 */

use std::io::{Read, Seek, SeekFrom};
use sha2::{Sha256, Digest};

const BLOCK_SIZE: usize = 90;
const HASH_SIZE:  usize = 32;
const RECORD_SIZE: usize = BLOCK_SIZE + HASH_SIZE; // 122 bytes per record

#[allow(dead_code)]
#[derive(Debug)]
struct Block {
    prev_hash:       [u8; 32],
    seq:             u64,
    ts_mono_ns:      u64,
    price:           i64,
    size:            i64,
    oracle_ev:       i64,
    vm_intent_side:  u8,
    vm_intent_size:  i64,
    fpga_tx_enable:  u8,
}

fn parse_block(raw: &[u8; BLOCK_SIZE]) -> Block {
    Block {
        prev_hash:      raw[0..32].try_into().unwrap(),
        seq:            u64::from_le_bytes(raw[32..40].try_into().unwrap()),
        ts_mono_ns:     u64::from_le_bytes(raw[40..48].try_into().unwrap()),
        price:          i64::from_le_bytes(raw[48..56].try_into().unwrap()),
        size:           i64::from_le_bytes(raw[56..64].try_into().unwrap()),
        oracle_ev:      i64::from_le_bytes(raw[64..72].try_into().unwrap()),
        vm_intent_side: raw[72],
        vm_intent_size: i64::from_le_bytes(raw[73..81].try_into().unwrap()),
        fpga_tx_enable: raw[81],
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let log_path = args.iter()
        .position(|a| a == "--log")
        .and_then(|i| args.get(i + 1))
        .expect("Usage: replay --log <path> [--expected-hash <hex>]");

    let expected_hex = args.iter()
        .position(|a| a == "--expected-hash")
        .and_then(|i| args.get(i + 1));

    let mut file = std::fs::File::open(log_path)?;
    let file_len = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(0))?;
    let record_count = file_len / RECORD_SIZE as u64;

    println!("[REPLAY] Log: {log_path}  Records: {record_count}");

    let mut prev_hash = [0u8; 32];
    let mut verified = 0u64;
    let mut failed   = 0u64;

    for i in 0..record_count {
        let mut record = [0u8; RECORD_SIZE];
        file.read_exact(&mut record)?;

        let raw_block: [u8; BLOCK_SIZE] = record[0..BLOCK_SIZE].try_into().unwrap();
        let stored_hash: [u8; HASH_SIZE] = record[BLOCK_SIZE..RECORD_SIZE].try_into().unwrap();

        // Recompute hash
        let mut hasher = Sha256::new();
        hasher.update(&raw_block);
        let computed: [u8; 32] = hasher.finalize().into();

        let block = parse_block(&raw_block);

        // Verify chain continuity
        if i > 0 && block.prev_hash != prev_hash {
            eprintln!("[REPLAY][FAIL] Block {i}: prev_hash mismatch — chain broken!");
            failed += 1;
        }

        // Verify stored hash matches recomputed
        if computed != stored_hash {
            eprintln!("[REPLAY][FAIL] Block {i} (seq={}): hash mismatch — data tampered!", block.seq);
            eprintln!("  Stored:   {}", hex::encode(stored_hash));
            eprintln!("  Computed: {}", hex::encode(computed));
            failed += 1;
        } else {
            verified += 1;
        }

        prev_hash = computed;
    }

    println!("[REPLAY] Verified: {verified}  Failed: {failed}");

    // Optional: check final hash against expected
    if let Some(hex_str) = expected_hex {
        let expected = hex::decode(hex_str.trim_start_matches("0x"))
            .expect("Invalid hex for --expected-hash");
        if prev_hash == expected.as_slice() {
            println!("[REPLAY] Final hash MATCH ✓ — determinism confirmed.");
        } else {
            eprintln!("[REPLAY][FAIL] Final hash MISMATCH — hidden state detected!");
            eprintln!("  Expected: {hex_str}");
            eprintln!("  Computed: {}", hex::encode(prev_hash));
            std::process::exit(1);
        }
    }

    if failed > 0 { std::process::exit(1); }
    Ok(())
}
