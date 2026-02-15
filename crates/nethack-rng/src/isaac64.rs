// Direct port of NetHack's ISAAC64 implementation (CC0 licensed).
// Original by Timothy B. Terriberry, based on Robert J. Jenkins Jr.'s ISAAC.
//
// We port this directly rather than using `rand_isaac` because NetHack uses
// a custom 8-byte little-endian seeding convention that must be matched exactly
// for save-game and replay compatibility.

const SZ_LOG: u32 = 8;
const SZ: usize = 1 << SZ_LOG; // 256
const SEED_SZ_MAX: usize = SZ << 3; // 2048

#[derive(Clone)]
pub struct Isaac64Ctx {
    n: usize,
    r: [u64; SZ],
    m: [u64; SZ],
    a: u64,
    b: u64,
    c: u64,
}

impl Isaac64Ctx {
    pub fn new(seed: &[u8]) -> Self {
        let mut ctx = Self {
            n: 0,
            r: [0; SZ],
            m: [0; SZ],
            a: 0,
            b: 0,
            c: 0,
        };
        ctx.reseed(seed);
        ctx
    }

    pub fn reseed(&mut self, seed: &[u8]) {
        let nseed = seed.len().min(SEED_SZ_MAX);

        // XOR seed bytes into r[] as little-endian u64s
        let full_words = nseed >> 3;
        for i in 0..full_words {
            let base = i << 3;
            self.r[i] ^= (seed[base | 7] as u64) << 56
                | (seed[base | 6] as u64) << 48
                | (seed[base | 5] as u64) << 40
                | (seed[base | 4] as u64) << 32
                | (seed[base | 3] as u64) << 24
                | (seed[base | 2] as u64) << 16
                | (seed[base | 1] as u64) << 8
                | (seed[base] as u64);
        }

        let remaining = nseed - (full_words << 3);
        if remaining > 0 {
            let base = full_words << 3;
            let mut ri = seed[base] as u64;
            for j in 1..remaining {
                ri |= (seed[base | j] as u64) << (j << 3);
            }
            self.r[full_words] ^= ri;
        }

        let mut x = [0x9E37_79B9_7F4A_7C13u64; 8];
        for _ in 0..4 {
            mix(&mut x);
        }
        for i in (0..SZ).step_by(8) {
            for (xj, rj) in x.iter_mut().zip(&self.r[i..i + 8]) {
                *xj = xj.wrapping_add(*rj);
            }
            mix(&mut x);
            self.m[i..i + 8].copy_from_slice(&x);
        }
        for i in (0..SZ).step_by(8) {
            for (xj, mj) in x.iter_mut().zip(self.m[i..i + 8].iter()) {
                *xj = xj.wrapping_add(*mj);
            }
            mix(&mut x);
            self.m[i..i + 8].copy_from_slice(&x);
        }
        self.update();
    }

    fn update(&mut self) {
        let a_ref = &mut self.a;
        let b_ref = &mut self.b;
        let c_ref = &mut self.c;
        let m = &mut self.m;
        let r = &mut self.r;

        *c_ref = c_ref.wrapping_add(1);
        *b_ref = b_ref.wrapping_add(*c_ref);

        let mut a = *a_ref;
        let mut b = *b_ref;

        for i in (0..SZ / 2).step_by(4) {
            let mut step = |i: usize, mix_op: fn(u64) -> u64, offset: usize| {
                let x = m[i];
                a = mix_op(a).wrapping_add(m[i + offset]);
                let y = m[lower_bits(x)].wrapping_add(a).wrapping_add(b);
                m[i] = y;
                b = m[upper_bits(y)].wrapping_add(x);
                r[i] = b;
            };
            step(i, |a| !a ^ (a << 21), SZ / 2);
            step(i + 1, |a| a ^ (a >> 5), SZ / 2);
            step(i + 2, |a| a ^ (a << 12), SZ / 2);
            step(i + 3, |a| a ^ (a >> 33), SZ / 2);
        }
        for i in (SZ / 2..SZ).step_by(4) {
            let mut step = |i: usize, mix_op: fn(u64) -> u64| {
                let x = m[i];
                a = mix_op(a).wrapping_add(m[i - SZ / 2]);
                let y = m[lower_bits(x)].wrapping_add(a).wrapping_add(b);
                m[i] = y;
                b = m[upper_bits(y)].wrapping_add(x);
                r[i] = b;
            };
            step(i, |a| !a ^ (a << 21));
            step(i + 1, |a| a ^ (a >> 5));
            step(i + 2, |a| a ^ (a << 12));
            step(i + 3, |a| a ^ (a >> 33));
        }

        *b_ref = b;
        *a_ref = a;
        self.n = SZ;
    }

