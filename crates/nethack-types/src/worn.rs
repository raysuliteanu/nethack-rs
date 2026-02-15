use bitflags::bitflags;
use serde::Serialize;

bitflags! {
    /// Worn/wielded equipment mask from `prop.h` (W_* constants).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
    pub struct WornMask: u32 {
        // Armor
        const ARM     = 0x0000_0001;
        const ARMC    = 0x0000_0002;
        const ARMH    = 0x0000_0004;
        const ARMS    = 0x0000_0008;
        const ARMG    = 0x0000_0010;
        const ARMF    = 0x0000_0020;
        const ARMU    = 0x0000_0040;
        const ARMOR   = 0x0000_007F;
        // Weapons
        const WEP     = 0x0000_0100;
        const QUIVER  = 0x0000_0200;
        const SWAPWEP = 0x0000_0400;
        const WEAPONS = 0x0000_0700;
        // Artifacts
        const ART     = 0x0000_1000;
        const ARTI    = 0x0000_2000;
        // Accessories
        const AMUL    = 0x0001_0000;
        const RINGL   = 0x0002_0000;
        const RINGR   = 0x0004_0000;
        const RING    = 0x0006_0000;
        const TOOL    = 0x0008_0000;
        const ACCESSORY = 0x000F_0000;
        // Other
        const SADDLE  = 0x0010_0000;
        const BALL    = 0x0020_0000;
        const CHAIN   = 0x0040_0000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armor_composite() {
        assert_eq!(
            WornMask::ARMOR,
            WornMask::ARM
                | WornMask::ARMC
                | WornMask::ARMH
                | WornMask::ARMS
                | WornMask::ARMG
                | WornMask::ARMF
                | WornMask::ARMU
        );
    }

    #[test]
    fn weapons_composite() {
        assert_eq!(
            WornMask::WEAPONS,
            WornMask::WEP | WornMask::SWAPWEP | WornMask::QUIVER
        );
    }

    #[test]
    fn ring_composite() {
        assert_eq!(WornMask::RING, WornMask::RINGL | WornMask::RINGR);
    }

    #[test]
    fn accessory_composite() {
        assert_eq!(
            WornMask::ACCESSORY,
            WornMask::RING | WornMask::AMUL | WornMask::TOOL
        );
    }
}
