use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};

/// Monster sizes from `monflag.h` (MZ_* constants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum MonsterSize {
    Tiny = 0,
    Small = 1,
    Medium = 2,
    Large = 3,
    Huge = 4,
    Gigantic = 7,
}

impl MonsterSize {
    /// MZ_HUMAN is an alias for MZ_MEDIUM.
    pub const HUMAN: Self = Self::Medium;
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn count() {
        assert_eq!(MonsterSize::COUNT, 6);
    }

    #[test]
    fn discriminants() {
        assert_eq!(MonsterSize::Tiny as u8, 0);
        assert_eq!(MonsterSize::Small as u8, 1);
        assert_eq!(MonsterSize::Medium as u8, 2);
        assert_eq!(MonsterSize::Large as u8, 3);
        assert_eq!(MonsterSize::Huge as u8, 4);
        assert_eq!(MonsterSize::Gigantic as u8, 7);
    }

    #[test]
    fn human_alias() {
        assert_eq!(MonsterSize::HUMAN, MonsterSize::Medium);
    }

    #[test]
    fn round_trip() {
        for ms in MonsterSize::iter() {
            assert_eq!(MonsterSize::from_repr(ms as u8), Some(ms));
        }
    }
}
