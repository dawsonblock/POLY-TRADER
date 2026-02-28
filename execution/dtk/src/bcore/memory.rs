/* 
 * BOREAL DECISION VM: PRE-ALLOCATED ARENA
 * Ensures ZERO dynamic memory allocation during the critical execution path.
 */

use crate::bcore::features::fixed_point::Fixed;
use crate::bcore::feed::tick::Tick;

pub const SCRATCHPAD_SIZE: usize = 128;

pub struct VmStateArea {
    // Segment A: Read-Only Market State (DMA-mapped representation)
    pub current_tick: Tick,
    pub vpin_toxicity: Fixed, // Real-time adverse selection probability

    // Segment B: Read-Write Scratchpad (Fixed-Size, Cleared Every Tick)
    pub registers: [Fixed; SCRATCHPAD_SIZE],

    // Segment C: Write-Only Intent
    pub order_intent_side: u8, // 0 = none, 1 = buy, 2 = sell
    pub order_intent_size: Fixed,
    pub order_intent_price: Fixed,
}

impl VmStateArea {
    pub fn new() -> Self {
        Self {
            current_tick: Tick::default(),
            vpin_toxicity: Fixed(0),
            registers: [Fixed(0); SCRATCHPAD_SIZE],
            order_intent_side: 0,
            order_intent_size: Fixed(0),
            order_intent_price: Fixed(0),
        }
    }

    #[inline(always)]
    pub fn clear_scratchpad_and_intent(&mut self) {
        // Zero out the registers and intent deterministically for the next tick
        for reg in self.registers.iter_mut() {
            *reg = Fixed(0);
        }
        self.order_intent_side = 0;
        self.order_intent_size = Fixed(0);
        self.order_intent_price = Fixed(0);
    }
}
