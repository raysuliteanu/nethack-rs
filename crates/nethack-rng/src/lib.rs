mod isaac64;

use isaac64::Isaac64Ctx;

/// Dual-stream RNG matching NetHack's ISAAC64-based random number generation.
///
/// NetHack maintains two independent RNG streams:
/// - **Core**: drives all gameplay-affecting randomness
/// - **Display**: drives cosmetic randomness (so players can't manipulate the core RNG
///   by triggering display refreshes)
#[derive(Clone)]
pub struct NhRng {
    core: Isaac64Ctx,
    display: Isaac64Ctx,
}

impl NhRng {
    /// Create a new dual-stream RNG seeded with `seed`.
    ///
    /// Both streams are seeded with the same value, matching NetHack's behavior
    /// where `init_isaac64` is called separately for core and display.
    pub fn new(seed: u64) -> Self {
        let seed_bytes = seed.to_le_bytes();
        Self {
            core: Isaac64Ctx::new(&seed_bytes),
            display: Isaac64Ctx::new(&seed_bytes),
        }
    }

    /// Create with separate seeds for core and display streams.
    pub fn new_dual(core_seed: u64, display_seed: u64) -> Self {
        Self {
            core: Isaac64Ctx::new(&core_seed.to_le_bytes()),
            display: Isaac64Ctx::new(&display_seed.to_le_bytes()),
        }
    }

    /// `0 <= rn2(x) < x` — uniform random integer on the core stream.
    pub fn rn2(&mut self, x: i32) -> i32 {
        if x <= 0 {
            log::warn!("rn2({x}) attempted");
            return 0;
        }
        (self.core.next_u64() % x as u64) as i32
    }

    /// `0 <= rn2_on_display_rng(x) < x` — uniform random on the display stream.
    pub fn rn2_on_display_rng(&mut self, x: i32) -> i32 {
        if x <= 0 {
            log::warn!("rn2_on_display_rng({x}) attempted");
            return 0;
        }
        (self.display.next_u64() % x as u64) as i32
    }

    /// `1 <= rnd(x) <= x` — uniform random integer.
    pub fn rnd(&mut self, x: i32) -> i32 {
        if x <= 0 {
            log::warn!("rnd({x}) attempted");
            return 1;
        }
        (self.core.next_u64() % x as u64) as i32 + 1
    }

    /// `n <= d(n, x) <= n*x` — sum of n rolls of a d-x die.
    pub fn d(&mut self, n: i32, x: i32) -> i32 {
        if x < 0 || n < 0 || (x == 0 && n != 0) {
            log::warn!("d({n},{x}) attempted");
            return 1;
        }
        // C implementation: tmp = n; while(n--) tmp += RND(x); return tmp;
        let mut tmp = n;
        for _ in 0..n {
            tmp += (self.core.next_u64() % x as u64) as i32;
        }
        tmp
    }

    /// Luck-adjusted random: good luck biases toward 0, bad luck toward x-1.
    pub fn rnl(&mut self, x: i32, luck: i32) -> i32 {
        if x <= 0 {
            log::warn!("rnl({x}) attempted");
            return 0;
        }

        let adjustment = if x <= 15 {
            // For small ranges, use luck/3 rounded away from 0
            (luck.abs() + 1) / 3 * luck.signum()
        } else {
            luck
        };

        let mut i = self.rn2(x);
        if adjustment != 0 && self.rn2(37 + adjustment.abs()) != 0 {
            i -= adjustment;
            if i < 0 {
                i = 0;
            } else if i >= x {
                i = x - 1;
            }
        }
        i
    }

    /// Experience-scaled random: `1 <= rne(x) <= max(ulevel/3, 5)`.
    pub fn rne(&mut self, x: i32, ulevel: i32) -> i32 {
        let utmp = if ulevel < 15 { 5 } else { ulevel / 3 };
        let mut tmp = 1;
        while tmp < utmp && self.rn2(x) == 0 {
            tmp += 1;
        }
        tmp
    }

