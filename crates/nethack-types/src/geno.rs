use bitflags::bitflags;
use serde::Serialize;

bitflags! {
    /// Generation/genocide flags from `monflag.h` (G_* constants).
    /// Stored in `permonst.geno`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
    pub struct GenoFlags: u16 {
        const UNIQ     = 0x1000;
        const NOHELL   = 0x0800;
        const HELL     = 0x0400;
        const NOGEN    = 0x0200;
        const SGROUP   = 0x0080;
        const LGROUP   = 0x0040;
        const GENO     = 0x0020;
        const NOCORPSE = 0x0010;
        const FREQ     = 0x0007;
    }
}

impl GenoFlags {
    /// Extract the creation frequency (bottom 3 bits).
    pub const fn frequency(self) -> u16 {
        self.bits() & Self::FREQ.bits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values() {
        assert_eq!(GenoFlags::UNIQ.bits(), 0x1000);
        assert_eq!(GenoFlags::NOHELL.bits(), 0x0800);
        assert_eq!(GenoFlags::HELL.bits(), 0x0400);
        assert_eq!(GenoFlags::NOGEN.bits(), 0x0200);
        assert_eq!(GenoFlags::SGROUP.bits(), 0x0080);
        assert_eq!(GenoFlags::LGROUP.bits(), 0x0040);
        assert_eq!(GenoFlags::GENO.bits(), 0x0020);
        assert_eq!(GenoFlags::NOCORPSE.bits(), 0x0010);
    }

    #[test]
    fn frequency_extraction() {
        let flags = GenoFlags::from_bits_truncate(0x1023); // UNIQ | GENO | freq=3
        assert_eq!(flags.frequency(), 3);
    }
}
