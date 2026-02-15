use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};

/// Level location types from `rm.h` (enum levl_typ_types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum LocationType {
    Stone = 0,
    VWall = 1,
    HWall = 2,
    TlCorner = 3,
    TrCorner = 4,
    BlCorner = 5,
    BrCorner = 6,
    CrossWall = 7,
    TuWall = 8,
    TdWall = 9,
    TlWall = 10,
    TrWall = 11,
    DbWall = 12,
    Tree = 13,
    SDoor = 14,
    SCorr = 15,
    Pool = 16,
    Moat = 17,
    Water = 18,
    DrawbridgeUp = 19,
    LavaPool = 20,
    IronBars = 21,
    Door = 22,
    Corr = 23,
    Room = 24,
    Stairs = 25,
    Ladder = 26,
    Fountain = 27,
    Throne = 28,
    Sink = 29,
    Grave = 30,
    Altar = 31,
    Ice = 32,
    DrawbridgeDown = 33,
    Air = 34,
    Cloud = 35,
}

impl LocationType {
    pub const MAX: usize = 36;

    pub const fn is_wall(self) -> bool {
        (self as u8) >= 1 && (self as u8) <= Self::DbWall as u8
    }

    pub const fn is_stwall(self) -> bool {
        (self as u8) <= Self::DbWall as u8
    }

    pub const fn is_rock(self) -> bool {
        (self as u8) < Self::Pool as u8
    }

    pub const fn is_door(self) -> bool {
        matches!(self, Self::Door)
    }

    pub const fn is_accessible(self) -> bool {
        (self as u8) >= Self::Door as u8
    }

    pub const fn is_room(self) -> bool {
        (self as u8) >= Self::Room as u8
    }

    pub const fn is_pool(self) -> bool {
        (self as u8) >= Self::Pool as u8 && (self as u8) <= Self::DrawbridgeUp as u8
    }

    pub const fn is_furniture(self) -> bool {
        (self as u8) >= Self::Stairs as u8 && (self as u8) <= Self::Altar as u8
    }

    pub const fn is_air(self) -> bool {
        matches!(self, Self::Air | Self::Cloud)
    }

    pub const fn is_drawbridge(self) -> bool {
        matches!(self, Self::DrawbridgeUp | Self::DrawbridgeDown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn count() {
        assert_eq!(LocationType::COUNT, 36);
    }

    #[test]
    fn discriminants() {
        assert_eq!(LocationType::Stone as u8, 0);
        assert_eq!(LocationType::Door as u8, 22);
        assert_eq!(LocationType::Cloud as u8, 35);
    }

    #[test]
    fn wall_classification() {
        assert!(LocationType::VWall.is_wall());
        assert!(LocationType::DbWall.is_wall());
        assert!(!LocationType::Stone.is_wall());
        assert!(!LocationType::Door.is_wall());
    }

    #[test]
    fn pool_classification() {
        assert!(LocationType::Pool.is_pool());
        assert!(LocationType::Moat.is_pool());
        assert!(LocationType::DrawbridgeUp.is_pool());
        assert!(!LocationType::LavaPool.is_pool());
    }

    #[test]
    fn furniture_classification() {
        assert!(LocationType::Stairs.is_furniture());
        assert!(LocationType::Fountain.is_furniture());
        assert!(LocationType::Altar.is_furniture());
        assert!(!LocationType::Room.is_furniture());
    }

    #[test]
    fn round_trip() {
        for lt in LocationType::iter() {
            assert_eq!(LocationType::from_repr(lt as u8), Some(lt));
        }
    }
}
