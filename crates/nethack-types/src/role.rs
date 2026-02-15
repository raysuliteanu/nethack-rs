use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};

use crate::alignment::Alignment;

/// Player role (class) kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum RoleKind {
    Archeologist = 0,
    Barbarian = 1,
    Caveman = 2,
    Healer = 3,
    Knight = 4,
    Monk = 5,
    Priest = 6,
    Ranger = 7,
    Rogue = 8,
    Samurai = 9,
    Tourist = 10,
    Valkyrie = 11,
    Wizard = 12,
}

/// Player race kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum RaceKind {
    Human = 0,
    Elf = 1,
    Dwarf = 2,
    Gnome = 3,
    Orc = 4,
}

/// Player gender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum Gender {
    Male = 0,
    Female = 1,
    Neuter = 2,
}

/// A name pair for male/female variants.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct RoleName {
    pub male: &'static str,
    pub female: Option<&'static str>,
}

/// Hit point or energy advancement rate.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct RoleAdvance {
    pub init_fixed: i8,
    pub init_random: i8,
    pub low_fixed: i8,
    pub low_random: i8,
    pub high_fixed: i8,
    pub high_random: i8,
}

/// Full role definition, matching C's `struct Role`.
#[derive(Debug, Clone, Serialize)]
pub struct RoleDefinition {
    pub name: RoleName,
    pub ranks: [RoleName; 9],
    pub lgod: &'static str,
    pub ngod: &'static str,
    pub cgod: &'static str,
    pub filecode: &'static str,
    pub homebase: &'static str,
    pub intermed: &'static str,
    pub male_num: Option<u16>,
    pub female_num: Option<u16>,
    pub pet_num: Option<u16>,
    pub leader_num: Option<u16>,
    pub guard_num: Option<u16>,
    pub nemesis_num: Option<u16>,
    pub enemy1_num: Option<u16>,
    pub enemy2_num: Option<u16>,
    pub enemy1_sym: u8,
    pub enemy2_sym: u8,
    pub quest_artifact: u16,
    pub allow_mask: u16,
    pub attr_base: [i8; 6],
    pub attr_dist: [i8; 6],
    pub hp_advance: RoleAdvance,
    pub en_advance: RoleAdvance,
    pub cutoff_level: i8,
    pub init_record: i8,
    pub spell_base: i32,
    pub spell_heal: i32,
    pub spell_shield: i32,
    pub spell_armor: i32,
    pub spell_stat: i32,
    pub spell_spec: i32,
    pub spell_bonus: i32,
}

/// Full race definition, matching C's `struct Race`.
#[derive(Debug, Clone, Serialize)]
pub struct RaceDefinition {
    pub noun: &'static str,
    pub adj: &'static str,
    pub coll: &'static str,
    pub filecode: &'static str,
    pub individual: RoleName,
    pub male_num: Option<u16>,
    pub female_num: Option<u16>,
    pub mummy_num: Option<u16>,
    pub zombie_num: Option<u16>,
    pub allow_mask: u16,
    pub self_mask: u16,
    pub love_mask: u16,
    pub hate_mask: u16,
    pub attr_min: [i8; 6],
    pub attr_max: [i8; 6],
    pub hp_advance: RoleAdvance,
    pub en_advance: RoleAdvance,
}

/// Gender definition, matching C's `struct Gender`.
#[derive(Debug, Clone, Serialize)]
pub struct GenderDefinition {
    pub adj: &'static str,
    pub he: &'static str,
    pub him: &'static str,
    pub his: &'static str,
    pub filecode: &'static str,
    pub allow: u16,
}

/// Alignment definition for character creation.
#[derive(Debug, Clone, Serialize)]
pub struct AlignDefinition {
    pub noun: &'static str,
    pub adj: &'static str,
    pub filecode: &'static str,
    pub allow: u16,
    pub alignment: Alignment,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_count() {
        assert_eq!(RoleKind::COUNT, 13);
    }

    #[test]
    fn race_count() {
        assert_eq!(RaceKind::COUNT, 5);
    }

    #[test]
    fn gender_count() {
        assert_eq!(Gender::COUNT, 3);
    }

    #[test]
    fn role_round_trip() {
        use strum::IntoEnumIterator;
        for r in RoleKind::iter() {
            assert_eq!(RoleKind::from_repr(r as u8), Some(r));
        }
    }
}
