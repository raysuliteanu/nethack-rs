use serde::Serialize;

use crate::color::Color;
use crate::material::Material;
use crate::object_class::ObjectClass;

/// An object type definition, matching C's `struct objclass`.
#[derive(Debug, Clone, Serialize)]
pub struct ObjectType {
    pub name: &'static str,
    pub description: Option<&'static str>,
    pub class: ObjectClass,
    pub sub_type: i8,
    pub prob: i16,
    pub delay: i8,
    pub weight: u16,
    pub cost: i16,
    pub damage_small: i8,
    pub damage_large: i8,
    pub oc1: i8,
    pub oc2: i8,
    pub material: Material,
    pub color: Color,
    pub nutrition: u16,
    pub prop: u8,
    pub flags: ObjectTypeFlags,
}

/// Boolean flags for object types, collapsed from the C bitfield members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ObjectTypeFlags {
    pub name_known: bool,
    pub merge: bool,
    pub uses_known: bool,
    pub pre_discovered: bool,
    pub magic: bool,
    pub charged: bool,
    pub unique: bool,
    pub no_wish: bool,
    pub big: bool,
    pub tough: bool,
    pub dir: u8,
}

impl ObjectTypeFlags {
    pub const EMPTY: Self = Self {
        name_known: false,
        merge: false,
        uses_known: false,
        pre_discovered: false,
        magic: false,
        charged: false,
        unique: false,
        no_wish: false,
        big: false,
        tough: false,
        dir: 0,
    };
}

/// Direction constants for wands/spells (oc_dir values).
pub const NODIR: u8 = 1;
pub const IMMEDIATE: u8 = 2;
pub const RAY: u8 = 3;

/// Weapon damage type constants.
pub const WHACK: u8 = 0;
pub const PIERCE: u8 = 1;
pub const SLASH: u8 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_flags() {
        let f = ObjectTypeFlags::EMPTY;
        assert!(!f.magic);
        assert!(!f.unique);
        assert_eq!(f.dir, 0);
    }
}
