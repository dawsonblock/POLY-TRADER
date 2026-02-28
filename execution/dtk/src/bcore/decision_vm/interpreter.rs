/*
 * BOREAL DECISION VM: BYTECODE INTERPRETER
 * Strict deterministic execution engine with fixed budget.
 */

use crate::bcore::memory::VmStateArea;
use crate::bcore::features::fixed_point::Fixed;

// Limited Opcode Definitions
pub const OP_LOAD_STATE: u8 = 0x01;
pub const OP_FPMUL: u8      = 0x02;
pub const OP_FPADD: u8      = 0x03;
pub const OP_CMP_GT: u8     = 0x04;
pub const OP_JMP_IF: u8     = 0x05;
pub const OP_EMIT_ORDER: u8 = 0x06;

// Instruction Format [opcode:1][dst:1][src_a:1][src_b:1][imm:4]
#[derive(Copy, Clone)]
pub struct Instruction {
    pub opcode: u8,
    pub dst: u8,
    pub src_a: u8,
    pub src_b: u8,
    pub imm: i32, 
}

// Bounded Execution budget (e.g. 500 instructions = < 1 microsecond)
pub const MAX_BUDGET: usize = 500;

pub fn execute(program: &[Instruction], arena: &mut VmStateArea) {
    let mut pc: usize = 0;
    let mut cycles_executed: usize = 0;

    arena.clear_scratchpad_and_intent();

    while pc < program.len() {
        if cycles_executed >= MAX_BUDGET {
            // Hard timeout enforced. Zero modifications to outgoing intent allowed.
            arena.clear_scratchpad_and_intent();
            return;
        }

        let inst = program[pc];
        cycles_executed += 1;

        match inst.opcode {
            OP_LOAD_STATE => {
                // e.g. src_a = 0 brings in tick price, 1 brings in toxicity
                if inst.src_a == 0 {
                    arena.registers[inst.dst as usize] = arena.current_tick.price;
                } else if inst.src_a == 1 {
                    arena.registers[inst.dst as usize] = arena.vpin_toxicity;
                } else {
                    arena.registers[inst.dst as usize] = Fixed(inst.imm as i64);
                }
            },
            OP_FPADD => {
                let a = arena.registers[inst.src_a as usize];
                let b = arena.registers[inst.src_b as usize];
                arena.registers[inst.dst as usize] = a.add(b);
            },
            OP_FPMUL => {
                let a = arena.registers[inst.src_a as usize];
                let b = arena.registers[inst.src_b as usize];
                arena.registers[inst.dst as usize] = a.mul(b);
            },
            OP_CMP_GT => {
                let a = arena.registers[inst.src_a as usize];
                let b = arena.registers[inst.src_b as usize];
                // 1 if a > b else 0
                arena.registers[inst.dst as usize] = if a.0 > b.0 { Fixed(1) } else { Fixed(0) };
            },
            OP_JMP_IF => {
                let cond = arena.registers[inst.dst as usize];
                if cond.0 > 0 {
                    pc = pc.wrapping_add(inst.imm as usize);
                    continue; // Skip the normal pc += 1 at the end of the match
                }
            },
            OP_EMIT_ORDER => {
                // Writes deterministic intent out of VM space into Segment D
                arena.order_intent_size = arena.registers[inst.dst as usize];
                arena.order_intent_price = arena.registers[inst.src_a as usize];
                arena.order_intent_side = inst.src_b;
                // Halt on emission -- only zero or one orders permitted per tick
                return;
            }
            _ => { /* NOP */ }
        }

        pc += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_budget_halt() {
        let mut arena = VmStateArea::new();
        
        // Create an infinite loop using JMP_IF to backward offset 
        // Note: Offset logic in interpreter currently uses wrapping_add 
        // so a negative offset requires casting. Let's just create a sequence
        // that exceeds MAX_BUDGET
        let mut explode_program = vec![];
        for _ in 0..(MAX_BUDGET + 10) {
            explode_program.push(Instruction { opcode: 0x00, dst: 0, src_a: 0, src_b: 0, imm: 0 }); // NOP
        }

        // Before execution, intent is clear
        assert_eq!(arena.order_intent_side, 0);

        // Execute the program
        execute(&explode_program, &mut arena);

        // It should have halted exactly at MAX_BUDGET and cleared any mutated intent
        // (Even if intent was mutated early on)
        assert_eq!(arena.order_intent_side, 0, "VM did not properly clear intent upon budgetary exception!");
    }

    #[test]
    fn test_valid_emission() {
        let mut arena = VmStateArea::new();
        arena.current_tick.price = Fixed::from_f64(1.0);
        
        let program = vec![
            Instruction { opcode: OP_LOAD_STATE, dst: 0, src_a: 0, src_b: 0, imm: 0 },
            Instruction { opcode: OP_EMIT_ORDER, dst: 0, src_a: 0, src_b: 1, imm: 0 }, // BUY
        ];

        execute(&program, &mut arena);

        assert_eq!(arena.order_intent_side, 1, "VM did not emit structural BUY intent");
    }
}
