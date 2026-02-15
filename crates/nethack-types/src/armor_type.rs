use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};

/// Armor sub-types from `objclass.h` (enum obj_armor_types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum ArmorType {
    Suit = 0,
    Shield = 1,
    Helm = 2,
    Gloves = 3,
    Boots = 4,
    Cloak = 5,
    Shirt = 6,
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn count() {
        assert_eq!(ArmorType::COUNT, 7);
    }

    #[test]
    fn discriminants() {
        assert_eq!(ArmorType::Suit as u8, 0);
        assert_eq!(ArmorType::Shield as u8, 1);
        assert_eq!(ArmorType::Shirt as u8, 6);
    }

    #[test]
    fn round_trip() {
        for at in ArmorType::iter() {
            assert_eq!(ArmorType::from_repr(at as u8), Some(at));
        }
    }
}
