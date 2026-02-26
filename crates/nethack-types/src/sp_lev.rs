use bitflags::bitflags;
use serde::Serialize;
use strum::FromRepr;

/// Opcodes for the special level bytecode interpreter.
/// Values match C's `enum opcode_defs` in `sp_lev.h:60-139`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, FromRepr)]
#[repr(u8)]
pub enum SpOpcode {
    Null = 0,
    Message = 1,
    Monster = 2,
    Object = 3,
    Engraving = 4,
    Room = 5,
    Subroom = 6,
    Door = 7,
    Stair = 8,
    Ladder = 9,
    Altar = 10,
    Fountain = 11,
    Sink = 12,
    Pool = 13,
    Trap = 14,
    Gold = 15,
    Corridor = 16,
    LevRegion = 17,
    Drawbridge = 18,
    MazeWalk = 19,
    NonDiggable = 20,
    NonPasswall = 21,
    Wallify = 22,
    Map = 23,
    RoomDoor = 24,
    Region = 25,
    Mineralize = 26,
    Cmp = 27,
    Jmp = 28,
    Jl = 29,
    Jle = 30,
    Jg = 31,
    Jge = 32,
    Je = 33,
    Jne = 34,
    Terrain = 35,
    ReplaceTerrain = 36,
    Exit = 37,
    EndRoom = 38,
    PopContainer = 39,
    Push = 40,
    Pop = 41,
    Rn2 = 42,
    Dec = 43,
    Inc = 44,
    MathAdd = 45,
    MathSub = 46,
    MathMul = 47,
    MathDiv = 48,
    MathMod = 49,
    MathSign = 50,
    Copy = 51,
    EndMonInvent = 52,
    Grave = 53,
    FramePush = 54,
    FramePop = 55,
    Call = 56,
    Return = 57,
    InitLevel = 58,
    LevelFlags = 59,
    VarInit = 60,
    ShuffleArray = 61,
    Dice = 62,
    SelAdd = 63,
    SelPoint = 64,
    SelRect = 65,
    SelFillRect = 66,
    SelLine = 67,
    SelRndLine = 68,
    SelGrow = 69,
    SelFlood = 70,
    SelRndCoord = 71,
    SelEllipse = 72,
    SelFilter = 73,
    SelGradient = 74,
    SelComplement = 75,
}

/// Typed operand pushed onto the stack with `SPO_PUSH`.
/// Variants match C's `SPOVAR_*` constants in `sp_lev.h:206-221`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SpOperand {
    Int(i64),
    String(String),
    Variable(String),
    Coord {
        x: i16,
        y: i16,
        is_random: bool,
        /// Humidity/location flags for random coords.
        flags: u32,
    },
    Region {
        x1: i16,
        y1: i16,
        x2: i16,
        y2: i16,
    },
    MapChar {
        typ: i16,
        lit: i16,
    },
    Monst {
        class: i16,
        id: i16,
    },
    Obj {
        class: i16,
        id: i16,
    },
    Sel(Vec<u8>),
}

/// A single instruction in the special level bytecode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpLevOpcode {
    pub opcode: SpOpcode,
    pub operand: Option<SpOperand>,
}

bitflags! {
    /// Per-level flags matching C's constants in `sp_lev.h:20-34`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
    pub struct LevelFlags: u32 {
        const NOTELEPORT          = 0x0000_0001;
        const HARDFLOOR           = 0x0000_0002;
        const NOMMAP              = 0x0000_0004;
        const SHORTSIGHTED        = 0x0000_0008;
        const ARBOREAL            = 0x0000_0010;
        const MAZELEVEL           = 0x0000_0020;
        const PREMAPPED           = 0x0000_0040;
        const SHROUD              = 0x0000_0080;
        const GRAVEYARD           = 0x0000_0100;
        const ICEDPOOLS           = 0x0000_0200;
        const SOLIDIFY            = 0x0000_0400;
        const CORRMAZE            = 0x0000_0800;
        const CHECK_INACCESSIBLES = 0x0000_1000;
    }
}

/// Level initialization style matching C's `enum lvlinit_types`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, FromRepr)]
#[repr(u8)]
pub enum LvlInitStyle {
    None = 0,
    SolidFill = 1,
    MazeGrid = 2,
    Mines = 3,
    Rogue = 4,
}

/// Monster modifier flag type matching C's `enum sp_mon_var_flags`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, FromRepr)]
#[repr(u8)]
pub enum SpMonVarFlag {
    Peaceful = 0,
    Align = 1,
    Asleep = 2,
    Appear = 3,
    Name = 4,
    Female = 5,
    Invis = 6,
    Cancelled = 7,
    Revived = 8,
    Avenge = 9,
    Fleeing = 10,
    Blinded = 11,
    Paralyzed = 12,
    Stunned = 13,
    Confused = 14,
    SeenTraps = 15,
    End = 16,
}

/// Object modifier flag type matching C's `enum sp_obj_var_flags`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, FromRepr)]
#[repr(u8)]
pub enum SpObjVarFlag {
    Spe = 0,
    Curse = 1,
    CorpseNm = 2,
    Name = 3,
    Quan = 4,
    Buried = 5,
    Lit = 6,
    Eroded = 7,
    Locked = 8,
    Trapped = 9,
    Recharged = 10,
    Invis = 11,
    Greased = 12,
    Broken = 13,
    Coord = 14,
    End = 15,
}

/// A compiled special level definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecialLevel {
    pub name: String,
    pub opcodes: Vec<SpLevOpcode>,
}

/// A parsed `.des` file containing one or more level definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesFile {
    pub levels: Vec<SpecialLevel>,
}
