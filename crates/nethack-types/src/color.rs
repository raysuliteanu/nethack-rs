use serde::Serialize;
use strum::{EnumCount, EnumIter, FromRepr};

/// Display colors matching NetHack's IBM PC color scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, EnumIter, EnumCount, FromRepr)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Red = 1,
    Green = 2,
    Brown = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    Gray = 7,
    NoColor = 8,
    Orange = 9,
    BrightGreen = 10,
    Yellow = 11,
    BrightBlue = 12,
    BrightMagenta = 13,
    BrightCyan = 14,
    White = 15,
}

impl Color {
    pub const MAX: usize = 16;
    pub const BRIGHT: u8 = 8;

    /// Material highlight colors used for object display.
    pub const HI_OBJ: Self = Self::Magenta;
    pub const HI_METAL: Self = Self::Cyan;
    pub const HI_COPPER: Self = Self::Yellow;
    pub const HI_SILVER: Self = Self::Gray;
    pub const HI_GOLD: Self = Self::Yellow;
    pub const HI_LEATHER: Self = Self::Brown;
    pub const HI_CLOTH: Self = Self::Brown;
    pub const HI_ORGANIC: Self = Self::Brown;
    pub const HI_WOOD: Self = Self::Brown;
    pub const HI_PAPER: Self = Self::White;
    pub const HI_GLASS: Self = Self::BrightCyan;
    pub const HI_MINERAL: Self = Self::Gray;
    pub const DRAGON_SILVER: Self = Self::BrightCyan;
    pub const HI_ZAP: Self = Self::BrightBlue;
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn count() {
        assert_eq!(Color::COUNT, 16);
    }

    #[test]
    fn discriminants() {
        assert_eq!(Color::Black as u8, 0);
        assert_eq!(Color::Red as u8, 1);
        assert_eq!(Color::Brown as u8, 3);
        assert_eq!(Color::Gray as u8, 7);
        assert_eq!(Color::NoColor as u8, 8);
        assert_eq!(Color::Orange as u8, 9);
        assert_eq!(Color::White as u8, 15);
    }

    #[test]
    fn round_trip() {
        for c in Color::iter() {
            assert_eq!(Color::from_repr(c as u8), Some(c));
        }
    }
}
