use bitflags::bitflags;
use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};

/// Alignment type matching NetHack's `aligntyp` (signed byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(i8)]
pub enum Alignment {
    None = -128, // A_NONE
    Chaotic = -1,
    Neutral = 0,
    Lawful = 1,
}

impl Alignment {
    /// Convert from alignment mask to alignment type.
    pub fn from_mask(mask: AlignmentMask) -> Self {
        if mask.is_empty() {
            Self::None
        } else if mask.contains(AlignmentMask::LAWFUL) {
            Self::Lawful
        } else {
            // AM_CHAOTIC(1) -> -1, AM_NEUTRAL(2) -> 0
            Self::from_repr(mask.bits() as i8 - 2).unwrap_or(Self::None)
        }
    }

    /// Convert alignment type to alignment mask.
    pub fn to_mask(self) -> AlignmentMask {
        match self {
            Self::None => AlignmentMask::NONE,
            Self::Lawful => AlignmentMask::LAWFUL,
            _ => AlignmentMask::from_bits_truncate((self as i8 + 2) as u8),
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
    pub struct AlignmentMask: u8 {
        const NONE    = 0;
        const CHAOTIC = 1;
        const NEUTRAL = 2;
        const LAWFUL  = 4;
        const MASK    = 7;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discriminants() {
        assert_eq!(Alignment::None as i8, -128);
        assert_eq!(Alignment::Chaotic as i8, -1);
        assert_eq!(Alignment::Neutral as i8, 0);
        assert_eq!(Alignment::Lawful as i8, 1);
    }

    #[test]
    fn mask_values() {
        assert_eq!(AlignmentMask::CHAOTIC.bits(), 1);
        assert_eq!(AlignmentMask::NEUTRAL.bits(), 2);
        assert_eq!(AlignmentMask::LAWFUL.bits(), 4);
        assert_eq!(AlignmentMask::MASK.bits(), 7);
    }

    #[test]
    fn mask_round_trip() {
        for a in [
            Alignment::None,
            Alignment::Chaotic,
            Alignment::Neutral,
            Alignment::Lawful,
        ] {
            assert_eq!(Alignment::from_mask(a.to_mask()), a);
        }
    }

    #[test]
    fn count() {
        assert_eq!(Alignment::COUNT, 4);
    }
}
