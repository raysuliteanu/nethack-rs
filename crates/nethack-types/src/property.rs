use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};

/// Player/monster properties from `prop.h` (enum prop_types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum Property {
    // Resistances
    FireRes = 1,
    ColdRes = 2,
    SleepRes = 3,
    DisintRes = 4,
    ShockRes = 5,
    PoisonRes = 6,
    AcidRes = 7,
    StoneRes = 8,
    DrainRes = 9,
    SickRes = 10,
    Invulnerable = 11,
    Antimagic = 12,
    // Troubles
    Stunned = 13,
    Confusion = 14,
    Blinded = 15,
    Deaf = 16,
    Sick = 17,
    Stoned = 18,
    Strangled = 19,
    Vomiting = 20,
    Glib = 21,
    Slimed = 22,
    Halluc = 23,
    HallucRes = 24,
    Fumbling = 25,
    WoundedLegs = 26,
    Sleepy = 27,
    Hunger = 28,
    // Vision and senses
    SeeInvis = 29,
    Telepat = 30,
    Warning = 31,
    WarnOfMon = 32,
    WarnUndead = 33,
    Searching = 34,
    Clairvoyant = 35,
    Infravision = 36,
    DetectMonsters = 37,
    // Appearance and behavior
    Adorned = 38,
    Invis = 39,
    Displaced = 40,
    Stealth = 41,
    AggravateMonster = 42,
    Conflict = 43,
    // Transportation
    Jumping = 44,
    Teleport = 45,
    TeleportControl = 46,
    Levitation = 47,
    Flying = 48,
    WaterWalking = 49,
    Swimming = 50,
    MagicalBreathing = 51,
    PassesWalls = 52,
    // Physical attributes
    SlowDigestion = 53,
    HalfSpellDamage = 54,
    HalfPhysDamage = 55,
    Regeneration = 56,
    EnergyRegeneration = 57,
    Protection = 58,
    ProtFromShapeChangers = 59,
    Polymorph = 60,
    PolymorphControl = 61,
    Unchanging = 62,
    Fast = 63,
    Reflecting = 64,
    FreeAction = 65,
    FixedAbil = 66,
    Lifesaved = 67,
}

impl Property {
    pub const LAST: Self = Self::Lifesaved;
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn count() {
        assert_eq!(Property::COUNT, 67);
    }

    #[test]
    fn discriminants() {
        assert_eq!(Property::FireRes as u8, 1);
        assert_eq!(Property::Lifesaved as u8, 67);
        assert_eq!(Property::Stunned as u8, 13);
        assert_eq!(Property::Fast as u8, 63);
    }

    #[test]
    fn round_trip() {
        for p in Property::iter() {
            assert_eq!(Property::from_repr(p as u8), Some(p));
        }
    }
}