    pub fn next_u64(&mut self) -> u64 {
        if self.n == 0 {
            self.update();
        }
        self.n -= 1;
        self.r[self.n]
    }
}

fn lower_bits(x: u64) -> usize {
    ((x & (((SZ as u64) - 1) << 3)) >> 3) as usize
}

fn upper_bits(y: u64) -> usize {
    ((y >> (SZ_LOG + 3)) & ((SZ as u64) - 1)) as usize
}

fn mix(x: &mut [u64; 8]) {
    const SHIFT: [u8; 8] = [9, 9, 23, 15, 14, 20, 17, 14];
    let mut i = 0;
    while i < 8 {
        x[i] = x[i].wrapping_sub(x[(i + 4) & 7]);
        x[(i + 5) & 7] ^= x[(i + 7) & 7] >> SHIFT[i];
        x[(i + 7) & 7] = x[(i + 7) & 7].wrapping_add(x[i]);
        i += 1;
        x[i] = x[i].wrapping_sub(x[(i + 4) & 7]);
        x[(i + 5) & 7] ^= x[(i + 7) & 7] << SHIFT[i];
        x[(i + 7) & 7] = x[(i + 7) & 7].wrapping_add(x[i]);
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_from_u64(seed: u64) -> [u8; 8] {
        seed.to_le_bytes()
    }

    #[test]
    fn raw_values_seed_42() {
        let mut ctx = Isaac64Ctx::new(&seed_from_u64(42));
        let expected: [u64; 20] = [
            13535040523913025898,
            11186036148076763066,
            17457813421150709648,
            14433197483349118045,
            7996039696826744184,
            8587010431704612506,
            11495013891180058003,
            6278830536540527714,
            3546132840364682437,
            17921203538582169288,
            12251707510711238641,
            13463295173609305520,
            12992402865462392704,
            4264784159588175189,
            2307885616746873304,
            7202578636770154407,
            8163890545887848702,
            3305014197741373632,
            5796535348653175950,
            9727585054239591942,
        ];
        for (i, &e) in expected.iter().enumerate() {
            let val = ctx.next_u64();
            assert_eq!(val, e, "mismatch at index {i}");
        }
    }

    #[test]
    fn raw_values_seed_0() {
        let mut ctx = Isaac64Ctx::new(&seed_from_u64(0));
        let expected: [u64; 20] = [
            11329126462075137345,
            3096006490854172103,
            4961560858198160711,
            11247167491742853858,
            8467686926187236489,
            3643601464190828991,
            1133690081497064057,
            16733846313379782858,
            972344712846728208,
            1875810966947487789,
            10810281711139472304,
            14997549008232787669,
            4665150172008230450,
            77499164859392917,
            6752165915987794405,
            2566923340161161676,
            419294011261754017,
            7466832458773678449,
            8379435287740149003,
            9012210492721573360,
        ];
        for (i, &e) in expected.iter().enumerate() {
            let val = ctx.next_u64();
            assert_eq!(val, e, "mismatch at index {i}");
        }
    }

    #[test]
    fn raw_values_seed_12345() {
        let mut ctx = Isaac64Ctx::new(&seed_from_u64(12345));
        let expected: [u64; 20] = [
            16749476496145720041,
            9916843529103186837,
            11968398467845635923,
            9337830697406450407,
            14531341148415096772,
            14306891581045654757,
            15746316097709038996,
            17219806417372584951,
            18413492739537913731,
            10153407053400273637,
            18042341210233986910,
            10590263203604389309,
            17852923035898560976,
            4411930199927605008,
            10997894802228112035,
            17024367307687391252,
            13212968853541836851,
            15120059102248361683,
            3249521119583917580,
            1880351232509086725,
        ];
        for (i, &e) in expected.iter().enumerate() {
            let val = ctx.next_u64();
            assert_eq!(val, e, "mismatch at index {i}");
        }
    }
}