    /// Timeout-scaling random (rnz).
    pub fn rnz(&mut self, i: i32) -> i32 {
        let mut x = i as i64;
        let mut tmp = 1000i64 + self.rn2(1000) as i64;
        tmp *= self.rne(4, 1) as i64; // rne uses ulevel; C code uses u.ulevel
        if self.rn2(2) != 0 {
            x = x * tmp / 1000;
        } else {
            x = x * 1000 / tmp;
        }
        x as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rn2_matches_c_seed_42() {
        let mut rng = NhRng::new(42);
        let expected = [
            98, 66, 48, 45, 84, 6, 3, 14, 37, 88, 41, 20, 4, 89, 4, 7, 2, 32, 50, 42,
        ];
        for (i, &e) in expected.iter().enumerate() {
            assert_eq!(rng.rn2(100), e, "rn2(100) mismatch at index {i}");
        }
    }

    #[test]
    fn rn2_matches_c_seed_0() {
        let mut rng = NhRng::new(0);
        let expected = [
            45, 3, 11, 58, 89, 91, 57, 58, 8, 89, 4, 69, 50, 17, 5, 76, 17, 49, 3, 60,
        ];
        for (i, &e) in expected.iter().enumerate() {
            assert_eq!(rng.rn2(100), e, "rn2(100) mismatch at index {i}");
        }
    }

    #[test]
    fn rn2_matches_c_seed_12345() {
        let mut rng = NhRng::new(12345);
        let expected = [
            41, 37, 23, 7, 72, 57, 96, 51, 31, 37, 10, 9, 76, 8, 35, 52, 51, 83, 80, 25,
        ];
        for (i, &e) in expected.iter().enumerate() {
            assert_eq!(rng.rn2(100), e, "rn2(100) mismatch at index {i}");
        }
    }

    #[test]
    fn rn2_range() {
        let mut rng = NhRng::new(42);
        for _ in 0..1000 {
            let v = rng.rn2(50);
            assert!(v >= 0 && v < 50, "rn2(50) = {v} out of range [0, 50)");
        }
    }

    #[test]
    fn rnd_range() {
        let mut rng = NhRng::new(42);
        for _ in 0..1000 {
            let v = rng.rnd(6);
            assert!(v >= 1 && v <= 6, "rnd(6) = {v} out of range [1, 6]");
        }
    }

    #[test]
    fn d_range() {
        let mut rng = NhRng::new(42);
        for _ in 0..1000 {
            let v = rng.d(3, 6);
            assert!(v >= 3 && v <= 18, "d(3,6) = {v} out of range [3, 18]");
        }
    }

    #[test]
    fn dual_stream_independence() {
        let mut rng1 = NhRng::new(42);
        let mut rng2 = NhRng::new(42);

        // Consume some display values on rng1
        for _ in 0..10 {
            rng1.rn2_on_display_rng(100);
        }

        // Core stream should still match
        for _ in 0..20 {
            assert_eq!(rng1.rn2(100), rng2.rn2(100));
        }
    }

    #[test]
    fn determinism() {
        let mut rng1 = NhRng::new(999);
        let mut rng2 = NhRng::new(999);
        for _ in 0..100 {
            assert_eq!(rng1.rn2(1000), rng2.rn2(1000));
        }
    }

    #[test]
    fn rn2_invalid_returns_zero() {
        let mut rng = NhRng::new(42);
        assert_eq!(rng.rn2(0), 0);
        assert_eq!(rng.rn2(-5), 0);
    }

    #[test]
    fn rnd_invalid_returns_one() {
        let mut rng = NhRng::new(42);
        assert_eq!(rng.rnd(0), 1);
        assert_eq!(rng.rnd(-1), 1);
    }

    #[test]
    fn d_invalid_returns_one() {
        let mut rng = NhRng::new(42);
        assert_eq!(rng.d(-1, 6), 1);
        assert_eq!(rng.d(1, -1), 1);
    }

    #[test]
    fn rnl_range() {
        let mut rng = NhRng::new(42);
        for luck in -13..=13 {
            for _ in 0..100 {
                let v = rng.rnl(20, luck);
                assert!(v >= 0 && v < 20, "rnl(20, {luck}) = {v} out of range");
            }
        }
    }

    #[test]
    fn rne_range() {
        let mut rng = NhRng::new(42);
        for _ in 0..1000 {
            let v = rng.rne(3, 10);
            assert!(v >= 1 && v <= 5, "rne(3, 10) = {v} out of range [1, 5]");
        }
    }
}
