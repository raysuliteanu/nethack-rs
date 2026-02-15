use bitflags::bitflags;
use serde::Serialize;

bitflags! {
    /// Monster resistances from `monflag.h` (MR_* constants).
    /// These are stored in `permonst.mresists` and `permonst.mconveys`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
    pub struct Resistance: u8 {
        const FIRE   = 0x01;
        const COLD   = 0x02;
        const SLEEP  = 0x04;
        const DISINT = 0x08;
        const ELEC   = 0x10;
        const POISON = 0x20;
        const ACID   = 0x40;
        const STONE  = 0x80;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values() {
        assert_eq!(Resistance::FIRE.bits(), 0x01);
        assert_eq!(Resistance::STONE.bits(), 0x80);
        assert_eq!(Resistance::all().bits(), 0xFF);
    }

    #[test]
    fn combinations() {
        let fire_cold = Resistance::FIRE | Resistance::COLD;
        assert!(fire_cold.contains(Resistance::FIRE));
        assert!(fire_cold.contains(Resistance::COLD));
        assert!(!fire_cold.contains(Resistance::ELEC));
    }
}
