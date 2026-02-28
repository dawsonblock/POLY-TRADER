// core/features/fixed_point.rs
// Q32.32 Fixed-Point Arithmetic for Absolute Determinism

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Fixed(pub i64);

impl Fixed {
    pub const SCALE: i64 = 1 << 32;

    pub fn from_f64(v: f64) -> Self {
        Fixed((v * Self::SCALE as f64) as i64)
    }

    pub fn to_f64(self) -> f64 {
        self.0 as f64 / Self::SCALE as f64
    }

    pub fn add(self, other: Self) -> Self {
        Fixed(self.0.wrapping_add(other.0))
    }

    pub fn sub(self, other: Self) -> Self {
        Fixed(self.0.wrapping_sub(other.0))
    }

    pub fn mul(self, other: Self) -> Self {
        let tmp = (self.0 as i128 * other.0 as i128) >> 32;
        Fixed(tmp as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_point_determinism() {
        let a = Fixed::from_f64(1.5);
        let b = Fixed::from_f64(2.0);
        
        let c = a.add(b);
        assert_eq!(c.to_f64(), 3.5, "Addition breached determinism.");

        let d = a.mul(b);
        assert_eq!(d.to_f64(), 3.0, "Multiplication breached Q32.32 bounds.");
        
        let sub = b.sub(a);
        assert_eq!(sub.to_f64(), 0.5, "Subtraction error.");
    }

    #[test]
    fn test_fp_overflow_wrapping() {
        let max = Fixed(i64::MAX);
        let wrapped = max.add(Fixed(1));
        assert_eq!(wrapped.0, i64::MIN, "System must wrap deterministically rather than panic dynamically.");
    }
}
