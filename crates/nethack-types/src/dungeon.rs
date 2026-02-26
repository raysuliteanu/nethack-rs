use serde::Serialize;

/// Complete dungeon topology parsed from `dungeon.def`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DungeonTopology {
    pub dungeons: Vec<DungeonDef>,
}

/// A single dungeon definition (e.g. "The Dungeons of Doom").
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DungeonDef {
    pub name: String,
    pub boneschar: String,
    pub base: i16,
    pub rand: i16,
    pub flags: DungeonFlags,
    /// Entry level override: -1 = bottom, -2 = second from bottom.
    pub entry: i16,
    pub protofile: Option<String>,
    pub levels: Vec<LevelDef>,
    pub branches: Vec<BranchDef>,
}

/// A special level within a dungeon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LevelDef {
    pub name: String,
    pub boneschar: String,
    /// For CHAINLEVEL: name of the level this is chained to.
    pub chain: Option<String>,
    pub offset_base: i16,
    pub offset_rand: i16,
    /// Number of random level variants (RNDLEVEL only).
    pub rndlevs: u8,
    /// Percentage chance of level appearing (100 = always).
    pub chance: u8,
    pub flags: DungeonFlags,
}

/// A branch connection between dungeons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BranchDef {
    pub name: String,
    /// For CHAINBRANCH: name of the level this is chained to.
    pub chain: Option<String>,
    pub offset_base: i16,
    pub offset_rand: i16,
    pub branch_type: BranchType,
    pub direction: Option<BranchDirection>,
}

/// Dungeon/level descriptor flags matching C's bitfield in `dgn_file.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct DungeonFlags {
    pub town: bool,
    pub hellish: bool,
    pub maze_like: bool,
    pub rogue_like: bool,
    pub align: DungeonAlignment,
}

/// Dungeon alignment matching C's `D_ALIGN_*` constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum DungeonAlignment {
    #[default]
    Unaligned,
    Lawful,
    Neutral,
    Chaotic,
    Noalign,
}

/// Branch connection type matching C's `TBR_*` constants in `dgn_file.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum BranchType {
    #[default]
    Stair,
    NoUp,
    NoDown,
    Portal,
}

/// Direction a branch goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BranchDirection {
    Up,
    Down,
}
