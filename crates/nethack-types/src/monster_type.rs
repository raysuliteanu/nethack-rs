use serde::Serialize;

use crate::alignment::Alignment;
use crate::attack_struct::{Attack, MAX_ATTACKS};
use crate::color::Color;
use crate::geno::GenoFlags;
use crate::monster_flags::{MonsterFlags1, MonsterFlags2, MonsterFlags3};
use crate::monster_size::MonsterSize;
use crate::monster_sound::MonsterSound;
use crate::resistance::Resistance;

/// A monster species definition, matching C's `struct permonst`.
#[derive(Debug, Clone, Serialize)]
pub struct MonsterType {
    pub name: &'static str,
    pub symbol: char,
    pub level: i8,
    pub move_speed: i8,
    pub ac: i8,
    pub magic_resistance: i8,
    pub alignment: Alignment,
    pub geno: GenoFlags,
    pub attacks: [Attack; MAX_ATTACKS],
    pub corpse_weight: u16,
    pub nutrition: u16,
    pub sound: MonsterSound,
    pub size: MonsterSize,
    pub resistances: Resistance,
    pub conveys: Resistance,
    pub flags1: MonsterFlags1,
    pub flags2: MonsterFlags2,
    pub flags3: MonsterFlags3,
    pub difficulty: u8,
    pub color: Color,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_check() {
        // Ensure the struct can be constructed
        let _mon = MonsterType {
            name: "test monster",
            symbol: 'T',
            level: 5,
            move_speed: 12,
            ac: 5,
            magic_resistance: 0,
            alignment: Alignment::Neutral,
            geno: GenoFlags::from_bits_truncate(0x0021),
            attacks: [Attack::NONE; MAX_ATTACKS],
            corpse_weight: 400,
            nutrition: 400,
            sound: MonsterSound::Silent,
            size: MonsterSize::Medium,
            resistances: Resistance::empty(),
            conveys: Resistance::empty(),
            flags1: MonsterFlags1::HUMANOID,
            flags2: MonsterFlags2::empty(),
            flags3: MonsterFlags3::empty(),
            difficulty: 5,
            color: Color::Brown,
        };
    }
}
