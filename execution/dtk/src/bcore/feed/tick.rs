use crate::bcore::features::fixed_point::Fixed;

#[derive(Clone, Default)]
pub struct Tick {
    pub seq: u64,
    pub source_id: u16,
    pub ts_mono_ns: u64,
    pub raw_hash: [u8; 32],
    pub price: Fixed,
    pub size: Fixed,
}
