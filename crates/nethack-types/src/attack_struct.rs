use serde::Serialize;

use crate::attack::{AttackType, DamageType};

/// A single monster attack, matching C's `struct attack` from `permonst.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Attack {
    pub attack_type: AttackType,
    pub damage_type: DamageType,
    pub dice_num: u8,
    pub dice_sides: u8,
}

impl Attack {
    pub const NONE: Self = Self {
        attack_type: AttackType::None,
        damage_type: DamageType::Physical,
        dice_num: 0,
        dice_sides: 0,
    };

    pub const fn new(
        attack_type: AttackType,
        damage_type: DamageType,
        dice_num: u8,
        dice_sides: u8,
    ) -> Self {
        Self {
            attack_type,
            damage_type,
            dice_num,
            dice_sides,
        }
    }

    pub const fn is_none(&self) -> bool {
        matches!(self.attack_type, AttackType::None)
    }
}

/// Maximum number of attacks per monster (NATTK in C).
pub const MAX_ATTACKS: usize = 6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_attack() {
        assert!(Attack::NONE.is_none());
        assert_eq!(Attack::NONE.dice_num, 0);
    }

    #[test]
    fn new_attack() {
        let atk = Attack::new(AttackType::Claw, DamageType::Physical, 1, 6);
        assert!(!atk.is_none());
        assert_eq!(atk.attack_type, AttackType::Claw);
        assert_eq!(atk.dice_num, 1);
        assert_eq!(atk.dice_sides, 6);
    }
}
