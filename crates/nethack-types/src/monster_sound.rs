use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};

/// Monster sounds from `monflag.h` (MS_* constants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum MonsterSound {
    Silent = 0,
    Bark = 1,
    Mew = 2,
    Roar = 3,
    Growl = 4,
    Squeak = 5,
    Squawk = 6,
    Hiss = 7,
    Buzz = 8,
    Grunt = 9,
    Neigh = 10,
    Wail = 11,
    Gurgle = 12,
    Burble = 13,
    // MS_ANIMAL = 13 (alias for Burble, marks end of animal sounds)
    // 14 unused
    Shriek = 15,
    Bones = 16,
    Laugh = 17,
    Mumble = 18,
    Imitate = 19,
    Humanoid = 20,
    Arrest = 21,
    Soldier = 22,
    Guard = 23,
    Djinni = 24,
    Nurse = 25,
    SeduceSound = 26,
    Vampire = 27,
    Bribe = 28,
    Cuss = 29,
    Rider = 30,
    Leader = 31,
    Nemesis = 32,
    Guardian = 33,
    Sell = 34,
    Oracle = 35,
    Priest = 36,
    Spell = 37,
    Were = 38,
    Boast = 39,
}

impl MonsterSound {
    /// MS_ANIMAL marks the boundary for animal noises.
    pub const ANIMAL: u8 = 13;

    /// MS_ORC is an alias for MS_GRUNT.
    pub const ORC: Self = Self::Grunt;
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn count() {
        assert_eq!(MonsterSound::COUNT, 39);
    }

    #[test]
    fn discriminants() {
        assert_eq!(MonsterSound::Silent as u8, 0);
        assert_eq!(MonsterSound::Burble as u8, 13);
        assert_eq!(MonsterSound::Shriek as u8, 15);
        assert_eq!(MonsterSound::Boast as u8, 39);
    }

    #[test]
    fn orc_alias() {
        assert_eq!(MonsterSound::ORC, MonsterSound::Grunt);
    }

    #[test]
    fn round_trip() {
        for ms in MonsterSound::iter() {
            assert_eq!(MonsterSound::from_repr(ms as u8), Some(ms));
        }
    }
}
