use bitflags::bitflags;
use serde::Serialize;

bitflags! {
    /// Monster flags set 1 from `monflag.h` (M1_* constants).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
    pub struct MonsterFlags1: u32 {
        const FLY          = 0x0000_0001;
        const SWIM         = 0x0000_0002;
        const AMORPHOUS    = 0x0000_0004;
        const WALLWALK     = 0x0000_0008;
        const CLING        = 0x0000_0010;
        const TUNNEL       = 0x0000_0020;
        const NEEDPICK     = 0x0000_0040;
        const CONCEAL      = 0x0000_0080;
        const HIDE         = 0x0000_0100;
        const AMPHIBIOUS   = 0x0000_0200;
        const BREATHLESS   = 0x0000_0400;
        const NOTAKE       = 0x0000_0800;
        const NOEYES       = 0x0000_1000;
        const NOHANDS      = 0x0000_2000;
        const NOLIMBS      = 0x0000_6000;
        const NOHEAD       = 0x0000_8000;
        const MINDLESS     = 0x0001_0000;
        const HUMANOID     = 0x0002_0000;
        const ANIMAL       = 0x0004_0000;
        const SLITHY       = 0x0008_0000;
        const UNSOLID      = 0x0010_0000;
        const THICK_HIDE   = 0x0020_0000;
        const OVIPAROUS    = 0x0040_0000;
        const REGEN        = 0x0080_0000;
        const SEE_INVIS    = 0x0100_0000;
        const TPORT        = 0x0200_0000;
        const TPORT_CNTRL  = 0x0400_0000;
        const ACID         = 0x0800_0000;
        const POIS         = 0x1000_0000;
        const CARNIVORE    = 0x2000_0000;
        const HERBIVORE    = 0x4000_0000;
        const OMNIVORE     = 0x6000_0000;
        const METALLIVORE  = 0x8000_0000;
    }
}

bitflags! {
    /// Monster flags set 2 from `monflag.h` (M2_* constants).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
    pub struct MonsterFlags2: u32 {
        const NOPOLY       = 0x0000_0001;
        const UNDEAD       = 0x0000_0002;
        const WERE         = 0x0000_0004;
        const HUMAN        = 0x0000_0008;
        const ELF          = 0x0000_0010;
        const DWARF        = 0x0000_0020;
        const GNOME        = 0x0000_0040;
        const ORC          = 0x0000_0080;
        const DEMON        = 0x0000_0100;
        const MERC         = 0x0000_0200;
        const LORD         = 0x0000_0400;
        const PRINCE       = 0x0000_0800;
        const MINION       = 0x0000_1000;
        const GIANT        = 0x0000_2000;
        const SHAPESHIFTER = 0x0000_4000;
        // 0x8000 unused
        const MALE         = 0x0001_0000;
        const FEMALE       = 0x0002_0000;
        const NEUTER       = 0x0004_0000;
        const PNAME        = 0x0008_0000;
        const HOSTILE      = 0x0010_0000;
        const PEACEFUL     = 0x0020_0000;
        const DOMESTIC     = 0x0040_0000;
        const WANDER       = 0x0080_0000;
        const STALK        = 0x0100_0000;
        const NASTY        = 0x0200_0000;
        const STRONG       = 0x0400_0000;
        const ROCKTHROW    = 0x0800_0000;
        const GREEDY       = 0x1000_0000;
        const JEWELS       = 0x2000_0000;
        const COLLECT      = 0x4000_0000;
        const MAGIC        = 0x8000_0000;
    }
}

bitflags! {
    /// Monster flags set 3 from `monflag.h` (M3_* constants).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
    pub struct MonsterFlags3: u16 {
        const WANTSAMUL    = 0x0001;
        const WANTSBELL    = 0x0002;
        const WANTSBOOK    = 0x0004;
        const WANTSCAND    = 0x0008;
        const WANTSARTI    = 0x0010;
        const WANTSALL     = 0x001F;
        const WAITFORU     = 0x0040;
        const CLOSE        = 0x0080;
        const COVETOUS     = 0x001F;
        const WAITMASK     = 0x00C0;
        const INFRAVISION  = 0x0100;
        const INFRAVISIBLE = 0x0200;
        const DISPLACES    = 0x0400;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m1_omnivore_is_carnivore_or_herbivore() {
        assert_eq!(
            MonsterFlags1::OMNIVORE,
            MonsterFlags1::CARNIVORE | MonsterFlags1::HERBIVORE
        );
    }

    #[test]
    fn m1_nolimbs_includes_nohands() {
        assert!(MonsterFlags1::NOLIMBS.contains(MonsterFlags1::NOHANDS));
    }

    #[test]
    fn m2_values() {
        assert_eq!(MonsterFlags2::NOPOLY.bits(), 0x0000_0001);
        assert_eq!(MonsterFlags2::MAGIC.bits(), 0x8000_0000);
        assert_eq!(MonsterFlags2::HOSTILE.bits(), 0x0010_0000);
    }

    #[test]
    fn m3_covetous_is_wants_all() {
        assert_eq!(MonsterFlags3::COVETOUS, MonsterFlags3::WANTSALL);
    }

    #[test]
    fn m3_waitmask() {
        assert_eq!(
            MonsterFlags3::WAITMASK,
            MonsterFlags3::WAITFORU | MonsterFlags3::CLOSE
        );
    }
}
