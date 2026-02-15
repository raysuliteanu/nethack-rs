use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};

/// Attack types from `monattk.h` (AT_* constants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum AttackType {
    None = 0,
    Claw = 1,
    Bite = 2,
    Kick = 3,
    Butt = 4,
    Touch = 5,
    Sting = 6,
    Hugs = 7,
    // 8-9 unused
    Spit = 10,
    Engulf = 11,
    Breath = 12,
    Explode = 13,
    Boom = 14,
    Gaze = 15,
    Tentacle = 16,
    // gap
    Weapon = 254,
    Magic = 255,
}

/// Damage types from `monattk.h` (AD_* constants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum DamageType {
    Physical = 0,
    MagicMissile = 1,
    Fire = 2,
    Cold = 3,
    Sleep = 4,
    Disintegration = 5,
    Electric = 6,
    DrainStr = 7,
    Acid = 8,
    Spc1 = 9,
    Spc2 = 10,
    Blind = 11,
    Stun = 12,
    Slow = 13,
    Paralyze = 14,
    DrainLife = 15,
    DrainEnergy = 16,
    Legs = 17,
    Stone = 18,
    Stick = 19,
    StealGold = 20,
    StealItem = 21,
    Seduce = 22,
    Teleport = 23,
    Rust = 24,
    Confuse = 25,
    Digest = 26,
    Heal = 27,
    Wrap = 28,
    Were = 29,
    DrainDex = 30,
    DrainCon = 31,
    DrainInt = 32,
    Disease = 33,
    Decay = 34,
    SuccubusSeduction = 35,
    Hallucination = 36,
    Death = 37,
    Pestilence = 38,
    Famine = 39,
    Slime = 40,
    Disenchant = 41,
    Corrode = 42,
    // gap
    Clerical = 240,
    Spell = 241,
    RandomBreath = 242,
    // gap
    StealAmulet = 252,
    Curse = 253,
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn attack_type_discriminants() {
        assert_eq!(AttackType::None as u8, 0);
        assert_eq!(AttackType::Claw as u8, 1);
        assert_eq!(AttackType::Butt as u8, 4);
        assert_eq!(AttackType::Spit as u8, 10);
        assert_eq!(AttackType::Weapon as u8, 254);
        assert_eq!(AttackType::Magic as u8, 255);
    }

    #[test]
    fn damage_type_discriminants() {
        assert_eq!(DamageType::Physical as u8, 0);
        assert_eq!(DamageType::Fire as u8, 2);
        assert_eq!(DamageType::Corrode as u8, 42);
        assert_eq!(DamageType::Clerical as u8, 240);
        assert_eq!(DamageType::StealAmulet as u8, 252);
        assert_eq!(DamageType::Curse as u8, 253);
    }

    #[test]
    fn round_trip_attack() {
        for at in AttackType::iter() {
            assert_eq!(AttackType::from_repr(at as u8), Some(at));
        }
    }

    #[test]
    fn round_trip_damage() {
        for dt in DamageType::iter() {
            assert_eq!(DamageType::from_repr(dt as u8), Some(dt));
        }
    }

    #[test]
    fn attack_count() {
        assert_eq!(AttackType::COUNT, 17);
    }

    #[test]
    fn damage_count() {
        assert_eq!(DamageType::COUNT, 48);
    }
}
