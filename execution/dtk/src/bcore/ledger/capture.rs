/*
 * BOREAL LEDGER: DETERMINISTIC TICK CAPTURE
 * Every tick that enters the system is logged in a binary, hash-chained format.
 * This is the foundation of the deterministic replay guarantee.
 *
 * Block format per tick (little-endian):
 *   [prev_hash: 32 bytes]
 *   [seq: 8 bytes]
 *   [ts_mono_ns: 8 bytes]
 *   [price: 8 bytes  (Q32.32 i64)]
 *   [size: 8 bytes   (Q32.32 i64)]
 *   [oracle_ev: 8 bytes (Q32.32 i64)]
 *   [vm_intent_side: 1 byte]
 *   [vm_intent_size: 8 bytes (Q32.32 i64)]
 *   [fpga_tx_enable: 1 byte]
 *   = 90 bytes per block
 */

use sha2::{Sha256, Digest};
use std::io::Write;

use crate::bcore::features::fixed_point::Fixed;

/// A single ledger block — everything that happened on one tick.
#[derive(Debug, Clone)]
pub struct LedgerBlock {
    pub seq:             u64,
    pub ts_mono_ns:      u64,
    pub price:           Fixed,
    pub size:            Fixed,
    pub oracle_ev:       Fixed,
    pub vm_intent_side:  u8,
    pub vm_intent_size:  Fixed,
    pub fpga_tx_enable:  u8,
}

/// Append-only ledger writer. In production: memory-mapped file.
pub struct LedgerCapture {
    writer: std::io::BufWriter<std::fs::File>,
    prev_hash: [u8; 32],
}

impl LedgerCapture {
    pub fn new(path: &str) -> std::io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            writer: std::io::BufWriter::new(file),
            prev_hash: [0u8; 32], // Genesis block: zeroes
        })
    }

    pub fn append(&mut self, block: &LedgerBlock) -> std::io::Result<[u8; 32]> {
        // Serialize block to bytes
        let mut raw = Vec::with_capacity(90);
        raw.extend_from_slice(&self.prev_hash);
        raw.extend_from_slice(&block.seq.to_le_bytes());
        raw.extend_from_slice(&block.ts_mono_ns.to_le_bytes());
        raw.extend_from_slice(&block.price.0.to_le_bytes());
        raw.extend_from_slice(&block.size.0.to_le_bytes());
        raw.extend_from_slice(&block.oracle_ev.0.to_le_bytes());
        raw.push(block.vm_intent_side);
        raw.extend_from_slice(&block.vm_intent_size.0.to_le_bytes());
        raw.push(block.fpga_tx_enable);

        // Hash this block
        let mut hasher = Sha256::new();
        hasher.update(&raw);
        let hash: [u8; 32] = hasher.finalize().into();

        // Write: raw block + hash
        self.writer.write_all(&raw)?;
        self.writer.write_all(&hash)?;

        self.prev_hash = hash;
        Ok(hash)
    }

    pub fn current_hash(&self) -> &[u8; 32] {
        &self.prev_hash
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
