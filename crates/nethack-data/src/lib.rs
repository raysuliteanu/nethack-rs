pub mod des_lexer;
pub mod des_parser;
pub mod dungeon_parser;
pub mod lev_reader;
pub mod monsters;
pub mod objects;

#[cfg(test)]
mod tests {
    use nethack_types::*;

    use crate::monsters::MONSTERS;
    use crate::objects::OBJECTS;

    #[test]
    fn monster_count() {
        assert_eq!(MONSTERS.len(), 382);
    }

    #[test]
    fn object_count() {
        assert_eq!(OBJECTS.len(), 454);
    }

    #[test]
    fn monster_id_maps_to_valid_index() {
        use strum::IntoEnumIterator;
        for id in MonsterId::iter() {
            let idx = id as usize;
            assert!(
                idx < MONSTERS.len(),
                "MonsterId::{:?} index {} out of bounds",
                id,
                idx
            );
        }
    }

    #[test]
    fn object_id_maps_to_valid_index() {
        use strum::IntoEnumIterator;
        for id in ObjectId::iter() {
            let idx = id as usize;
            assert!(
                idx < OBJECTS.len(),
                "ObjectId::{:?} index {} out of bounds",
                id,
                idx
            );
        }
    }

    #[test]
    fn no_empty_monster_names() {
        for (i, m) in MONSTERS.iter().enumerate() {
            assert!(!m.name.is_empty(), "monster at index {} has empty name", i);
        }
    }

    #[test]
    fn no_empty_object_names() {
        // Some objects (extra scroll/wand descriptions) have no real name
        // but the first object (strange object) should have a name
        assert_eq!(OBJECTS[0].name, "strange object");
    }

    #[test]
    fn giant_ant_spot_check() {
        let ant = &MONSTERS[MonsterId::GiantAnt as usize];
        assert_eq!(ant.name, "giant ant");
        assert_eq!(ant.symbol, 'a');
        assert_eq!(ant.level, 2);
        assert_eq!(ant.move_speed, 18);
        assert_eq!(ant.ac, 3);
        assert_eq!(ant.magic_resistance, 0);
        assert_eq!(ant.alignment, Alignment::Neutral);
        assert_eq!(ant.corpse_weight, 10);
        assert_eq!(ant.nutrition, 10);
        assert_eq!(ant.sound, MonsterSound::Silent);
        assert_eq!(ant.size, MonsterSize::Tiny);
        assert_eq!(ant.difficulty, 4);
        assert_eq!(ant.color, Color::Brown);
        // First attack: bite for 1d4
        assert_eq!(ant.attacks[0].attack_type, AttackType::Bite);
        assert_eq!(ant.attacks[0].damage_type, DamageType::Physical);
        assert_eq!(ant.attacks[0].dice_num, 1);
        assert_eq!(ant.attacks[0].dice_sides, 4);
    }

    #[test]
    fn arrow_spot_check() {
        let arrow = &OBJECTS[ObjectId::Arrow as usize];
        assert_eq!(arrow.name, "arrow");
        assert_eq!(arrow.class, ObjectClass::Weapon);
        assert_eq!(arrow.prob, 55);
        assert_eq!(arrow.weight, 1);
        assert_eq!(arrow.cost, 2);
        assert_eq!(arrow.damage_small, 6);
        assert_eq!(arrow.damage_large, 6);
        assert_eq!(arrow.material, Material::Iron);
    }

    #[test]
    fn long_sword_spot_check() {
        let ls = &OBJECTS[ObjectId::LongSword as usize];
        assert_eq!(ls.name, "long sword");
        assert_eq!(ls.class, ObjectClass::Weapon);
        assert_eq!(ls.sub_type, 7); // P_LONG_SWORD
        assert_eq!(ls.prob, 50);
        assert_eq!(ls.weight, 40);
        assert_eq!(ls.cost, 15);
        assert_eq!(ls.damage_small, 8);
        assert_eq!(ls.damage_large, 12);
        assert_eq!(ls.material, Material::Iron);
        assert_eq!(ls.color, Color::Cyan); // HI_METAL
    }

    #[test]
    fn plate_mail_spot_check() {
        let pm = &OBJECTS[ObjectId::PlateMail as usize];
        assert_eq!(pm.name, "plate mail");
        assert_eq!(pm.class, ObjectClass::Armor);
        assert_eq!(pm.prob, 44);
        assert_eq!(pm.weight, 450);
        assert_eq!(pm.cost, 600);
        assert_eq!(pm.material, Material::Iron);
    }

    #[test]
    fn wizard_of_yendor_spot_check() {
        let wiz = &MONSTERS[MonsterId::WizardOfYendor as usize];
        assert_eq!(wiz.name, "Wizard of Yendor");
        assert_eq!(wiz.symbol, '@');
        assert_eq!(wiz.level, 30);
        assert_eq!(wiz.magic_resistance, 100);
        // First attack: claw for 2d12 steal amulet
        assert_eq!(wiz.attacks[0].attack_type, AttackType::Claw);
        assert_eq!(wiz.attacks[0].damage_type, DamageType::StealAmulet);
        assert_eq!(wiz.attacks[0].dice_num, 2);
        assert_eq!(wiz.attacks[0].dice_sides, 12);
    }

    #[test]
    fn succubus_has_seduction_attack() {
        let succ = &MONSTERS[MonsterId::Succubus as usize];
        assert_eq!(succ.name, "succubus");
        assert_eq!(succ.attacks[0].attack_type, AttackType::Bite);
        assert_eq!(succ.attacks[0].damage_type, DamageType::SuccubusSeduction);
    }

    #[test]
    fn human_werewolf_distinct_from_animal() {
        let animal = &MONSTERS[MonsterId::Werewolf as usize];
        let human = &MONSTERS[MonsterId::HumanWerewolf as usize];
        assert_eq!(animal.name, "werewolf");
        assert_eq!(human.name, "werewolf");
        assert_eq!(animal.symbol, 'd'); // S_DOG
        assert_eq!(human.symbol, '@'); // S_HUMAN
    }
}
