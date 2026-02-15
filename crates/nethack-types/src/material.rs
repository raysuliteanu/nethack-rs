use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};

/// Material types from `objclass.h` (enum obj_material_types).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum Material {
    Liquid = 1,
    Wax = 2,
    Veggy = 3,
    Flesh = 4,
    Paper = 5,
    Cloth = 6,
    Leather = 7,
    Wood = 8,
    Bone = 9,
    DragonHide = 10,
    Iron = 11,
    Metal = 12,
    Copper = 13,
    Silver = 14,
    Gold = 15,
    Platinum = 16,
    Mithril = 17,
    Plastic = 18,
    Glass = 19,
    Gemstone = 20,
    Mineral = 21,
}

impl Material {
    pub fn is_organic(self) -> bool {
        (self as u8) <= Self::Wood as u8
    }

    pub fn is_metallic(self) -> bool {
        (self as u8) >= Self::Iron as u8 && (self as u8) <= Self::Mithril as u8
    }

    pub fn is_rustprone(self) -> bool {
        self == Self::Iron
    }

    pub fn is_corrodeable(self) -> bool {
        self == Self::Copper || self == Self::Iron
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn count() {
        assert_eq!(Material::COUNT, 21);
    }

    #[test]
    fn discriminants() {
        assert_eq!(Material::Liquid as u8, 1);
        assert_eq!(Material::Mineral as u8, 21);
    }

    #[test]
    fn organic() {
        assert!(Material::Wood.is_organic());
        assert!(Material::Flesh.is_organic());
        assert!(!Material::Iron.is_organic());
    }

    #[test]
    fn metallic() {
        assert!(Material::Iron.is_metallic());
        assert!(Material::Silver.is_metallic());
        assert!(Material::Mithril.is_metallic());
        assert!(!Material::Glass.is_metallic());
        assert!(!Material::Wood.is_metallic());
    }

    #[test]
    fn round_trip() {
        for m in Material::iter() {
            assert_eq!(Material::from_repr(m as u8), Some(m));
        }
    }
}
