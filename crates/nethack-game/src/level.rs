//! Level map representation.
//!
//! A single dungeon level is an 80×21 grid of [`MapCell`]s plus metadata
//! about rooms, level flags, and the special level provenance. This module
//! defines the types; level *generation* lives in sibling modules.

use bitflags::bitflags;
use nethack_types::LocationType;
use serde::Serialize;

/// Map width (columns), matching C's `COLNO` in `global.h`.
pub const COLNO: usize = 80;
/// Map height (rows), matching C's `ROWNO` in `global.h`.
pub const ROWNO: usize = 21;

/// Maximum number of rooms per level, matching C's `MAXNROFROOMS`.
pub const MAX_ROOMS: usize = 40;
/// Maximum doors per level, matching C's `DOORMAX`.
pub const MAX_DOORS: usize = 120;

/// A single map cell, corresponding to C's `struct rm` in `rm.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MapCell {
    /// What the hero thinks is here (display glyph index).
    pub glyph: i32,
    /// Actual terrain type.
    pub typ: LocationType,
    /// Seen-from vector: 8 bits, one per octant.
    pub seenv: u8,
    /// Extra type-specific info (door state, wall mode, etc.) — 5 bits in C.
    pub flags: u8,
    /// Is this wall/door horizontal?
    pub horizontal: bool,
    /// Is the room lit?
    pub lit: bool,
    /// Was the room lit before darkness?
    pub waslit: bool,
    /// Room number (0-based) or `NO_ROOM`.
    pub roomno: u8,
    /// Boundary marker for special rooms.
    pub edge: bool,
    /// Exception flag: was a trapdoor here?
    pub candig: bool,
}

/// Sentinel value for "no room" in `MapCell::roomno`.
pub const NO_ROOM: u8 = 0x3F; // 6-bit max, matching C

impl Default for MapCell {
    fn default() -> Self {
        Self {
            glyph: 0,
            typ: LocationType::Stone,
            seenv: 0,
            flags: 0,
            horizontal: false,
            lit: false,
            waslit: false,
            roomno: NO_ROOM,
            edge: false,
            candig: false,
        }
    }
}

/// Room type constants matching C's `mkroom.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[repr(u8)]
#[derive(Default)]
pub enum RoomType {
    #[default]
    Ordinary = 0,
    Court = 2,
    Swamp = 3,
    Vault = 4,
    Beehive = 5,
    Morgue = 6,
    Barracks = 7,
    Zoo = 8,
    Delphi = 9,
    Temple = 10,
    LeprechaunHall = 11,
    CockatriceNest = 12,
    AntHole = 13,
    ShopBase = 14,
}

/// A room on the current level, matching C's `struct mkroom`.
#[derive(Debug, Clone, Serialize)]
pub struct Room {
    /// Left x bound.
    pub lx: u8,
    /// Right x bound.
    pub hx: u8,
    /// Top y bound.
    pub ly: u8,
    /// Bottom y bound.
    pub hy: u8,
    /// Room type.
    pub rtype: RoomType,
    /// Original room type (before any modifications).
    pub orig_rtype: RoomType,
    /// Is the room lit?
    pub rlit: bool,
    /// sp_lev: needs filling?
    pub needfill: bool,
    /// sp_lev: needs joining to corridors?
    pub needjoining: bool,
    /// Number of doors.
    pub doorct: u8,
    /// Index of first door in the level's door table.
    pub fdoor: u8,
    /// Is this room non-rectangular?
    pub irregular: bool,
}

impl Room {
    /// Width of the room interior (inclusive).
    pub fn width(&self) -> u8 {
        self.hx - self.lx + 1
    }

    /// Height of the room interior (inclusive).
    pub fn height(&self) -> u8 {
        self.hy - self.ly + 1
    }
}

bitflags! {
    /// Runtime level flags matching C's `struct levelflags`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct LevelStateFlags: u32 {
        const HAS_SHOP     = 1 << 0;
        const HAS_VAULT    = 1 << 1;
        const HAS_ZOO      = 1 << 2;
        const HAS_COURT    = 1 << 3;
        const HAS_MORGUE   = 1 << 4;
        const HAS_BEEHIVE  = 1 << 5;
        const HAS_BARRACKS = 1 << 6;
        const HAS_TEMPLE   = 1 << 7;
        const HAS_SWAMP    = 1 << 8;
        const NOTELEPORT   = 1 << 9;
        const HARDFLOOR    = 1 << 10;
        const NOMMAP       = 1 << 11;
        const HERO_MEMORY  = 1 << 12;
        const SHORTSIGHTED = 1 << 13;
        const GRAVEYARD    = 1 << 14;
        const SOKOBAN      = 1 << 15;
        const IS_MAZE      = 1 << 16;
        const IS_CAVERNOUS = 1 << 17;
        const ARBOREAL     = 1 << 18;
        const WIZARD_BONES = 1 << 19;
        const CORRMAZE     = 1 << 20;
    }
}

/// A complete dungeon level, corresponding to C's `dlevel_t`.
///
/// The map is stored column-major (`locations[x][y]`) to match C's
/// `levl[COLNO][ROWNO]` access pattern.
#[derive(Debug, Clone)]
pub struct Level {
    /// The map grid, indexed as `locations[x][y]`.
    pub locations: Box<[[MapCell; ROWNO]; COLNO]>,
    /// Rooms on this level.
    pub rooms: Vec<Room>,
    /// Runtime level flags.
    pub flags: LevelStateFlags,
    /// Fountain count.
    pub n_fountains: u8,
    /// Sink count.
    pub n_sinks: u8,
}

impl Level {
    /// Create a blank level filled with stone.
    pub fn new() -> Self {
        Self {
            locations: Box::new([[MapCell::default(); ROWNO]; COLNO]),
            rooms: Vec::new(),
            flags: LevelStateFlags::HERO_MEMORY,
            n_fountains: 0,
            n_sinks: 0,
        }
    }

    /// Get a reference to the cell at (x, y).
    pub fn at(&self, x: usize, y: usize) -> &MapCell {
        &self.locations[x][y]
    }

    /// Get a mutable reference to the cell at (x, y).
    pub fn at_mut(&mut self, x: usize, y: usize) -> &mut MapCell {
        &mut self.locations[x][y]
    }

    /// Fill a rectangular region with a terrain type.
    pub fn fill_rect(&mut self, x1: usize, y1: usize, x2: usize, y2: usize, typ: LocationType) {
        for x in x1..=x2.min(COLNO - 1) {
            for y in y1..=y2.min(ROWNO - 1) {
                self.locations[x][y].typ = typ;
            }
        }
    }
}

impl Default for Level {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_dimensions() {
        assert_eq!(COLNO, 80);
        assert_eq!(ROWNO, 21);
    }

    #[test]
    fn blank_level_is_all_stone() {
        let level = Level::new();
        for x in 0..COLNO {
            for y in 0..ROWNO {
                assert_eq!(level.at(x, y).typ, LocationType::Stone);
            }
        }
    }

    #[test]
    fn fill_rect_works() {
        let mut level = Level::new();
        level.fill_rect(5, 3, 10, 7, LocationType::Room);
        assert_eq!(level.at(5, 3).typ, LocationType::Room);
        assert_eq!(level.at(10, 7).typ, LocationType::Room);
        assert_eq!(level.at(4, 3).typ, LocationType::Stone);
        assert_eq!(level.at(11, 7).typ, LocationType::Stone);
    }

    #[test]
    fn default_level_has_hero_memory() {
        let level = Level::new();
        assert!(level.flags.contains(LevelStateFlags::HERO_MEMORY));
    }
}
