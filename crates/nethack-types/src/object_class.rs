use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};

/// Object class types from `objclass.h` (enum obj_class_types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum ObjectClass {
    Random = 0,
    IllObj = 1,
    Weapon = 2,
    Armor = 3,
    Ring = 4,
    Amulet = 5,
    Tool = 6,
    Food = 7,
    Potion = 8,
    Scroll = 9,
    SpellBook = 10,
    Wand = 11,
    Coin = 12,
    Gem = 13,
    Rock = 14,
    Ball = 15,
    Chain = 16,
    Venom = 17,
}

impl ObjectClass {
    pub const MAX: usize = 18;

    /// Default display symbol for this object class.
    pub const fn symbol(self) -> char {
        match self {
            Self::Random => '\0',
            Self::IllObj => ']',
            Self::Weapon => ')',
            Self::Armor => '[',
            Self::Ring => '=',
            Self::Amulet => '"',
            Self::Tool => '(',
            Self::Food => '%',
            Self::Potion => '!',
            Self::Scroll => '?',
            Self::SpellBook => '+',
            Self::Wand => '/',
            Self::Coin => '$',
            Self::Gem => '*',
            Self::Rock => '`',
            Self::Ball => '0',
            Self::Chain => '_',
            Self::Venom => '.',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn count() {
        assert_eq!(ObjectClass::COUNT, 18);
    }

    #[test]
    fn discriminants() {
        assert_eq!(ObjectClass::Random as u8, 0);
        assert_eq!(ObjectClass::Weapon as u8, 2);
        assert_eq!(ObjectClass::Venom as u8, 17);
    }

    #[test]
    fn symbols() {
        assert_eq!(ObjectClass::Weapon.symbol(), ')');
        assert_eq!(ObjectClass::Armor.symbol(), '[');
        assert_eq!(ObjectClass::Coin.symbol(), '$');
    }

    #[test]
    fn round_trip() {
        for oc in ObjectClass::iter() {
            assert_eq!(ObjectClass::from_repr(oc as u8), Some(oc));
        }
    }
}
