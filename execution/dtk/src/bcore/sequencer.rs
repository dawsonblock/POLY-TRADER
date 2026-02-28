/*
 * BOREAL STATE RECONCILIATION LOGIC
 * Purpose: Ensures the VM immediately hard-panics if external network drops 
 * cause an unrecoverable lag state. "Amnesia State".
 */

#[derive(Debug, PartialEq, Eq, Default)]
pub enum SyncState {
    #[default]
    Valid,
    Amnesia, // Network gap detected. VM must Halt.
    Reconciling, // Waiting for Out-Of-Band REST response
}

#[derive(Default)]
pub struct Sequencer {
    pub state: SyncState,
    pub expected_next_seq: u64,
}

impl Sequencer {
    pub fn new() -> Self {
        Self {
            state: SyncState::Valid, // Assume connected on boot
            expected_next_seq: 0,
        }
    }

    // Ingests the packet sequence number and validates against gap
    pub fn validate_tick_sequence(&mut self, incoming_seq: u64) -> bool {
        if self.state != SyncState::Valid {
            return false;
        }

        if incoming_seq != self.expected_next_seq && self.expected_next_seq != 0 {
            // TCP sequence gap detected. Network frame dropped.
            // Entering Amnesia State immediately.
            self.state = SyncState::Amnesia;
            return false;
        }

        self.expected_next_seq = incoming_seq + 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_sequence() {
        let mut seq = Sequencer::new();
        assert!(seq.validate_tick_sequence(100));
        assert!(seq.validate_tick_sequence(101));
        assert!(seq.validate_tick_sequence(102));
        assert_eq!(seq.state, SyncState::Valid);
    }

    #[test]
    fn test_amnesia_trigger_on_gap() {
        let mut seq = Sequencer::new();
        assert!(seq.validate_tick_sequence(100));
        
        // Simulating a missed UDP packet or dropped TCP frame from the relayer
        let ok = seq.validate_tick_sequence(102); 
        
        assert!(!ok, "Sequencer failed to catch the dropped frame!");
        assert_eq!(seq.state, SyncState::Amnesia, "System did not enter Amnesia State!");
        
        // Further ticks should be ignored until OOB rest recovery happens
        let ok_after = seq.validate_tick_sequence(103);
        assert!(!ok_after, "System processed a frame while in Amnesia State!");
    }
}
