use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};

/// Monster class symbols from `monsym.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum MonsterClass {
    Ant = 1,
    Blob = 2,
    Cockatrice = 3,
    Dog = 4,
    Eye = 5,
    Feline = 6,
    Gremlin = 7,
    Humanoid = 8,
    Imp = 9,
    Jelly = 10,
    Kobold = 11,
    Leprechaun = 12,
    Mimic = 13,
    Nymph = 14,
    Orc = 15,
    Piercer = 16,
    Quadruped = 17,
    Rodent = 18,
    Spider = 19,
    Trapper = 20,
    Unicorn = 21,
    Vortex = 22,
    Worm = 23,
    Xan = 24,
    Light = 25,
    Zruty = 26,
    Angel = 27,
    Bat = 28,
    Centaur = 29,
    Dragon = 30,
    Elemental = 31,
    Fungus = 32,
    Gnome = 33,
    Giant = 34,
    Invisible = 35,
    Jabberwock = 36,
    Kop = 37,
    Lich = 38,
    Mummy = 39,
    Naga = 40,
    Ogre = 41,
    Pudding = 42,
    QuantumMechanic = 43,
    RustMonster = 44,
    Snake = 45,
    Troll = 46,
    Umber = 47,
    Vampire = 48,
    Wraith = 49,
    Xorn = 50,
    Yeti = 51,
    Zombie = 52,
    Human = 53,
    Ghost = 54,
    Golem = 55,
    Demon = 56,
    Eel = 57,
    Lizard = 58,
    WormTail = 59,
    MimicDef = 60,
}

impl MonsterClass {
    pub const MAX: usize = 61;

    /// Default display symbol for this monster class.
    pub const fn default_symbol(self) -> char {
        match self {
            Self::Ant => 'a',
            Self::Blob => 'b',
            Self::Cockatrice => 'c',
            Self::Dog => 'd',
            Self::Eye => 'e',
            Self::Feline => 'f',
            Self::Gremlin => 'g',
            Self::Humanoid => 'h',
            Self::Imp => 'i',
            Self::Jelly => 'j',
            Self::Kobold => 'k',
            Self::Leprechaun => 'l',
            Self::Mimic => 'm',
            Self::Nymph => 'n',
            Self::Orc => 'o',
            Self::Piercer => 'p',
            Self::Quadruped => 'q',
            Self::Rodent => 'r',
            Self::Spider => 's',
            Self::Trapper => 't',
            Self::Unicorn => 'u',
            Self::Vortex => 'v',
            Self::Worm => 'w',
            Self::Xan => 'x',
            Self::Light => 'y',
            Self::Zruty => 'z',
            Self::Angel => 'A',
            Self::Bat => 'B',
            Self::Centaur => 'C',
            Self::Dragon => 'D',
            Self::Elemental => 'E',
            Self::Fungus => 'F',
            Self::Gnome => 'G',
            Self::Giant => 'H',
            Self::Invisible => 'I',
            Self::Jabberwock => 'J',
            Self::Kop => 'K',
            Self::Lich => 'L',
            Self::Mummy => 'M',
            Self::Naga => 'N',
            Self::Ogre => 'O',
            Self::Pudding => 'P',
            Self::QuantumMechanic => 'Q',
            Self::RustMonster => 'R',
            Self::Snake => 'S',
            Self::Troll => 'T',
            Self::Umber => 'U',
            Self::Vampire => 'V',
            Self::Wraith => 'W',
            Self::Xorn => 'X',
            Self::Yeti => 'Y',
            Self::Zombie => 'Z',
            Self::Human => '@',
            Self::Ghost => ' ',
            Self::Golem => '\'',
            Self::Demon => '&',
            Self::Eel => ';',
            Self::Lizard => ':',
            Self::WormTail => '~',
            Self::MimicDef => ']',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn count() {
        assert_eq!(MonsterClass::COUNT, 60); // 60 variants (1..=60)
    }

    #[test]
    fn discriminants() {
        assert_eq!(MonsterClass::Ant as u8, 1);
        assert_eq!(MonsterClass::Lizard as u8, 58);
        assert_eq!(MonsterClass::MimicDef as u8, 60);
    }

    #[test]
    fn round_trip() {
        for mc in MonsterClass::iter() {
            assert_eq!(MonsterClass::from_repr(mc as u8), Some(mc));
        }
    }

    #[test]
    fn symbols() {
        assert_eq!(MonsterClass::Ant.default_symbol(), 'a');
        assert_eq!(MonsterClass::Human.default_symbol(), '@');
        assert_eq!(MonsterClass::Demon.default_symbol(), '&');
        assert_eq!(MonsterClass::Ghost.default_symbol(), ' ');
    }
}
